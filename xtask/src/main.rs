use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::Parser;

#[derive(Debug, Parser)]
#[remain::sorted]
#[command(about, author, disable_help_subcommand = true, propagate_version = true, version)]
pub enum Cmd {
    Build {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    MiriTest {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    Pprof {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
}

#[remain::check]
fn main() -> Result<()> {
    let cmd = Cmd::parse();
    #[remain::sorted]
    match cmd {
        Cmd::Build { args } => {
            // wasm-pack
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/lox-wasm");
            run_cmd(
                Command::new("wasm-pack")
                    .args(&["build", "--out-dir=pkg", "--release", "--target=web"])
                    .current_dir(path),
            )?;

            // npm
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/lox-playground/ui/");
            run_cmd(Command::new("npm").arg("ci").current_dir(&path))?;
            run_cmd(Command::new("npm").args(&["run", "build"]).current_dir(path))?;

            // cargo
            run_cmd(Command::new("cargo").arg("build").args(args))?;
        }
        Cmd::MiriTest { args } => run_cmd(
            Command::new("cargo")
                .args(&[
                    "+nightly",
                    "miri",
                    "nextest",
                    "run",
                    "--no-default-features",
                    "--no-fail-fast",
                    "--package=lox-vm",
                ])
                .args(args)
                .envs([("RUST_BACKTRACE", "1"), ("MIRIFLAGS", "-Zmiri-disable-isolation")]),
        )?,
        Cmd::Pprof { args } => run_cmd(
            Command::new("cargo")
                .args(&["run", "--features=pprof", "--no-default-features", "--profile=pprof"])
                .args(args),
        )?,
    }
    Ok(())
}

fn run_cmd(cmd: &mut Command) -> Result<()> {
    print!(">>> {:?}", cmd.get_program());
    for arg in cmd.get_args() {
        print!(" {:?}", arg);
    }
    println!();

    let status = cmd
        .status()
        .with_context(|| format!("command {:?} exited with an error", cmd.get_program()))?;
    if !status.success() {
        bail!("command {:?} exited with exit code: {:?}", cmd.get_program(), status.code());
    }

    Ok(())
}

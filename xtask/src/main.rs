use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(about, author, disable_help_subcommand = true, propagate_version = true, version)]
pub enum Cmd {
    /// Compile TypeScript/WASM modules
    Codegen,
    /// Build a release binary
    Build {
        #[clap(long)]
        native: bool,
        #[clap(long)]
        pgo: bool,
    },
    /// Run tests using Miri
    Test {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cmd = Cmd::parse();
    match cmd {
        Cmd::Build { native, pgo } => {
            run_codegen()?;

            // Build optimized binary.
            let mut rustflags = Vec::new();
            if pgo {
                let merged_path = run_pgo(native)?;
                rustflags.push(format!(
                    "-Cprofile-use={}",
                    merged_path.to_str().expect("path contained invalid UTF-8")
                ));
            }
            if native {
                rustflags.push("-Ctarget-cpu=native".to_string());
            }

            run_cmd(
                Command::new("cargo")
                    .args(["build", "--release"])
                    .env("CARGO_ENCODED_RUSTFLAGS", rustflags.join("\x1f")),
            )?;
        }
        Cmd::Codegen => run_codegen()?,
        Cmd::Test { args } => run_cmd(
            Command::new("cargo")
                .args(["+nightly", "miri", "nextest", "run", "--no-fail-fast"])
                .args(args)
                .env("MIRIFLAGS", "-Zmiri-disable-isolation"),
        )?,
    }
    Ok(())
}

fn run_codegen() -> Result<()> {
    // wasm-pack
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/lox-wasm");
    run_cmd(
        Command::new("wasm-pack")
            .args(["build", "--out-dir=pkg/", "--release", "--target=web"])
            .current_dir(path),
    )?;

    // npm
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/lox-playground/ui/");
    run_cmd(Command::new("npm").arg("ci").current_dir(&path))?;
    run_cmd(Command::new("npm").args(["run", "build"]).current_dir(path))?;

    Ok(())
}

fn run_pgo(native: bool) -> Result<PathBuf> {
    // Clean existing profiling data.
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../pgo/");
    match fs::remove_dir_all(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => Err(e).expect("could not remove pgo directory"),
    }

    // Collect profiles for each benchmark.
    let mut rustflags = vec![format!(
        "-Cprofile-generate={}",
        path.to_str().expect("path contained invalid UTF-8")
    )];
    if native {
        rustflags.push("-Ctarget-cpu=native".to_string());
    }
    let benchmarks = Path::new(env!("CARGO_MANIFEST_DIR")).join("../benchmarks/");
    for benchmark in fs::read_dir(benchmarks)? {
        run_cmd(
            Command::new("cargo")
                .args(["run", "--release"])
                .args(["--", "run"])
                .arg(benchmark?.path())
                .env("CARGO_ENCODED_RUSTFLAGS", rustflags.join("\x1f")),
        )?;
    }

    // Merge collected profiles.
    let merged_path = path.join("merged.profdata");
    run_cmd(
        Command::new("cargo").args(["profdata", "--", "merge", "-o"]).args([&merged_path, &path]),
    )?;

    Ok(merged_path)
}

fn run_cmd(cmd: &mut Command) -> Result<()> {
    print!(">>>");
    for (key, value) in cmd.get_envs() {
        if let Some(value) = value {
            print!(" {key:?}={value:?}");
        }
    }
    print!(" {:?}", cmd.get_program());
    for arg in cmd.get_args() {
        print!(" {arg:?}");
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

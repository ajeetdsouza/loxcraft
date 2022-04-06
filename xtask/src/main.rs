use anyhow::{anyhow, Context, Result};
use clap::Parser;

use std::fs;
use std::path::PathBuf;

#[derive(Debug, Parser)]
enum App {
    Codegen,
}

fn main() -> Result<()> {
    let app = App::parse();
    match app {
        App::Codegen => run_codegen(),
    }
}

fn run_codegen() -> Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest_dir = manifest_dir.parent().context("could not find workspace root")?;

    let build_opts = wasm_pack::command::build::BuildOptions {
        path: Some(manifest_dir.join("crates/lox-wasm")),
        disable_dts: true,
        target: wasm_pack::command::build::Target::NoModules,
        release: true,
        out_dir: "pkg".to_string(),
        out_name: Some("lox".to_string()),
        ..Default::default()
    };
    wasm_pack::command::build::Build::try_from_opts(build_opts)
        .map_err(|e| anyhow!(e.to_string()))?
        .run()
        .map_err(|e| anyhow!(e.to_string()))?;

    fs::copy(
        manifest_dir.join("crates/lox-wasm/pkg/lox_bg.wasm"),
        manifest_dir.join("crates/lox-playground/res/lox_bg.wasm"),
    )?;
    fs::copy(
        manifest_dir.join("crates/lox-wasm/pkg/lox.js"),
        manifest_dir.join("crates/lox-playground/res/lox.js"),
    )?;

    Ok(())
}

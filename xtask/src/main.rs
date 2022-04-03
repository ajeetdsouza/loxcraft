use anyhow::{anyhow, Context, Result};
use clap::Parser;

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
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("could not find workspace root")?
        .join("crates/lox-wasm");
    let build_opts = wasm_pack::command::build::BuildOptions {
        path: Some(dir),
        disable_dts: true,
        target: wasm_pack::command::build::Target::Web,
        release: true,
        out_dir: "pkg".to_string(),
        out_name: Some("lox".to_string()),
        ..Default::default()
    };
    wasm_pack::command::build::Build::try_from_opts(build_opts)
        .map_err(|e| anyhow!(e.to_string()))?
        .run()
        .map_err(|e| anyhow!(e.to_string()))
}

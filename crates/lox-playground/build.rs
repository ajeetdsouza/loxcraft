use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=ui/lox-wasm/src/");
    println!("cargo:rerun-if-changed=ui/lox-wasm/Cargo.toml");
    println!("cargo:rerun-if-changed=ui/lox-wasm/Cargo.lock");
    println!("cargo:rerun-if-changed=ui/src/");
    println!("cargo:rerun-if-changed=ui/package-lock.json");
    println!("cargo:rerun-if-changed=ui/package.json");

    let ui_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
    if let Err(e) = fs::remove_dir_all(ui_dir.join("dist")) {
        if e.kind() != io::ErrorKind::NotFound {
            panic!("unable to remove directory: ui/dist/")
        }
    };
    if !Command::new("npm")
        .arg("ci")
        .current_dir(&ui_dir)
        .status()
        .map_or(false, |status| status.success())
    {
        panic!("`npm ci` exited with an error");
    }
    if !Command::new("npm")
        .args(&["run", "build"])
        .current_dir(&ui_dir)
        .status()
        .map_or(false, |status| status.success())
    {
        panic!("`npm run build` exited with an error");
    }
}

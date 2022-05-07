use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

fn main() {
    let ui_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
    if let Err(e) = fs::remove_dir_all(ui_dir.join("dist")) {
        if e.kind() != io::ErrorKind::NotFound {
            panic!("unable to remove directory: ui/dist/")
        }
    };
    if !Command::new("npm")
        .arg("install")
        .current_dir(&ui_dir)
        .status()
        .map_or(false, |status| status.success())
    {
        panic!("`npm install` exited with an error");
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

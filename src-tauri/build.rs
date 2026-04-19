use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=tauri.conf.json");

    let icons_dir = Path::new("icons");
    if let Ok(entries) = fs::read_dir(icons_dir) {
        for entry in entries.flatten() {
            println!("cargo:rerun-if-changed={}", entry.path().display());
        }
    }

    tauri_build::build()
}

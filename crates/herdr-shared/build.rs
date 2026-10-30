//! Embeds the program icons (defaults/icons/*) so Herdr can install them.
//! Paths stay relative to this package, as the app's build script keeps them.

use std::path::Path;

fn main() {
    let icons = "../../defaults/icons";
    println!("cargo:rerun-if-changed={icons}");
    let base = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(icons);
    let mut names: Vec<String> = std::fs::read_dir(&base).unwrap().flatten().filter(|e| e.path().is_file()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    let lines: Vec<String> = names
        .iter()
        .map(|n| {
            println!("cargo:rerun-if-changed={icons}/{n}");
            format!("    ({n:?}, include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{icons}/{n}\"))),")
        })
        .collect();
    let code = format!("pub const DEFAULT_ICONS: &[(&str, &[u8])] = &[\n{}\n];\n", lines.join("\n"));
    std::fs::write(Path::new(&std::env::var("OUT_DIR").unwrap()).join("default_icons.rs"), code).unwrap();
}

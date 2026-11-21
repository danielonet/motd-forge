//! Embeds every themes/*.toml into the binary (no files needed at runtime).
use std::{env, fs, path::Path};

fn main() {
    let dir = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("themes");
    println!("cargo:rerun-if-changed=themes");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .expect("themes dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect();
    names.sort();
    let mut out = String::from("pub static EMBEDDED: &[(&str, &str)] = &[\n");
    for n in &names {
        out.push_str(&format!(
            "    ({n:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/themes/{n}.toml\"))),\n"
        ));
    }
    out.push_str("];\n");
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("themes_gen.rs"),
        out,
    )
    .unwrap();
}

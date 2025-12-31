use std::path::Path;

fn main() {
    // Only regenerate bindings if WIT files change
    let wit_dir = Path::new("src/wit");
    println!("cargo::rerun-if-changed={}", wit_dir.display());

    // Ensure the bindings module is generated
    println!("cargo:rustc-env=GROMNIE_SCRIPTING_BINDINGS_GENERATED=1");
}

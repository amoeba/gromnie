use anyhow::Result;
use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build scripts
    Scripts {
        #[command(subcommand)]
        command: ScriptCommands,
    },
    /// Build gromnie-web wasm artifacts
    Web {
        #[command(subcommand)]
        command: WebCommands,
    },
}

#[derive(Subcommand)]
enum ScriptCommands {
    /// Build all scripts
    Build,
    /// Install built scripts to ~/.config/gromnie/scripts
    Install,
}

#[derive(Subcommand)]
enum WebCommands {
    /// Build and package gromnie-web for browser usage
    Build,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scripts { command } => match command {
            ScriptCommands::Build => build_scripts()?,
            ScriptCommands::Install => install_scripts()?,
        },
        Commands::Web { command } => match command {
            WebCommands::Build => build_web()?,
        },
    }

    Ok(())
}

fn project_root() -> Result<PathBuf> {
    match env::var("CARGO_MANIFEST_DIR") {
        Ok(manifest_dir) => Ok(std::path::PathBuf::from(manifest_dir)
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Could not determine project root"))?
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Could not determine project root"))?
            .to_path_buf()),
        Err(_) => Ok(env::current_dir()?),
    }
}

fn ensure_rust_target(target: &str) -> Result<()> {
    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()?;
    let installed_targets = String::from_utf8(output.stdout)?;

    if installed_targets.lines().any(|line| line.trim() == target) {
        println!("✓ Rust target installed: {target}");
        return Ok(());
    }

    println!("Installing Rust target: {target}");
    let status = Command::new("rustup")
        .args(["target", "add", target])
        .status()?;

    if status.success() {
        println!("✓ Installed Rust target: {target}");
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to install Rust target: {target}"))
    }
}

fn ensure_wasm_bindgen() -> Result<()> {
    match Command::new("wasm-bindgen").arg("--version").status() {
        Ok(status) if status.success() => {
            println!("✓ wasm-bindgen CLI detected");
            Ok(())
        }
        Ok(status) => Err(anyhow::anyhow!(
            "wasm-bindgen CLI exists but failed to run (status: {status}). Try reinstalling with: cargo install wasm-bindgen-cli"
        )),
        Err(_) => Err(anyhow::anyhow!(
            "wasm-bindgen CLI is not installed. Install with: cargo install wasm-bindgen-cli"
        )),
    }
}

fn build_web() -> Result<()> {
    println!("Building gromnie-web wasm package...\n");

    ensure_rust_target("wasm32-unknown-unknown")?;
    ensure_wasm_bindgen()?;

    let project_root = project_root()?;
    let web_crate_dir = project_root.join("crates/gromnie-web");
    let out_dir = web_crate_dir.join("pkg");
    let wasm_input = project_root.join("target/wasm32-unknown-unknown/release/gromnie_web.wasm");
    let out_name = "gromnie_web";

    println!("Running cargo build for gromnie-web...");
    let build_status = Command::new("cargo")
        .args([
            "build",
            "-p",
            "gromnie-web",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
        ])
        .current_dir(&project_root)
        .status()?;
    if !build_status.success() {
        return Err(anyhow::anyhow!("cargo build failed for gromnie-web"));
    }

    if !wasm_input.exists() {
        return Err(anyhow::anyhow!(
            "Expected wasm input not found: {}",
            wasm_input.display()
        ));
    }

    fs::create_dir_all(&out_dir)?;

    println!("Running wasm-bindgen...");
    let bindgen_status = Command::new("wasm-bindgen")
        .arg(&wasm_input)
        .args([
            "--target",
            "web",
            "--out-dir",
            out_dir
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Output path contains invalid UTF-8"))?,
            "--out-name",
            out_name,
        ])
        .status()?;
    if !bindgen_status.success() {
        return Err(anyhow::anyhow!("wasm-bindgen failed"));
    }

    // Generate auto-initializing wrapper module and update package.json
    generate_wrapper_files(&out_dir, out_name)?;

    let js_path = out_dir.join(format!("{out_name}.js"));
    let wasm_path = out_dir.join(format!("{out_name}_bg.wasm"));
    let dts_path = out_dir.join(format!("{out_name}.d.ts"));
    let index_path = out_dir.join("index.mjs");
    let index_dts_path = out_dir.join("index.d.ts");

    println!("\nBuild complete.");
    println!("Artifacts:");
    println!("  {}", index_path.display());
    println!("  {}", js_path.display());
    println!("  {}", wasm_path.display());
    if dts_path.exists() {
        println!("  {}", dts_path.display());
    }
    if index_dts_path.exists() {
        println!("  {}", index_dts_path.display());
    }

    Ok(())
}

/// Write the auto-initializing wrapper module (`index.mjs`), its type
/// declarations (`index.d.ts`), and update `package.json` to point at them.
fn generate_wrapper_files(out_dir: &std::path::Path, out_name: &str) -> Result<()> {
    // index.mjs — auto-initializes the WASM module on import
    let index_mjs = format!(
        "// Auto-initializing wrapper for gromnie-web.\n\
         //\n\
         // Importing from this module automatically initializes the WASM module,\n\
         // so consumers don't need to call `init()` separately:\n\
         //\n\
         //   import {{ GromnieClient }} from \"gromnie-web\";\n\
         //   const client = new GromnieClient(\"wss://example.com/wisp/\");\n\
         //   await client.connect(\"play.example.com\", 9000, \"account\", \"password\");\n\
         //\n\n\
         import init, {{ GromnieClient }} from \"./{out_name}.js\";\n\
         \n\
         await init();\n\
         \n\
         export {{ GromnieClient }};\n"
    );
    fs::write(out_dir.join("index.mjs"), index_mjs)?;

    // index.d.ts — re-export types from the wasm-bindgen generated declarations
    let index_dts = format!(
        "// Type declarations for the auto-initializing gromnie-web wrapper.\n\
         //\n\
         // Re-exports GromnieClient from the wasm-bindgen generated declarations.\n\n\
         export {{ GromnieClient }} from \"./{out_name}.d.ts\";\n"
    );
    fs::write(out_dir.join("index.d.ts"), index_dts)?;

    // Update package.json to point at the wrapper
    let pkg_path = out_dir.join("package.json");
    if pkg_path.exists() {
        let mut pkg = fs::read_to_string(&pkg_path)?;

        // Add index.mjs and index.d.ts to the files array
        pkg = pkg.replace(
            "\"files\": [\n    \"gromnie_web_bg.wasm\"",
            "\"files\": [\n    \"index.mjs\",\n    \"index.d.ts\",\n    \"gromnie_web_bg.wasm\"",
        );

        // Update main and types to point at the wrapper
        pkg = pkg.replace("\"main\": \"gromnie_web.js\"", "\"main\": \"index.mjs\"");
        pkg = pkg.replace(
            "\"types\": \"gromnie_web.d.ts\"",
            "\"types\": \"index.d.ts\"",
        );

        fs::write(&pkg_path, pkg)?;
    }

    Ok(())
}

fn build_scripts() -> Result<()> {
    println!("Building scripts...\n");

    // Check if wasm32-wasip2 target is installed
    let output = Command::new("rustup").args(["target", "list"]).output()?;
    let target_list = String::from_utf8(output.stdout)?;

    if !target_list.contains("wasm32-wasip2 (installed)") {
        println!("Installing wasm32-wasip2 target...");
        Command::new("rustup")
            .args(["target", "add", "wasm32-wasip2"])
            .status()?
            .success()
            .then_some(())
            .ok_or_else(|| anyhow::anyhow!("Failed to install wasm32-wasip2 target"))?;
    }

    // Check if wasm-tools is installed
    if Command::new("wasm-tools")
        .arg("--version")
        .output()
        .is_err()
    {
        return Err(anyhow::anyhow!(
            "wasm-tools is not installed\nInstall with: cargo install wasm-tools"
        ));
    }

    // Get the project root using CARGO_MANIFEST_DIR
    let project_root = project_root()?;

    // Scripts are in ./scripts directory
    let scripts_dir = project_root.join("scripts");
    let output_dir = scripts_dir.join("target/wasm32-wasip2/release");

    // Discover all script directories in scripts/ (exclude special directories)
    let exclude_dirs = ["target", "xtask", ".cargo"];
    let scripts: Vec<_> = fs::read_dir(&scripts_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            // Check if it's a directory and has a Cargo.toml
            if !path.is_dir() {
                return None;
            }

            let dir_name = path.file_name()?.to_str()?;

            // Skip excluded directories and hidden directories
            if exclude_dirs.contains(&dir_name) || dir_name.starts_with('.') {
                return None;
            }

            // Check if it has a Cargo.toml
            if !path.join("Cargo.toml").exists() {
                return None;
            }

            // Derive package name from directory name (replace - with _)
            let pkg_name = format!("{}_script", dir_name.replace("-", "_"));

            Some((dir_name.to_string(), pkg_name))
        })
        .collect();

    if scripts.is_empty() {
        return Err(anyhow::anyhow!("No script packages found in workspace"));
    }

    for (script_dir, script_pkg) in scripts {
        let script_name = script_dir.replace("-", "_");

        println!("Building {}...", script_dir);

        // Run cargo build for this script
        let status = Command::new("cargo")
            .args(["build", "--release", "--target", "wasm32-wasip2"])
            .current_dir(scripts_dir.join(&script_dir))
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to build {}", script_dir));
        }

        // Ensure output directory exists
        fs::create_dir_all(&output_dir)?;

        // Copy the compiled WASM binary
        let src = output_dir.join(format!("{}.wasm", script_pkg));
        let dest = output_dir.join(format!("{}.wasm", script_name));

        if src.exists() {
            fs::copy(&src, &dest)?;
            println!("  ✓ Created {}", dest.display());
        } else {
            return Err(anyhow::anyhow!(
                "WASM binary not found at: {}",
                src.display()
            ));
        }
    }

    println!("\nScripts built successfully!");
    println!("Output directory: {}", output_dir.display());

    // List built files
    if let Ok(entries) = fs::read_dir(&output_dir) {
        let wasm_files: Vec<_> = entries
            .filter_map(|e| {
                e.ok().and_then(|entry| {
                    if entry.path().extension().is_some_and(|ext| ext == "wasm") {
                        Some(entry.path())
                    } else {
                        None
                    }
                })
            })
            .collect();

        if !wasm_files.is_empty() {
            for file in wasm_files {
                if let Ok(metadata) = fs::metadata(&file) {
                    let size = metadata.len();
                    let size_kb = size / 1024;
                    println!("  {} ({} KB)", file.display(), size_kb);
                }
            }
        }
    }

    Ok(())
}

fn install_scripts() -> Result<()> {
    println!("Installing scripts...\n");

    // Get the project root using CARGO_MANIFEST_DIR
    let project_root = project_root()?;

    let src_dir = project_root.join("scripts/target/wasm32-wasip2/release");
    let dest_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
        .join("gromnie")
        .join("scripts");

    // Create destination directory
    fs::create_dir_all(&dest_dir)?;

    // Discover all built script files
    let scripts: Vec<_> = fs::read_dir(&src_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            // Only process .wasm files
            if path.extension()?.to_str()? != "wasm" {
                return None;
            }

            // Get the file stem (name without extension)
            let file_name = path.file_stem()?.to_str()?;

            // Skip files ending with _script (those are intermediate builds)
            if file_name.ends_with("_script") {
                return None;
            }

            Some(file_name.to_string())
        })
        .collect();

    if scripts.is_empty() {
        println!("⚠ No scripts found in {}", src_dir.display());
        println!("  Run `cargo xtask scripts build` first");
        return Ok(());
    }

    for script in scripts {
        let src_file = src_dir.join(format!("{}.wasm", script));
        let dest_file = dest_dir.join(format!("{}.wasm", script));

        if !src_file.exists() {
            println!("⚠ Source file not found: {}", src_file.display());
            println!("  Run `cargo xtask scripts build` first");
            continue;
        }

        fs::copy(&src_file, &dest_file)?;
        println!("✓ Installed {} to {}", script, dest_file.display());
    }

    println!("\nScripts installed successfully!");
    println!("Destination: {}", dest_dir.display());

    Ok(())
}

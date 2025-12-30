use anyhow::Result;
use clap::{Parser, Subcommand};
use std::env;
use std::fs;

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
}

#[derive(Subcommand)]
enum ScriptCommands {
    /// Build all scripts
    Build,
    /// Install built scripts to ~/.config/gromnie/scripts
    Install,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scripts { command } => match command {
            ScriptCommands::Build => build_scripts()?,
            ScriptCommands::Install => install_scripts()?,
        },
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
    let project_root = match env::var("CARGO_MANIFEST_DIR") {
        Ok(manifest_dir) => {
            // CARGO_MANIFEST_DIR points to crates/xtask, so get parent twice
            std::path::PathBuf::from(manifest_dir)
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Could not determine project root"))?
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Could not determine project root"))?
                .to_path_buf()
        }
        Err(_) => {
            // Fallback: assume current dir is project root
            env::current_dir()?
        }
    };

    // Scripts are in ./crates directory with gromnie-script- prefix
    let crates_dir = project_root.join("crates");
    let output_dir = project_root.join("target/wasm32-wasip2/release");

    // Discover all script directories in crates/ (those starting with gromnie-script-)
    let exclude_dirs = ["target", "xtask", ".cargo"];
    let scripts: Vec<_> = fs::read_dir(&crates_dir)?
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

            // Only process directories that start with gromnie-script-
            if !dir_name.starts_with("gromnie-script-") {
                return None;
            }

            // Check if it has a Cargo.toml
            if !path.join("Cargo.toml").exists() {
                return None;
            }

            // Extract script name from directory (remove gromnie-script- prefix)
            let script_name = dir_name.strip_prefix("gromnie-script-")?.to_string();
            // Package name is script-name + "-script" with dashes (as in Cargo.toml)
            let pkg_name = format!("{}-script", script_name);

            Some((script_name.to_string(), pkg_name))
        })
        .collect();

    if scripts.is_empty() {
        return Err(anyhow::anyhow!("No script packages found in workspace"));
    }

    for (script_name, script_pkg) in scripts {
        let dir_name = format!("gromnie-script-{}", script_name);

        println!("Building {}...", script_name);

        // Run cargo build for this script
        let status = Command::new("cargo")
            .args(["build", "--release", "--target", "wasm32-wasip2"])
            .current_dir(crates_dir.join(&dir_name))
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to build {}", script_name));
        }

        // Ensure output directory exists
        fs::create_dir_all(&output_dir)?;

        // The compiled output uses underscores (cargo converts dashes to underscores in binary names)
        let pkg_name_normalized = script_pkg.replace("-", "_");
        let src = output_dir.join(format!("{}.wasm", pkg_name_normalized));
        let dest = output_dir.join(format!("{}.wasm", script_name));

        if src.exists() {
            fs::copy(&src, &dest)?;
            println!("  ✓ Created {}", dest.display());
        } else {
            return Err(anyhow::anyhow!(
                "WASM binary not found at: {}\nLooking for compiled package: {}",
                src.display(),
                pkg_name_normalized
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
    let project_root = match env::var("CARGO_MANIFEST_DIR") {
        Ok(manifest_dir) => {
            // CARGO_MANIFEST_DIR points to crates/xtask, so get parent twice
            std::path::PathBuf::from(manifest_dir)
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Could not determine project root"))?
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Could not determine project root"))?
                .to_path_buf()
        }
        Err(_) => {
            // Fallback: assume current dir is project root
            env::current_dir()?
        }
    };

    let src_dir = project_root.join("target/wasm32-wasip2/release");
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

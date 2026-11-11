//! The interactive gromnie entry point. No network or credential flags.
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "gromnie",
    version,
    about = "Command-driven Asheron's Call terminal client"
)]
struct Cli;

fn init_file_logging() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("GROMNIE_LOG_FILE").is_none() && std::env::var_os("RUST_LOG").is_none() {
        return Ok(());
    }
    let paths = gromnie_client::config::paths::ProjectPaths::new("gromnie")
        .ok_or_else(|| std::io::Error::other("Could not determine log directory"))?;
    let directory = paths.data_dir().join("logs");
    std::fs::create_dir_all(&directory)?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(directory.join("gromnie.log"))?;
    // A file-only subscriber keeps diagnostics off the fullscreen terminal.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_ansi(false)
        .with_writer(std::sync::Mutex::new(file))
        .try_init()
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    tracing::info!("Starting gromnie");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Cli::parse();
    init_file_logging()?;
    gromnie_tui::command_ui::run().await
}

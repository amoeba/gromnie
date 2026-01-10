use std::fs::{self, File};
use std::io::{self, BufWriter};
use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize logging for a component.
///
/// File logging is enabled via the `GROMNIE_LOG_FILE` environment variable.
/// When set (to any value), logs will be written to both console and file.
/// When unset, only console logging is enabled.
///
/// - `component_name`: Name of the component (e.g., "cli", "tui", "discord")
///
/// Returns a guard that must be kept alive for the duration of the program (if file logging is enabled).
pub fn init_logging(component_name: &str) -> io::Result<Option<WorkerGuard>> {
    // Check if file logging is enabled via environment variable
    let file_logging_enabled = std::env::var("GROMNIE_LOG_FILE").is_ok();

    if file_logging_enabled {
        // Create the log directory in the data directory
        let log_dir = get_log_directory()?;
        fs::create_dir_all(&log_dir)?;

        // Create log file (File::create truncates if exists, creating fresh log each run)
        let log_filename = format!("{}.log", component_name);
        let log_path = log_dir.join(&log_filename);
        let file = File::create(&log_path)?;

        let (non_blocking_file, guard) = tracing_appender::non_blocking(BufWriter::new(file));

        // Set up layered subscriber with both console and file output
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().with_writer(io::stdout).with_ansi(true))
            .with(
                fmt::layer()
                    .with_writer(non_blocking_file)
                    .with_ansi(false)
                    .with_target(true),
            )
            .init();

        tracing::info!("Logging to file: {}", log_path.display());

        Ok(Some(guard))
    } else {
        // Console-only logging
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .init();

        Ok(None)
    }
}

/// Get the log directory path.
fn get_log_directory() -> io::Result<PathBuf> {
    // Use the same data directory as the rest of the app
    let proj_paths = gromnie_client::config::paths::ProjectPaths::new("gromnie")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Failed to find home directory"))?;

    Ok(proj_paths.data_dir().join("logs"))
}

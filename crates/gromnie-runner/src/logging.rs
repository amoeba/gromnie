use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter};
use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

const MAX_LOG_SIZE: u64 = 1024 * 1024; // 1MB

/// Initialize logging for a component.
///
/// - `component_name`: Name of the component (e.g., "cli", "tui", "discord")
/// - `enabled`: If true, enables file logging. If false, only console logging.
///
/// Returns a guard that must be kept alive for the duration of the program.
pub fn init_logging(component_name: &str, enabled: bool) -> io::Result<Option<WorkerGuard>> {
    if enabled {
        // Create the log directory in the data directory
        let log_dir = get_log_directory()?;
        fs::create_dir_all(&log_dir)?;

        // Create log file
        let log_filename = format!("{}.log", component_name);
        let log_path = log_dir.join(&log_filename);

        // Truncate if over 1MB
        truncate_if_needed(&log_path)?;

        // Open file for appending
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        let (non_blocking_file, guard) = tracing_appender::non_blocking(BufWriter::new(file));

        // Set up layered subscriber with both console and file output
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"));

        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .with_writer(io::stdout)
                    .with_ansi(true),
            )
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
    let proj_paths =
        gromnie_client::config::paths::ProjectPaths::new("gromnie")
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Failed to find home directory"))?;

    Ok(proj_paths.data_dir().join("logs"))
}

/// Truncate log file if it exceeds MAX_LOG_SIZE.
fn truncate_if_needed(log_path: &PathBuf) -> io::Result<()> {
    if log_path.exists() {
        let metadata = fs::metadata(log_path)?;
        if metadata.len() > MAX_LOG_SIZE {
            // Truncate the file
            let file = File::create(log_path)?;
            file.set_len(0)?;
        }
    }
    Ok(())
}

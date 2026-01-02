use tracing::{debug, info};

/// Setup SIGUSR2 handler that calls a callback when reload is requested
///
/// This function spawns a task that listens for SIGUSR2 signals and calls
/// the provided callback when the signal is received.
#[cfg(unix)]
pub fn setup_reload_signal_handler<F>(on_reload: F)
where
    F: Fn() + Send + Sync + 'static,
{
    tokio::spawn(async move {
        use tokio::signal::unix;

        // Create signal stream for SIGUSR2
        let mut signal = match unix::signal(unix::SignalKind::user_defined2()) {
            Ok(sig) => sig,
            Err(e) => {
                tracing::error!(target: "scripting", "Failed to create SIGUSR2 handler: {}", e);
                return;
            }
        };

        debug!(target: "scripting", "SIGUSR2 handler installed");

        // Block on signal
        loop {
            match signal.recv().await {
                Some(()) => {
                    info!(target: "scripting", "Received reload signal (SIGUSR2)");
                    on_reload();
                }
                None => {
                    tracing::warn!(target: "scripting", "Signal handler closed");
                    break;
                }
            }
        }
    });
}

/// Setup reload signal handler (non-Unix platforms don't support SIGUSR2)
#[cfg(not(unix))]
pub fn setup_reload_signal_handler<F>(_on_reload: F)
where
    F: Fn() + Send + Sync + 'static,
{
    tracing::warn!(target: "scripting", "SIGUSR2 reload not supported on this platform");
}

use std::path::PathBuf;
use tokio::sync::watch;
use tracing::info;

/// Reload signal type (empty - just signals that a reload is requested)
#[derive(Debug, Clone)]
pub struct ReloadSignal;

/// Create a reload signal channel and spawn SIGUSR2 handler
///
/// Returns a receiver that will be notified when SIGUSR2 is received
#[cfg(unix)]
pub fn setup_reload_signal(_script_dir: PathBuf) -> watch::Receiver<Option<ReloadSignal>> {
    let (reload_tx, reload_rx) = watch::channel(None);

    tokio::spawn(async move {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigusr2 = match signal(SignalKind::user_defined2()) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(target: "scripting", "Failed to register SIGUSR2 handler: {}", e);
                return;
            }
        };

        loop {
            // Check if receiver is still alive - exit if dropped
            if reload_tx.is_closed() {
                info!(target: "scripting", "Reload signal receiver dropped, shutting down SIGUSR2 handler");
                break;
            }

            sigusr2.recv().await;
            info!(target: "scripting", "Received SIGUSR2 - triggering script reload");

            // Send reload signal
            if reload_tx.send(Some(ReloadSignal)).is_err() {
                tracing::error!(target: "scripting", "Failed to send reload signal - receiver dropped");
                break;
            }

            // Clear the signal after a moment to allow detection
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            let _ = reload_tx.send(None);
        }
    });

    reload_rx
}

/// Create a reload signal channel (non-Unix platforms don't support SIGUSR2)
#[cfg(not(unix))]
pub fn setup_reload_signal(_script_dir: PathBuf) -> watch::Receiver<Option<ReloadSignal>> {
    let (reload_tx, reload_rx) = watch::channel(None);
    tracing::warn!(target: "scripting", "SIGUSR2 reload not supported on this platform");
    // Keep the sender alive but never send signals
    std::mem::forget(reload_tx);
    reload_rx
}

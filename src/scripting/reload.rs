use std::path::PathBuf;
use tokio::sync::watch;
use tracing::info;

/// Reload signal type
#[derive(Debug, Clone)]
pub struct ReloadSignal {
    /// Path to the WASM directory to reload from
    pub wasm_dir: PathBuf,
}

/// Create a reload signal channel and spawn SIGHUP handler
///
/// Returns a receiver that will be notified when SIGHUP is received
#[cfg(unix)]
pub fn setup_reload_signal(wasm_dir: PathBuf) -> watch::Receiver<Option<ReloadSignal>> {
    let (reload_tx, reload_rx) = watch::channel(None);

    tokio::spawn(async move {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sighup = match signal(SignalKind::hangup()) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(target: "scripting", "Failed to register SIGHUP handler: {}", e);
                return;
            }
        };

        loop {
            // Check if receiver is still alive - exit if dropped
            if reload_tx.is_closed() {
                info!(target: "scripting", "Reload signal receiver dropped, shutting down SIGHUP handler");
                break;
            }

            sighup.recv().await;
            info!(target: "scripting", "Received SIGHUP - triggering WASM script reload");

            // Send reload signal
            let signal = ReloadSignal {
                wasm_dir: wasm_dir.clone(),
            };

            if reload_tx.send(Some(signal)).is_err() {
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

/// Create a reload signal channel (non-Unix platforms don't support SIGHUP)
#[cfg(not(unix))]
pub fn setup_reload_signal(_wasm_dir: PathBuf) -> watch::Receiver<Option<ReloadSignal>> {
    let (reload_tx, reload_rx) = watch::channel(None);
    tracing::warn!(target: "scripting", "SIGHUP reload not supported on this platform");
    // Keep the sender alive but never send signals
    std::mem::forget(reload_tx);
    reload_rx
}

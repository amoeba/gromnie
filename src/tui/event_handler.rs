use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub enum TuiEvent {
    Key(KeyEvent),
    Quit,
    Tick,
}

pub struct EventHandler {
    tx: mpsc::UnboundedSender<TuiEvent>,
    shutdown_tx: mpsc::UnboundedSender<()>,
    task_handle: Option<JoinHandle<()>>,
}

impl EventHandler {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<TuiEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let (shutdown_tx, _) = mpsc::unbounded_channel();
        (Self { tx, shutdown_tx, task_handle: None }, rx)
    }

    pub async fn start(mut self) -> Self {
        let tx = self.tx.clone();
        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel();
        self.shutdown_tx = shutdown_tx;
        
        // Spawn a task that polls for terminal events
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        // Shutdown signal received
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {
                        if event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
                            if let Ok(Event::Key(key)) = event::read() {
                                // Check for Ctrl+C to quit
                                if key.modifiers.contains(KeyModifiers::CONTROL)
                                    && key.code == KeyCode::Char('c')
                                {
                                    let _ = tx.send(TuiEvent::Quit);
                                } else {
                                    let _ = tx.send(TuiEvent::Key(key));
                                }
                            }
                        }
                        // Send periodic tick events
                        let _ = tx.send(TuiEvent::Tick);
                    }
                }
            }
        });
        self.task_handle = Some(handle);
        self
    }

    pub fn send_event(&self, event: TuiEvent) {
        let _ = self.tx.send(event);
    }

    pub fn shutdown(self) {
        // Send shutdown signal to the event handler task
        let _ = self.shutdown_tx.send(());
        
        // Give it a moment to shut down gracefully
        if let Some(handle) = self.task_handle {
            handle.abort();
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        let (tx, _) = mpsc::unbounded_channel();
        let (shutdown_tx, _) = mpsc::unbounded_channel();
        Self { tx, shutdown_tx, task_handle: None }
    }
}

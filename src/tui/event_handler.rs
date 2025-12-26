use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum TuiEvent {
    Key(KeyEvent),
    Quit,
    Tick,
}

pub struct EventHandler {
    tx: mpsc::UnboundedSender<TuiEvent>,
}

impl EventHandler {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<TuiEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { tx }, rx)
    }

    pub async fn start(self) {
        // Spawn a task that polls for terminal events
        tokio::spawn(async move {
            loop {
                if event::poll(std::time::Duration::from_millis(200)).unwrap_or(false) {
                    if let Ok(Event::Key(key)) = event::read() {
                        // Check for Ctrl+C to quit
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.code == KeyCode::Char('c')
                        {
                            let _ = self.tx.send(TuiEvent::Quit);
                        } else {
                            let _ = self.tx.send(TuiEvent::Key(key));
                        }
                    }
                }
                // Send periodic tick events
                let _ = self.tx.send(TuiEvent::Tick);
            }
        });
    }

    pub fn send_event(&self, event: TuiEvent) {
        let _ = self.tx.send(event);
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        let (tx, _) = mpsc::unbounded_channel();
        Self { tx }
    }
}

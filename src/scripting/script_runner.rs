use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error};

use super::context::{ClientStateSnapshot, ScriptContext};
use super::script::Script;
use super::timer::TimerManager;
use crate::client::events::{ClientAction, GameEvent};
use crate::runner::EventConsumer;

/// Default tick rate for scripts (50ms = 20Hz)
const DEFAULT_TICK_INTERVAL: Duration = Duration::from_millis(50);

/// Runs scripts and dispatches events to them
pub struct ScriptRunner {
    /// All registered scripts (both Rust and WASM)
    scripts: Vec<Box<dyn Script>>,
    /// WASM engine (if WASM support is enabled)
    wasm_engine: Option<wasmtime::Engine>,
    /// Channel for sending client actions
    action_tx: UnboundedSender<ClientAction>,
    /// Timer manager shared across all scripts
    timer_manager: TimerManager,
    /// Current client state snapshot
    client_state: ClientStateSnapshot,
    /// Last time scripts were ticked
    last_tick: Instant,
    /// Interval between ticks (default 50ms for 20Hz)
    tick_interval: Duration,
}

impl ScriptRunner {
    /// Create a new script runner with default tick rate (20Hz)
    pub fn new(action_tx: UnboundedSender<ClientAction>) -> Self {
        Self::new_with_tick_rate(action_tx, DEFAULT_TICK_INTERVAL)
    }

    /// Create a new script runner with custom tick rate
    pub fn new_with_tick_rate(
        action_tx: UnboundedSender<ClientAction>,
        tick_interval: Duration,
    ) -> Self {
        Self {
            scripts: Vec::new(),
            wasm_engine: None,
            action_tx,
            timer_manager: TimerManager::new(),
            client_state: ClientStateSnapshot::new(),
            last_tick: Instant::now(),
            tick_interval,
        }
    }

    /// Create a new script runner with WASM support enabled
    pub fn new_with_wasm(action_tx: UnboundedSender<ClientAction>) -> Self {
        let wasm_engine = match crate::scripting::wasm::create_engine() {
            Ok(engine) => {
                debug!(target: "scripting", "WASM engine initialized");
                Some(engine)
            }
            Err(e) => {
                error!(target: "scripting", "Failed to initialize WASM engine: {:#}", e);
                None
            }
        };

        Self {
            scripts: Vec::new(),
            wasm_engine,
            action_tx,
            timer_manager: TimerManager::new(),
            client_state: ClientStateSnapshot::new(),
            last_tick: Instant::now(),
            tick_interval: DEFAULT_TICK_INTERVAL,
        }
    }

    /// Register a script
    pub fn register_script(&mut self, mut script: Box<dyn Script>) {
        debug!(target: "scripting", "Registering script: {} ({})", script.name(), script.id());

        // Create context for on_load
        let mut ctx = unsafe {
            ScriptContext::new(
                self.action_tx.clone(),
                &mut self.timer_manager as *mut TimerManager,
                self.client_state.clone(),
                Instant::now(),
            )
        };

        // Call on_load
        script.on_load(&mut ctx);

        self.scripts.push(script);
    }

    /// Get the number of registered scripts
    pub fn script_count(&self) -> usize {
        self.scripts.len()
    }

    /// Get the IDs of all registered scripts
    pub fn script_ids(&self) -> Vec<&str> {
        self.scripts.iter().map(|s| s.id()).collect()
    }

    /// Update client state based on events
    fn update_client_state(&mut self, event: &GameEvent) {
        match event {
            GameEvent::AuthenticationSucceeded => {
                self.client_state.is_authenticated = true;
            }
            GameEvent::AuthenticationFailed { .. } => {
                self.client_state.is_authenticated = false;
            }
            GameEvent::LoginSucceeded {
                character_id,
                character_name,
            } => {
                self.client_state.character_id = Some(*character_id);
                self.client_state.character_name = Some(character_name.clone());
                self.client_state.is_ingame = true;
            }
            GameEvent::LoginFailed { .. } => {
                self.client_state.is_ingame = false;
            }
            _ => {}
        }
    }

    /// Load WASM scripts from a directory
    pub fn load_wasm_scripts(&mut self, dir: &std::path::Path) {
        let Some(ref engine) = self.wasm_engine else {
            debug!(target: "scripting", "WASM engine not available, skipping WASM script loading");
            return;
        };

        let wasm_scripts = crate::scripting::wasm::load_wasm_scripts(engine, dir);

        for script in wasm_scripts {
            self.register_script(Box::new(script));
        }
    }

    /// Reload WASM scripts (for hot-reload)
    /// This unloads all existing WASM scripts and loads new ones from the directory
    pub fn reload_wasm_scripts(&mut self, dir: &std::path::Path) {
        debug!(target: "scripting", "Reloading WASM scripts from {}", dir.display());

        // Unload existing WASM scripts
        self.unload_wasm_scripts();

        // Load new ones
        self.load_wasm_scripts(dir);
    }

    /// Unload all WASM scripts
    fn unload_wasm_scripts(&mut self) {
        // Filter out WasmScript instances and call on_unload
        let mut to_remove = Vec::new();

        for (idx, script) in self.scripts.iter_mut().enumerate() {
            // Check if this is a WasmScript by attempting downcast
            if script
                .as_any_mut()
                .downcast_ref::<crate::scripting::wasm::WasmScript>()
                .is_some()
            {
                // Call on_unload before dropping
                let mut ctx = unsafe {
                    ScriptContext::new(
                        self.action_tx.clone(),
                        &mut self.timer_manager as *mut TimerManager,
                        self.client_state.clone(),
                        Instant::now(),
                    )
                };

                debug!(target: "scripting", "Unloading WASM script: {} ({})", script.name(), script.id());
                script.on_unload(&mut ctx);

                to_remove.push(idx);
            }
        }

        // Remove in reverse order to preserve indices
        for idx in to_remove.into_iter().rev() {
            self.scripts.remove(idx);
        }
    }

    /// Process timers and return fired timer IDs
    fn tick_timers(&mut self, now: Instant) -> Vec<(super::timer::TimerId, String)> {
        self.timer_manager.tick(now)
    }

    /// Tick all scripts if enough time has elapsed
    fn tick_scripts(&mut self, now: Instant) {
        let elapsed = now.duration_since(self.last_tick);

        if elapsed < self.tick_interval {
            return; // Not time to tick yet
        }

        // Update last tick time
        self.last_tick = now;

        // Call on_tick for each script
        for script in &mut self.scripts {
            // Create context for this script
            let mut ctx = unsafe {
                ScriptContext::new(
                    self.action_tx.clone(),
                    &mut self.timer_manager as *mut TimerManager,
                    self.client_state.clone(),
                    now,
                )
            };

            // Call the script's tick handler
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                script.on_tick(&mut ctx, elapsed);
            })) {
                Ok(_) => {}
                Err(e) => {
                    error!(target: "scripting",
                        "Script {} ({}) panicked during tick: {:?}",
                        script.name(),
                        script.id(),
                        e
                    );
                }
            }
        }
    }
}

impl EventConsumer for ScriptRunner {
    fn handle_event(&mut self, event: GameEvent) {
        let now = Instant::now();

        // Update client state
        self.update_client_state(&event);

        // Tick timers FIRST so they're marked as fired
        let fired_timers = self.tick_timers(now);
        if !fired_timers.is_empty() {
            debug!(target: "scripting", "Timers fired: {:?}", fired_timers);
        }

        // THEN tick scripts so they can detect fired timers
        self.tick_scripts(now);

        // Dispatch event to each script that's interested
        for script in &mut self.scripts {
            // Check if this script is subscribed to this event
            let subscribed = script
                .subscribed_events()
                .iter()
                .any(|filter| filter.matches(&event));

            if !subscribed {
                continue;
            }

            // Create context for this script
            let mut ctx = unsafe {
                ScriptContext::new(
                    self.action_tx.clone(),
                    &mut self.timer_manager as *mut TimerManager,
                    self.client_state.clone(),
                    now,
                )
            };

            // Call the script's event handler
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                script.on_event(&event, &mut ctx);
            })) {
                Ok(_) => {}
                Err(e) => {
                    error!(target: "scripting",
                        "Script {} ({}) panicked while handling event: {:?}",
                        script.name(),
                        script.id(),
                        e
                    );
                }
            }
        }
    }
}

impl Drop for ScriptRunner {
    fn drop(&mut self) {
        // Call on_unload for all scripts
        for script in &mut self.scripts {
            let mut ctx = unsafe {
                ScriptContext::new(
                    self.action_tx.clone(),
                    &mut self.timer_manager as *mut TimerManager,
                    self.client_state.clone(),
                    Instant::now(),
                )
            };

            script.on_unload(&mut ctx);
        }
    }
}

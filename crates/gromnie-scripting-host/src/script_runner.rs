use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error};

use super::Script;
use super::context::{ClientStateSnapshot, ScriptContext};
use super::timer::TimerManager;
use super::wasm::WasmScript;
use super::EventFilter;
use gromnie_client::client::events::{ClientAction, GameEvent};

/// Default tick rate for scripts (50ms = 20Hz)
const DEFAULT_TICK_INTERVAL: Duration = Duration::from_millis(50);

/// Runs scripts and dispatches events to them
pub struct ScriptRunner {
    /// All registered WASM scripts
    scripts: Vec<WasmScript>,
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
        let wasm_engine = match super::wasm::create_engine() {
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

    /// Register a WASM script
    pub fn register_script(&mut self, script: WasmScript) {
        debug!(target: "scripting", "Registering script: {} ({})", script.name(), script.id());

        // Create context for on_load
        let mut ctx = Self::create_script_context(
            self.action_tx.clone(),
            &mut self.timer_manager,
            self.client_state.clone(),
            Instant::now()
        );

        // Call on_load
        let mut script = script; // Make mutable
        script.on_load(&mut ctx);

        self.scripts.push(script);
    }

    /// Get the number of registered scripts
    pub fn script_count(&self) -> usize {
        self.scripts.len()
    }

    /// Check if WASM engine is available
    pub fn has_wasm_engine(&self) -> bool {
        self.wasm_engine.is_some()
    }

    /// Get the IDs of all registered scripts
    pub fn script_ids(&self) -> Vec<&str> {
        self.scripts.iter().map(|s| s.id()).collect()
    }

    /// Create a script context for the current state
    fn create_script_context(
        action_tx: UnboundedSender<ClientAction>,
        timer_manager: &mut TimerManager,
        client_state: ClientStateSnapshot,
        now: Instant
    ) -> ScriptContext {
        unsafe {
            ScriptContext::new(
                action_tx,
                timer_manager as *mut TimerManager,
                client_state,
                now,
            )
        }
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

    /// Load scripts from a directory
    pub fn load_scripts(&mut self, dir: &std::path::Path) {
        let Some(ref engine) = self.wasm_engine else {
            debug!(target: "scripting", "Script engine not available, skipping script loading");
            return;
        };

        let scripts = super::wasm::load_wasm_scripts(engine, dir);

        for script in scripts {
            self.register_script(script);
        }
    }

    /// Reload scripts (for hot-reload)
    /// This unloads all existing scripts and loads new ones from the directory
    pub fn reload_scripts(&mut self, dir: &std::path::Path) {
        debug!(target: "scripting", "Reloading scripts from {}", dir.display());

        // Unload existing scripts
        self.unload_scripts();

        // Load new ones
        self.load_scripts(dir);
    }

    /// Unload all scripts
    ///
    /// This method unloads all scripts by calling their on_unload methods.
    /// The system only supports WASM scripts, so all scripts are WASM scripts.
    pub fn unload_scripts(&mut self) {
        // Create context once before the loop
        let mut ctx = Self::create_script_context(
            self.action_tx.clone(),
            &mut self.timer_manager,
            self.client_state.clone(),
            Instant::now()
        );
        
        // Identify WASM scripts and call on_unload
        let mut to_remove = Vec::new();

        // Unload all scripts directly
        for (idx, script) in self.scripts.iter_mut().enumerate() {
            debug!(target: "scripting", "Unloading script: {} ({})", script.name(), script.id());
            script.on_unload(&mut ctx);

            to_remove.push(idx);
        }

        // Remove scripts in reverse order to preserve indices during removal
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

        // Create context once before the loop
        let mut ctx = Self::create_script_context(
            self.action_tx.clone(),
            &mut self.timer_manager,
            self.client_state.clone(),
            now
        );
        
        // Call on_tick for each script
        for script in &mut self.scripts {
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

impl ScriptRunner {
    /// Handle a game event
    pub fn handle_event(&mut self, event: GameEvent) {
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

        // Create context once before the loop
        let mut ctx = Self::create_script_context(
            self.action_tx.clone(),
            &mut self.timer_manager,
            self.client_state.clone(),
            now
        );
        
        // Dispatch event to each script that's interested
        for script in &mut self.scripts {
            // Check if this script is subscribed to this event
            let subscribed = script
                .subscribed_events()
                .iter()
                .any(|filter: &EventFilter| filter.matches(&event));

            if !subscribed {
                continue;
            }

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
        // Create context once before the loop
        let mut ctx = Self::create_script_context(
            self.action_tx.clone(),
            &mut self.timer_manager,
            self.client_state.clone(),
            Instant::now()
        );
        
        // Call on_unload for all scripts
        for script in &mut self.scripts {
            let script: &mut WasmScript = script;
            script.on_unload(&mut ctx);
        }
    }
}

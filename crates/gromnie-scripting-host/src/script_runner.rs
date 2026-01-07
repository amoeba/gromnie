use gromnie_client::client::Client;
use gromnie_client::config::scripting_config::ScriptingConfig;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info, warn};

use super::EventFilter;
use super::Script;
use super::context::ScriptContext;
use super::script_scanner::ScriptScanner;
use super::timer::TimerManager;
use super::wasm::WasmScript;
use crate::create_runner_from_config;
use gromnie_events::{ClientEvent, ClientSystemEvent, SimpleClientAction};
use gromnie_events::{EventConsumer, EventEnvelope};

/// Default tick rate for scripts (50ms = 20Hz)
const DEFAULT_TICK_INTERVAL: Duration = Duration::from_millis(50);

/// Runs scripts and dispatches events to them
pub struct ScriptRunner {
    /// Shared reference to the client
    client: Arc<RwLock<Client>>,
    /// All registered WASM scripts
    scripts: Vec<WasmScript>,
    /// WASM engine (if WASM support is enabled)
    wasm_engine: Option<wasmtime::Engine>,
    /// Channel for sending client actions
    action_tx: UnboundedSender<SimpleClientAction>,
    /// Timer manager shared across all scripts
    timer_manager: TimerManager,
    /// Last time scripts were ticked
    last_tick: Instant,
    /// Interval between ticks (default 50ms for 20Hz)
    tick_interval: Duration,
    /// Script configuration for reload operations
    script_config: Option<HashMap<String, toml::Value>>,
    /// Script directory path
    script_dir: Option<std::path::PathBuf>,
}

impl ScriptRunner {
    /// Create a new script runner with default tick rate (20Hz)
    pub fn new(
        client: Arc<RwLock<Client>>,
        action_tx: UnboundedSender<SimpleClientAction>,
    ) -> Self {
        Self::new_with_tick_rate(client, action_tx, DEFAULT_TICK_INTERVAL)
    }

    /// Create a new script runner with custom tick rate
    pub fn new_with_tick_rate(
        client: Arc<RwLock<Client>>,
        action_tx: UnboundedSender<SimpleClientAction>,
        tick_interval: Duration,
    ) -> Self {
        Self {
            client,
            scripts: Vec::new(),
            wasm_engine: None,
            action_tx,
            timer_manager: TimerManager::new(),
            last_tick: Instant::now(),
            tick_interval,
            script_config: None,
            script_dir: None,
        }
    }

    /// Create a new script runner with WASM support enabled
    pub fn new_with_wasm(
        client: Arc<RwLock<Client>>,
        action_tx: UnboundedSender<SimpleClientAction>,
    ) -> Self {
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
            client,
            scripts: Vec::new(),
            wasm_engine,
            action_tx,
            timer_manager: TimerManager::new(),
            last_tick: Instant::now(),
            tick_interval: DEFAULT_TICK_INTERVAL,
            script_config: None,
            script_dir: None,
        }
    }

    /// Register a WASM script
    pub fn register_script(&mut self, script: WasmScript) {
        debug!(target: "scripting", "Registering script: {} ({})", script.name(), script.id());

        // Create context for on_load
        let mut ctx = Self::create_script_context(
            self.client.clone(),
            self.action_tx.clone(),
            &mut self.timer_manager,
            SystemTime::now(),
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
        client: Arc<RwLock<Client>>,
        action_tx: UnboundedSender<SimpleClientAction>,
        timer_manager: &mut TimerManager,
        now: SystemTime,
    ) -> ScriptContext {
        unsafe { ScriptContext::new(client, action_tx, timer_manager as *mut TimerManager, now) }
    }

    /// Load scripts from a directory
    pub fn load_scripts(
        &mut self,
        dir: &std::path::Path,
        script_config: &HashMap<String, toml::Value>,
    ) {
        debug!(target: "scripting", "Loading scripts from {}", dir.display());

        let Some(ref engine) = self.wasm_engine else {
            tracing::warn!(target: "scripting", "Script engine not available, skipping script loading");
            return;
        };

        let scripts = super::wasm::load_wasm_scripts(engine, dir, script_config);
        let loaded_count = scripts.len();

        for script in scripts {
            debug!(target: "scripting", "Registering script: {} ({})", script.name(), script.id());
            self.register_script(script);
        }

        if loaded_count > 0 {
            info!(target: "scripting", "Loaded {} script(s) (total: {})", loaded_count, self.scripts.len());
        }

        // Store script config and directory for hot reload
        self.script_config = Some(script_config.clone());
        self.script_dir = Some(dir.to_path_buf());
    }

    /// Reload scripts (for hot-reload)
    /// This unloads all existing scripts and loads new ones from the directory
    /// If reload fails and no scripts are loaded, this will warn but continue with no scripts
    pub fn reload_scripts(
        &mut self,
        dir: &std::path::Path,
        script_config: &HashMap<String, toml::Value>,
    ) {
        let old_script_count = self.scripts.len();
        debug!(target: "scripting", "Reloading scripts from {}", dir.display());

        // Unload existing scripts
        self.unload_scripts();

        // Force drop of any remaining script resources by clearing the vector
        self.scripts.clear();
        self.scripts.shrink_to_fit();

        // Give the system a moment to fully release WASM/WASI resources
        // This is important because wasmtime stores might not be immediately dropped
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Load new ones
        self.load_scripts(dir, script_config);

        let new_script_count = self.scripts.len();

        // Warn if we went from having scripts to having none
        if old_script_count > 0 && new_script_count == 0 {
            tracing::warn!(
                target: "scripting",
                "Script reload resulted in zero scripts (was {}). Check logs for loading errors.",
                old_script_count
            );
        } else if new_script_count > 0 {
            info!(
                target: "scripting",
                "Reloaded {} script(s)",
                new_script_count
            );
        }
    }

    /// Unload all scripts
    ///
    /// This method unloads all scripts by calling their on_unload methods.
    /// The system only supports WASM scripts, so all scripts are WASM scripts.
    pub fn unload_scripts(&mut self) {
        let count = self.scripts.len();

        if count == 0 {
            return;
        }

        debug!(target: "scripting", "Unloading {} script(s)", count);

        // Create context once before the loop
        let mut ctx = Self::create_script_context(
            self.client.clone(),
            self.action_tx.clone(),
            &mut self.timer_manager,
            SystemTime::now(),
        );

        // Unload all scripts directly and call on_unload
        for script in self.scripts.iter_mut() {
            debug!(target: "scripting", "Calling on_unload for: {} ({})", script.name(), script.id());
            script.on_unload(&mut ctx);
        }

        // Clear the scripts vector
        self.scripts.clear();
    }

    /// Handle detected script changes from the scanner
    fn handle_script_changes(&mut self, changes: super::script_scanner::ScanResult) {
        let Some(engine) = self.wasm_engine.clone() else {
            return; // No WASM engine, can't reload
        };

        let script_config = self.script_config.as_ref().cloned().unwrap_or_default();

        // Detect file replacements: when a file is both removed and added, it's actually
        // a modification (common with install scripts that delete then copy)
        let mut replacements = std::collections::HashSet::new();

        // Find removed scripts that have corresponding added scripts with the same filename
        for removed_path in &changes.removed {
            if let Some(filename) = removed_path.file_name() {
                for added_path in &changes.added {
                    if added_path.file_name() == Some(filename) {
                        // Same filename - this is a replacement, not a true add/remove
                        replacements.insert(filename);
                        break;
                    }
                }
            }
        }

        // Handle removed scripts that were actually replaced (treat as modification)
        for removed_path in &changes.removed {
            if let Some(filename) = removed_path.file_name()
                && replacements.contains(filename)
            {
                // Find the corresponding added path
                if let Some(added_path) = changes
                    .added
                    .iter()
                    .find(|p| p.file_name() == Some(filename))
                {
                    info!(
                        target: "scripting",
                        "Hot-reloading replaced script: {} -> {}",
                        removed_path.display(),
                        added_path.display()
                    );
                    self.reload_script_by_path_replacement(
                        removed_path,
                        added_path,
                        &engine,
                        &script_config,
                    );
                }
            }
        }

        // Handle changed scripts (true modifications)
        for (path, _new_time) in &changes.changed {
            info!(target: "scripting", "Hot-reloading modified script: {}", path.display());
            self.reload_script_by_path(path, &engine, &script_config);
        }

        // Handle truly added scripts (not replacements)
        for path in &changes.added {
            if let Some(filename) = path.file_name()
                && !replacements.contains(filename)
            {
                info!(target: "scripting", "Loading new script: {}", path.display());
                self.load_script_by_path(path, &engine, &script_config);
            }
        }

        // Handle truly removed scripts (not replacements)
        for path in &changes.removed {
            if let Some(filename) = path.file_name()
                && !replacements.contains(filename)
            {
                info!(target: "scripting", "Unloading removed script: {}", path.display());
                self.unload_script_by_path(path);
            }
        }
    }

    /// Reload a specific script by file path
    fn reload_script_by_path(
        &mut self,
        path: &std::path::Path,
        engine: &wasmtime::Engine,
        script_config: &HashMap<String, toml::Value>,
    ) {
        // Find the index of the script with this path
        let index = match self.scripts.iter().position(|s| s.file_path() == path) {
            Some(idx) => idx,
            None => {
                warn!(
                    target: "scripting",
                    "Cannot reload script at {}: not found in loaded scripts",
                    path.display()
                );
                return;
            }
        };

        let old_script = &self.scripts[index];
        let script_id = old_script.id().to_string();

        // Create context for on_unload
        let mut ctx = Self::create_script_context(
            self.client.clone(),
            self.action_tx.clone(),
            &mut self.timer_manager,
            SystemTime::now(),
        );

        // Call on_unload on old instance
        debug!(
            target: "scripting",
            "Calling on_unload for: {} ({})",
            old_script.name(),
            script_id
        );
        let mut old_script = self.scripts.remove(index);
        old_script.on_unload(&mut ctx);

        // Try to load new instance
        match WasmScript::from_file(engine, path) {
            Ok(new_script) => {
                // Check if this script is enabled in config
                let is_enabled = script_config
                    .get(&script_id)
                    .and_then(|config| config.get("enabled"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                if !is_enabled {
                    debug!(
                        target: "scripting",
                        "Skipping reload of disabled script: {} ({})",
                        new_script.name(),
                        script_id
                    );
                    return;
                }

                // Call on_load on new instance
                let mut new_script = new_script;
                new_script.on_load(&mut ctx);

                // Add to scripts
                self.scripts.push(new_script);

                info!(
                    target: "scripting",
                    "Successfully reloaded script: {} ({})",
                    self.scripts.last().unwrap().name(),
                    script_id
                );
            }
            Err(e) => {
                error!(
                    target: "scripting",
                    "Failed to reload script {} from {}: {:#}",
                    script_id,
                    path.display(),
                    e
                );
                // Note: Old script was already removed, so it's not running anymore
                // This is intentional - we don't want to keep a broken version running
            }
        }
    }

    /// Load a new script by file path
    fn load_script_by_path(
        &mut self,
        path: &std::path::Path,
        engine: &wasmtime::Engine,
        script_config: &HashMap<String, toml::Value>,
    ) {
        // Try to load the script
        match WasmScript::from_file(engine, path) {
            Ok(script) => {
                let script_id = script.id().to_string();

                // Check if we already have a script with this ID
                if self.scripts.iter().any(|s| s.id() == script_id) {
                    warn!(
                        target: "scripting",
                        "Cannot load script {} ({}): script ID already exists",
                        script.name(),
                        script_id
                    );
                    return;
                }

                // Check if this script is enabled in config
                let is_enabled = script_config
                    .get(&script_id)
                    .and_then(|config| config.get("enabled"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                if !is_enabled {
                    debug!(
                        target: "scripting",
                        "Skipping disabled script: {} ({})",
                        script.name(),
                        script_id
                    );
                    return;
                }

                // Create context and call on_load
                let mut ctx = Self::create_script_context(
                    self.client.clone(),
                    self.action_tx.clone(),
                    &mut self.timer_manager,
                    SystemTime::now(),
                );

                let mut script = script;
                script.on_load(&mut ctx);

                self.scripts.push(script);

                info!(
                    target: "scripting",
                    "Successfully loaded new script: {} ({})",
                    self.scripts.last().unwrap().name(),
                    script_id
                );
            }
            Err(e) => {
                error!(
                    target: "scripting",
                    "Failed to load script from {}: {:#}",
                    path.display(),
                    e
                );
            }
        }
    }

    /// Reload a script when its file was replaced (removed then added)
    /// This handles the case where install scripts delete old files then copy new ones
    fn reload_script_by_path_replacement(
        &mut self,
        old_path: &std::path::Path,
        new_path: &std::path::Path,
        engine: &wasmtime::Engine,
        script_config: &HashMap<String, toml::Value>,
    ) {
        // Find the script by the old path
        let index = match self.scripts.iter().position(|s| s.file_path() == old_path) {
            Some(idx) => idx,
            None => {
                // Script not found at old path - just try to load from new path
                debug!(
                    target: "scripting",
                    "Script not found at old path {}, loading from new path {}",
                    old_path.display(),
                    new_path.display()
                );
                self.load_script_by_path(new_path, engine, script_config);
                return;
            }
        };

        let old_script = &self.scripts[index];
        let script_id = old_script.id().to_string();

        // Create context for on_unload
        let mut ctx = Self::create_script_context(
            self.client.clone(),
            self.action_tx.clone(),
            &mut self.timer_manager,
            SystemTime::now(),
        );

        // Call on_unload on old instance
        debug!(
            target: "scripting",
            "Calling on_unload for replaced script: {} ({})",
            old_script.name(),
            script_id
        );
        let mut old_script = self.scripts.remove(index);
        old_script.on_unload(&mut ctx);

        // Try to load new instance
        match WasmScript::from_file(engine, new_path) {
            Ok(new_script) => {
                // Check if this script is enabled in config
                let is_enabled = script_config
                    .get(&script_id)
                    .and_then(|config| config.get("enabled"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                if !is_enabled {
                    debug!(
                        target: "scripting",
                        "Skipping reload of disabled script: {} ({})",
                        new_script.name(),
                        script_id
                    );
                    return;
                }

                // Call on_load on new instance
                let mut new_script = new_script;
                new_script.on_load(&mut ctx);

                // Add to scripts
                self.scripts.push(new_script);

                info!(
                    target: "scripting",
                    "Successfully reloaded replaced script: {} ({})",
                    self.scripts.last().unwrap().name(),
                    script_id
                );
            }
            Err(e) => {
                error!(
                    target: "scripting",
                    "Failed to reload replaced script {} from {}: {:#}",
                    script_id,
                    new_path.display(),
                    e
                );
            }
        }
    }

    /// Unload a script by its file path
    fn unload_script_by_path(&mut self, path: &std::path::Path) {
        // Find the script by path
        let index = match self.scripts.iter().position(|s| s.file_path() == path) {
            Some(idx) => idx,
            None => {
                debug!(
                    target: "scripting",
                    "Script at {} not found, may have already been unloaded",
                    path.display()
                );
                return;
            }
        };

        let script = &self.scripts[index];
        let script_id = script.id().to_string();
        let script_name = script.name().to_string();

        // Create context for on_unload
        let mut ctx = Self::create_script_context(
            self.client.clone(),
            self.action_tx.clone(),
            &mut self.timer_manager,
            SystemTime::now(),
        );

        // Call on_unload and remove
        debug!(
            target: "scripting",
            "Calling on_unload for removed script: {} ({})",
            script_name,
            script_id
        );
        let mut script = self.scripts.remove(index);
        script.on_unload(&mut ctx);

        info!(
            target: "scripting",
            "Unloaded removed script: {} ({})",
            script_name,
            script_id
        );
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
            self.client.clone(),
            self.action_tx.clone(),
            &mut self.timer_manager,
            SystemTime::now(),
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
    /// Handle a raw event
    pub fn handle_event(&mut self, raw_event: ClientEvent) {
        let now = Instant::now();

        debug!(
            target: "scripting",
            "Event received: {:?}",
            std::mem::discriminant(&raw_event)
        );

        // Tick timers FIRST so they're marked as fired
        let fired_timers = self.tick_timers(now);
        if !fired_timers.is_empty() {
            debug!(target: "scripting", "Timers fired: {:?}", fired_timers);
        }

        // THEN tick scripts so they can detect fired timers
        self.tick_scripts(now);

        // Create context once before the loop
        let mut ctx = Self::create_script_context(
            self.client.clone(),
            self.action_tx.clone(),
            &mut self.timer_manager,
            SystemTime::now(),
        );

        // Dispatch event to each script that's interested
        for script in &mut self.scripts {
            // Check if this script is subscribed to this event
            let subscribed = script
                .subscribed_events()
                .iter()
                .any(|filter: &EventFilter| filter.matches(&raw_event));

            debug!(
                target: "scripting",
                "Script {} ({}) subscribed to {:?}, event matches: {}",
                script.name(),
                script.id(),
                script.subscribed_events(),
                subscribed
            );

            if !subscribed {
                continue;
            }

            // Call the script's event handler
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                script.on_event(&raw_event, &mut ctx);
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
            self.client.clone(),
            self.action_tx.clone(),
            &mut self.timer_manager,
            SystemTime::now(),
        );

        // Call on_unload for all scripts
        for script in &mut self.scripts {
            let script: &mut WasmScript = script;
            script.on_unload(&mut ctx);
        }
    }
}

/// Wrapper around ScriptRunner that implements EventConsumer trait
pub struct ScriptConsumer {
    runner: Arc<std::sync::Mutex<ScriptRunner>>,
    script_dir: Option<std::path::PathBuf>,
    script_config: Option<std::collections::HashMap<String, toml::Value>>,
    hot_reload_task: Option<tokio::task::JoinHandle<()>>,
}

impl ScriptConsumer {
    pub fn new(runner: ScriptRunner) -> Self {
        Self {
            runner: Arc::new(std::sync::Mutex::new(runner)),
            script_dir: None,
            script_config: None,
            hot_reload_task: None,
        }
    }

    pub fn with_reload_config(
        mut self,
        script_dir: std::path::PathBuf,
        script_config: std::collections::HashMap<String, toml::Value>,
        hot_reload_enabled: bool,
        hot_reload_interval_ms: u64,
    ) -> Self {
        self.script_dir = Some(script_dir.clone());
        self.script_config = Some(script_config.clone());

        // Load initial scripts
        {
            let mut runner = self.runner.lock().expect("Failed to lock runner");
            runner.load_scripts(&script_dir, &script_config);
        }

        // Enable hot reload if requested
        if hot_reload_enabled {
            let interval = Duration::from_millis(hot_reload_interval_ms);
            info!(
                target: "scripting",
                "Enabling hot reload with interval: {}ms",
                hot_reload_interval_ms
            );

            // Spawn background task for hot reload
            let runner = self.runner.clone();
            let scanner = ScriptScanner::with_interval(script_dir, interval);

            let task = tokio::spawn(async move {
                Self::hot_reload_task(runner, scanner).await;
            });

            self.hot_reload_task = Some(task);

            info!(
                target: "scripting",
                "Hot reload enabled, scanning every {}ms",
                hot_reload_interval_ms
            );
        }

        self
    }

    /// Background task for hot reload
    /// Periodically scans for script changes and applies them
    async fn hot_reload_task(
        runner: Arc<std::sync::Mutex<ScriptRunner>>,
        mut scanner: ScriptScanner,
    ) {
        let interval = scanner.scan_interval();
        let mut interval_timer = tokio::time::interval(interval);
        interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        info!(
            target: "scripting",
            "Hot reload background task started, scanning every {:?}",
            interval
        );

        loop {
            interval_timer.tick().await;

            debug!(target: "scripting", "Hot reload: scanning for changes...");

            // Scan for changes
            let changes = scanner.scan_changes();

            if changes.has_changes() {
                info!(
                    target: "scripting",
                    "Hot reload: detected changes - {} modified, {} added, {} removed",
                    changes.changed.len(),
                    changes.added.len(),
                    changes.removed.len()
                );

                // Acquire lock and handle changes
                if let Ok(mut runner) = runner.lock() {
                    runner.handle_script_changes(changes);
                } else {
                    error!(target: "scripting", "Failed to acquire lock for hot reload");
                }
            }
        }
    }
}

impl EventConsumer for ScriptConsumer {
    fn handle_event(&mut self, envelope: EventEnvelope) {
        // Check for reload event
        if let gromnie_events::EventType::System(gromnie_events::SystemEvent::ReloadScripts {
            ..
        }) = &envelope.event
        {
            if let Some(ref script_dir) = self.script_dir {
                if let Some(ref script_config) = self.script_config {
                    // Acquire lock and reload
                    if let Ok(mut runner) = self.runner.lock() {
                        runner.reload_scripts(script_dir, script_config);
                    } else {
                        tracing::error!(target: "scripting", "Failed to acquire lock for script reload");
                    }
                } else {
                    tracing::warn!(target: "scripting", "ReloadScripts event received but script_config is None");
                }
            } else {
                tracing::warn!(target: "scripting", "ReloadScripts event received but script_dir is None");
            }
            return; // Don't process reload events as game events
        }

        // Extract ClientEvent from EventEnvelope
        let client_event = match envelope.event {
            gromnie_events::EventType::Game(game_event) => ClientEvent::Game(game_event),
            gromnie_events::EventType::State(state_event) => {
                // Pass through state event directly (new granular states)
                ClientEvent::State(state_event)
            }
            gromnie_events::EventType::System(system_event) => {
                // Convert SystemEvent to ClientSystemEvent
                match system_event {
                    gromnie_events::SystemEvent::AuthenticationSucceeded { .. } => {
                        ClientEvent::System(ClientSystemEvent::AuthenticationSucceeded)
                    }
                    gromnie_events::SystemEvent::AuthenticationFailed { reason, .. } => {
                        ClientEvent::System(ClientSystemEvent::AuthenticationFailed { reason })
                    }
                    gromnie_events::SystemEvent::ConnectingStarted { .. } => {
                        ClientEvent::System(ClientSystemEvent::ConnectingStarted)
                    }
                    gromnie_events::SystemEvent::ConnectingDone { .. } => {
                        ClientEvent::System(ClientSystemEvent::ConnectingDone)
                    }
                    gromnie_events::SystemEvent::UpdatingStarted { .. } => {
                        ClientEvent::System(ClientSystemEvent::UpdatingStarted)
                    }
                    gromnie_events::SystemEvent::UpdatingDone { .. } => {
                        ClientEvent::System(ClientSystemEvent::UpdatingDone)
                    }
                    gromnie_events::SystemEvent::LoginSucceeded {
                        character_id,
                        character_name,
                    } => ClientEvent::System(ClientSystemEvent::LoginSucceeded {
                        character_id,
                        character_name,
                    }),
                    gromnie_events::SystemEvent::Disconnected {
                        will_reconnect,
                        reconnect_attempt,
                        delay_secs,
                        ..
                    } => ClientEvent::System(ClientSystemEvent::Disconnected {
                        will_reconnect,
                        reconnect_attempt,
                        delay_secs,
                    }),
                    gromnie_events::SystemEvent::Reconnecting {
                        attempt,
                        delay_secs,
                        ..
                    } => ClientEvent::System(ClientSystemEvent::Reconnecting {
                        attempt,
                        delay_secs,
                    }),
                    // These events don't have ClientSystemEvent equivalents
                    gromnie_events::SystemEvent::ReloadScripts { .. }
                    | gromnie_events::SystemEvent::LogScriptMessage { .. }
                    | gromnie_events::SystemEvent::Shutdown => {
                        return;
                    }
                }
            }
        };

        // Acquire lock to handle event
        if let Ok(mut runner) = self.runner.lock() {
            runner.handle_event(client_event);
        } else {
            tracing::error!(target: "scripting", "Failed to acquire lock for event handling");
        }
    }
}

impl Drop for ScriptConsumer {
    fn drop(&mut self) {
        // Abort the hot reload task if it's running
        if let Some(task) = self.hot_reload_task.take() {
            task.abort();
        }
    }
}

/// Create a script runner consumer with the specified configuration
pub fn create_script_consumer(
    client: Arc<RwLock<Client>>,
    action_tx: UnboundedSender<SimpleClientAction>,
    scripting_config: &ScriptingConfig,
) -> ScriptConsumer {
    let runner = create_runner_from_config(client, action_tx, scripting_config);
    let mut consumer = ScriptConsumer::new(runner);

    if scripting_config.enabled {
        let script_dir = scripting_config.script_dir();
        consumer = consumer.with_reload_config(
            script_dir,
            scripting_config.config.clone(),
            scripting_config.hot_reload,
            scripting_config.hot_reload_interval_ms,
        );
    }

    consumer
}

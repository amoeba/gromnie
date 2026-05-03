use gromnie_client::client::Client;
use gromnie_client::config::scripting_config::ScriptingConfig;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

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

/// Message sent to the runner task
enum RunnerMessage {
    Event(ClientEvent),
    Reload {
        dir: PathBuf,
        script_config: HashMap<String, toml::Value>,
    },
}

enum ReloadCandidate {
    Loaded(WasmScript),
    Disabled { script_id: String },
    Failed,
}

/// Runs scripts and dispatches events to them — managed by a background tokio task
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

    /// Register a WASM script (async — calls on_load)
    pub async fn register_script(&mut self, script: WasmScript) {
        debug!(target: "scripting", "Registering script: {} ({})", script.name(), script.id());

        // Create context for on_load
        let mut ctx = Self::create_script_context(
            self.client.clone(),
            self.action_tx.clone(),
            &mut self.timer_manager,
            SystemTime::now(),
        );

        // Call on_load
        let mut script = script;
        script.on_load(&mut ctx).await;

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

    fn is_script_enabled(script_id: &str, script_config: &HashMap<String, toml::Value>) -> bool {
        script_config
            .get(script_id)
            .and_then(|config| config.get("enabled"))
            .and_then(|value| value.as_bool())
            .unwrap_or(true)
    }

    fn current_script_files(dir: &Path) -> HashMap<PathBuf, SystemTime> {
        let mut scripts = HashMap::new();

        if !dir.exists() {
            debug!(
                target: "scripting",
                "Script directory does not exist: {}",
                dir.display()
            );
            return scripts;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(err) => {
                tracing::warn!(
                    target: "scripting",
                    "Failed to read script directory {}: {}",
                    dir.display(),
                    err
                );
                return scripts;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("wasm") {
                continue;
            }

            match std::fs::metadata(&path).and_then(|metadata| metadata.modified()) {
                Ok(modified_time) => {
                    scripts.insert(path, modified_time);
                }
                Err(err) => {
                    tracing::warn!(
                        target: "scripting",
                        "Failed to read metadata for {}: {}",
                        path.display(),
                        err
                    );
                }
            }
        }

        scripts
    }

    fn script_index_by_path(&self, path: &Path) -> Option<usize> {
        self.scripts
            .iter()
            .position(|script| script.file_path() == path)
    }

    async fn load_reload_candidate(
        engine: &wasmtime::Engine,
        path: &Path,
        script_config: &HashMap<String, toml::Value>,
    ) -> ReloadCandidate {
        let script = match WasmScript::from_file(engine, path).await {
            Ok(script) => script,
            Err(err) => {
                tracing::error!(
                    target: "scripting",
                    "Failed to reload script {}: {:#}",
                    path.display(),
                    err
                );
                return ReloadCandidate::Failed;
            }
        };

        let script_id = script.id().to_string();
        if !Self::is_script_enabled(&script_id, script_config) {
            debug!(
                target: "scripting",
                "Skipping disabled script during reload: {} ({})",
                script.name(),
                script_id
            );
            return ReloadCandidate::Disabled { script_id };
        }

        ReloadCandidate::Loaded(script)
    }

    async fn unload_scripts_by_paths(&mut self, paths: &[PathBuf]) -> usize {
        let mut removed_indices: Vec<usize> = paths
            .iter()
            .filter_map(|path| self.script_index_by_path(path))
            .collect();

        if removed_indices.is_empty() {
            return 0;
        }

        removed_indices.sort_unstable();
        removed_indices.dedup();

        let mut unloaded = 0;
        for index in removed_indices.into_iter().rev() {
            let mut script = self.scripts.remove(index);
            let mut ctx = Self::create_script_context(
                self.client.clone(),
                self.action_tx.clone(),
                &mut self.timer_manager,
                SystemTime::now(),
            );

            debug!(
                target: "scripting",
                "Calling on_unload for: {} ({})",
                script.name(),
                script.id()
            );
            script.on_unload(&mut ctx).await;
            unloaded += 1;
        }

        unloaded
    }

    /// Load scripts from a directory
    pub async fn load_scripts(
        &mut self,
        dir: &std::path::Path,
        script_config: &HashMap<String, toml::Value>,
    ) {
        debug!(target: "scripting", "Loading scripts from {}", dir.display());

        let Some(ref engine) = self.wasm_engine else {
            tracing::warn!(target: "scripting", "Script engine not available, skipping script loading");
            return;
        };

        let scripts = super::wasm::load_wasm_scripts(engine, dir, script_config).await;
        let loaded_count = scripts.len();

        for script in scripts {
            debug!(target: "scripting", "Registering script: {} ({})", script.name(), script.id());
            self.register_script(script).await;
        }

        if loaded_count > 0 {
            info!(target: "scripting", "Loaded {} script(s) (total: {})", loaded_count, self.scripts.len());
        }

        // Store script config and directory for hot reload
        self.script_config = Some(script_config.clone());
        self.script_dir = Some(dir.to_path_buf());
    }

    /// Reload scripts (for hot-reload)
    pub async fn reload_scripts(
        &mut self,
        dir: &Path,
        script_config: &HashMap<String, toml::Value>,
    ) {
        let old_script_count = self.scripts.len();
        debug!(target: "scripting", "Reloading scripts from {}", dir.display());

        let Some(ref engine) = self.wasm_engine else {
            tracing::warn!(target: "scripting", "Script engine not available, skipping script reload");
            return;
        };

        let current_files = Self::current_script_files(dir);
        let mut removed_paths = Vec::new();
        let mut changed_paths = Vec::new();
        let mut added_paths = Vec::new();

        for script in &self.scripts {
            let path = script.file_path().clone();
            match current_files.get(&path) {
                Some(modified_time) if Self::is_script_enabled(script.id(), script_config) => {
                    if &script.modified_time() != modified_time {
                        changed_paths.push(path);
                    }
                }
                Some(_) | None => {
                    removed_paths.push(path);
                }
            }
        }

        for path in current_files.keys() {
            if self.script_index_by_path(path).is_none() {
                added_paths.push(path.clone());
            }
        }

        if removed_paths.is_empty() && changed_paths.is_empty() && added_paths.is_empty() {
            debug!(target: "scripting", "No script changes detected during reload");
            self.script_config = Some(script_config.clone());
            self.script_dir = Some(dir.to_path_buf());
            return;
        }

        changed_paths.sort();
        added_paths.sort();
        removed_paths.sort();

        let mut paths_to_remove = removed_paths;
        let mut pending_scripts = Vec::new();

        for path in changed_paths {
            match Self::load_reload_candidate(engine, &path, script_config).await {
                ReloadCandidate::Loaded(script) => pending_scripts.push((path, script)),
                ReloadCandidate::Disabled { script_id } => {
                    info!(
                        target: "scripting",
                        "Unloading script at {} because {} is disabled",
                        path.display(),
                        script_id
                    );
                    paths_to_remove.push(path);
                }
                ReloadCandidate::Failed => {
                    tracing::warn!(
                        target: "scripting",
                        "Keeping existing script at {} because the updated version failed to load",
                        path.display()
                    );
                }
            }
        }

        for path in added_paths {
            if let ReloadCandidate::Loaded(script) =
                Self::load_reload_candidate(engine, &path, script_config).await
            {
                pending_scripts.push((path, script));
            }
        }

        let stable_ids: std::collections::HashSet<String> = self
            .scripts
            .iter()
            .filter(|script| {
                !paths_to_remove
                    .iter()
                    .any(|path| path == script.file_path())
            })
            .filter(|script| {
                !pending_scripts
                    .iter()
                    .any(|(path, _)| path.as_path() == script.file_path())
            })
            .map(|script| script.id().to_string())
            .collect();

        let mut seen_ids = stable_ids;
        let mut accepted_scripts = Vec::new();

        for (path, script) in pending_scripts {
            let script_id = script.id().to_string();
            if !seen_ids.insert(script_id.clone()) {
                tracing::warn!(
                    target: "scripting",
                    "Skipping reload of {} because script ID '{}' would duplicate an existing script",
                    path.display(),
                    script_id
                );
                continue;
            }

            accepted_scripts.push((path, script));
        }

        let unloaded_count = self.unload_scripts_by_paths(&paths_to_remove).await;
        let mut reloaded_count = 0;
        let mut added_count = 0;

        for (path, mut script) in accepted_scripts {
            let replacing_existing = self.script_index_by_path(&path).is_some();

            debug!(
                target: "scripting",
                "Registering script: {} ({})",
                script.name(),
                script.id()
            );

            let mut ctx = Self::create_script_context(
                self.client.clone(),
                self.action_tx.clone(),
                &mut self.timer_manager,
                SystemTime::now(),
            );
            script.on_load(&mut ctx).await;

            if let Some(index) = self.script_index_by_path(&path) {
                let mut old_script = std::mem::replace(&mut self.scripts[index], script);
                let mut unload_ctx = Self::create_script_context(
                    self.client.clone(),
                    self.action_tx.clone(),
                    &mut self.timer_manager,
                    SystemTime::now(),
                );
                debug!(
                    target: "scripting",
                    "Calling on_unload for replaced script: {} ({})",
                    old_script.name(),
                    old_script.id()
                );
                old_script.on_unload(&mut unload_ctx).await;
            } else {
                self.scripts.push(script);
            }

            if replacing_existing {
                reloaded_count += 1;
            } else {
                added_count += 1;
            }
        }

        let new_script_count = self.scripts.len();

        if old_script_count > 0 && new_script_count == 0 {
            tracing::warn!(
                target: "scripting",
                "Script reload resulted in zero scripts (was {}). Check logs for loading errors.",
                old_script_count
            );
        } else {
            info!(
                target: "scripting",
                "Reloaded scripts: {} replaced, {} added, {} removed (total: {})",
                reloaded_count,
                added_count,
                unloaded_count,
                new_script_count
            );
        }

        self.script_config = Some(script_config.clone());
        self.script_dir = Some(dir.to_path_buf());
    }

    /// Unload all scripts
    pub async fn unload_scripts(&mut self) {
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

        // Unload all scripts
        for script in self.scripts.iter_mut() {
            debug!(target: "scripting", "Calling on_unload for: {} ({})", script.name(), script.id());
            script.on_unload(&mut ctx).await;
        }

        self.scripts.clear();
    }

    /// Process timers and return fired timer IDs
    fn tick_timers(&mut self, now: Instant) -> Vec<(super::timer::TimerId, String)> {
        self.timer_manager.tick(now)
    }

    /// Tick all scripts if enough time has elapsed
    async fn tick_scripts(&mut self, now: Instant) {
        let elapsed = now.duration_since(self.last_tick);

        if elapsed < self.tick_interval {
            return;
        }

        self.last_tick = now;

        for script in &mut self.scripts {
            let mut ctx = Self::create_script_context(
                self.client.clone(),
                self.action_tx.clone(),
                &mut self.timer_manager,
                SystemTime::now(),
            );

            script.on_tick(&mut ctx, elapsed).await;
        }
    }

    /// Handle a raw event
    pub async fn handle_event(&mut self, raw_event: ClientEvent) {
        let now = Instant::now();

        debug!(
            target: "scripting",
            "Event received: {:?}",
            std::mem::discriminant(&raw_event)
        );

        // Tick timers FIRST
        let fired_timers = self.tick_timers(now);
        if !fired_timers.is_empty() {
            debug!(target: "scripting", "Timers fired: {:?}", fired_timers);
        }

        // THEN tick scripts
        self.tick_scripts(now).await;

        // Create context once before the loop
        let mut ctx = Self::create_script_context(
            self.client.clone(),
            self.action_tx.clone(),
            &mut self.timer_manager,
            SystemTime::now(),
        );

        // Dispatch event to each script that's interested
        for script in &mut self.scripts {
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

            script.on_event(&raw_event, &mut ctx).await;
        }
    }
}

impl Drop for ScriptRunner {
    fn drop(&mut self) {
        if self.scripts.is_empty() {
            return;
        }

        // Note: We can't await in Drop, so we just clear scripts
        // The scripts will be dropped and their WASM resources released
        self.scripts.clear();
    }
}

/// Wrapper around ScriptRunner that implements EventConsumer trait
/// Dispatches events to the async runner via a channel
pub struct ScriptConsumer {
    msg_tx: Option<UnboundedSender<RunnerMessage>>,
    runner_task: Option<tokio::task::JoinHandle<()>>,
    script_dir: Option<std::path::PathBuf>,
    script_config: Option<std::collections::HashMap<String, toml::Value>>,
    hot_reload_task: Option<tokio::task::JoinHandle<()>>,
}

impl ScriptConsumer {
    pub fn new(_runner: ScriptRunner) -> Self {
        // The runner is now managed internally, not passed in
        // This constructor is kept for API compatibility
        // Use `new_with_config` instead
        Self {
            msg_tx: None,
            runner_task: None,
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

        // NOTE: Initial loading is deferred to when the runner starts
        // since we don't have a runner instance here anymore

        if hot_reload_enabled {
            let interval = Duration::from_millis(hot_reload_interval_ms);
            info!(
                target: "scripting",
                "Enabling hot reload with interval: {}ms",
                hot_reload_interval_ms
            );

            let script_dir = script_dir.clone();
            let script_config = script_config.clone();
            let msg_tx_for_scanner = self.msg_tx.clone();

            let task = tokio::spawn(async move {
                Self::hot_reload_task(msg_tx_for_scanner, script_dir, script_config, interval)
                    .await;
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
    async fn hot_reload_task(
        msg_tx: Option<UnboundedSender<RunnerMessage>>,
        script_dir: std::path::PathBuf,
        script_config: std::collections::HashMap<String, toml::Value>,
        interval: Duration,
    ) {
        let mut interval_timer = tokio::time::interval(interval);
        interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        info!(
            target: "scripting",
            "Hot reload background task started, scanning every {:?}",
            interval
        );

        let mut scanner = ScriptScanner::with_interval(script_dir.clone(), interval);

        loop {
            interval_timer.tick().await;

            debug!(target: "scripting", "Hot reload: scanning for changes...");

            let changes = scanner.scan_changes();

            if changes.has_changes() {
                info!(
                    target: "scripting",
                    "Hot reload: detected changes - {} modified, {} added, {} removed",
                    changes.changed.len(),
                    changes.added.len(),
                    changes.removed.len()
                );

                if let Some(ref tx) = msg_tx {
                    let _ = tx.send(RunnerMessage::Reload {
                        dir: script_dir.clone(),
                        script_config: script_config.clone(),
                    });
                }
            }
        }
    }

    /// Start the runner task. Call this after construction.
    pub fn start(
        &mut self,
        client: Arc<RwLock<Client>>,
        action_tx: UnboundedSender<SimpleClientAction>,
        scripting_config: &ScriptingConfig,
    ) {
        let mut runner = create_runner_from_config(client, action_tx, scripting_config);

        let (msg_tx, mut msg_rx) = tokio::sync::mpsc::unbounded_channel::<RunnerMessage>();
        self.msg_tx = Some(msg_tx.clone());

        // Load initial scripts if scripting is enabled
        let script_dir = if scripting_config.enabled {
            Some(scripting_config.script_dir())
        } else {
            None
        };
        let script_config = scripting_config.config.clone();
        let hot_reload = scripting_config.hot_reload;
        let hot_reload_interval = scripting_config.hot_reload_interval_ms;

        // Clone for hot reload setup (moved into async block below)
        let hot_reload_dir = script_dir.clone();
        let hot_reload_config = script_config.clone();
        let hot_reload_msg_tx = msg_tx.clone();

        self.runner_task = Some(tokio::spawn(async move {
            // Load initial scripts
            if let Some(ref dir) = script_dir {
                runner.load_scripts(dir, &script_config).await;
            }

            // Start tick timer
            let mut tick_interval = tokio::time::interval(runner.tick_interval);
            tick_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    Some(msg) = msg_rx.recv() => {
                        match msg {
                            RunnerMessage::Event(event) => {
                                runner.handle_event(event).await;
                            }
                            RunnerMessage::Reload { dir, script_config } => {
                                runner.reload_scripts(&dir, &script_config).await;
                            }
                        }
                    }
                    _ = tick_interval.tick() => {
                        let now = Instant::now();
                        runner.tick_timers(now);
                        runner.tick_scripts(now).await;
                    }
                    else => break,
                }
            }
        }));

        // Update hot reload task with the new msg_tx
        if hot_reload && let Some(dir) = hot_reload_dir {
            let interval = Duration::from_millis(hot_reload_interval);
            info!(
                target: "scripting",
                "Enabling hot reload with interval: {}ms",
                hot_reload_interval
            );

            let task = tokio::spawn(async move {
                Self::hot_reload_task(Some(hot_reload_msg_tx), dir, hot_reload_config, interval)
                    .await;
            });

            self.hot_reload_task = Some(task);
        }
    }
}

impl EventConsumer for ScriptConsumer {
    fn handle_event(&mut self, envelope: EventEnvelope) {
        let Some(ref tx) = self.msg_tx else {
            tracing::error!(target: "scripting", "ScriptConsumer not started — call start() first");
            return;
        };

        // Check for reload event
        if let gromnie_events::EventType::System(gromnie_events::SystemEvent::ReloadScripts {
            ..
        }) = &envelope.event
        {
            if let Some(ref script_dir) = self.script_dir {
                if let Some(ref script_config) = self.script_config {
                    let _ = tx.send(RunnerMessage::Reload {
                        dir: script_dir.clone(),
                        script_config: script_config.clone(),
                    });
                } else {
                    tracing::warn!(target: "scripting", "ReloadScripts event received but script_config is None");
                }
            } else {
                tracing::warn!(target: "scripting", "ReloadScripts event received but script_dir is None");
            }
            return;
        }

        // Extract ClientEvent from EventEnvelope
        let client_event = match envelope.event {
            gromnie_events::EventType::Game(game_event) => ClientEvent::Game(game_event),
            gromnie_events::EventType::State(state_event) => ClientEvent::State(state_event),
            gromnie_events::EventType::System(system_event) => match system_event {
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
                gromnie_events::SystemEvent::ReloadScripts { .. }
                | gromnie_events::SystemEvent::LogScriptMessage { .. }
                | gromnie_events::SystemEvent::Shutdown => {
                    return;
                }
            },
        };

        let _ = tx.send(RunnerMessage::Event(client_event));
    }
}

impl Drop for ScriptConsumer {
    fn drop(&mut self) {
        if let Some(task) = self.runner_task.take() {
            task.abort();
        }
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
    let mut consumer = ScriptConsumer::new(ScriptRunner::new(client.clone(), action_tx.clone()));
    consumer.start(client, action_tx, scripting_config);
    consumer
}

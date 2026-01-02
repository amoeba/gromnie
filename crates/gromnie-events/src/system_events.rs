#[derive(Debug, Clone)]
pub enum SystemEvent {
    /// Reload scripts (internal use for hot-reload)
    ReloadScripts { script_dir: std::path::PathBuf },
    /// Log a message from a script
    LogScriptMessage { script_id: String, message: String },
    /// Authentication succeeded for a client
    AuthenticationSucceeded { client_id: u32 },
    /// Authentication failed for a client
    AuthenticationFailed { client_id: u32, reason: String },
    /// Client started connecting phase
    ConnectingStarted { client_id: u32 },
    /// Client finished connecting phase
    ConnectingDone { client_id: u32 },
    /// Client started updating/patching phase
    UpdatingStarted { client_id: u32 },
    /// Client finished updating/patching phase
    UpdatingDone { client_id: u32 },
    /// Character login succeeded
    LoginSucceeded { character_id: u32, character_name: String },
    /// System shutdown requested
    Shutdown,
}

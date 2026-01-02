/// Types of script-related events
#[derive(Debug, Clone)]
pub enum ScriptEventType {
    Loaded,
    Unloaded,
    Error { message: String },
    Log { message: String },
}

// Generate guest bindings from WIT
wit_bindgen::generate!({
    path: "../../wit",
    world: "script",
});

// Re-export types for convenience
pub use exports::gromnie::scripting::guest::Guest;
pub use gromnie::scripting::host;

// Make the export macro public by creating a wrapper
#[macro_export]
macro_rules! export_script {
    ($t:ident) => {
        $crate::export!($t with_types_in $crate);
    };
}

// Event filter constants (matching the discriminants in EventFilter enum)
pub const EVENT_ALL: u32 = 0;
pub const EVENT_CHARACTER_LIST_RECEIVED: u32 = 1;
pub const EVENT_CREATE_OBJECT: u32 = 2;
pub const EVENT_CHAT_MESSAGE_RECEIVED: u32 = 3;

// Legacy constants removed (no longer available in WIT):
// - EVENT_LOGIN_SUCCEEDED
// - EVENT_LOGIN_FAILED
// - EVENT_DDD_INTERROGATION
// - EVENT_NETWORK_MESSAGE
// - EVENT_CONNECTING_*
// - EVENT_UPDATING_*
// - EVENT_AUTHENTICATION_*

/// Event filter constants corresponding to event discriminants
/// These match the discriminants in the EventFilter enum in gromnie-scripting-host
///
/// Filter 0: All events
pub const EVENT_ALL: u32 = 0;

// Game events (1-99)
pub const EVENT_CHARACTER_LIST_RECEIVED: u32 = 1;
pub const EVENT_CREATE_OBJECT: u32 = 2;
pub const EVENT_CHAT_MESSAGE_RECEIVED: u32 = 3;

// State events (100-199)
pub const EVENT_STATE_CONNECTING: u32 = 100;
pub const EVENT_STATE_CONNECTED: u32 = 101;
pub const EVENT_STATE_CONNECTING_FAILED: u32 = 102;
pub const EVENT_STATE_PATCHING: u32 = 103;
pub const EVENT_STATE_PATCHED: u32 = 104;
pub const EVENT_STATE_PATCHING_FAILED: u32 = 105;
pub const EVENT_STATE_CHARACTER_SELECT: u32 = 106;
pub const EVENT_STATE_ENTERING_WORLD: u32 = 107;
pub const EVENT_STATE_IN_WORLD: u32 = 108;
pub const EVENT_STATE_EXITING_WORLD: u32 = 109;
pub const EVENT_STATE_CHARACTER_ERROR: u32 = 110;

// System events (200-299)
pub const EVENT_SYSTEM_AUTHENTICATION_SUCCEEDED: u32 = 200;
pub const EVENT_SYSTEM_AUTHENTICATION_FAILED: u32 = 201;
pub const EVENT_SYSTEM_CONNECTING_STARTED: u32 = 202;
pub const EVENT_SYSTEM_CONNECTING_DONE: u32 = 203;
pub const EVENT_SYSTEM_UPDATING_STARTED: u32 = 204;
pub const EVENT_SYSTEM_UPDATING_DONE: u32 = 205;
pub const EVENT_SYSTEM_LOGIN_SUCCEEDED: u32 = 206;
pub const EVENT_SYSTEM_RELOAD_SCRIPTS: u32 = 207;
pub const EVENT_SYSTEM_SHUTDOWN: u32 = 208;

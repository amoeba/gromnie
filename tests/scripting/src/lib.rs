// Test script that exercises all major scripting functionality
// This will be compiled to WASM for testing the scripting host

// Generate bindings from WIT
wit_bindgen::generate!({
    path: "../../crates/gromnie-scripting-api/src/wit",
    world: "script",
});

// Import the generated bindings and host functions
use exports::gromnie::scripting::guest::Guest;

// The wit_bindgen::generate! macro generates a `gromnie` module
// with all the host functions
use gromnie::scripting::host;

#[derive(Default)]
pub struct TestScript {
    load_called: bool,
    unload_called: bool,
    event_count: u32,
    tick_count: u32,
    last_event: Option<String>,
}

impl TestScript {
    pub fn new() -> Self {
        Self::default()
    }
}

// Implement the Guest trait as required by WIT
impl Guest for TestScript {
    fn init() {
        // Initialize the script (called first)
        host::log("Test script initialized");
    }

    fn get_id() -> String {
        "test_script".to_string()
    }

    fn get_name() -> String {
        "Test Script".to_string()
    }

    fn get_description() -> String {
        "Comprehensive test script for scripting system".to_string()
    }

    fn on_load() {
        host::log("Test script loaded successfully");
        host::send_chat("Hello from test script!");
    }

    fn on_unload() {
        host::log("Test script unloaded successfully");
    }

    fn subscribed_events() -> Vec<u32> {
        // Subscribe to all event types for testing
        vec![0xFFFFFFFF] // All events
    }

    fn on_event(event: host::ScriptEvent) {
        host::log(&format!("Received event: {:?}", event));

        match event {
            host::ScriptEvent::Game(game_event) => match game_event {
                host::GameEvent::CharacterListReceived(account_data) => {
                    host::log(&format!(
                        "Character list received for account: {} with {} characters",
                        account_data.account,
                        account_data.characters.len()
                    ));
                }
                host::GameEvent::CharacterError(error_data) => {
                    host::log(&format!(
                        "Character error: code={}, msg={}",
                        error_data.error_code, error_data.error_message
                    ));
                }
                host::GameEvent::CreateObject(object_data) => {
                    host::log(&format!(
                        "Object created: {} (ID: {})",
                        object_data.name, object_data.id
                    ));
                }
                host::GameEvent::ChatMessageReceived(chat_data) => {
                    host::log(&format!("Chat message: {}", chat_data.message));
                }
                host::GameEvent::Protocol(protocol_event) => {
                    host::log(&format!("Protocol event received"));
                }
            },
            host::ScriptEvent::State(state_event) => {
                host::log(&format!("State event: {:?}", state_event));
            }
            host::ScriptEvent::System(system_event) => match system_event {
                host::SystemEvent::AuthenticationSucceeded => {
                    host::log("System: AuthenticationSucceeded");
                }
                host::SystemEvent::LoginSucceeded(login_info) => {
                    host::log(&format!(
                        "System: LoginSucceeded - {} (ID: {})",
                        login_info.character_name, login_info.character_id
                    ));
                }
                _ => {
                    host::log(&format!("System event: {:?}", system_event));
                }
            },
        }
    }

    fn on_tick(delta_millis: u64) {
        // Test periodic functionality
        host::log(&format!("Tick: {}ms", delta_millis));

        // Test client state access
        let state = host::get_client_state();
        host::log(&format!(
            "Client state: session={:?}, scene={:?}",
            state.session.state, state.scene
        ));
    }
}

export!(TestScript);

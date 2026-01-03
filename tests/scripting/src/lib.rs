// Test script that exercises all major scripting functionality
// This will be compiled to WASM for testing the scripting host

// Generate WIT bindings for this WASM component
// Note: Each WASM component must generate its own bindings per the WIT component model.
// We reference the WIT definitions from gromnie-scripting-api to avoid duplication.
wit_bindgen::generate!({
    path: "../../crates/gromnie-scripting-api/src/wit",
    world: "script",
});

// Import the generated Guest trait and host interface
use exports::gromnie::scripting::guest::Guest;
use gromnie::scripting::host;

/// Helper function to handle protocol events with detailed pattern matching
/// This demonstrates how scripts can access strongly-typed protocol events
fn handle_protocol_event(protocol_event: host::ProtocolEvent) {
    match protocol_event {
        // Top-level S2C messages
        host::ProtocolEvent::S2c(s2c_event) => match s2c_event {
            host::S2cEvent::LoginCreatePlayer(msg) => {
                host::log(&format!(
                    "[Protocol] LoginCreatePlayer - Character ID: 0x{:08X}",
                    msg.character_id
                ));
            }
            host::S2cEvent::LoginCharacterSet(msg) => {
                host::log(&format!(
                    "[Protocol] LoginCharacterSet - Account: {}, {} characters, {} slots",
                    msg.account,
                    msg.characters.len(),
                    msg.num_slots
                ));
                for character in msg.characters {
                    let status = if character.delete_pending {
                        "PENDING DELETION"
                    } else {
                        "Active"
                    };
                    host::log(&format!(
                        "  - {} (ID: 0x{:08X}) [{}]",
                        character.name, character.id, status
                    ));
                }
            }
            host::S2cEvent::ItemCreateObject(msg) => {
                host::log(&format!(
                    "[Protocol] ItemCreateObject - {} (ID: 0x{:08X})",
                    msg.name, msg.object_id
                ));
            }
            host::S2cEvent::CharacterError(err) => {
                host::log(&format!(
                    "[Protocol] CharacterError - Code: 0x{:04X}, Message: {}",
                    err.error_code, err.error_message
                ));
            }
            host::S2cEvent::HearSpeech(msg) => {
                host::log(&format!(
                    "[Protocol] HearSpeech - {} says: \"{}\" (type: 0x{:02X})",
                    msg.sender_name, msg.message, msg.message_type
                ));
            }
            host::S2cEvent::HearRangedSpeech(msg) => {
                host::log(&format!(
                    "[Protocol] HearRangedSpeech - {} says: \"{}\" (type: 0x{:02X})",
                    msg.sender_name, msg.message, msg.message_type
                ));
            }
            host::S2cEvent::DddInterrogation(msg) => {
                host::log(&format!(
                    "[Protocol] DDDInterrogation - Language: {}, Region: {}, Product: {}",
                    msg.language, msg.region, msg.product
                ));
            }
            host::S2cEvent::ChargenVerificationResponse => {
                host::log("[Protocol] CharGenVerificationResponse - Character creation verified");
            }
        },
        // Nested game events with metadata
        host::ProtocolEvent::GameEvent(game_event) => {
            host::log(&format!(
                "[Protocol] GameEvent - Object: 0x{:08X}, Sequence: {}",
                game_event.object_id, game_event.sequence
            ));
            match game_event.event {
                host::GameEventMsg::HearDirectSpeech(msg) => {
                    host::log(&format!(
                        "  -> HearDirectSpeech: {} (0x{:08X}) tells you: \"{}\" (type: 0x{:02X})",
                        msg.sender_name, msg.sender_id, msg.message, msg.message_type
                    ));
                }
                host::GameEventMsg::TransientString(msg) => {
                    host::log(&format!("  -> TransientString: {}", msg.message));
                }
            }
        }
    }
}

#[derive(Default)]
pub struct TestScript;

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
                    // Demonstrate full protocol event handling
                    handle_protocol_event(protocol_event);
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

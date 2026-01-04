// Test script that exercises all major scripting functionality
// This will be compiled to WASM for testing the scripting host

use gromnie::ScriptEvent;
use gromnie_scripting_api as gromnie;

/// Helper function to handle protocol events with detailed pattern matching
/// This demonstrates how scripts can access strongly-typed protocol events
fn handle_protocol_event(protocol_event: gromnie::ProtocolEvent) {
    use gromnie::host::*;

    match protocol_event {
        // Top-level S2C messages
        ProtocolEvent::S2c(s2c_event) => match s2c_event {
            S2cEvent::LoginCreatePlayer(msg) => {
                gromnie::log(&format!(
                    "[Protocol] LoginCreatePlayer - Character ID: 0x{:08X}",
                    msg.character_id
                ));
            }
            S2cEvent::LoginCharacterSet(msg) => {
                gromnie::log(&format!(
                    "[Protocol] LoginCharacterSet - Account: {}, {} characters, {} slots",
                    msg.account,
                    msg.characters.len(),
                    msg.num_slots
                ));
                for character in msg.characters {
                    host::log(&format!(
                        "  - {} (ID: 0x{:08X})",
                        character.name, character.character_id
                    ));
                }
            }
            S2cEvent::ItemCreateObject(msg) => {
                gromnie::log(&format!(
                    "[Protocol] ItemCreateObject - {} (ID: 0x{:08X})",
                    msg.name, msg.object_id
                ));
            }
            S2cEvent::CharacterError(err) => {
                gromnie::log(&format!(
                    "[Protocol] CharacterError - Code: 0x{:04X}, Message: {}",
                    err.error_code, err.error_message
                ));
            }
            S2cEvent::HearSpeech(msg) => {
                gromnie::log(&format!(
                    "[Protocol] HearSpeech - {} says: \"{}\" (type: 0x{:02X})",
                    msg.sender_name, msg.message, msg.message_type
                ));
            }
            S2cEvent::HearRangedSpeech(msg) => {
                gromnie::log(&format!(
                    "[Protocol] HearRangedSpeech - {} says: \"{}\" (type: 0x{:02X})",
                    msg.sender_name, msg.message, msg.message_type
                ));
            }
            S2cEvent::DddInterrogation(msg) => {
                gromnie::log(&format!(
                    "[Protocol] DDDInterrogation - Language: {}, Region: {}, Product: {}",
                    msg.language, msg.region, msg.product
                ));
            }
            S2cEvent::ChargenVerificationResponse => {
                gromnie::log(
                    "[Protocol] CharGenVerificationResponse - Character creation verified",
                );
            }
        },
        // Nested game events with metadata
        ProtocolEvent::GameEvent(game_event) => {
            gromnie::log(&format!(
                "[Protocol] GameEvent - Object: 0x{:08X}, Sequence: {}",
                game_event.object_id, game_event.sequence
            ));
            match game_event.event {
                GameEventMsg::HearDirectSpeech(msg) => {
                    gromnie::log(&format!(
                        "  -> HearDirectSpeech: {} (0x{:08X}) tells you: \"{}\" (type: 0x{:02X})",
                        msg.sender_name, msg.sender_id, msg.message, msg.message_type
                    ));
                }
                GameEventMsg::TransientString(msg) => {
                    gromnie::log(&format!("  -> TransientString: {}", msg.message));
                }
            }
        }
    }
}

#[derive(Default)]
pub struct TestScript;

impl gromnie::Script for TestScript {
    fn new() -> Self {
        Self
    }

    fn id(&self) -> &str {
        "test_script"
    }

    fn name(&self) -> &str {
        "Test Script"
    }

    fn description(&self) -> &str {
        "Comprehensive test script for scripting system"
    }

    fn on_load(&mut self) {
        gromnie::log("Test script loaded successfully");
        gromnie::send_chat("Hello from test script!");
    }

    fn on_unload(&mut self) {
        gromnie::log("Test script unloaded successfully");
    }

    fn subscribed_events(&self) -> Vec<u32> {
        // Subscribe to all event types for testing
        vec![0xFFFFFFFF] // All events
    }

    fn on_event(&mut self, event: ScriptEvent) {
        use gromnie::GameEvent::*;
        use gromnie::ScriptEvent::*;
        use gromnie::SystemEvent::*;

        match event {
            Game(game_event) => match game_event {
                CharacterListReceived(account_data) => {
                    gromnie::log(&format!(
                        "Character list received for account: {} with {} characters",
                        account_data.account,
                        account_data.characters.len()
                    ));
                }
                CharacterError(error_data) => {
                    gromnie::log(&format!(
                        "Character error: code={}, msg={}",
                        error_data.error_code, error_data.error_message
                    ));
                }
                CreateObject(object_data) => {
                    gromnie::log(&format!(
                        "Object created: {} (ID: {})",
                        object_data.name, object_data.id
                    ));
                }
                ChatMessageReceived(chat_data) => {
                    gromnie::log(&format!("Chat message: {}", chat_data.message));
                }
                Protocol(protocol_event) => {
                    // Demonstrate full protocol event handling
                    handle_protocol_event(protocol_event);
                }
            },
            State(state_event) => {
                gromnie::log(&format!("State event: {:?}", state_event));
            }
            System(system_event) => match system_event {
                AuthenticationSucceeded => {
                    gromnie::log("System: AuthenticationSucceeded");
                }
                LoginSucceeded(login_info) => {
                    gromnie::log(&format!(
                        "System: LoginSucceeded - {} (ID: {})",
                        login_info.character_name, login_info.character_id
                    ));
                }
                _ => {
                    gromnie::log(&format!("System event: {:?}", system_event));
                }
            },
        }
    }

    fn on_tick(&mut self, delta_millis: u64) {
        // Test periodic functionality
        gromnie::log(&format!("Tick: {}ms", delta_millis));

        // Test client state access
        let state = gromnie::get_client_state();
        gromnie::log(&format!(
            "Client state: session={:?}, scene={:?}",
            state.session.state, state.scene
        ));
    }
}

gromnie::register_script!(TestScript);

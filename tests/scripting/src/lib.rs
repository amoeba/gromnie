// Test script that exercises all major scripting functionality
// This will be compiled to WASM for testing the scripting host

use gromnie::host_interface::ProtocolEvent;
use gromnie::ScriptEvent;
use gromnie_scripting_api as gromnie;

/// Helper function to handle protocol events with detailed pattern matching
/// This demonstrates how scripts can access strongly-typed protocol events
fn handle_protocol_event(protocol_event: ProtocolEvent) {
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
                    gromnie::log(&format!(
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
            S2cEvent::MovementPosition(msg) => {
                gromnie::log(&format!(
                    "[Protocol] MovementPosition - obj=0x{:08X} cell=0x{:08X} ({:.2}, {:.2}, {:.2})",
                    msg.object_id, msg.position.landcell,
                    msg.position.x, msg.position.y, msg.position.z
                ));
            }
            S2cEvent::MovementPositionAndMovement(msg) => {
                gromnie::log(&format!(
                    "[Protocol] MovementPositionAndMovement - obj=0x{:08X} cell=0x{:08X} ({:.2}, {:.2}, {:.2})",
                    msg.object_id, msg.position.landcell,
                    msg.position.x, msg.position.y, msg.position.z
                ));
            }
            S2cEvent::MovementSetObjectMovement(msg) => {
                gromnie::log(&format!(
                    "[Protocol] MovementSetObjectMovement - obj=0x{:08X} seq={}",
                    msg.object_id, msg.object_instance_sequence
                ));
            }
            S2cEvent::EffectsPlayerTeleport(msg) => {
                gromnie::log(&format!(
                    "[Protocol] EffectsPlayerTeleport - seq={}",
                    msg.object_teleport_sequence
                ));
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
                _ => {}
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

    fn on_load<'a>(&'a mut self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'a>> {
        Box::pin(async move {
            gromnie::log("Test script loaded successfully");
            gromnie::send_chat("Hello from test script!");
        })
    }

    fn on_unload<'a>(&'a mut self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'a>> {
        Box::pin(async move {
            gromnie::log("Test script unloaded successfully");
        })
    }

    fn subscribed_events(&self) -> Vec<u32> {
        // Subscribe to all event types for testing
        vec![0xFFFFFFFF] // All events
    }

    fn on_event<'a>(&'a mut self, event: ScriptEvent) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'a>> {
        Box::pin(async move {
            use gromnie::GameEvent::*;
            use gromnie::ScriptEvent::*;
            use gromnie::SystemEvent::*;

            match event {
                Game(game_event) => match game_event {
                    CharacterListReceived(account_data) => {
                        let msg = format!(
                            "Character list received for account: {} with {} characters",
                            account_data.account,
                            account_data.characters.len()
                        );
                        gromnie::log(&msg);
                    }
                    CharacterError(error_data) => {
                        let msg = format!(
                            "Character error: code={}, msg={}",
                            error_data.error_code, error_data.error_message
                        );
                        gromnie::log(&msg);
                    }
                    CreateObject(object_data) => {
                        let msg = format!(
                            "Object created: {} (ID: {})",
                            object_data.name, object_data.id
                        );
                        gromnie::log(&msg);
                    }
                    ChatMessageReceived(chat_data) => {
                        let msg = format!("Chat message: {}", chat_data.message);
                        gromnie::log(&msg);
                    }
                    Protocol(protocol_event) => {
                        // Demonstrate full protocol event handling
                        handle_protocol_event(protocol_event);
                    }
                },
                State(state_event) => {
                    let msg = format!("State event: {:?}", state_event);
                    gromnie::log(&msg);
                }
                System(system_event) => match system_event {
                    AuthenticationSucceeded => {
                        gromnie::log("System: AuthenticationSucceeded");
                    }
                    LoginSucceeded(login_info) => {
                        let msg = format!(
                            "System: LoginSucceeded - {} (ID: {})",
                            login_info.character_name, login_info.character_id
                        );
                        gromnie::log(&msg);
                    }
                    _ => {
                        let msg = format!("System event: {:?}", system_event);
                        gromnie::log(&msg);
                    }
                },
            }
        })
    }

    fn on_tick<'a>(&'a mut self, delta_millis: u64) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'a>> {
        Box::pin(async move {
            // Test periodic functionality
            let msg = format!("Tick: {}ms", delta_millis);
            gromnie::log(&msg);

            // Test client state access
            let state = gromnie::get_client_state();
            let msg = format!(
                "Client state: session={:?}, scene={:?}",
                state.session.state, state.scene
            );
            gromnie::log(&msg);
        })
    }
}

gromnie::register_script!(TestScript);

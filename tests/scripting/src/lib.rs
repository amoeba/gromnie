// Test script that exercises all major scripting functionality
// This will be compiled to WASM for testing the scripting host

use gromnie::*;
use gromnie_scripting_api as gromnie;

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

impl Script for TestScript {
    fn new() -> Self {
        TestScript::new()
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
        self.load_called = true;
        log("Test script loaded successfully");

        // Test basic host functions
        log("Test script loaded successfully");
        send_chat("Hello from test script!");
    }

    fn on_unload(&mut self) {
        self.unload_called = true;
        log("Test script unloaded successfully");
    }

    fn subscribed_events(&self) -> Vec<u32> {
        // Subscribe to all event types for testing
        vec![gromnie::events::EVENT_ALL] // All events
    }

    fn on_event(&mut self, event: gromnie::ScriptEvent) {
        self.event_count += 1;

        match event {
            gromnie::ScriptEvent::Game(game_event) => match game_event {
                gromnie::GameEvent::CharacterListReceived(account_data) => {
                    self.last_event = Some(format!(
                        "Game/CharacterList: {} chars",
                        account_data.character_list.len()
                    ));
                    log(&format!(
                        "Received character list for account: {}",
                        account_data.name
                    ));
                }
                gromnie::GameEvent::CharacterError(error_data) => {
                    self.last_event = Some(format!(
                        "Game/CharacterError: code={}, msg={}",
                        error_data.error_code, error_data.error_message
                    ));
                    log(&format!(
                        "Character error: code={}, msg={}",
                        error_data.error_code, error_data.error_message
                    ));
                }
                gromnie::GameEvent::CreateObject(object_data) => {
                    self.last_event = Some(format!("Game/CreateObject: {}", object_data.name));
                    log(&format!(
                        "Object created: {} (ID: {})",
                        object_data.name, object_data.id
                    ));
                }
                gromnie::GameEvent::ChatMessageReceived(chat_data) => {
                    self.last_event = Some(format!("Game/Chat: {}", chat_data.message));
                    log(&format!("Chat message: {}", chat_data.message));
                }
            },
            gromnie::ScriptEvent::State(state_event) => match state_event {
                gromnie::StateEvent::Connecting => {
                    self.last_event = Some("State/Connecting".to_string());
                    log("State: Connecting");
                }
                gromnie::StateEvent::Connected => {
                    self.last_event = Some("State/Connected".to_string());
                    log("State: Connected");
                }
                gromnie::StateEvent::InWorld => {
                    self.last_event = Some("State/InWorld".to_string());
                    log("State: InWorld");
                }
                _ => {
                    self.last_event = Some(format!("State/Other: {:?}", state_event));
                    log(&format!("State event: {:?}", state_event));
                }
            },
            gromnie::ScriptEvent::System(system_event) => match system_event {
                gromnie::SystemEvent::AuthenticationSucceeded => {
                    self.last_event = Some("System/AuthSucceeded".to_string());
                    log("System: AuthenticationSucceeded");
                }
                gromnie::SystemEvent::LoginSucceeded(login_info) => {
                    self.last_event = Some(format!("System/Login: {}", login_info.character_name));
                    log(&format!(
                        "System: LoginSucceeded - {} (ID: {})",
                        login_info.character_name, login_info.character_id
                    ));
                }
                _ => {
                    self.last_event = Some(format!("System/Other: {:?}", system_event));
                    log(&format!("System event: {:?}", system_event));
                }
            },
        }
    }

    fn on_tick(&mut self, _delta_millis: u64) {
        self.tick_count += 1;

        // Every 10 ticks, test some functionality
        if self.tick_count.is_multiple_of(10) {
            // Test client state access
            let state = get_client_state();
            log(&format!(
                "Tick {} - Client state: {:?}",
                self.tick_count, state
            ));

            // Simple test every 10 ticks
            log(&format!("Tick count: {}", self.tick_count));
        }
    }
}

// Register the script
register_script!(TestScript);

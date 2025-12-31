// Test script that exercises all major scripting functionality
// This will be compiled to WASM for testing the scripting host

use gromnie::*;
use gromnie_scripting_api as gromnie;

pub struct TestScript {
    load_called: bool,
    unload_called: bool,
    event_count: u32,
    tick_count: u32,
    last_event: Option<String>,
}

impl TestScript {
    pub fn new() -> Self {
        Self {
            load_called: false,
            unload_called: false,
            event_count: 0,
            tick_count: 0,
            last_event: None,
        }
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
        vec![0, 1, 2, 3] // All, CharacterListReceived, CreateObject, ChatMessageReceived
    }

    fn on_event(&mut self, event: GameEvent) {
        self.event_count += 1;

        match event {
            GameEvent::CharacterListReceived(account_data) => {
                self.last_event = Some(format!(
                    "CharacterList: {} chars",
                    account_data.character_list.len()
                ));
                log(&format!(
                    "Received character list for account: {}",
                    account_data.name
                ));
            }
            GameEvent::CreateObject(object_data) => {
                self.last_event = Some(format!("CreateObject: {}", object_data.name));
                log(&format!(
                    "Object created: {} (ID: {})",
                    object_data.name, object_data.id
                ));
            }
            GameEvent::ChatMessageReceived(chat_data) => {
                self.last_event = Some(format!("Chat: {}", chat_data.message));
                log(&format!("Chat message: {}", chat_data.message));
            }
            // All event variants are handled above, so this should never be reached
            // Keep for future-proofing if new event types are added
            _ => {
                self.last_event = Some("Unknown event".to_string());
                log("Received unknown event type");
            }
        }
    }

    fn on_tick(&mut self, _delta_millis: u64) {
        self.tick_count += 1;

        // Every 10 ticks, test some functionality
        if self.tick_count % 10 == 0 {
            // Test client state access
            let state = get_client_state();
            log(&format!(
                "Tick {} - Client state: {:?}",
                self.tick_count, state
            ));

            // Simple test every 10 ticks
            if self.tick_count % 10 == 0 {
                log(&format!("Tick count: {}", self.tick_count));
            }
        }
    }
}

// Register the script
register_script!(TestScript);

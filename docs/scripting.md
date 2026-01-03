# Scripting

Gromnie scripts are WASM components that can respond to game events, send actions to client, and query client state.

## Client State

Scripts can access state of the client in two ways:

1. Listen for client state change events
2. Get a snapshot with `host::get_client_state()`

### Client State Events

Scripts receive state change events through the `on_event` callback. Here's how to handle them:

```rs
use gromnie_scripting_api as gromnie;
use gromnie::host;

fn on_event(&mut self, event: gromnie::ScriptEvent) {
    match event {
        // State events - UI/scene changes
        gromnie::ScriptEvent::State(state_event) => match state_event {
            host::StateEvent::InWorld => {
                host::log("Now in-game!");
                // Start doing in-world tasks
            }
            host::StateEvent::CharacterSelect => {
                host::log("At character select screen");
                let state = host::get_client_state();
                if let host::Scene::CharacterSelect(select) = &state.scene {
                    host::log(&format!(
                        "Account: {}, {} characters available",
                        select.account_name,
                        select.characters.len()
                    ));
                }
            }
            host::StateEvent::Connecting => {
                host::log("Connecting to server...");
            }
            host::StateEvent::ConnectingFailed(err) => {
                host::log(&format!("Connection failed: {}", err));
            }
            _ => {}
        },

        // System events - lifecycle and auth
        gromnie::ScriptEvent::System(system_event) => match system_event {
            host::SystemEvent::AuthenticationSucceeded => {
                host::log("Authenticated successfully!");
            }
            host::SystemEvent::LoginSucceeded(info) => {
                host::log(&format!(
                    "Logged in as {} (ID: {})",
                    info.character_name, info.character_id
                ));
            }
            host::SystemEvent::Disconnected => {
                host::log("Disconnected from server");
            }
            host::SystemEvent::Reconnecting => {
                host::log("Attempting to reconnect...");
            }
            host::SystemEvent::Shutdown => {
                host::log("Shutting down");
            }
            _ => {}
        },

        // Game events - see protocol events below
        _ => {}
    }
}
```

### Client Snapshots

Use `host::get_client_state()` to query current client state at any time:

```rs
// Get current state snapshot
let state = host::get_client_state();

// Check protocol connection state
match state.session.state {
    host::SessionState::WorldConnected => {
        host::log("Connected to world server");
        // Safe to send game actions
    }
    host::SessionState::AuthConnected => {
        host::log("Connected to auth server");
        // Waiting to select character
    }
    _ => {
        host::log("Not connected yet");
    }
}

// Check UI scene state
match &state.scene {
    host::Scene::Connecting(conn) => {
        host::log(&format!(
            "Connecting: patch progress = {:?}",
            conn.patch_progress
        ));
    }
    host::Scene::CharacterSelect(select) => {
        host::log(&format!("Account: {}", select.account_name));
        for char in &select.characters {
            host::log(&format!(
                "  - {} (ID: 0x{:08X})",
                char.name, char.id
            ));
        }

        // Auto-login with specific character
        if let Some(char) = select.characters.iter().find(|c| c.name == "MyCharacter") {
            host::login_character(
                select.account_name.clone(),
                char.id,
                char.name.clone()
            );
        }
    }
    host::Scene::InWorld(in_world) => {
        host::log(&format!(
            "Playing as {} (ID: {})",
            in_world.character_name, in_world.character_id
        ));
        // Game active - can send chat, etc.
    }
    host::Scene::Error(err) => {
        host::log(&format!("Error: {:?}", err.error));
        if err.can_retry {
            host::log("Can retry");
        }
    }
    _ => {}
}
```

### Example: Complete Script Structure

Here's a complete example showing how to put it all together:

```rs
use gromnie_scripting_api as gromnie;
use gromnie::host;

wit_bindgen::generate!({
    path: "../../crates/gromnie-scripting-api/src/wit",
    world: "script",
});

use exports::gromnie::scripting::guest::Guest;

#[derive(Default)]
pub struct MyScript;

impl MyScript {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Guest for MyScript {
    fn init() {
        host::log("Script initialized");
    }

    fn get_id() -> String {
        "my_script".to_string()
    }

    fn get_name() -> String {
        "My Script".to_string()
    }

    fn get_description() -> String {
        "A simple example script".to_string()
    }

    fn on_load() {
        host::log("Script loaded!");
    }

    fn on_unload() {
        host::log("Script unloading...");
    }

    fn subscribed_events() -> Vec<u32> {
        // Subscribe to all events
        vec![0xFFFFFFFF]
    }

    fn on_event(event: gromnie::ScriptEvent) {
        match event {
            gromnie::ScriptEvent::Game(game_event) => {
                handle_game_event(game_event);
            }
            gromnie::ScriptEvent::State(state_event) => {
                handle_state_event(state_event);
            }
            gromnie::ScriptEvent::System(system_event) => {
                handle_system_event(system_event);
            }
        }
    }

    fn on_tick(delta_millis: u64) {
        // Check current state periodically
        let state = host::get_client_state();

        if let host::Scene::InWorld(_) = &state.scene {
            // Do periodic in-world tasks
            if delta_millis > 1000 {
                host::log("Still in world");
            }
        }
    }
}

fn handle_game_event(game_event: gromnie::GameEvent) {
    match game_event {
        gromnie::GameEvent::Protocol(proto_event) => {
            match proto_event {
                host::ProtocolEvent::S2c(host::S2cEvent::ItemCreateObject(msg)) => {
                    host::log(&format!("Object: {} created", msg.name));
                }
                host::ProtocolEvent::GameEvent(game) => {
                    match game.event {
                        host::GameEventMsg::HearDirectSpeech(msg) => {
                            host::log(&format!("{}: {}", msg.sender_name, msg.message));

                            // Respond to specific messages
                            if msg.message.contains("hello") {
                                host::send_chat("Hello there!");
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn handle_state_event(state_event: host::StateEvent) {
    match state_event {
        host::StateEvent::InWorld => {
            host::log("Entered world!");
        }
        host::StateEvent::CharacterSelect => {
            host::log("At character selection");
        }
        _ => {}
    }
}

fn handle_system_event(system_event: host::SystemEvent) {
    match system_event {
        host::SystemEvent::LoginSucceeded(info) => {
            host::log(&format!("Welcome back, {}!", info.character_name));
        }
        host::SystemEvent::Disconnected => {
            host::log("Lost connection");
        }
        _ => {}
    }
}

export!(MyScript);
```

#![allow(unsafe_op_in_unsafe_fn)]

// Generate bindings directly in this crate
wit_bindgen::generate!({
    path: "../../crates/gromnie-scripting/src/wit",
    world: "script",
});

use self::gromnie::scripting::host;
use self::exports::gromnie::scripting::guest::Guest;

// Event filter constants
const EVENT_CHARACTER_LIST_RECEIVED: u32 = 1;

/// Script state (using static mut since WASM is single-threaded)
struct AutoLoginScript {
    handled: bool,
}

static mut SCRIPT: AutoLoginScript = AutoLoginScript { handled: false };

struct MyGuest;

impl Guest for MyGuest {
    fn get_id() -> String {
        "auto_login_wasm".to_string()
    }

    fn get_name() -> String {
        "Auto Login (WASM)".to_string()
    }

    fn get_description() -> String {
        "Automatically logs in using the first available character. Errors if no characters exist.".to_string()
    }

    fn on_load() {
        host::log("Auto Login script loaded");
    }

    fn on_unload() {
        host::log("Auto Login script unloaded");
    }

    fn subscribed_events() -> Vec<u32> {
        // Subscribe to CharacterListReceived events
        vec![EVENT_CHARACTER_LIST_RECEIVED]
    }

    fn on_event(event: host::GameEvent) {
        unsafe {
            match event {
                host::GameEvent::CharacterListReceived(account) => {
                    // Only handle once
                    if SCRIPT.handled {
                        return;
                    }
                    SCRIPT.handled = true;

                    // Check if there are any characters
                    if account.character_list.is_empty() {
                        host::log(&format!(
                            "Auto Login: No characters found on account '{}'. Cannot auto-login.",
                            account.name
                        ));
                        return;
                    }

                    // Use the first character
                    let first_char = &account.character_list[0];
                    host::log(&format!(
                        "Auto Login: Logging in as first character '{}' (ID: {})",
                        first_char.name, first_char.id
                    ));

                    // Send the login action
                    host::login_character(
                        &account.name,
                        first_char.id,
                        &first_char.name,
                    );
                }
                _ => {}
            }
        }
    }

    fn on_tick(_delta_millis: u64) {
        // No periodic logic needed
    }
}

// Export the Guest implementation
export!(MyGuest);

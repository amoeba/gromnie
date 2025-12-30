#![allow(unsafe_op_in_unsafe_fn)]

// Generate bindings directly in this crate
wit_bindgen::generate!({
    path: "../../crates/gromnie-scripting/src/wit",
    world: "script",
});

use self::gromnie::scripting::host;
use self::exports::gromnie::scripting::guest::Guest;
use std::fs::OpenOptions;
use std::io::Write;

// Event filter constants
const EVENT_CHAT_MESSAGE_RECEIVED: u32 = 3;

/// Script state
struct FileLoggerScript {
    log_file: Option<std::fs::File>,
}

static mut SCRIPT: FileLoggerScript = FileLoggerScript { log_file: None };

struct MyGuest;

impl Guest for MyGuest {
    fn get_id() -> String {
        "file_logger_wasm".to_string()
    }

    fn get_name() -> String {
        "File Logger (WASM)".to_string()
    }

    fn get_description() -> String {
        "WASM component that logs chat messages to /script_data/chat.log".to_string()
    }

    fn on_load() {
        unsafe {
            // Open log file in append mode
            // WASI preopens /script_data to ~/.config/gromnie/script_data/
            match OpenOptions::new()
                .create(true)
                .append(true)
                .open("/script_data/chat.log")
            {
                Ok(file) => {
                    SCRIPT.log_file = Some(file);

                    // Write a startup message
                    if let Some(ref mut f) = SCRIPT.log_file {
                        let _ = writeln!(f, "\n=== File Logger WASM started ===");
                        let _ = f.flush();
                    }

                    host::log("File logger started - logging to /script_data/chat.log");
                }
                Err(e) => {
                    // Can't log to file
                    let error_msg = format!("Failed to open log file: {}", e);
                    host::log(&error_msg);
                    host::send_chat(&error_msg);
                }
            }
        }
    }

    fn on_unload() {
        unsafe {
            // Write shutdown message and close file
            if let Some(ref mut f) = SCRIPT.log_file {
                let _ = writeln!(f, "=== File Logger WASM stopped ===\n");
                let _ = f.flush();
            }
            SCRIPT.log_file = None;
            host::log("File logger stopped");
        }
    }

    fn subscribed_events() -> Vec<u32> {
        // Subscribe to chat messages
        vec![EVENT_CHAT_MESSAGE_RECEIVED]
    }

    fn on_event(event: host::GameEvent) {
        unsafe {
            match event {
                host::GameEvent::ChatMessageReceived(chat_info) => {
                    if let Some(ref mut file) = SCRIPT.log_file {
                        // Get current timestamp
                        let timestamp = host::get_event_time_millis();

                        // Log the chat message
                        let log_entry = format!(
                            "[{}] [channel:{}] {}\n",
                            timestamp,
                            chat_info.channel,
                            chat_info.message
                        );

                        if let Err(e) = file.write_all(log_entry.as_bytes()) {
                            // If write fails, log the error
                            let error_msg = format!("Log write failed: {}", e);
                            host::log(&error_msg);
                            host::send_chat(&error_msg);
                        } else {
                            // Flush to ensure it's written immediately
                            let _ = file.flush();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn on_tick(_delta_millis: u64) {
        // No periodic logic needed
    }
}

export!(MyGuest);

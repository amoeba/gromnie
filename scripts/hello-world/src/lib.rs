// Generate bindings directly in this crate
wit_bindgen::generate!({
    path: "../../wit",
    world: "script",
});

use gromnie::scripting::host;

// Event filter constants
const EVENT_CREATE_OBJECT: u32 = 2;

/// Script state (using static mut since WASM is single-threaded)
struct HelloWorldScript {
    timer_id: Option<u64>,
}

static mut SCRIPT: HelloWorldScript = HelloWorldScript { timer_id: None };

struct MyGuest;

impl exports::gromnie::scripting::guest::Guest for MyGuest {
    fn get_id() -> String {
        "hello_world_wasm".to_string()
    }

    fn get_name() -> String {
        "Hello World (WASM)".to_string()
    }

    fn get_description() -> String {
        "WASM component that sends a greeting 5 seconds after an object is created".to_string()
    }

    fn on_load() {
        // Log when script loads
        host::log("Hello World script loaded!");
    }

    fn on_unload() {
        // Log when script unloads
        host::log("Hello World script unloaded!");
    }

    fn subscribed_events() -> Vec<u32> {
        // Subscribe to CreateObject events
        vec![EVENT_CREATE_OBJECT]
    }

    fn on_event(event: host::GameEvent) {
        unsafe {
            match event {
                host::GameEvent::CreateObject(obj) => {
                    // Log the object creation
                    host::log(&format!("Object created: {} (ID: {})", obj.name, obj.id));

                    // Schedule a 5-second timer
                    let timer_id = host::schedule_timer(5, "greeting");
                    SCRIPT.timer_id = Some(timer_id);

                    host::log("Scheduled 5-second greeting timer");
                }
                _ => {}
            }
        }
    }

    fn on_tick(_delta_millis: u64) {
        unsafe {
            // Check if our timer has fired
            if let Some(timer_id) = SCRIPT.timer_id {
                if host::check_timer(timer_id) {
                    // Timer fired! Send greeting
                    host::log("Timer fired! Sending greeting...");
                    host::send_chat("Hello from WASM! 👋");
                    SCRIPT.timer_id = None;
                }
            }
        }
    }
}

// Export the Guest implementation
export!(MyGuest);

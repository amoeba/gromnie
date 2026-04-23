//! Echo Tell Script
//!
//! Listens for incoming tells (direct messages) and echoes them back to the sender.
//! Demonstrates the chat/tell interaction loop.
//!
//! Usage: Load this script while in-world. Any player who sends you a tell will
//! receive an automatic reply.

use gromnie::host_interface::ProtocolEvent;
use gromnie::ScriptEvent;
use gromnie_scripting_api as gromnie;

#[derive(Default)]
pub struct EchoTellScript;

impl gromnie::Script for EchoTellScript {
    fn new() -> Self {
        Self
    }

    fn id(&self) -> &str {
        "echo_tell"
    }

    fn name(&self) -> &str {
        "Echo Tell"
    }

    fn description(&self) -> &str {
        "Automatically replies to incoming tells, echoing the message back."
    }

    fn on_load(&mut self) {
        gromnie::log("Echo Tell script loaded. Will reply to all incoming tells.");
    }

    fn on_unload(&mut self) {
        gromnie::log("Echo Tell script unloaded.");
    }

    fn subscribed_events(&self) -> Vec<u32> {
        // Subscribe to all events (0xFFFFFFFF) so we receive everything
        vec![0xFFFFFFFF]
    }

    fn on_event(&mut self, event: ScriptEvent) {
        use gromnie::host::*;

        if let ScriptEvent::Game(GameEvent::Protocol(proto)) = event {
            if let ProtocolEvent::GameEvent(game_event) = proto {
                if let GameEventMsg::HearDirectSpeech(msg) = game_event.event {
                    // msg.message_type for tells is typically 0x44 (ChatMessageType::Tell)
                    // We reply regardless of type since HearDirectSpeech is always a tell
                    let reply = format!("You said: {}", msg.message);
                    gromnie::log(&format!(
                        "Replying to tell from {} (0x{:08X}): {}",
                        msg.sender_name, msg.sender_id, msg.message
                    ));
                    gromnie::send_tell(&msg.sender_name, &reply);
                }
            }
        }
    }

    fn on_tick(&mut self, _delta_millis: u64) {}
}

gromnie::register_script!(EchoTellScript);

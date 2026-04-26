//! Position Tracker Script
//!
//! Tracks and logs the player's world position from server movement events.
//!
//! Events watched:
//!   - MovementPositionEvent (0xF748): periodic position corrections for any object
//!   - MovementPositionAndMovementEvent (0xF619): position + movement, fired on
//!     teleport/recall/portal entry
//!   - EffectsPlayerTeleport (0xF751): server signals a teleport has occurred

use gromnie::ScriptEvent;
use gromnie_scripting_api as gromnie;

#[derive(Default)]
pub struct PositionTracker {
    /// Character ID learned from LoginCreatePlayer, used to filter movement
    /// events to our own character rather than every nearby object.
    character_id: Option<u32>,
}

impl gromnie::Script for PositionTracker {
    fn new() -> Self {
        Self::default()
    }

    fn id(&self) -> &str {
        "position-tracker"
    }

    fn name(&self) -> &str {
        "Position Tracker"
    }

    fn description(&self) -> &str {
        "Logs world position updates and teleport events for the local player."
    }

    fn on_load(&mut self) {
        gromnie::log("Position Tracker loaded - watching for movement events");
    }

    fn on_unload(&mut self) {
        gromnie::log("Position Tracker unloaded");
    }

    fn subscribed_events(&self) -> Vec<u32> {
        // Subscribe to all events; we filter in on_event
        vec![gromnie_scripting_api::events::EVENT_ALL]
    }

    fn on_event(&mut self, event: ScriptEvent) {
        use gromnie::host::*;

        match event {
            ScriptEvent::Game(GameEvent::Protocol(ProtocolEvent::S2c(s2c_event))) => {
                match s2c_event {
                    // Learn our own character ID when we enter the world
                    S2cEvent::LoginCreatePlayer(msg) => {
                        self.character_id = Some(msg.character_id);
                        gromnie::log(&format!(
                            "[PositionTracker] Tracking character ID: 0x{:08X}",
                            msg.character_id
                        ));
                    }

                    // Periodic position correction - filter to local player only
                    S2cEvent::MovementPosition(msg) => {
                        if self.is_player(msg.object_id) {
                            log_position("PositionUpdate", msg.object_id, &msg.position);
                        }
                    }

                    // Position + movement (portal, recall, lifestone) - filter to player
                    S2cEvent::MovementPositionAndMovement(msg) => {
                        if self.is_player(msg.object_id) {
                            log_position("PositionAndMovement", msg.object_id, &msg.position);
                        }
                    }

                    // Teleport effect - always for the local player
                    S2cEvent::EffectsPlayerTeleport(msg) => {
                        gromnie::log(&format!(
                            "[PositionTracker] Teleport! (seq {})",
                            msg.object_teleport_sequence
                        ));
                    }

                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn on_tick(&mut self, _delta_millis: u64) {}
}

impl PositionTracker {
    fn is_player(&self, object_id: u32) -> bool {
        self.character_id.map_or(false, |id| id == object_id)
    }
}

fn log_position(label: &str, object_id: u32, pos: &gromnie::host::PositionPack) {
    let cell = pos.landcell;
    let block = cell >> 16;
    let cell_id = cell & 0xFFFF;

    gromnie::log(&format!(
        "[PositionTracker] {} obj=0x{:08X} block=0x{:04X} cell=0x{:04X} ({:.2}, {:.2}, {:.2})",
        label, object_id, block, cell_id, pos.x, pos.y, pos.z
    ));
}

gromnie::register_script!(PositionTracker);

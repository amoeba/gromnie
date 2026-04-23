//! Movement Demo Script
//!
//! Demonstrates sending movement commands from a script.  When you say "/walk"
//! in nearby chat the script walks the character forward for 2 seconds, then
//! stops automatically.  Say "/stop" to cancel early.
//!
//! Motion constants come from the Asheron's Call protocol.  The values used
//! here are:
//!   0x41000003 = WalkForward
//!   0x6500000D = TurnRight
//!   0x6500000E = TurnLeft
//!
//! Hold key values (passed to do/stop-movement-command):
//!   1 = None  (walk speed)
//!   2 = Run   (run speed)

use gromnie_scripting_api as gromnie;
use gromnie::ScriptEvent;

const MOTION_WALK_FORWARD: u32 = 0x41000003;
const HOLD_KEY_NONE: u32 = 1;
const WALK_DURATION_SECS: u64 = 2;

#[derive(Default, PartialEq)]
enum State {
    #[default]
    Idle,
    Walking,
}

#[derive(Default)]
pub struct MovementDemo {
    state: State,
    walk_timer_id: Option<u64>,
}

impl gromnie::Script for MovementDemo {
    fn new() -> Self {
        Self::default()
    }

    fn id(&self) -> &str {
        "movement-demo"
    }

    fn name(&self) -> &str {
        "Movement Demo"
    }

    fn description(&self) -> &str {
        "Say \"/walk\" to walk forward for 2 seconds. Say \"/stop\" to cancel."
    }

    fn on_load(&mut self) {
        gromnie::log("Movement Demo loaded - say \"/walk\" to test movement");
    }

    fn on_unload(&mut self) {
        // Stop any active movement when unloaded
        if self.state == State::Walking {
            gromnie::stop_movement_command(MOTION_WALK_FORWARD, HOLD_KEY_NONE);
        }
        gromnie::log("Movement Demo unloaded");
    }

    fn subscribed_events(&self) -> Vec<u32> {
        vec![gromnie_scripting_api::events::EVENT_ALL]
    }

    fn on_event(&mut self, event: ScriptEvent) {
        use gromnie::host::*;

        match event {
            ScriptEvent::Game(GameEvent::Protocol(ProtocolEvent::S2c(s2c_event))) => {
                match s2c_event {
                    S2cEvent::HearSpeech(msg) => {
                        match msg.message.trim() {
                            "/walk" => self.start_walk(),
                            "/stop" => self.stop_walk(),
                            _ => {}
                        }
                    }
                    // Cancel walk on teleport to avoid getting stuck in motion state
                    S2cEvent::EffectsPlayerTeleport(_) => {
                        if self.state == State::Walking {
                            self.stop_walk();
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn on_tick(&mut self, _delta_millis: u64) {
        // Check whether the walk stop timer has fired
        if let Some(timer_id) = self.walk_timer_id {
            if gromnie::check_timer(timer_id) {
                self.stop_walk();
            }
        }
    }
}

impl MovementDemo {
    fn start_walk(&mut self) {
        if self.state == State::Walking {
            gromnie::log("[MovementDemo] Already walking");
            return;
        }

        gromnie::log(&format!(
            "[MovementDemo] Walking forward for {} seconds",
            WALK_DURATION_SECS
        ));

        gromnie::do_movement_command(MOTION_WALK_FORWARD, 1.0, HOLD_KEY_NONE);

        let timer_id = gromnie::schedule_timer(WALK_DURATION_SECS, "walk-stop");
        self.walk_timer_id = Some(timer_id);
        self.state = State::Walking;
    }

    fn stop_walk(&mut self) {
        if self.state == State::Idle {
            return;
        }

        gromnie::log("[MovementDemo] Stopping walk");
        gromnie::stop_movement_command(MOTION_WALK_FORWARD, HOLD_KEY_NONE);

        if let Some(timer_id) = self.walk_timer_id.take() {
            gromnie::cancel_timer(timer_id);
        }

        self.state = State::Idle;
    }
}

gromnie::register_script!(MovementDemo);

//! Spell Caster Script
//!
//! Demonstrates spell casting by:
//!  1. Casting a self-buff (untargeted) on a recurring timer to keep it active
//!  2. Recasting the buff when the enchantment is removed (wears off)
//!
//! Spell IDs reference the Asheron's Call spell database.
//! Example spell IDs (untargeted / self-cast):
//!   - 2 = Imperil Other I (life magic debuff, for testing only)
//!   - 4 = Strength Self I (creature magic buff)
//!   - 26 = Sprint (var speed buff)
//!
//! For targeted spells, supply a target object ID from ItemCreateObject events.

use gromnie::host_interface::ProtocolEvent;
use gromnie::ScriptEvent;
use gromnie_scripting_api as gromnie;

// Spell ID to continuously maintain - change to suit your character
const SELF_BUFF_SPELL_ID: u32 = 4; // Strength Self I

// How often to recast the buff (seconds) - should be shorter than the spell duration
const RECAST_INTERVAL_SECS: u64 = 60;

pub struct SpellCasterScript {
    recast_timer: Option<u64>,
    in_world: bool,
}

impl Default for SpellCasterScript {
    fn default() -> Self {
        Self {
            recast_timer: None,
            in_world: false,
        }
    }
}

impl gromnie::Script for SpellCasterScript {
    fn new() -> Self {
        Self::default()
    }

    fn id(&self) -> &str {
        "spell_caster"
    }

    fn name(&self) -> &str {
        "Spell Caster"
    }

    fn description(&self) -> &str {
        "Maintains a self-buff by recasting it on a timer and when it wears off."
    }

    fn on_load(&mut self) {
        gromnie::log(&format!(
            "Spell Caster loaded. Will maintain spell {} every {}s.",
            SELF_BUFF_SPELL_ID, RECAST_INTERVAL_SECS
        ));
    }

    fn on_unload(&mut self) {
        if let Some(timer_id) = self.recast_timer.take() {
            gromnie::cancel_timer(timer_id);
        }
        gromnie::log("Spell Caster unloaded.");
    }

    fn subscribed_events(&self) -> Vec<u32> {
        vec![0xFFFFFFFF]
    }

    fn on_event(&mut self, event: ScriptEvent) {
        use gromnie::host::*;

        match event {
            ScriptEvent::State(StateEvent::InWorld) => {
                // We entered the world - start the recast timer and cast immediately
                gromnie::log("In world, casting initial buff...");
                self.in_world = true;
                self.cast_buff();
                let timer_id = gromnie::schedule_recurring(RECAST_INTERVAL_SECS, "recast_buff");
                self.recast_timer = Some(timer_id);
            }
            ScriptEvent::State(StateEvent::ExitingWorld) => {
                self.in_world = false;
                if let Some(timer_id) = self.recast_timer.take() {
                    gromnie::cancel_timer(timer_id);
                }
            }
            ScriptEvent::Game(GameEvent::Protocol(proto)) => {
                if let ProtocolEvent::GameEvent(ordered) = proto {
                    match ordered.event {
                        GameEventMsg::EnchantmentUpdated(msg) => {
                            if msg.spell_id == SELF_BUFF_SPELL_ID {
                                gromnie::log(&format!(
                                    "Buff {} active: duration={:.1}s, power={}",
                                    msg.spell_id, msg.duration, msg.power_level
                                ));
                            }
                        }
                        GameEventMsg::EnchantmentRemoved(msg) => {
                            if msg.spell_id == SELF_BUFF_SPELL_ID && self.in_world {
                                // Buff wore off - recast immediately
                                gromnie::log(&format!(
                                    "Buff {} removed, recasting immediately.",
                                    msg.spell_id
                                ));
                                self.cast_buff();
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn on_tick(&mut self, _delta_millis: u64) {
        // Check if the recast timer fired
        if let Some(timer_id) = self.recast_timer {
            if gromnie::check_timer(timer_id) {
                gromnie::log("Recast timer fired, refreshing buff.");
                self.cast_buff();
            }
        }
    }
}

impl SpellCasterScript {
    fn cast_buff(&self) {
        gromnie::log(&format!("Casting self-buff spell {}", SELF_BUFF_SPELL_ID));
        // Use untargeted (self) cast for buff spells
        gromnie::cast_untargeted_spell(SELF_BUFF_SPELL_ID);
    }
}

gromnie::register_script!(SpellCasterScript);

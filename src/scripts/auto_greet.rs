use std::any::Any;
use std::time::Duration;

use crate::client::events::GameEvent;
use crate::scripting::{EventFilter, Script, ScriptContext};

/// Script that immediately sends a greeting message when login succeeds
#[derive(Default)]
pub struct AutoGreetScript;

impl Script for AutoGreetScript {
    fn id(&self) -> &'static str {
        "auto_greet"
    }

    fn name(&self) -> &'static str {
        "Auto Greet"
    }

    fn description(&self) -> &'static str {
        "Automatically sends a greeting message when login succeeds"
    }

    fn on_load(&mut self, _ctx: &mut ScriptContext) {
        // Nothing to do on load
    }

    fn on_unload(&mut self, _ctx: &mut ScriptContext) {
        // Nothing to do on unload
    }

    fn subscribed_events(&self) -> &[EventFilter] {
        &[EventFilter::All]
    }

    fn on_event(&mut self, event: &GameEvent, ctx: &mut ScriptContext) {
        if let GameEvent::LoginSucceeded { .. } = event {
            // Send greeting immediately
            ctx.send_chat("Hello from gromnie!");
        }
    }

    fn on_tick(&mut self, _ctx: &mut ScriptContext, _delta: Duration) {
        // No periodic logic needed for this script
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

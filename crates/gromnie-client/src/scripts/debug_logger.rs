use std::any::Any;
use std::time::Duration;
use tracing::debug;

use crate::client::events::GameEvent;
use gromnie_scripting::{EventFilter, Script, ScriptContext};

/// Debug script that logs all game events
#[derive(Default)]
pub struct DebugLoggerScript;

impl Script for DebugLoggerScript {
    fn id(&self) -> &'static str {
        "debug_logger"
    }

    fn name(&self) -> &'static str {
        "Debug Logger"
    }

    fn description(&self) -> &'static str {
        "Logs all game events for debugging purposes"
    }

    fn on_load(&mut self, _ctx: &mut ScriptContext) {
        debug!(target: "scripts", "DebugLoggerScript loaded");
    }

    fn on_unload(&mut self, _ctx: &mut ScriptContext) {
        debug!(target: "scripts", "DebugLoggerScript unloaded");
    }

    fn subscribed_events(&self) -> &[EventFilter] {
        // Subscribe to all events
        &[EventFilter::All]
    }

    fn on_event(&mut self, event: &GameEvent, _ctx: &mut ScriptContext) {
        debug!(target: "scripts", "Event: {:?}", event);
    }

    fn on_tick(&mut self, _ctx: &mut ScriptContext, _delta: Duration) {
        // No periodic logic needed for this script
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

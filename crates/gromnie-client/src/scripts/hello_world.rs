use std::any::Any;
use std::time::Duration;
use tracing::info;

use crate::client::events::GameEvent;
use gromnie_scripting::{EventFilter, Script, ScriptContext, TimerId};

/// Example script that sends a greeting message after login
#[derive(Default)]
pub struct HelloWorldScript {
    /// Timer ID for the delayed greeting
    timer_id: Option<TimerId>,
}

impl Script for HelloWorldScript {
    fn id(&self) -> &'static str {
        "hello_world"
    }

    fn name(&self) -> &'static str {
        "Hello World"
    }

    fn description(&self) -> &'static str {
        "Sends a greeting message 5 seconds after logging in"
    }

    fn on_load(&mut self, _ctx: &mut ScriptContext) {
        info!(target: "scripts", "HelloWorldScript loaded and ready!");
    }

    fn on_unload(&mut self, _ctx: &mut ScriptContext) {
        // Nothing to do on unload
    }

    fn subscribed_events(&self) -> &[EventFilter] {
        // Subscribe to all events and filter for LoginSucceeded in on_event
        &[EventFilter::All]
    }

    fn on_event(&mut self, event: &GameEvent, ctx: &mut ScriptContext) {
        if let GameEvent::LoginSucceeded { .. } = event {
            // Schedule a timer to send greeting after 5 seconds
            info!(target: "scripts", "HelloWorldScript: Login detected, scheduling 5-second greeting timer");
            let timer_id = ctx.schedule_timer(5, "greeting");
            self.timer_id = Some(timer_id);
        }
    }

    fn on_tick(&mut self, ctx: &mut ScriptContext, _delta: Duration) {
        // Check if our timer has fired
        if let Some(timer_id) = self.timer_id
            && ctx.check_timer(timer_id)
        {
            info!(target: "scripts", "HelloWorldScript: Timer fired! Sending greeting");
            ctx.send_chat("Hello, world!");
            // TODO: Add wave animation support
            // For now, just send the chat message
            self.timer_id = None;
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

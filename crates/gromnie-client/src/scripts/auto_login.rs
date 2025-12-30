use std::any::Any;
use std::time::Duration;

use crate::client::events::{ClientAction, GameEvent};
use gromnie_scripting::{EventFilter, Script, ScriptContext};
use tracing::{error, info};

/// Script that automatically logs in using the first available character
/// Errors if no characters exist on the account
#[derive(Default)]
pub struct AutoLoginScript {
    handled: bool,
}

impl Script for AutoLoginScript {
    fn id(&self) -> &'static str {
        "auto_login"
    }

    fn name(&self) -> &'static str {
        "Auto Login"
    }

    fn description(&self) -> &'static str {
        "Automatically logs in using the first available character. Errors if no characters exist."
    }

    fn on_load(&mut self, _ctx: &mut ScriptContext) {
        info!(target: "scripts", "Auto Login script loaded");
    }

    fn on_unload(&mut self, _ctx: &mut ScriptContext) {
        // Nothing to do on unload
    }

    fn subscribed_events(&self) -> &[EventFilter] {
        &[EventFilter::CharacterListReceived]
    }

    fn on_event(&mut self, event: &GameEvent, ctx: &mut ScriptContext) {
        if let GameEvent::CharacterListReceived {
            account,
            characters,
            num_slots: _,
        } = event
        {
            // Only handle once
            if self.handled {
                return;
            }
            self.handled = true;

            // Check if there are any characters
            if characters.is_empty() {
                error!(target: "scripts", "Auto Login: No characters found on account '{}'. Cannot auto-login.", account);
                return;
            }

            // Use the first character
            let first_char = &characters[0];
            info!(target: "scripts", "Auto Login: Logging in as first character '{}' (ID: {})", first_char.name, first_char.id);

            // Send the login action
            ctx.send_action(ClientAction::LoginCharacter {
                character_id: first_char.id,
                character_name: first_char.name.clone(),
                account: account.clone(),
            });
        }
    }

    fn on_tick(&mut self, _ctx: &mut ScriptContext, _delta: Duration) {
        // No periodic logic needed
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

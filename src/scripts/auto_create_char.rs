use std::any::Any;
use std::time::Duration;

use crate::client::OutgoingMessageContent;
use crate::client::events::GameEvent;
use crate::runner::CharacterBuilder;
use crate::scripting::{EventFilter, Script, ScriptContext};
use tracing::info;

/// Script that automatically creates a character with a random name
/// when the character list is empty
#[derive(Default)]
pub struct AutoCreateCharScript {
    handled: bool,
}

impl Script for AutoCreateCharScript {
    fn id(&self) -> &'static str {
        "auto_create_char"
    }

    fn name(&self) -> &'static str {
        "Auto Create Character"
    }

    fn description(&self) -> &'static str {
        "Automatically creates a character with a random name when the character list is empty"
    }

    fn on_load(&mut self, _ctx: &mut ScriptContext) {
        info!(target: "scripts", "Auto Create Character script loaded");
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

            // Only create a character if the list is empty
            if characters.is_empty() {
                // Generate a random name using timestamp
                let char_name = format!("Gromnie{}", chrono::Utc::now().timestamp() % 100000);

                info!(target: "scripts", "Auto Create Character: No characters found on account '{}'. Creating character '{}'", account, char_name);

                // Create character using CharacterBuilder
                let char_gen = CharacterBuilder::new(char_name).build();

                // Send the character creation message
                ctx.send_message(OutgoingMessageContent::CharacterCreationAce(
                    account.clone(),
                    char_gen,
                ));
            }
        }
    }

    fn on_tick(&mut self, _ctx: &mut ScriptContext, _delta: Duration) {
        // No periodic logic needed
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

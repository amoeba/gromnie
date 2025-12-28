mod character_gen;
mod client_runner;
mod event_consumer;

pub use character_gen::CharacterBuilder;
pub use client_runner::{run_client, run_client_with_action_channel, ClientConfig};
pub use event_consumer::{
    handle_character_list, handle_character_list_with_name, EventConsumer, LoggingConsumer,
    TuiConsumer,
};

#[cfg(feature = "discord")]
pub use event_consumer::{DiscordConsumer, UptimeData};

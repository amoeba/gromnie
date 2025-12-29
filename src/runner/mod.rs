mod character_gen;
mod client_runner;
mod event_consumer;

pub use character_gen::CharacterBuilder;
pub use client_runner::{ClientConfig, run_client, run_client_with_action_channel, run_client_with_consumers};
pub use event_consumer::{
    EventConsumer, LoggingConsumer, TuiConsumer, AutoLoginConsumer, create_script_consumer, handle_character_list,
};

pub use event_consumer::{DiscordConsumer, UptimeData};

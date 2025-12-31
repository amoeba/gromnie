mod character_gen;
mod client_runner;
mod event_consumer;
mod event_bus;

pub use character_gen::CharacterBuilder;
pub use client_runner::{
    ClientConfig, run_client, run_client_with_action_channel, run_client_with_consumers,
};
pub use event_consumer::{
    EventConsumer, LoggingConsumer, ScriptConsumer, TuiConsumer, create_script_consumer,
};
pub use gromnie_client::client::event_bus::{
    ClientEvent, ClientStateEvent, EventBus, EventContext, EventEnvelope, EventSource, 
    ScriptEventType, SystemEvent,
};

pub use event_consumer::{DiscordConsumer, UptimeData};

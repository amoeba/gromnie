mod character_gen;
mod client_runner;
mod event_consumer;

pub use character_gen::CharacterBuilder;
pub use client_runner::{
    ClientConfig, EventBusManager, create_event_bus_manager, run_client, run_client_with_action_channel, run_client_with_consumers,
};
pub use event_consumer::{
    EventConsumer, LoggingConsumer, ScriptConsumer, TuiConsumer, create_script_consumer,
};
pub use gromnie_client::client::event_bus::{
    ClientEvent, ClientStateEvent, EventBus, EventContext, EventEnvelope, EventSource, 
    ScriptEventType, SystemEvent,
};

pub use event_consumer::{DiscordConsumer, UptimeData};

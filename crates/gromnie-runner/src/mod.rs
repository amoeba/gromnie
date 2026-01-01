mod character_gen;
mod client_runner;
mod event_consumer;
pub mod event_bus;
mod event_wrapper;

pub use character_gen::CharacterBuilder;
pub use client_runner::{
    ClientConfig, EventBusManager, create_event_bus_manager, run_client, run_client_with_action_channel, run_client_with_consumers,
};
pub use event_consumer::{
    EventConsumer, LoggingConsumer, TuiConsumer, DiscordConsumer, UptimeData,
};
pub use event_bus::{
    EventType, ClientStateEvent, EventBus, EventContext, EventEnvelope, EventSource,
    ScriptEventType, SystemEvent,
};
pub use event_wrapper::EventWrapper;

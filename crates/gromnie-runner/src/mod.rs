mod character_gen;
mod client_runner;
pub mod event_bus;
mod event_consumer;
mod event_wrapper;

pub use character_gen::CharacterBuilder;
pub use client_runner::{
    ClientConfig, EventBusManager, create_event_bus_manager, run_client,
    run_client_with_action_channel, run_client_with_consumers,
};
pub use event_bus::{
    ClientStateEvent, EventBus, EventContext, EventEnvelope, EventSource, EventType,
    ScriptEventType, SystemEvent,
};
pub use event_consumer::{
    DiscordConsumer, EventConsumer, LoggingConsumer, TuiConsumer, UptimeData,
};
pub use event_wrapper::EventWrapper;

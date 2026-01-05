mod character_gen;
mod client_naming;
mod client_runner;
pub mod client_runner_builder;
pub mod event_bus;
mod event_consumer;
mod event_wrapper;

pub use character_gen::CharacterBuilder;
pub use client_naming::{ClientNaming, encode_client_id};
pub use client_runner::{
    ClientConfig, ConsumerBuilder, EventBusManager, FnConsumerBuilder, FnConsumerFactory,
    MultiClientConfig, MultiClientConsumerFactory, MultiClientStats, RunConfig, RunResult,
    create_event_bus_manager, run, run_client, run_client_with_action_channel,
    run_client_with_consumers, run_multi_client,
};
pub use client_runner_builder::{
    BuildError, ClientMode, ClientRunner, ClientRunnerBuilder, ConsumerContext, ConsumerFactory,
    RunResult as BuilderRunResult,
};
pub use event_bus::{
    ClientStateEvent, EventBus, EventContext, EventEnvelope, EventSender, EventSource, EventType,
    ScriptEventType, SystemEvent, TuiEvent,
};
pub use event_consumer::{
    AutoLoginConsumer, AutoLoginState, CompositeConsumer, DiscordConsumer, EventConsumer,
    LoggingConsumer, StatsConsumer, TuiConsumer, UptimeData,
};
pub use event_wrapper::EventWrapper;

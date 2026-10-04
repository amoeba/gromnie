//! How an embedder builds [`EventConsumer`]s for a client.
//!
//! This lives in `gromnie-client` rather than `gromnie-events` because the
//! context hands out a [`ClientSender`]: a consumer has to be able to act on
//! the client it is observing, and `gromnie-client` is the crate that depends
//! on `gromnie-events`, not the other way round.
//!
//! Consumers are created lazily once the client exists, which is why the
//! context is passed to [`ConsumerFactory::create`] rather than to the
//! consumer's constructor.

use gromnie_events::EventConsumer;

use super::command::ClientSender;

/// Context provided to consumer factories when creating consumers.
///
/// The `Config` type parameter is generic to avoid a circular dependency.
/// Most consumers use the default `()` type since they only need `client_id`
/// and `client`. Advanced use cases can provide a custom config type.
pub struct ConsumerContext<'a, Config = ()> {
    /// The client ID
    pub client_id: u32,
    /// The client configuration (type provided by consumer)
    ///
    /// Note: Most consumers use `Config = ()` to avoid circular dependencies.
    /// This field is available for advanced consumers that need configuration
    /// access.
    pub client_config: &'a Config,
    /// Channels for acting on the client.
    pub sender: ClientSender,
}

/// Factory trait for creating event consumers.
///
/// This trait allows consumers to be created lazily when the client is ready,
/// providing access to the client and its configuration.
pub trait ConsumerFactory<Config = ()>: Send + Sync + 'static {
    /// Create a consumer for the given client context
    fn create(&self, ctx: &ConsumerContext<Config>) -> Box<dyn EventConsumer>;
}

// Allow closures to be used as consumer factories
impl<F, Config> ConsumerFactory<Config> for F
where
    F: Fn(&ConsumerContext<Config>) -> Box<dyn EventConsumer> + Send + Sync + 'static,
    Config: 'static,
{
    fn create(&self, ctx: &ConsumerContext<Config>) -> Box<dyn EventConsumer> {
        (self)(ctx)
    }
}

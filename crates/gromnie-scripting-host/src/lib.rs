/// Host runtime for loading and executing WASM scripts
///
/// This crate provides the runtime for the game client to load and run WASM scripts.
/// Scripts should depend on gromnie-scripting-api, not this crate.
use gromnie_scripting_api as api;
use std::any::Any;
use std::time::Duration;

pub mod context;
pub mod registry;
pub mod reload;
pub mod script_runner;
pub mod timer;
pub mod wasm;

// Re-export commonly used types for host-side scripting
pub use api::Script as ApiScript;
pub use context::{ClientState, ClientStateSnapshot, ScriptContext};
pub use reload::setup_reload_signal_handler;
pub use script_runner::{ScriptConsumer, ScriptRunner, create_script_consumer};
pub use timer::{TimerId, TimerManager};

// Registry is now just a utility function
pub use registry::create_runner_from_config;

// Host-friendly trait wrapper
/// Trait that scripts must implement (combines API and host requirements)
pub trait Script: Send + 'static {
    /// Unique identifier for this script (e.g., "hello_world")
    fn id(&self) -> &'static str;

    /// Human-readable name for this script
    fn name(&self) -> &'static str;

    /// Description of what this script does
    fn description(&self) -> &'static str;

    /// Called when the script is first loaded
    fn on_load(&mut self, ctx: &mut ScriptContext);

    /// Called when the script is being unloaded
    fn on_unload(&mut self, ctx: &mut ScriptContext);

    /// Return the list of events this script wants to receive
    fn subscribed_events(&self) -> &[EventFilter];

    /// Handle an event that matches one of the subscribed filters
    /// This receives the full ClientEvent which can be a Game, State, or System event
    fn on_event(&mut self, event: &gromnie_events::ClientEvent, ctx: &mut ScriptContext);

    /// Called periodically at a fixed rate (configurable, default ~20Hz)
    /// Use this for timer checks, periodic updates, and time-based logic
    ///
    /// # Arguments
    /// * `ctx` - Script context for accessing client state and sending actions
    /// * `delta` - Time elapsed since last tick
    fn on_tick(&mut self, ctx: &mut ScriptContext, delta: Duration);

    /// Allow downcasting to concrete script type for state access
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Filter for subscribing to specific events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventFilter {
    /// Subscribe to all events
    All,

    // Game events
    /// Character list received from server
    CharacterListReceived,
    /// Character error received from server
    CharacterError,
    /// Object created in game world
    CreateObject,
    /// Chat message received
    ChatMessageReceived,

    // State events
    /// Client state: Connecting
    StateConnecting,
    /// Client state: Connected
    StateConnected,
    /// Client state: Connecting failed
    StateConnectingFailed,
    /// Client state: Patching
    StatePatching,
    /// Client state: Patched
    StatePatched,
    /// Client state: Patching failed
    StatePatchingFailed,
    /// Client state: Character select
    StateCharacterSelect,
    /// Client state: Entering world
    StateEnteringWorld,
    /// Client state: In world
    StateInWorld,
    /// Client state: Exiting world
    StateExitingWorld,
    /// Client state: Character error
    StateCharacterError,

    // System events
    /// System: Authentication succeeded
    SystemAuthenticationSucceeded,
    /// System: Authentication failed
    SystemAuthenticationFailed,
    /// System: Connecting started
    SystemConnectingStarted,
    /// System: Connecting done
    SystemConnectingDone,
    /// System: Updating started
    SystemUpdatingStarted,
    /// System: Updating done
    SystemUpdatingDone,
    /// System: Login succeeded
    SystemLoginSucceeded,
    /// System: Reload scripts
    SystemReloadScripts,
    /// System: Shutdown
    SystemShutdown,
}

impl EventFilter {
    /// Check if this filter matches the given event
    pub fn matches(&self, event: &gromnie_events::ClientEvent) -> bool {
        use gromnie_events::{ClientEvent, ClientStateEvent, SimpleGameEvent as GameEvent};

        match self {
            EventFilter::All => true,

            // Game event filters
            EventFilter::CharacterListReceived => {
                matches!(
                    event,
                    ClientEvent::Game(GameEvent::CharacterListReceived { .. })
                )
            }
            EventFilter::CharacterError => {
                matches!(event, ClientEvent::Game(GameEvent::CharacterError { .. }))
            }
            EventFilter::CreateObject => {
                matches!(event, ClientEvent::Game(GameEvent::CreateObject { .. }))
            }
            EventFilter::ChatMessageReceived => {
                matches!(
                    event,
                    ClientEvent::Game(GameEvent::ChatMessageReceived { .. })
                )
            }

            // State event filters
            EventFilter::StateConnecting => {
                matches!(event, ClientEvent::State(ClientStateEvent::Connecting))
            }
            EventFilter::StateConnected => {
                matches!(event, ClientEvent::State(ClientStateEvent::Connected))
            }
            EventFilter::StateConnectingFailed => {
                matches!(
                    event,
                    ClientEvent::State(ClientStateEvent::ConnectingFailed { .. })
                )
            }
            EventFilter::StatePatching => {
                matches!(event, ClientEvent::State(ClientStateEvent::Patching))
            }
            EventFilter::StatePatched => {
                matches!(event, ClientEvent::State(ClientStateEvent::Patched))
            }
            EventFilter::StatePatchingFailed => {
                matches!(
                    event,
                    ClientEvent::State(ClientStateEvent::PatchingFailed { .. })
                )
            }
            EventFilter::StateCharacterSelect => {
                matches!(event, ClientEvent::State(ClientStateEvent::CharacterSelect))
            }
            EventFilter::StateEnteringWorld => {
                matches!(event, ClientEvent::State(ClientStateEvent::EnteringWorld))
            }
            EventFilter::StateInWorld => {
                matches!(event, ClientEvent::State(ClientStateEvent::InWorld))
            }
            EventFilter::StateExitingWorld => {
                matches!(event, ClientEvent::State(ClientStateEvent::ExitingWorld))
            }
            EventFilter::StateCharacterError => {
                matches!(event, ClientEvent::State(ClientStateEvent::CharacterError))
            }

            // System event filters
            EventFilter::SystemAuthenticationSucceeded => {
                matches!(
                    event,
                    ClientEvent::System(gromnie_events::ClientSystemEvent::AuthenticationSucceeded)
                )
            }
            EventFilter::SystemAuthenticationFailed => {
                matches!(
                    event,
                    ClientEvent::System(
                        gromnie_events::ClientSystemEvent::AuthenticationFailed { .. }
                    )
                )
            }
            EventFilter::SystemConnectingStarted => {
                matches!(
                    event,
                    ClientEvent::System(gromnie_events::ClientSystemEvent::ConnectingStarted)
                )
            }
            EventFilter::SystemConnectingDone => {
                matches!(
                    event,
                    ClientEvent::System(gromnie_events::ClientSystemEvent::ConnectingDone)
                )
            }
            EventFilter::SystemUpdatingStarted => {
                matches!(
                    event,
                    ClientEvent::System(gromnie_events::ClientSystemEvent::UpdatingStarted)
                )
            }
            EventFilter::SystemUpdatingDone => {
                matches!(
                    event,
                    ClientEvent::System(gromnie_events::ClientSystemEvent::UpdatingDone)
                )
            }
            EventFilter::SystemLoginSucceeded => {
                matches!(
                    event,
                    ClientEvent::System(gromnie_events::ClientSystemEvent::LoginSucceeded { .. })
                )
            }
            EventFilter::SystemReloadScripts => {
                // ReloadScripts is only in the event bus SystemEvent, not ClientSystemEvent
                // Scripts won't receive this as it's handled at the runner level
                false
            }
            EventFilter::SystemShutdown => {
                // Shutdown is only in the event bus SystemEvent, not ClientSystemEvent
                // Scripts won't receive this as it's handled at the runner level
                false
            }
        }
    }

    /// Convert a u32 discriminant to an EventFilter
    /// These discriminants correspond to the WIT interface event IDs
    pub fn from_discriminant(id: u32) -> Option<Self> {
        match id {
            0 => Some(EventFilter::All),
            // Game events (1-99)
            1 => Some(EventFilter::CharacterListReceived),
            2 => Some(EventFilter::CharacterError),
            3 => Some(EventFilter::CreateObject),
            4 => Some(EventFilter::ChatMessageReceived),
            // State events (100-199)
            100 => Some(EventFilter::StateConnecting),
            101 => Some(EventFilter::StateConnected),
            102 => Some(EventFilter::StateConnectingFailed),
            103 => Some(EventFilter::StatePatching),
            104 => Some(EventFilter::StatePatched),
            105 => Some(EventFilter::StatePatchingFailed),
            106 => Some(EventFilter::StateCharacterSelect),
            107 => Some(EventFilter::StateEnteringWorld),
            108 => Some(EventFilter::StateInWorld),
            109 => Some(EventFilter::StateExitingWorld),
            110 => Some(EventFilter::StateCharacterError),
            // System events (200-299)
            200 => Some(EventFilter::SystemAuthenticationSucceeded),
            201 => Some(EventFilter::SystemAuthenticationFailed),
            202 => Some(EventFilter::SystemConnectingStarted),
            203 => Some(EventFilter::SystemConnectingDone),
            204 => Some(EventFilter::SystemUpdatingStarted),
            205 => Some(EventFilter::SystemUpdatingDone),
            206 => Some(EventFilter::SystemLoginSucceeded),
            207 => Some(EventFilter::SystemReloadScripts),
            208 => Some(EventFilter::SystemShutdown),
            _ => None,
        }
    }

    /// Get the discriminant value for this event filter
    pub fn to_discriminant(&self) -> u32 {
        match self {
            EventFilter::All => 0,
            // Game events (1-99)
            EventFilter::CharacterListReceived => 1,
            EventFilter::CharacterError => 2,
            EventFilter::CreateObject => 3,
            EventFilter::ChatMessageReceived => 4,
            // State events (100-199)
            EventFilter::StateConnecting => 100,
            EventFilter::StateConnected => 101,
            EventFilter::StateConnectingFailed => 102,
            EventFilter::StatePatching => 103,
            EventFilter::StatePatched => 104,
            EventFilter::StatePatchingFailed => 105,
            EventFilter::StateCharacterSelect => 106,
            EventFilter::StateEnteringWorld => 107,
            EventFilter::StateInWorld => 108,
            EventFilter::StateExitingWorld => 109,
            EventFilter::StateCharacterError => 110,
            // System events (200-299)
            EventFilter::SystemAuthenticationSucceeded => 200,
            EventFilter::SystemAuthenticationFailed => 201,
            EventFilter::SystemConnectingStarted => 202,
            EventFilter::SystemConnectingDone => 203,
            EventFilter::SystemUpdatingStarted => 204,
            EventFilter::SystemUpdatingDone => 205,
            EventFilter::SystemLoginSucceeded => 206,
            EventFilter::SystemReloadScripts => 207,
            EventFilter::SystemShutdown => 208,
        }
    }
}

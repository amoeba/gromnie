//! Session-level commands, and the sender used to deliver them.
//!
//! There are exactly two ways to ask a running client to do something, and
//! they are deliberately split by what the message is:
//!
//! * [`GameActionMessage`]s — anything the *server* needs to hear about:
//!   chat, movement, trade, magic. Unbounded and cheap to express.
//! * [`ClientCommand`]s — everything that moves the *session* between scenes:
//!   entering the world, acknowledging login, disconnecting. Deliberately
//!   narrow, because these change state the client owns.
//!
//! [`ClientSender`] bundles both channels so an embedder holds one clonable
//! value instead of tracking the split itself. It replaces the old
//! `SimpleClientAction` channel, which duplicated the `GameActionMessage`
//! variants and needed a `pending_auto_login` field on the client to carry a
//! half-processed action across loop iterations.
//!
//! This module is available on wasm32: `gromnie-web` drives its own loop over
//! a WISP transport, but the command vocabulary is the same one.

use std::fmt;

use asheron_rs::enums::HoldKey;
use asheron_rs::gameactions::{
    CommunicationTalk, CommunicationTalkDirectByName, MovementDoMovementCommand,
    MovementStopMovementCommand,
};
use asheron_rs::message::GameActionMessage;
use tokio::sync::mpsc;
use tracing::info;

/// Session-level commands issued to a running client.
///
/// Intentionally narrow. Game-level operations (`say`, `tell`, movement) are
/// sent as [`GameActionMessage`]s over `game_action_tx`; UI-level observation
/// happens on the event channel. This channel carries only the transitions
/// that are not game actions.
#[derive(Debug, Clone)]
pub enum ClientCommand {
    /// Request entry into the world for a character.
    EnterWorld {
        character_id: u32,
        character_name: String,
        account: String,
    },
    /// Send the `LoginComplete` notification once initial world state arrives.
    SendLoginComplete,
    /// Disconnect and stop the driver loop.
    Disconnect,
}

/// Why a driver loop stopped.
///
/// Shared by [`spawn_client_loop`](crate::client::spawn_client_loop) and by
/// embedders that drive the client by hand, since [`Client::drain_commands`]
/// hands one back when a disconnect is requested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopExit {
    /// Shutdown was requested through the shutdown channel.
    Shutdown,
    /// The client entered an unrecoverable error state (see `ClientError`).
    Failed,
    /// Reconnection is disabled or has exhausted its attempts.
    ReconnectUnavailable,
    /// The initial `LoginRequest` could not be sent.
    LoginRequestFailed,
    /// A [`ClientCommand::Disconnect`] was received.
    DisconnectRequested,
    /// The driver task unwound (panicked) or was cancelled. Carries the detail.
    ///
    /// Without this the loop would stop silently and anything waiting on
    /// [`ClientHandle::wait_for_exit`](crate::client::ClientHandle::wait_for_exit)
    /// would hang forever.
    TaskFailed(String),
}

/// The client is no longer being driven, so the command was never queued.
///
/// Returned instead of handing the command back: there is nothing to retry
/// against, and it keeps the large `GameActionMessage` out of the error type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientLoopStopped;

impl fmt::Display for ClientLoopStopped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "client driver loop is no longer running")
    }
}

impl std::error::Error for ClientLoopStopped {}

/// The send side of a running client.
///
/// Clone it freely and hand it to anything that needs to act on the client —
/// a UI, a bot, a script. Every method is non-blocking and never touches the
/// client lock, so these are safe to call from any task, including from inside
/// a WASM guest callback.
///
/// The typed helpers ([`say`](Self::say), [`tell`](Self::tell), …) exist so
/// callers do not have to name a `GameActionMessage` variant. Use
/// [`send_game_action`](Self::send_game_action) for anything not covered here.
#[derive(Debug, Clone)]
pub struct ClientSender {
    commands: mpsc::UnboundedSender<ClientCommand>,
    game_actions: mpsc::UnboundedSender<GameActionMessage>,
}

impl ClientSender {
    /// Bundle the two channels a client exposes.
    pub fn new(
        commands: mpsc::UnboundedSender<ClientCommand>,
        game_actions: mpsc::UnboundedSender<GameActionMessage>,
    ) -> Self {
        Self {
            commands,
            game_actions,
        }
    }

    /// Send a session-level command.
    pub fn send(&self, command: ClientCommand) -> Result<(), ClientLoopStopped> {
        self.commands.send(command).map_err(|_| ClientLoopStopped)
    }

    /// Send a game action. Prefer the typed helpers below where they fit.
    pub fn send_game_action(&self, action: GameActionMessage) -> Result<(), ClientLoopStopped> {
        self.game_actions
            .send(action)
            .map_err(|_| ClientLoopStopped)
    }

    /// Send a general chat message, equivalent to `/say`.
    pub fn say(&self, message: impl Into<String>) -> Result<(), ClientLoopStopped> {
        let message = message.into();
        info!(target: "net", "Sending chat say: {}", message);
        self.send_game_action(GameActionMessage::CommunicationTalk(CommunicationTalk {
            message,
        }))
    }

    /// Send a private message to another player by name.
    pub fn tell(
        &self,
        recipient: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<(), ClientLoopStopped> {
        let (target_name, message) = (recipient.into(), message.into());
        info!(target: "net", "Sending tell to '{}': {}", target_name, message);
        self.send_game_action(GameActionMessage::CommunicationTalkDirectByName(
            CommunicationTalkDirectByName {
                message,
                target_name,
            },
        ))
    }

    /// Send a movement command (`MovementDoMovementCommand`).
    ///
    /// `hold_key` is a [`HoldKey`] discriminant: `0` is Invalid, `1` None,
    /// `2` Run. Anything unrecognised is sent as `None`.
    pub fn do_movement_command(
        &self,
        motion: u32,
        speed: f32,
        hold_key: u32,
    ) -> Result<(), ClientLoopStopped> {
        let hold_key = HoldKey::try_from(hold_key).unwrap_or(HoldKey::None);
        info!(target: "net", "Sending movement command: motion=0x{:08X}, speed={}, hold_key={:?}",
            motion, speed, hold_key);
        self.send_game_action(GameActionMessage::MovementDoMovementCommand(
            MovementDoMovementCommand {
                motion,
                speed,
                hold_key,
            },
        ))
    }

    /// Stop a movement command. `motion` and `hold_key` must match the
    /// originating [`do_movement_command`](Self::do_movement_command).
    pub fn stop_movement_command(
        &self,
        motion: u32,
        hold_key: u32,
    ) -> Result<(), ClientLoopStopped> {
        let hold_key = HoldKey::try_from(hold_key).unwrap_or(HoldKey::None);
        info!(target: "net", "Sending stop movement command: motion=0x{:08X}, hold_key={:?}",
            motion, hold_key);
        self.send_game_action(GameActionMessage::MovementStopMovementCommand(
            MovementStopMovementCommand { motion, hold_key },
        ))
    }

    /// Request entry into the world for a character.
    ///
    /// Only valid while the client is in `Scene::CharacterSelect`; otherwise
    /// the client logs the rejection and the character stays at select.
    pub fn enter_world(
        &self,
        character_id: u32,
        character_name: impl Into<String>,
        account: impl Into<String>,
    ) -> Result<(), ClientLoopStopped> {
        self.send(ClientCommand::EnterWorld {
            character_id,
            character_name: character_name.into(),
            account: account.into(),
        })
    }

    /// Acknowledge login by sending `LoginComplete` to the server.
    pub fn send_login_complete(&self) -> Result<(), ClientLoopStopped> {
        self.send(ClientCommand::SendLoginComplete)
    }

    /// Ask the client to disconnect. The driver loop stops on its next turn.
    pub fn disconnect(&self) -> Result<(), ClientLoopStopped> {
        self.send(ClientCommand::Disconnect)
    }

    /// Whether the client is still listening on both channels.
    ///
    /// Useful for reporting a connection failure to a user rather than waiting
    /// for a send to fail.
    pub fn is_connected(&self) -> bool {
        !self.commands.is_closed() && !self.game_actions.is_closed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sender() -> (ClientSender, mpsc::UnboundedReceiver<ClientCommand>) {
        let (commands, command_rx) = mpsc::unbounded_channel();
        let (game_actions, _game_action_rx) = mpsc::unbounded_channel();
        (ClientSender::new(commands, game_actions), command_rx)
    }

    /// A sender plus the receiving end of its game-action channel.
    fn game_action_sender() -> (ClientSender, mpsc::UnboundedReceiver<GameActionMessage>) {
        let (commands, _command_rx) = mpsc::unbounded_channel();
        let (game_actions, game_action_rx) = mpsc::unbounded_channel();
        (ClientSender::new(commands, game_actions), game_action_rx)
    }

    #[test]
    fn typed_helpers_map_onto_game_actions() {
        let (sender, mut rx) = game_action_sender();

        sender.say("hello").expect("queued");
        sender.tell("Ari", "psst").expect("queued");

        assert!(matches!(
            rx.try_recv(),
            Ok(GameActionMessage::CommunicationTalk(_))
        ));
        assert!(matches!(
            rx.try_recv(),
            Ok(GameActionMessage::CommunicationTalkDirectByName(_))
        ));
    }

    #[test]
    fn unknown_hold_keys_degrade_to_none() {
        let (sender, mut rx) = game_action_sender();

        sender
            .do_movement_command(0x48, 1.0, u32::MAX)
            .expect("queued");

        match rx.try_recv() {
            Ok(GameActionMessage::MovementDoMovementCommand(movement)) => {
                assert_eq!(movement.hold_key, HoldKey::None);
            }
            other => panic!("expected a movement command, got {other:?}"),
        }
    }

    #[test]
    fn enter_world_carries_the_session_fields() {
        let (sender, mut rx) = sender();

        sender.enter_world(7, "Ari", "account").expect("queued");

        match rx.try_recv() {
            Ok(ClientCommand::EnterWorld {
                character_id,
                character_name,
                account,
            }) => {
                assert_eq!(character_id, 7);
                assert_eq!(character_name, "Ari");
                assert_eq!(account, "account");
            }
            other => panic!("expected EnterWorld, got {other:?}"),
        }
    }

    #[test]
    fn sending_after_the_client_goes_away_reports_stopped() {
        let (sender, rx) = sender();
        drop(rx);

        assert_eq!(sender.disconnect(), Err(ClientLoopStopped));
        assert!(!sender.is_connected());
    }
}

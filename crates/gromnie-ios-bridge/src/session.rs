use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use gromnie_client::client::{Client, ClientEvent};
use gromnie_client::transport::{ClientTransport, NativeUdpTransport};
use gromnie_events::{ClientSystemEvent, SimpleClientAction, SimpleGameEvent};
use tokio::sync::mpsc::{Receiver as CommandReceiver, Sender as CommandSender};

use crate::event::{BridgeEvent, BridgeEventKind, Character};
const COMMAND_CAPACITY: usize = 1_024;
const EVENT_CAPACITY: usize = 4_096;
/// `gromnie-client` publishes raw events with `try_send`, so this buffer is the
/// only protection against drops before the actor forwards them. It matches the
/// bridge event queue; a burst larger than this between two actor iterations
/// (50 ms) can still drop raw events, which is acceptable for a chat-only v1.
const RAW_EVENT_CAPACITY: usize = 4_096;
/// The client does not retransmit `CharacterEnterWorldRequest`, so give up if
/// the character login is not acknowledged within this window.
const ENTER_WORLD_TIMEOUT: Duration = Duration::from_secs(20);
/// How long the initial login may sit without a server response before we
/// surface an authentication failure. Rejected credentials are usually reported
/// much faster via the server's `LoginAccountBooted` message (see
/// `message_handlers.rs`); this is the fallback for an unreachable or silent
/// host so the app never pins the "Connecting…" spinner. Shorter than the
/// client's 20s default.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub enum Command {
    SelectCharacter(u32),
    SendChat(String),
    Disconnect,
}

pub struct RunningSession {
    pub command_tx: CommandSender<Command>,
    pub event_rx: Receiver<BridgeEvent>,
    pub worker: thread::JoinHandle<()>,
}

pub fn start(host: String, port: u16, username: String, password: String) -> RunningSession {
    start_with_timeout(host, port, username, password, LOGIN_TIMEOUT, None)
}

/// Start a session with an explicitly supplied transport.
///
/// Production code passes `None` so the actor binds a native UDP socket; tests
/// pass a fake transport to drive the actor without a network.
pub fn start_with_transport(
    host: String,
    port: u16,
    username: String,
    password: String,
    transport: Option<Box<dyn ClientTransport>>,
) -> RunningSession {
    start_with_timeout(host, port, username, password, LOGIN_TIMEOUT, transport)
}

/// Start a session with both a custom login timeout and transport; tests use a
/// short timeout to exercise the login-failure path quickly.
fn start_with_timeout(
    host: String,
    port: u16,
    username: String,
    password: String,
    login_timeout: Duration,
    transport: Option<Box<dyn ClientTransport>>,
) -> RunningSession {
    let (command_tx, command_rx) = tokio::sync::mpsc::channel(COMMAND_CAPACITY);
    let (event_tx, event_rx) = mpsc::sync_channel(EVENT_CAPACITY);
    let worker = thread::Builder::new()
        .name("gromnie-ios-session".to_string())
        .spawn(move || {
            run(
                host,
                port,
                username,
                password,
                command_rx,
                event_tx,
                login_timeout,
                transport,
            )
        })
        .expect("failed to create gromnie session thread");

    RunningSession {
        command_tx,
        event_rx,
        worker,
    }
}

fn run(
    host: String,
    port: u16,
    username: String,
    password: String,
    command_rx: CommandReceiver<Command>,
    event_tx: SyncSender<BridgeEvent>,
    login_timeout: Duration,
    transport: Option<Box<dyn ClientTransport>>,
) {
    let mut emitter = Emitter::new(event_tx);
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            emitter.error("runtime", error.to_string(), "form");
            emitter.disconnected("failed to start Rust runtime".to_string(), false);
            return;
        }
    };

    // A panic anywhere in the client loop must still produce a terminal event so
    // Swift never waits forever on a dead actor.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.block_on(run_client(
            host,
            port,
            username,
            password,
            command_rx,
            login_timeout,
            &mut emitter,
            transport,
        ));
    }));
    if outcome.is_err() {
        emitter.error(
            "internal",
            "The session actor stopped unexpectedly.".to_string(),
            "form",
        );
        emitter.disconnected("internal session error".to_string(), false);
    }
}

async fn run_client(
    host: String,
    port: u16,
    username: String,
    password: String,
    mut command_rx: CommandReceiver<Command>,
    login_timeout: Duration,
    emitter: &mut Emitter,
    transport: Option<Box<dyn ClientTransport>>,
) {
    emitter.emit(BridgeEventKind::Connecting);

    let (event_sender, mut raw_events) = tokio::sync::mpsc::channel(RAW_EVENT_CAPACITY);
    let address = format!("{host}:{port}");
    let transport: Box<dyn ClientTransport> = match transport {
        Some(transport) => transport,
        None => match NativeUdpTransport::bind_ephemeral().await {
            Ok(transport) => Box::new(transport),
            Err(error) => {
                emitter.error("network", error.to_string(), "form");
                emitter.disconnected("could not open a UDP socket".to_string(), false);
                return;
            }
        },
    };
    let (mut client, action_tx) = Client::new_with_transport(
        1,
        address,
        username.clone(),
        password,
        None,
        event_sender,
        false,
        transport,
    )
    .await;
    client.set_login_timeout(login_timeout);

    if let Err(error) = client.do_login().await {
        emitter.error("connect", error.to_string(), "form");
        emitter.disconnected("could not send login request".to_string(), false);
        return;
    }

    let mut character_names = HashMap::new();
    let mut last_keepalive = tokio::time::Instant::now();
    let mut entering_world_since: Option<tokio::time::Instant> = None;
    let mut buffer = [0_u8; 65_536];
    let mut user_initiated = false;
    let terminal_reason = 'session: loop {
        match tokio::time::timeout(Duration::from_millis(50), client.recv_packet(&mut buffer)).await
        {
            Ok(Ok((size, peer))) => {
                client.process_packet(&buffer[..size], size, &peer).await;
                if client.has_messages() {
                    client.process_messages();
                }
                client.process_actions();
                client.process_game_actions();
                if client.has_pending_outgoing_messages()
                    && let Err(error) = client.send_pending_messages().await
                {
                    emitter.error("network", error.to_string(), "form");
                    break 'session "failed to send game data".to_string();
                }
            }
            Ok(Err(error)) => {
                emitter.error("network", error.to_string(), "form");
                break 'session "UDP receive failed".to_string();
            }
            Err(_) => {}
        }

        loop {
            match command_rx.try_recv() {
                Ok(Command::SelectCharacter(character_id)) => {
                    let Some(character_name) = character_names.get(&character_id).cloned() else {
                        emitter.error(
                            "character",
                            "The selected character is not available.".to_string(),
                            "characters",
                        );
                        continue;
                    };
                    if action_tx
                        .send(SimpleClientAction::LoginCharacter {
                            character_id,
                            character_name,
                            account: username.clone(),
                        })
                        .is_err()
                    {
                        emitter.error(
                            "internal",
                            "The client action channel closed.".to_string(),
                            "form",
                        );
                        break 'session "client action channel closed".to_string();
                    }
                    client.process_actions();
                    client.process_game_actions();
                    if let Err(error) = client.send_pending_messages().await {
                        emitter.error("network", error.to_string(), "characters");
                    } else {
                        entering_world_since = Some(tokio::time::Instant::now());
                        emitter.emit(BridgeEventKind::EnteringWorld { character_id });
                    }
                }
                Ok(Command::SendChat(message)) => {
                    if action_tx
                        .send(SimpleClientAction::SendChatSay { message })
                        .is_err()
                    {
                        emitter.error(
                            "internal",
                            "The client action channel closed.".to_string(),
                            "form",
                        );
                        break 'session "client action channel closed".to_string();
                    }
                    client.process_actions();
                    client.process_game_actions();
                    if let Err(error) = client.send_pending_messages().await {
                        emitter.error("network", error.to_string(), "form");
                        break 'session "failed to send chat".to_string();
                    }
                }
                Ok(Command::Disconnect) => {
                    user_initiated = true;
                    break 'session "disconnected by user".to_string();
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    break 'session "session handle dropped".to_string();
                }
            }
        }

        while let Ok(event) = raw_events.try_recv() {
            if matches!(
                event,
                ClientEvent::Game(
                    SimpleGameEvent::LoginSucceeded { .. }
                        | SimpleGameEvent::CharacterError { .. }
                        | SimpleGameEvent::LoginFailed { .. }
                )
            ) {
                entering_world_since = None;
            }
            // An authentication failure ends the session. Servers accept any
            // LoginRequest at the transport handshake and reject the
            // credentials afterwards with `LoginAccountBooted`; `check_state_timeout`
            // covers the silent-host case. Either way, forward the failure
            // before the terminal event so Swift returns to the server form
            // with the real reason instead of pinning "Connecting…".
            if matches!(
                event,
                ClientEvent::System(ClientSystemEvent::AuthenticationFailed { .. })
            ) {
                forward_event(emitter, event, &mut character_names);
                break 'session "authentication failed".to_string();
            }
            forward_event(emitter, event, &mut character_names);
        }

        // Give up on a stalled handshake instead of leaving the UI stuck on a
        // spinner. `check_state_timeout` emits an AuthenticationFailed event;
        // the final drain below forwards it before the terminal event.
        if client.check_state_timeout() {
            break 'session "connection timed out".to_string();
        }

        // A dropped `EnterWorldRequest`/`LoginCreatePlayer` would otherwise
        // leave the UI on "Entering world…" forever.
        if let Some(since) = entering_world_since
            && since.elapsed() >= ENTER_WORLD_TIMEOUT
        {
            emitter.error(
                "character",
                "The server did not respond to the character login.".to_string(),
                "characters",
            );
            break 'session "entering world timed out".to_string();
        }

        // If the initial LoginRequest was lost, retry it like the native
        // runner does (only true while the scene is still Connecting).
        if client.should_retry() {
            if let Err(error) = client.do_login().await {
                emitter.error("connect", error.to_string(), "form");
                break 'session "could not send login retry".to_string();
            }
            client.update_retry_time();
        }

        if last_keepalive.elapsed() >= Duration::from_secs(5) {
            if let Err(error) = client.send_keepalive().await {
                emitter.error("network", error.to_string(), "form");
                break 'session "failed to send keepalive".to_string();
            }
            last_keepalive = tokio::time::Instant::now();
        }
    };

    // Forward anything the client emitted on the way out (for example the
    // timeout's AuthenticationFailed) before the terminal disconnected event.
    while let Ok(event) = raw_events.try_recv() {
        forward_event(emitter, event, &mut character_names);
    }

    emitter.disconnected(terminal_reason, user_initiated);
}

fn forward_event(
    emitter: &mut Emitter,
    event: ClientEvent,
    character_names: &mut HashMap<u32, String>,
) {
    match event {
        ClientEvent::Game(SimpleGameEvent::CharacterListReceived {
            account,
            characters,
            num_slots,
        }) => {
            let characters = characters
                .into_iter()
                .map(|character| {
                    character_names.insert(character.character_id.0, character.name.clone());
                    Character {
                        id: character.character_id.0,
                        name: character.name,
                    }
                })
                .collect();
            emitter.emit(BridgeEventKind::Characters {
                account,
                slots: num_slots,
                characters,
            });
        }
        ClientEvent::Game(SimpleGameEvent::LoginSucceeded {
            character_id,
            character_name,
        }) => {
            emitter.emit(BridgeEventKind::EnteredWorld {
                character_id,
                character_name,
            });
        }
        ClientEvent::Game(SimpleGameEvent::ChatMessageReceived {
            message,
            message_type,
        }) => {
            emitter.emit(BridgeEventKind::Chat {
                message,
                message_type,
            });
        }
        ClientEvent::Game(SimpleGameEvent::LoginFailed { reason }) => {
            emitter.error("login", reason, "characters")
        }
        ClientEvent::Game(SimpleGameEvent::CharacterError { error_message, .. }) => {
            emitter.error("character", error_message, "characters");
        }
        ClientEvent::System(ClientSystemEvent::AuthenticationFailed { reason }) => {
            emitter.error("authentication", reason, "form")
        }
        _ => {}
    }
}

struct Emitter {
    event_tx: SyncSender<BridgeEvent>,
    sequence: u64,
}

impl Emitter {
    fn new(event_tx: SyncSender<BridgeEvent>) -> Self {
        Self {
            event_tx,
            sequence: 0,
        }
    }

    fn emit(&mut self, kind: BridgeEventKind) {
        self.sequence += 1;
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let _ = self.event_tx.send(BridgeEvent {
            sequence: self.sequence,
            timestamp_ms,
            kind,
        });
    }

    fn error(&mut self, code: &'static str, message: String, recover_to: &'static str) {
        self.emit(BridgeEventKind::Error {
            code,
            message,
            recover_to,
        });
    }

    fn disconnected(&mut self, reason: String, user_initiated: bool) {
        self.emit(BridgeEventKind::Disconnected {
            reason,
            user_initiated,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gromnie_client::client::ServerInfo;
    use gromnie_client::transport::{TransportChannel, TransportFuture};
    use gromnie_events::ClientStateEvent;
    use std::net::SocketAddr;

    /// A transport that accepts outgoing packets but fails every receive, which
    /// exercises the actor's fatal-error path without a network.
    struct FailingTransport;

    impl ClientTransport for FailingTransport {
        fn send<'a>(
            &'a mut self,
            _server: &'a ServerInfo,
            _channel: TransportChannel,
            _bytes: Vec<u8>,
        ) -> TransportFuture<'a, ()> {
            Box::pin(async { Ok(()) })
        }

        fn recv<'a>(&'a mut self, _buf: &'a mut [u8]) -> TransportFuture<'a, (usize, SocketAddr)> {
            Box::pin(async {
                Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionReset,
                    "fake transport failure",
                ))
            })
        }
    }

    /// A transport that accepts outgoing packets and never delivers one, so the
    /// actor idles until it is told to disconnect.
    struct PendingTransport;

    impl ClientTransport for PendingTransport {
        fn send<'a>(
            &'a mut self,
            _server: &'a ServerInfo,
            _channel: TransportChannel,
            _bytes: Vec<u8>,
        ) -> TransportFuture<'a, ()> {
            Box::pin(async { Ok(()) })
        }

        fn recv<'a>(&'a mut self, _buf: &'a mut [u8]) -> TransportFuture<'a, (usize, SocketAddr)> {
            Box::pin(std::future::pending::<
                Result<(usize, SocketAddr), std::io::Error>,
            >())
        }
    }

    fn emitter_pair() -> (Emitter, Receiver<BridgeEvent>) {
        let (tx, rx) = mpsc::sync_channel(EVENT_CAPACITY);
        (Emitter::new(tx), rx)
    }

    fn character(id: u32, name: &str) -> asheron_rs::types::CharacterIdentity {
        asheron_rs::types::CharacterIdentity {
            character_id: asheron_rs::types::ObjectId(id),
            name: name.to_string(),
            seconds_greyed_out: 0,
        }
    }

    fn collect_until_terminal(rx: &Receiver<BridgeEvent>) -> Vec<BridgeEvent> {
        let mut events = Vec::new();
        while let Ok(event) = rx.recv_timeout(Duration::from_secs(5)) {
            let terminal = matches!(event.kind, BridgeEventKind::Disconnected { .. });
            events.push(event);
            if terminal {
                break;
            }
        }
        events
    }

    fn terminal_count(events: &[BridgeEvent]) -> usize {
        events
            .iter()
            .filter(|event| matches!(event.kind, BridgeEventKind::Disconnected { .. }))
            .count()
    }

    #[test]
    fn forward_event_maps_character_list_and_records_names() {
        let (mut emitter, rx) = emitter_pair();
        let mut names = HashMap::new();
        forward_event(
            &mut emitter,
            ClientEvent::Game(SimpleGameEvent::CharacterListReceived {
                account: "acct".to_string(),
                characters: vec![character(1, "Alice"), character(2, "Bob")],
                num_slots: 5,
            }),
            &mut names,
        );

        match rx.try_recv().expect("expected a characters event").kind {
            BridgeEventKind::Characters {
                account,
                slots,
                characters,
            } => {
                assert_eq!(account, "acct");
                assert_eq!(slots, 5);
                assert_eq!(characters.len(), 2);
                assert_eq!(characters[0].id, 1);
                assert_eq!(characters[0].name, "Alice");
            }
            other => panic!("unexpected event: {other:?}"),
        }
        assert_eq!(names.get(&1).map(String::as_str), Some("Alice"));
        assert_eq!(names.get(&2).map(String::as_str), Some("Bob"));
        assert!(rx.try_recv().is_err(), "only one event should be emitted");
    }

    #[test]
    fn forward_event_maps_login_succeeded_to_entered_world() {
        let (mut emitter, rx) = emitter_pair();
        let mut names = HashMap::new();
        forward_event(
            &mut emitter,
            ClientEvent::Game(SimpleGameEvent::LoginSucceeded {
                character_id: 7,
                character_name: "Alice".to_string(),
            }),
            &mut names,
        );

        match rx.try_recv().expect("expected an entered_world event").kind {
            BridgeEventKind::EnteredWorld {
                character_id,
                character_name,
            } => {
                assert_eq!(character_id, 7);
                assert_eq!(character_name, "Alice");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn forward_event_maps_chat() {
        let (mut emitter, rx) = emitter_pair();
        let mut names = HashMap::new();
        forward_event(
            &mut emitter,
            ClientEvent::Game(SimpleGameEvent::ChatMessageReceived {
                message: "hello".to_string(),
                message_type: 2,
            }),
            &mut names,
        );

        match rx.try_recv().expect("expected a chat event").kind {
            BridgeEventKind::Chat {
                message,
                message_type,
            } => {
                assert_eq!(message, "hello");
                assert_eq!(message_type, 2);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn forward_event_routes_failures_to_the_right_screen() {
        let cases = [
            (
                ClientEvent::Game(SimpleGameEvent::LoginFailed {
                    reason: "nope".to_string(),
                }),
                "login",
                "characters",
            ),
            (
                ClientEvent::Game(SimpleGameEvent::CharacterError {
                    error_code: 1,
                    error_message: "bad".to_string(),
                }),
                "character",
                "characters",
            ),
            (
                ClientEvent::System(ClientSystemEvent::AuthenticationFailed {
                    reason: "bad".to_string(),
                }),
                "authentication",
                "form",
            ),
        ];

        for (event, code, recover_to) in cases {
            let (mut emitter, rx) = emitter_pair();
            let mut names = HashMap::new();
            forward_event(&mut emitter, event, &mut names);

            match rx.try_recv().expect("expected an error event").kind {
                BridgeEventKind::Error {
                    code: actual_code,
                    recover_to: actual_recover,
                    ..
                } => {
                    assert_eq!(actual_code, code);
                    assert_eq!(actual_recover, recover_to);
                }
                other => panic!("unexpected event: {other:?}"),
            }
        }
    }

    #[test]
    fn forward_event_ignores_unmapped_events() {
        let (mut emitter, rx) = emitter_pair();
        let mut names = HashMap::new();
        forward_event(
            &mut emitter,
            ClientEvent::State(ClientStateEvent::InWorld),
            &mut names,
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn emitter_sequences_events_monotonically() {
        let (mut emitter, rx) = emitter_pair();
        emitter.emit(BridgeEventKind::Connecting);
        emitter.disconnected("done".to_string(), true);

        let first = rx.try_recv().unwrap();
        let second = rx.try_recv().unwrap();
        assert_eq!(first.sequence, 1);
        assert_eq!(second.sequence, 2);
        assert!(second.timestamp_ms >= first.timestamp_ms);
    }

    #[test]
    fn actor_reports_transport_failure_then_one_terminal_event() {
        let session = start_with_transport(
            "127.0.0.1".to_string(),
            9000,
            "acct".to_string(),
            "pw".to_string(),
            Some(Box::new(FailingTransport)),
        );

        let events = collect_until_terminal(&session.event_rx);
        session.worker.join().unwrap();

        assert!(matches!(
            events.first().map(|event| &event.kind),
            Some(BridgeEventKind::Connecting)
        ));
        assert!(
            events
                .iter()
                .any(|event| matches!(event.kind, BridgeEventKind::Error { .. })),
            "a fatal transport error must surface as an error event"
        );
        assert_eq!(terminal_count(&events), 1);
        assert!(matches!(
            events.last().map(|event| &event.kind),
            Some(BridgeEventKind::Disconnected {
                user_initiated: false,
                ..
            })
        ));
    }

    #[test]
    fn actor_disconnect_command_emits_one_terminal_event() {
        let session = start_with_transport(
            "127.0.0.1".to_string(),
            9000,
            "acct".to_string(),
            "pw".to_string(),
            Some(Box::new(PendingTransport)),
        );

        let first = session
            .event_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("expected a connecting event");
        assert!(matches!(first.kind, BridgeEventKind::Connecting));

        session.command_tx.try_send(Command::Disconnect).unwrap();

        let events = collect_until_terminal(&session.event_rx);
        session.worker.join().unwrap();

        assert_eq!(terminal_count(&events), 1);
        assert!(matches!(
            events.last().map(|event| &event.kind),
            Some(BridgeEventKind::Disconnected {
                user_initiated: true,
                ..
            })
        ));
    }
}

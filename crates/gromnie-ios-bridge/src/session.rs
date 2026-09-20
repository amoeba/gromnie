use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use gromnie_client::client::{Client, ClientEvent};
use gromnie_events::{ClientSystemEvent, SimpleClientAction, SimpleGameEvent};
use tokio::sync::mpsc::{Receiver as CommandReceiver, Sender as CommandSender};

use crate::event::{BridgeEvent, BridgeEventKind, Character};
const COMMAND_CAPACITY: usize = 1_024;
const EVENT_CAPACITY: usize = 4_096;

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
    let (command_tx, command_rx) = tokio::sync::mpsc::channel(COMMAND_CAPACITY);
    let (event_tx, event_rx) = mpsc::sync_channel(EVENT_CAPACITY);
    let worker = thread::Builder::new()
        .name("gromnie-ios-session".to_string())
        .spawn(move || run(host, port, username, password, command_rx, event_tx))
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
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let mut emitter = Emitter::new(event_tx);
            emitter.error("runtime", error.to_string(), "form");
            emitter.disconnected("failed to start Rust runtime".to_string(), false);
            return;
        }
    };

    runtime.block_on(run_client(
        host, port, username, password, command_rx, event_tx,
    ));
}

async fn run_client(
    host: String,
    port: u16,
    username: String,
    password: String,
    mut command_rx: CommandReceiver<Command>,
    event_tx: SyncSender<BridgeEvent>,
) {
    let mut emitter = Emitter::new(event_tx);
    emitter.emit(BridgeEventKind::Connecting);

    let (event_sender, mut raw_events) = tokio::sync::mpsc::channel(1_024);
    let address = format!("{host}:{port}");
    let (mut client, action_tx) = Client::new(
        1,
        address,
        username.clone(),
        password,
        None,
        event_sender,
        false,
    )
    .await;

    if let Err(error) = client.do_login().await {
        emitter.error("connect", error.to_string(), "form");
        emitter.disconnected("could not send login request".to_string(), false);
        return;
    }

    let mut character_names = HashMap::new();
    let mut last_keepalive = tokio::time::Instant::now();
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
            forward_event(&mut emitter, event, &mut character_names);
        }

        if last_keepalive.elapsed() >= Duration::from_secs(5) {
            if let Err(error) = client.send_keepalive().await {
                emitter.error("network", error.to_string(), "form");
                break 'session "failed to send keepalive".to_string();
            }
            last_keepalive = tokio::time::Instant::now();
        }
    };

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

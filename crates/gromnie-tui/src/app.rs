use acprotocol::types::CharacterIdentity;
use gromnie_events::{ClientStateEvent, SimpleClientAction, SimpleGameEvent};

// Type alias for backward compatibility
pub type GameEvent = SimpleGameEvent;
use std::collections::VecDeque;
use tokio::sync::{broadcast, mpsc};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppView {
    Game,
    Debug,
}

/// Session state - protocol-level connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Unknown,
    AuthLoginRequest,
    AuthConnectResponse,
    AuthConnected,
    WorldConnected,
}

impl SessionState {
    /// Display name for the session state
    pub fn display_name(&self) -> &'static str {
        match self {
            SessionState::Unknown => "Unknown",
            SessionState::AuthLoginRequest => "AuthLoginRequest",
            SessionState::AuthConnectResponse => "AuthConnectResponse",
            SessionState::AuthConnected => "AuthConnected",
            SessionState::WorldConnected => "WorldConnected",
        }
    }
}

/// Scene state - UI-level state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneState {
    Unknown,
    Connecting,
    CharacterSelect,
    EnteringWorld,
    InWorld,
    Error(String),
}

impl SceneState {
    /// Display name for the scene state
    pub fn display_name(&self) -> String {
        match self {
            SceneState::Unknown => "Unknown".to_string(),
            SceneState::Connecting => "Connecting".to_string(),
            SceneState::CharacterSelect => "CharacterSelect".to_string(),
            SceneState::EnteringWorld => "EnteringWorld".to_string(),
            SceneState::InWorld => "InWorld".to_string(),
            SceneState::Error(msg) => format!("Error: {}", msg),
        }
    }
}

/// Scene states for the GameView
#[derive(Debug, Clone, PartialEq)]
pub enum GameScene {
    /// Authenticating and waiting for DDD
    Logging {
        authenticated: bool,
        ddd_received: bool,
    },
    /// Character selection - showing list and allowing selection
    CharacterSelect,
    /// In game world - showing created objects
    GameWorld {
        state: GameWorldState,
        created_objects: Vec<(u32, String)>,
    },
    /// Error state - showing an error message
    Error(String),
}

/// Sub-states within the GameWorld scene
#[derive(Debug, Clone, PartialEq)]
pub enum GameWorldState {
    /// Logging in - waiting for LoginComplete notification
    LoggingIn,
    /// Logged in and active in game
    LoggedIn,
    /// Logging out - received LogOff notification
    LoggingOut,
}

/// Tabs available in the GameWorld scene
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameWorldTab {
    World,
    Chat,
    Map,
    Inventory,
}

#[derive(Debug, Clone)]
pub struct ClientStatus {
    pub account_name: String,
    pub current_character: Option<String>,
    pub characters: Vec<CharacterIdentity>,
    /// Session state from the client (protocol-level state)
    pub session_state: SessionState,
    /// Scene state from the client (UI-level state)
    pub scene_state: SceneState,
}

impl ClientStatus {
    /// Check if the client is connected (has progressed beyond initial connection states)
    pub fn is_connected(&self) -> bool {
        matches!(
            self.session_state,
            SessionState::AuthConnectResponse
                | SessionState::AuthConnected
                | SessionState::WorldConnected
        )
    }

    /// Check if the client is logged in (in world with a character)
    pub fn is_logged_in(&self) -> bool {
        self.session_state == SessionState::WorldConnected
            && matches!(self.scene_state, SceneState::InWorld)
            && self.current_character.is_some()
    }

    /// Get a human-readable connection status
    pub fn connection_status(&self) -> String {
        if self.is_logged_in() {
            format!(
                "Logged in {}",
                self.current_character.as_deref().unwrap_or("Unknown")
            )
        } else if self.is_connected() {
            "Connected".to_string()
        } else {
            "Disconnected".to_string()
        }
    }
}

impl Default for ClientStatus {
    fn default() -> Self {
        Self {
            account_name: String::new(),
            current_character: None,
            characters: Vec::new(),
            session_state: SessionState::Unknown,
            scene_state: SceneState::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub enum NetworkMessage {
    Sent {
        opcode: String,
        description: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    Received {
        opcode: String,
        description: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

/// A chat message to display in the chat window
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub text: String,
    pub message_type: u32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct App {
    pub should_quit: bool,
    pub current_view: AppView,
    pub game_scene: GameScene,
    pub client_status: ClientStatus,
    pub network_messages: VecDeque<NetworkMessage>,
    pub max_network_messages: usize,
    pub event_rx: Option<broadcast::Receiver<GameEvent>>,
    pub action_tx: Option<mpsc::UnboundedSender<SimpleClientAction>>,
    /// Currently selected character index in the character list
    pub selected_character_index: usize,
    /// Chat messages received from the server
    pub chat_messages: VecDeque<ChatMessage>,
    pub max_chat_messages: usize,
    /// Current chat input text being typed by the user
    pub chat_input: String,
    /// Whether the chat input is active (visible and ready for input)
    pub chat_input_active: bool,
    /// Currently active tab in the GameWorld scene
    pub game_world_tab: GameWorldTab,

    /// Progress for connecting/authenticating (0.0 to 1.0)
    pub connecting_progress: f64,
    /// Progress for updating/DDD (0.0 to 1.0)
    pub updating_progress: f64,
    /// Timestamp of last progress update for connecting
    pub last_connecting_update: Option<std::time::Instant>,
    /// Timestamp of last progress update for updating
    pub last_updating_update: Option<std::time::Instant>,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            current_view: AppView::Game,
            game_scene: GameScene::Logging {
                authenticated: false,
                ddd_received: false,
            },
            client_status: ClientStatus::default(),
            network_messages: VecDeque::new(),
            max_network_messages: 1000,
            event_rx: None,
            action_tx: None,
            selected_character_index: 0,
            chat_messages: VecDeque::new(),
            max_chat_messages: 100,
            chat_input: String::new(),
            chat_input_active: false,
            game_world_tab: GameWorldTab::World,
            connecting_progress: 0.0,
            updating_progress: 0.0,
            last_connecting_update: None,
            last_updating_update: None,
        }
    }

    pub fn set_channels(
        &mut self,
        event_rx: broadcast::Receiver<GameEvent>,
        action_tx: mpsc::UnboundedSender<SimpleClientAction>,
    ) {
        self.event_rx = Some(event_rx);
        self.action_tx = Some(action_tx);
    }

    pub fn switch_view(&mut self, view: AppView) {
        self.current_view = view;
    }

    pub fn add_network_message(&mut self, message: NetworkMessage) {
        self.network_messages.push_back(message);
        if self.network_messages.len() > self.max_network_messages {
            self.network_messages.pop_front();
        }
    }

    pub fn update_from_system_event(&mut self, event: gromnie_runner::SystemEvent) {
        match event {
            gromnie_runner::SystemEvent::AuthenticationSucceeded { .. } => {
                // Mark that authentication succeeded (received ConnectRequest)
                if let GameScene::Logging { ddd_received, .. } = self.game_scene {
                    self.game_scene = GameScene::Logging {
                        authenticated: true,
                        ddd_received,
                    };
                }

                self.add_network_message(NetworkMessage::Received {
                    opcode: "CONNECT".to_string(),
                    description: "Authentication succeeded - ConnectRequest received".to_string(),
                    timestamp: chrono::Utc::now(),
                });
            }
            gromnie_runner::SystemEvent::AuthenticationFailed { reason, .. } => {
                self.client_status.scene_state = SceneState::Error(reason.clone());

                self.add_network_message(NetworkMessage::Received {
                    opcode: "ERROR".to_string(),
                    description: format!("Authentication failed: {}", reason),
                    timestamp: chrono::Utc::now(),
                });
            }
            gromnie_runner::SystemEvent::LoginSucceeded {
                character_id,
                character_name,
            } => {
                self.client_status.current_character = Some(character_name.clone());

                // Transition from LoggingIn to LoggedIn
                if let GameScene::GameWorld { ref mut state, .. } = self.game_scene {
                    *state = GameWorldState::LoggedIn;
                }

                self.add_network_message(NetworkMessage::Received {
                    opcode: "0xF656".to_string(),
                    description: format!(
                        "Login succeeded: {} (ID: {})",
                        character_name, character_id
                    ),
                    timestamp: chrono::Utc::now(),
                });
            }
            _ => {
                // Other system events don't need special handling in the TUI
            }
        }
    }

    pub fn update_from_event(&mut self, event: GameEvent) {
        match event {
            GameEvent::CharacterListReceived {
                account,
                characters,
                num_slots: _,
            } => {
                self.client_status.account_name = account;
                self.client_status.characters = characters;
                self.selected_character_index = 0; // Reset to first character when list updates

                // Transition to CharacterSelect scene when we receive the character list
                if !self.client_status.characters.is_empty() {
                    self.game_scene = GameScene::CharacterSelect;
                }

                self.add_network_message(NetworkMessage::Received {
                    opcode: "0xF4A0".to_string(),
                    description: format!("Character list for {}", self.client_status.account_name),
                    timestamp: chrono::Utc::now(),
                });
            }

            GameEvent::LoginSucceeded {
                character_id,
                character_name,
            } => {
                self.client_status.current_character = Some(character_name.clone());

                // Transition from LoggingIn to LoggedIn
                if let GameScene::GameWorld { ref mut state, .. } = self.game_scene {
                    *state = GameWorldState::LoggedIn;
                }

                self.add_network_message(NetworkMessage::Received {
                    opcode: "0xF656".to_string(),
                    description: format!(
                        "Login succeeded: {} (ID: {})",
                        character_name, character_id
                    ),
                    timestamp: chrono::Utc::now(),
                });
            }
            GameEvent::LoginFailed { reason } => {
                self.add_network_message(NetworkMessage::Received {
                    opcode: "0xF656".to_string(),
                    description: format!("Login failed: {}", reason),
                    timestamp: chrono::Utc::now(),
                });
            }
            GameEvent::CharacterError {
                error_code,
                error_message,
            } => {
                self.game_scene = GameScene::Error(error_message.clone());

                self.add_network_message(NetworkMessage::Received {
                    opcode: format!("0x{:04X}", error_code),
                    description: format!("Character error: {}", error_message),
                    timestamp: chrono::Utc::now(),
                });
            }
            GameEvent::CreateObject {
                object_id,
                object_name,
            } => {
                // Add object to the list if we're in the game world scene
                if let GameScene::GameWorld {
                    ref mut state,
                    ref mut created_objects,
                } = self.game_scene
                {
                    created_objects.push((object_id, object_name.clone()));

                    // After receiving first object while logging in, send LoginComplete notification
                    if *state == GameWorldState::LoggingIn
                        && created_objects.len() == 1
                        && let Some(ref tx) = self.action_tx
                    {
                        let _ = tx.send(SimpleClientAction::SendLoginComplete);
                    }
                }

                self.add_network_message(NetworkMessage::Received {
                    opcode: "0xF745".to_string(),
                    description: format!("CreateObject: {} (ID: {})", object_name, object_id),
                    timestamp: chrono::Utc::now(),
                });
            }
            GameEvent::ChatMessageReceived {
                message,
                message_type,
            } => {
                // Add chat message to the list
                self.add_chat_message(ChatMessage {
                    text: message.clone(),
                    message_type,
                    timestamp: chrono::Utc::now(),
                });

                self.add_network_message(NetworkMessage::Received {
                    opcode: "0xF7E0".to_string(),
                    description: format!("Chat (type {}): {}", message_type, message),
                    timestamp: chrono::Utc::now(),
                });
            }
            GameEvent::ConnectingSetProgress { progress } => {
                self.connecting_progress = progress.clamp(0.0, 1.0);
            }
            GameEvent::UpdatingSetProgress { progress } => {
                self.updating_progress = progress.clamp(0.0, 1.0);
            }

            GameEvent::CreatePlayer { character_id } => {
                self.add_network_message(NetworkMessage::Received {
                    opcode: "0xF7B0".to_string(),
                    description: format!("CreatePlayer: Character ID {}", character_id),
                    timestamp: chrono::Utc::now(),
                });
            }
        }
    }

    pub fn add_chat_message(&mut self, message: ChatMessage) {
        self.chat_messages.push_back(message);
        if self.chat_messages.len() > self.max_chat_messages {
            self.chat_messages.pop_front();
        }
    }

    /// Update from state events from the client
    pub fn update_from_state_event(&mut self, state_event: ClientStateEvent) {
        let (session, scene, game_scene_update) = match state_event {
            ClientStateEvent::Connecting => {
                (SessionState::AuthLoginRequest, SceneState::Connecting, None)
            }
            ClientStateEvent::Connected => (
                SessionState::AuthConnectResponse,
                SceneState::Connecting,
                None,
            ),
            ClientStateEvent::ConnectingFailed { reason } => (
                SessionState::AuthLoginRequest,
                SceneState::Error(reason.clone()),
                Some(GameScene::Error(reason)),
            ),
            ClientStateEvent::Patching => {
                (SessionState::AuthConnected, SceneState::Connecting, None)
            }
            ClientStateEvent::Patched => (
                SessionState::AuthConnected,
                SceneState::CharacterSelect,
                None,
            ),
            ClientStateEvent::PatchingFailed { reason } => (
                SessionState::AuthConnected,
                SceneState::Error(reason.clone()),
                Some(GameScene::Error(reason)),
            ),
            ClientStateEvent::CharacterSelect => (
                SessionState::AuthConnected,
                SceneState::CharacterSelect,
                None,
            ),
            ClientStateEvent::EnteringWorld => {
                (SessionState::AuthConnected, SceneState::EnteringWorld, None)
            }
            ClientStateEvent::InWorld => (SessionState::WorldConnected, SceneState::InWorld, None),
            ClientStateEvent::ExitingWorld => (
                SessionState::AuthConnected,
                SceneState::CharacterSelect,
                None,
            ),
            ClientStateEvent::CharacterError => (
                SessionState::AuthConnected,
                SceneState::Error("Character error".to_string()),
                Some(GameScene::Error("Character error".to_string())),
            ),
        };

        self.client_status.session_state = session;
        self.client_status.scene_state = scene;
        if let Some(scene) = game_scene_update {
            self.game_scene = scene;
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

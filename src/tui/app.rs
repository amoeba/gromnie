use crate::client::events::{ClientAction, GameEvent, CharacterInfo};
use tokio::sync::{broadcast, mpsc};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppView {
    Game,
    Debug,
}

/// Scene states for the GameView
#[derive(Debug, Clone, PartialEq)]
pub enum GameScene {
    /// Authenticating and waiting for DDD
    Logging {
        ddd_received: bool,
    },
    /// Character selection - showing list and allowing selection
    CharacterSelect,
    /// In game world - showing created objects
    GameWorld {
        state: GameWorldState,
        created_objects: Vec<(u32, String)>,
    },
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

#[derive(Debug, Clone)]
pub struct ClientStatus {
    pub connected: bool,
    pub logged_in: bool,
    pub account_name: String,
    pub current_character: Option<String>,
    pub characters: Vec<CharacterInfo>,
}

impl Default for ClientStatus {
    fn default() -> Self {
        Self {
            connected: false,
            logged_in: false,
            account_name: String::new(),
            current_character: None,
            characters: Vec::new(),
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
    pub action_tx: Option<mpsc::UnboundedSender<ClientAction>>,
    /// Currently selected character index in the character list
    pub selected_character_index: usize,
    /// Chat messages received from the server
    pub chat_messages: VecDeque<ChatMessage>,
    pub max_chat_messages: usize,
    /// Current chat input text being typed by the user
    pub chat_input: String,
    /// Whether the chat input is focused (user is typing)
    pub chat_input_focused: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            current_view: AppView::Game,
            game_scene: GameScene::Logging {
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
            chat_input_focused: false,
        }
    }

    pub fn set_channels(
        &mut self,
        event_rx: broadcast::Receiver<GameEvent>,
        action_tx: mpsc::UnboundedSender<ClientAction>,
    ) {
        self.event_rx = Some(event_rx);
        self.action_tx = Some(action_tx);
    }

    pub fn toggle_view(&mut self) {
        self.current_view = match self.current_view {
            AppView::Game => AppView::Debug,
            AppView::Debug => AppView::Game,
        };
    }

    /// Select the next character in the list
    pub fn select_next_character(&mut self) {
        if !self.client_status.characters.is_empty() {
            self.selected_character_index =
                (self.selected_character_index + 1) % self.client_status.characters.len();
        }
    }

    /// Select the previous character in the list
    pub fn select_previous_character(&mut self) {
        if !self.client_status.characters.is_empty() {
            if self.selected_character_index == 0 {
                self.selected_character_index = self.client_status.characters.len() - 1;
            } else {
                self.selected_character_index -= 1;
            }
        }
    }

    /// Get the currently selected character, if any
    pub fn get_selected_character(&self) -> Option<&CharacterInfo> {
        self.client_status
            .characters
            .get(self.selected_character_index)
    }

    /// Login with the selected character
    pub fn login_selected_character(&mut self) -> Result<(), String> {
        // Get character info first to avoid borrow conflicts
        let (character_id, character_name) = if let Some(character) = self.get_selected_character() {
            (character.id, character.name.clone())
        } else {
            return Err("No character selected".to_string());
        };

        if let Some(ref tx) = self.action_tx {
            // Immediately transition to GameWorld::LoggingIn
            self.game_scene = GameScene::GameWorld {
                state: GameWorldState::LoggingIn,
                created_objects: Vec::new(),
            };

            tx.send(ClientAction::LoginCharacter {
                character_id,
                character_name,
                account: self.client_status.account_name.clone(),
            })
            .map_err(|e| format!("Failed to send login action: {}", e))
        } else {
            Err("No action channel available".to_string())
        }
    }

    pub fn add_network_message(&mut self, message: NetworkMessage) {
        self.network_messages.push_back(message);
        if self.network_messages.len() > self.max_network_messages {
            self.network_messages.pop_front();
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
                
                // Transition to character select scene when we have characters
                if !self.client_status.characters.is_empty() {
                    self.game_scene = GameScene::CharacterSelect;
                }
                
                self.add_network_message(NetworkMessage::Received {
                    opcode: "0xF4A0".to_string(),
                    description: format!("Character list for {}", self.client_status.account_name),
                    timestamp: chrono::Utc::now(),
                });
            }
            GameEvent::DDDInterrogation {
                language,
                region,
            } => {
                // Mark that we received DDD interrogation
                if let GameScene::Logging { .. } = self.game_scene {
                    self.game_scene = GameScene::Logging {
                        ddd_received: true,
                    };
                }
                
                self.add_network_message(NetworkMessage::Received {
                    opcode: "0xF758".to_string(),
                    description: format!("DDD Interrogation (lang={}, region={})", language, region),
                    timestamp: chrono::Utc::now(),
                });
            }
            GameEvent::LoginSucceeded {
                character_id,
                character_name,
            } => {
                self.client_status.logged_in = true;
                self.client_status.current_character = Some(character_name.clone());

                // Transition from LoggingIn to LoggedIn
                if let GameScene::GameWorld { ref mut state, .. } = self.game_scene {
                    *state = GameWorldState::LoggedIn;
                }

                self.add_network_message(NetworkMessage::Received {
                    opcode: "0xF656".to_string(),
                    description: format!("Login succeeded: {} (ID: {})", character_name, character_id),
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
            GameEvent::CreateObject { object_id, object_name } => {
                // Add object to the list if we're in the game world scene
                if let GameScene::GameWorld { ref mut state, ref mut created_objects } = self.game_scene {
                    created_objects.push((object_id, object_name.clone()));

                    // After receiving first object while logging in, send LoginComplete notification
                    if *state == GameWorldState::LoggingIn && created_objects.len() == 1 {
                        if let Some(ref tx) = self.action_tx {
                            let _ = tx.send(ClientAction::SendLoginComplete);
                        }
                    }
                }

                self.add_network_message(NetworkMessage::Received {
                    opcode: "0xF745".to_string(),
                    description: format!("CreateObject: {} (ID: {})", object_name, object_id),
                    timestamp: chrono::Utc::now(),
                });
            }
            GameEvent::ChatMessageReceived { message, message_type } => {
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
            GameEvent::NetworkMessage { direction, message_type } => {
                // Add all network messages to the debug view
                use crate::client::events::MessageDirection;
                match direction {
                    MessageDirection::Received => {
                        self.add_network_message(NetworkMessage::Received {
                            opcode: "".to_string(),
                            description: message_type,
                            timestamp: chrono::Utc::now(),
                        });
                    }
                    MessageDirection::Sent => {
                        self.add_network_message(NetworkMessage::Sent {
                            opcode: "".to_string(),
                            description: message_type,
                            timestamp: chrono::Utc::now(),
                        });
                    }
                }
            }
        }
    }

    pub fn add_chat_message(&mut self, message: ChatMessage) {
        self.chat_messages.push_back(message);
        if self.chat_messages.len() > self.max_chat_messages {
            self.chat_messages.pop_front();
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

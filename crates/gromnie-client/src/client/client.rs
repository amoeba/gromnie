use std::collections::{HashMap, VecDeque};
use std::io::Cursor;
use std::net::SocketAddr;

use acprotocol::enums::{
    AuthFlags, FragmentGroup, GameEvent as GameEventType, PacketHeaderFlags, S2CMessage,
};

use acprotocol::gameactions::CharacterLoginCompleteNotification;
use acprotocol::message::{C2SMessage, GameActionMessage};
use acprotocol::messages::c2s::{
    CharacterSendCharGenResult, LoginSendEnterWorld, LoginSendEnterWorldRequest,
};
use tokio::sync::mpsc;

// Import from our new modules
use crate::client::connection::ServerInfo;
use crate::client::messages::{OutgoingMessage, OutgoingMessageContent};
use crate::client::protocol::{C2SPacketExt, CustomLoginRequest};
use crate::client::scene::{
    CharacterCreateScene, CharacterSelectScene, ClientError,
    ConnectingProgress as SceneConnectingProgress, ConnectingScene, EnteringWorldState, ErrorScene,
    InWorldScene, PatchingProgress as ScenePatchingProgress, Scene,
};
use crate::client::session::{Account, ClientSession, ConnectionState, SessionState};

use acprotocol::network::packet::PacketHeader;
use acprotocol::network::{Fragment, RawMessage};
use acprotocol::packets::c2s_packet::C2SPacket;
use acprotocol::packets::s2c_packet::S2CPacket;
use acprotocol::readers::ACDataType;
use acprotocol::types::{BlobFragments, ConnectRequestHeader};
use acprotocol::writers::{ACWritable, write_string, write_u32};
use tokio::net::UdpSocket;
use tracing::{debug, error, info, warn};

use crate::client::constants::*;
use crate::client::game_event_handler::dispatch_game_event;
use crate::client::message_handler::dispatch_message;
use crate::client::protocol_conversions::{
    hear_direct_speech_to_game_event_msg, transient_string_to_game_event_msg,
};
use crate::client::{ClientEvent, ClientSystemEvent, GameEvent};
use crate::crypto::crypto_system::CryptoSystem;
use crate::crypto::magic_number::get_magic_number;
use acprotocol::gameevents::{CommunicationHearDirectSpeech, CommunicationTransientString};

/// Maximum number of packets we can send without receiving before considering connection dead
const MAX_UNACKED_SENDS: u32 = 20;

// NOTE: ConnectingProgress and PatchingProgress are now defined in scene.rs
// Import them from there instead
use crate::client::scene::{ConnectingProgress, PatchingProgress};

// TODO: Don't require both bind_address and connect_address. I had to do this
// to get things to work but I should be able to listen on any random port so
// I'm not sure what I'm doing wrong
pub struct Client {
    pub id: u32,
    pub server: ServerInfo,
    pub socket: UdpSocket,
    account: Account,

    // ========== Session-Based Architecture ==========
    pub session: ClientSession, // Protocol state + metadata
    pub scene: Scene,           // UI state

    pub send_count: u32,
    pub recv_count: u32,
    last_ack_sent: u32,      // Track the last sequence we ACKed to the server
    unacked_send_count: u32, // Track how many sends we've done without receiving a packet
    fragment_sequence: u32,  // Counter for outgoing fragment sequences
    next_game_action_sequence: u32, // Sequence counter for GameAction messages
    pending_fragments: HashMap<u32, Fragment>, // Track incomplete fragment sequences
    message_queue: VecDeque<RawMessage>, // Queue of parsed messages to process
    pub(crate) outgoing_message_queue: VecDeque<OutgoingMessage>, // Queue of messages to send with optional delays
    pub(crate) raw_event_tx: mpsc::Sender<ClientEvent>,           // Raw event sender to runner
    action_rx: mpsc::UnboundedReceiver<gromnie_events::SimpleClientAction>, // Receive actions from handlers
    pub(crate) ddd_response: Option<OutgoingMessageContent>, // Cached DDD response for retries
    pub(crate) known_characters: Vec<acprotocol::types::CharacterIdentity>, // Track characters from list and creation
    // Reconnection state
    reconnect_config: crate::config::ReconnectConfig,
    pub(crate) reconnect_attempt_count: u32, // Track reconnection attempts across state transitions
    pub(crate) reconnect_at: Option<std::time::Instant>, // When to attempt reconnection (None if not waiting)
    /// Optional character name to auto-login with after receiving character list
    pub(crate) character: Option<String>,
    /// Pending auto-login action to be processed after character list is received
    pub(crate) pending_auto_login: Option<gromnie_events::SimpleClientAction>,
}

impl Client {
    pub async fn new(
        id: u32,
        address: String,
        name: String,
        password: String,
        character: Option<String>,
        raw_event_tx: mpsc::Sender<ClientEvent>, // Raw event sender to runner
        reconnect: bool,
    ) -> (
        Client,
        mpsc::UnboundedSender<gromnie_events::SimpleClientAction>,
    ) {
        Self::new_with_reconnect(
            id,
            address,
            name,
            password,
            character,
            raw_event_tx,
            reconnect,
        )
        .await
    }

    pub async fn new_with_reconnect(
        id: u32,
        address: String,
        name: String,
        password: String,
        character: Option<String>,
        raw_event_tx: mpsc::Sender<ClientEvent>, // Raw event sender to runner
        reconnect_enabled: bool,
    ) -> (
        Client,
        mpsc::UnboundedSender<gromnie_events::SimpleClientAction>,
    ) {
        let sok = UdpSocket::bind("0.0.0.0:0").await.unwrap();

        // Parse address to extract host and port
        let parts: Vec<&str> = address.split(':').collect();
        let host = parts[0].to_string();
        let login_port = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(9000);

        // Action channel: Handlers send actions back to client
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        // Build reconnect config from bool
        let reconnect_config = if reconnect_enabled {
            crate::config::ReconnectConfig {
                enabled: true,
                ..Default::default()
            }
        } else {
            crate::config::ReconnectConfig::default()
        };

        let client = Client {
            id,
            server: ServerInfo::new(host, login_port),
            account: Account { name, password },
            socket: sok,

            // Initialize session-based architecture
            session: ClientSession::new(SessionState::AuthLoginRequest),
            scene: Scene::Connecting(ConnectingScene::new()),

            send_count: 0,
            recv_count: 0,
            last_ack_sent: 0,             // Initialize to 0
            unacked_send_count: 0,        // Initialize to 0
            fragment_sequence: 1,         // Start at 1 as per actestclient
            next_game_action_sequence: 0, // Start at 0 for GameAction sequences
            pending_fragments: HashMap::new(),
            message_queue: VecDeque::new(),
            outgoing_message_queue: VecDeque::new(),
            raw_event_tx, // Raw event sender to runner
            action_rx,
            ddd_response: None,
            known_characters: Vec::new(),
            reconnect_config,
            reconnect_attempt_count: 0,
            reconnect_at: None,
            character,
            pending_auto_login: None,
        };

        (client, action_tx)
    }

    /// Centralized packet sending with sequence management
    /// Matches actestclient's Send() method logic:
    /// - incrementSequence: increment send_count BEFORE using it
    /// - includeSequence: use send_count in packet header, otherwise use 0
    /// - Id/Table are only set if ClientId > 0 (actestclient line 292-297)
    /// - Automatically includes ACKs when we have received packets that need acknowledging
    async fn send_packet(
        &mut self,
        mut packet: C2SPacket,
        include_sequence: bool,
        increment_sequence: bool,
    ) -> Result<(), std::io::Error> {
        // Increment sequence FIRST if requested (matches actestclient behavior)
        if increment_sequence {
            self.send_count += 1;
        }

        // Set sequence based on include_sequence flag
        packet.sequence = if include_sequence { self.send_count } else { 0 };

        // Track unacked sends - increment each time we send
        self.unacked_send_count += 1;

        // Check if we've sent too many packets without receiving a response
        if self.unacked_send_count >= MAX_UNACKED_SENDS {
            error!(
                target: "net",
                "Sent {} packets without receiving from server - connection appears dead",
                self.unacked_send_count
            );
            self.enter_disconnected();
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "Server not responding",
            ));
        }

        // CRITICAL: Automatically include ACK if we have received packets that need acknowledging
        // This matches actestclient behavior (NetworkManager.Send lines 266-270)
        // The server uses ACKs to determine if the client is still alive!
        if self.recv_count > self.last_ack_sent {
            let ack_seq = self.recv_count;
            packet = packet.with_ack_sequence(ack_seq); // Safely sets both field and flag
            self.last_ack_sent = ack_seq;
            debug!(target: "net", "📤 Sending ACK for server seq={} in outgoing packet (send_count={})",
                ack_seq, self.send_count);
        } else if self.recv_count > 0 {
            debug!(target: "net", "📤 No new ACK needed (recv_count={}, last_ack_sent={})",
                self.recv_count, self.last_ack_sent);
        }

        // CRITICAL: Only set recipient_id and iteration if ClientId > 0
        // (matches actestclient NetworkManager.Send lines 292-297)
        // When ClientId is 0, these should remain at their default values from packet construction
        // This affects checksum calculation!

        // Serialize with checksum (pass session for encryption key if fragmented)
        let buffer = packet.serialize(self.session.connection.as_ref())?;

        // Determine destination address
        let dest_addr = if packet.flags.contains(PacketHeaderFlags::CONNECT_RESPONSE) {
            // ConnectResponse goes to world port (9001)
            self.server.world_addr().await?
        } else {
            // Everything else goes to login port (9000)
            self.server.login_addr().await?
        };

        debug!(target: "net", "Sending packet: seq={}, id={}, flags={:?}, dest={}",
            packet.sequence, packet.recipient_id, packet.flags, dest_addr);

        self.socket.send_to(&buffer, dest_addr).await?;
        Ok(())
    }

    /// Check if there are messages waiting to be processed
    pub fn has_messages(&self) -> bool {
        !self.message_queue.is_empty()
    }

    /// Process all queued messages
    pub fn process_messages(&mut self) {
        while let Some(message) = self.message_queue.pop_front() {
            self.handle_message(message);
        }
    }

    /// Check if there are pending outgoing messages to send
    pub fn has_pending_outgoing_messages(&self) -> bool {
        // Check if there are any messages ready to be sent
        self.outgoing_message_queue.iter().any(|msg| msg.is_ready())
    }

    /// Send all pending outgoing messages that are ready
    pub async fn send_pending_messages(&mut self) -> Result<(), std::io::Error> {
        // Create a temporary queue to hold messages that aren't ready yet
        let mut remaining_messages = VecDeque::new();

        while let Some(msg) = self.outgoing_message_queue.pop_front() {
            if msg.is_ready() {
                // Message is ready to be sent
                let msg_discriminant = std::mem::discriminant(&msg.content);
                info!(target: "net", "send_pending_messages: sending message: {:?}", msg_discriminant);

                // Send the message content
                self.send_outgoing_message(msg.content).await?;
            } else {
                // Message is not ready yet, keep for later
                remaining_messages.push_back(msg);
            }
        }

        // Put back the messages that aren't ready yet
        self.outgoing_message_queue = remaining_messages;

        Ok(())
    }

    /// Send keep-alive packet (TimeSync) to maintain connection
    /// Note: ACKs should be piggybacked on outgoing packets, not sent standalone
    pub async fn send_keepalive(&mut self) -> Result<(), std::io::Error> {
        if self.session.connection.is_some() {
            debug!(target: "net", "Sending TimeSync keep-alive");
            self.send_timesync().await?;
        }
        Ok(())
    }

    /// Attempt to log in as the specified character
    /// Returns Ok if login was queued, or an error if already attempted or in wrong state
    pub fn attempt_character_login(
        &mut self,
        character_id: u32,
        character_name: String,
        account: String,
    ) -> Result<(), String> {
        // Check that we're in CharacterSelect scene
        if self.scene.as_character_select().is_none() {
            return Err(format!(
                "Cannot login: not in CharacterSelect scene (current scene: {:?})",
                self.scene
            ));
        }

        // Check if we've already attempted login
        if let Some(char_select) = self.scene.as_character_select()
            && char_select.is_entering_world()
        {
            return Err("Login already in progress".to_string());
        }

        // Step 1: Send CharacterEnterWorldRequest (0xF7C8)
        // This tells the server we want to enter the world
        self.outgoing_message_queue.push_back(OutgoingMessage::new(
            OutgoingMessageContent::EnterWorldRequest,
        ));

        // Update scene to begin entering world
        if let Some(char_select) = self.scene.as_character_select_mut() {
            char_select.begin_entering_world(character_id, character_name.clone(), account.clone());
        }

        info!(target: "net", "Sent CharacterEnterWorldRequest for character: {} (ID: {})", character_name, character_id);
        Ok(())
    }

    /// Send LoginComplete notification to server after receiving initial world state
    pub fn send_login_complete_notification(&mut self) {
        // Check if we're in CharacterSelect with entering_world state
        let entering = if let Some(char_select) = self.scene.as_character_select() {
            char_select.entering_world.clone()
        } else {
            None
        };

        let Some(entering) = entering else {
            warn!(target: "net", "send_login_complete_notification: not in CharacterSelect or no entering_world state");
            return;
        };

        // Only send once - check if we're already succeeded
        if entering.login_complete {
            info!(target: "net", "LoginComplete notification already sent");
            return;
        }

        info!(target: "net", "Sending LoginComplete notification to server");

        // Create OrderedGameAction with CharacterLoginCompleteNotification
        let mut message_data = Vec::new();
        {
            let mut cursor = Cursor::new(&mut message_data);
            let action = GameActionMessage::CharacterLoginCompleteNotification(
                CharacterLoginCompleteNotification {},
            );
            let msg = C2SMessage::OrderedGameAction {
                sequence: self.next_game_action_sequence,
                action,
            };
            self.next_game_action_sequence += 1;
            msg.write(&mut cursor).expect("write failed");
        }

        // Queue for sending
        self.outgoing_message_queue.push_back(OutgoingMessage::new(
            OutgoingMessageContent::GameAction(message_data),
        ));

        let character_id = entering.character_id;
        let character_name = entering.character_name.clone();

        // Emit LoginSucceeded event to update UI
        let game_event = GameEvent::LoginSucceeded {
            character_id,
            character_name: character_name.clone(),
        };

        let _ = self.raw_event_tx.try_send(ClientEvent::Game(game_event));

        // Mark login as complete in scene
        if let Some(char_select) = self.scene.as_character_select_mut() {
            char_select.mark_login_complete();
        }

        // Transition to InWorld scene now that login is complete
        self.scene = Scene::InWorld(InWorldScene::new(character_id, character_name));
        info!(target: "net", "Scene transition: CharacterSelect -> InWorld");

        info!(target: "net", "LoginComplete notification queued and event emitted");
    }

    /// Send a chat message to the server
    /// This sends a general chat message that will appear as a /say command
    /// TODO: Parse message for @tell, /say, /emote, etc. commands
    fn send_chat_say(&mut self, message: String) {
        info!(target: "net", "Sending chat say: {}", message);

        // Create OrderedGameAction with CommunicationTalk (for general chat)
        // This is the correct message type for general chat (equivalent to /say command)
        let mut message_data = Vec::new();
        {
            let mut cursor = Cursor::new(&mut message_data);
            use acprotocol::gameactions::CommunicationTalk;
            let action = GameActionMessage::CommunicationTalk(CommunicationTalk {
                message: message.clone(),
            });
            let msg = C2SMessage::OrderedGameAction {
                sequence: self.next_game_action_sequence,
                action,
            };
            self.next_game_action_sequence += 1;
            msg.write(&mut cursor).expect("write failed");
        }

        // Queue for sending
        self.outgoing_message_queue.push_back(OutgoingMessage::new(
            OutgoingMessageContent::GameAction(message_data),
        ));
        info!(target: "net", "Chat say message queued for sending");
    }

    fn send_chat_tell(&mut self, recipient_name: String, message: String) {
        info!(target: "net", "Sending tell to '{}': {}", recipient_name, message);

        // Create OrderedGameAction with CommunicationTalkDirectByName (for direct messages)
        let mut message_data = Vec::new();
        {
            let mut cursor = Cursor::new(&mut message_data);
            use acprotocol::gameactions::CommunicationTalkDirectByName;
            let action =
                GameActionMessage::CommunicationTalkDirectByName(CommunicationTalkDirectByName {
                    message: message.clone(),
                    target_name: recipient_name.clone(),
                });
            let msg = C2SMessage::OrderedGameAction {
                sequence: self.next_game_action_sequence,
                action,
            };
            self.next_game_action_sequence += 1;
            msg.write(&mut cursor).expect("write failed");
        }

        // Queue for sending
        self.outgoing_message_queue.push_back(OutgoingMessage::new(
            OutgoingMessageContent::GameAction(message_data),
        ));
        info!(target: "net", "Chat tell message queued for sending");
    }

    /// Send a TimeSync packet to keep connection alive
    /// Uses includeSequence=false, incrementSequence=false (sequence will be 0)
    async fn send_timesync(&mut self) -> Result<(), std::io::Error> {
        let (client_id, table) = {
            let connection = self
                .session
                .connection
                .as_ref()
                .ok_or_else(|| std::io::Error::other("Session not established"))?;
            (connection.client_id, connection.table)
        };

        // Get current time as Unix timestamp (seconds since epoch)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // CRITICAL: Only set recipient_id and iteration if client_id > 0 (matches actestclient line 292-297)
        // When client_id is 0, these should be 0 (default values)
        let (recipient_id, iteration) = if client_id > 0 {
            (client_id, table)
        } else {
            (0, 0)
        };

        let packet = C2SPacket {
            sequence: 0, // Will be set by send_packet
            flags: PacketHeaderFlags::TIME_SYNC,
            checksum: 0,
            recipient_id,
            time_since_last_packet: 0,
            size: 8, // TimeSync payload is a u64
            iteration,
            server_switch: None,
            retransmit_sequences: None,
            reject_sequences: None,
            ack_sequence: None,
            login_request: None,
            world_login_request: None,
            connect_response: None,
            cicmd_command: None,
            time: Some(current_time),
            echo_time: None,
            flow: None,
            fragments: None,
        };

        // TimeSync: includeSequence=false, incrementSequence=false (like actestclient line 678)
        self.send_packet(packet, false, false).await
    }

    /// Get the client ID
    pub fn client_id(&self) -> u32 {
        self.id
    }

    /// Check if current state has timed out (20s timeout for Connecting and Patching)
    pub fn check_state_timeout(&mut self) -> bool {
        const TIMEOUT_DURATION: std::time::Duration = std::time::Duration::from_secs(20);

        if let Some(connecting) = self.scene.as_connecting()
            && connecting.has_timed_out(TIMEOUT_DURATION)
        {
            // Determine if we're in Connecting or Patching phase
            let phase = if matches!(connecting.patch_progress, PatchingProgress::NotStarted) {
                "LoginRequest"
            } else {
                "Patching"
            };

            info!(target: "net", "{} timeout - no response after 20s (patch_progress: {:?})", phase, connecting.patch_progress);

            // Reconnection behavior on timeout:
            // - Initial connection attempts (reconnect_attempt_count == 0) always fail permanently
            //   to provide fast feedback when the server is genuinely unavailable
            // - Subsequent reconnection attempts (reconnect_attempt_count > 0) re-enter Disconnected
            //   state to retry with exponential backoff
            if self.reconnect_config.enabled && self.reconnect_attempt_count > 0 {
                info!(target: "net", "Reconnection attempt timed out - re-entering Disconnected state to retry");
                self.enter_disconnected();
            } else {
                // Initial connection attempt timeout or reconnection disabled - fail permanently
                let error = if matches!(connecting.patch_progress, PatchingProgress::NotStarted) {
                    ClientError::LoginTimeout
                } else {
                    ClientError::PatchingTimeout
                };

                self.scene = Scene::Error(ErrorScene::new(error, false));

                // Emit authentication failed system event
                let _ = self.raw_event_tx.try_send(ClientEvent::System(
                    ClientSystemEvent::AuthenticationFailed {
                        reason: "Connection timeout - server not responding".to_string(),
                    },
                ));
                return true;
            }
        }
        false
    }

    /// Check if it's time to retry in current state (2s retry interval)
    pub fn should_retry(&self) -> bool {
        const RETRY_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

        if let Some(connecting) = self.scene.as_connecting() {
            connecting.should_retry(RETRY_INTERVAL)
        } else {
            false
        }
    }

    /// Update last retry time for current state
    pub fn update_retry_time(&mut self) {
        if let Some(connecting) = self.scene.as_connecting_mut() {
            connecting.update_retry_time();
        }
    }

    /// Check if client should attempt to reconnect
    pub fn should_reconnect(&mut self) -> bool {
        if let Some(reconnect_at) = self.reconnect_at {
            // Check if we've reached the reconnect time
            return std::time::Instant::now() >= reconnect_at;
        }
        false
    }

    /// Initiate reconnection attempt
    /// Returns true if reconnection is being attempted, false if disabled or max retries reached
    /// Transitions the client to Connecting state with an appropriate backoff delay
    pub fn start_reconnection(&mut self) -> bool {
        if !self.reconnect_config.enabled {
            info!(target: "net", "Reconnection disabled, entering Error state");
            self.scene = Scene::Error(ErrorScene::new(
                ClientError::ConnectionFailed(
                    "Connection lost and reconnection disabled".to_string(),
                ),
                false,
            ));
            return false;
        }

        // Check if we've exceeded max retry attempts
        if !self
            .reconnect_config
            .should_attempt_reconnect(self.reconnect_attempt_count)
        {
            error!(
                target: "net",
                "Max reconnection attempts reached ({}), entering Error state",
                self.reconnect_config.max_attempts
            );
            self.scene = Scene::Error(ErrorScene::new(
                ClientError::ConnectionFailed(format!(
                    "Max reconnection attempts ({}) reached",
                    self.reconnect_config.max_attempts
                )),
                false,
            ));
            return false;
        }

        let delay = self
            .reconnect_config
            .delay_for_attempt(self.reconnect_attempt_count);

        info!(
            target: "net",
            "Starting reconnection attempt {} (waiting {:?} before reconnecting)",
            self.reconnect_attempt_count, delay
        );

        // Emit reconnection event
        let delay_secs: u64 = delay.as_secs();
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::System(ClientSystemEvent::Reconnecting {
                attempt: self.reconnect_attempt_count,
                delay_secs,
            }));

        // Transition back to Connecting scene for reconnection attempt
        self.scene = Scene::Connecting(ConnectingScene::new());

        // Clear reconnect_at since we're now reconnecting
        self.reconnect_at = None;

        // Update timing to account for backoff delay
        if let Some(connecting) = self.scene.as_connecting_mut() {
            connecting.last_retry_at = std::time::Instant::now() + delay; // Don't retry until after the backoff delay
        }

        true
    }

    /// Enter disconnected state and prepare for potential reconnection
    pub fn enter_disconnected(&mut self) {
        // Increment attempt counter (stored separately from state to survive transitions)
        self.reconnect_attempt_count += 1;

        let delay = self
            .reconnect_config
            .delay_for_attempt(self.reconnect_attempt_count);

        info!(
            target: "net",
            "Connection lost - waiting for reconnection (attempt {}, next retry in {:?})",
            self.reconnect_attempt_count, delay
        );

        // Set reconnect_at for later checking
        self.reconnect_at = Some(std::time::Instant::now() + delay);

        // Clear session state
        self.session.connection = None;
        self.pending_fragments.clear();

        // Reset sequence counters for fresh connection
        self.send_count = 0;
        self.recv_count = 0;
        self.last_ack_sent = 0;
        self.unacked_send_count = 0;
        self.fragment_sequence = 1;
        self.next_game_action_sequence = 0;

        // Emit disconnected event
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::System(ClientSystemEvent::Disconnected {
                will_reconnect: self.reconnect_config.enabled,
                reconnect_attempt: self.reconnect_attempt_count,
                delay_secs: delay.as_secs(),
            }));
    }

    /// Get cached DDD response for retries
    pub fn get_ddd_response(&self) -> Option<&OutgoingMessageContent> {
        self.ddd_response.as_ref()
    }

    /// Retry sending DDD response (used in Patching state)
    pub async fn retry_ddd_response(&mut self) -> Result<(), std::io::Error> {
        if let Some(ref ddd_response) = self.ddd_response {
            self.outgoing_message_queue
                .push_back(OutgoingMessage::new(ddd_response.clone()));
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "No DDD response cached",
            ))
        }
    }

    /// Process actions sent from event handlers
    pub fn process_actions(&mut self) {
        // Process pending auto-login action first (if any)
        if let Some(action) = self.pending_auto_login.take() {
            match action {
                gromnie_events::SimpleClientAction::LoginCharacter {
                    character_id,
                    character_name,
                    account,
                } => {
                    info!(target: "events", "Auto-login: Logging in as character {} (ID: {})", character_name, character_id);
                    if let Err(e) =
                        self.attempt_character_login(character_id, character_name, account)
                    {
                        error!(target: "events", "Failed to attempt auto-login: {}", e);
                    }
                }
                _ => {
                    // Put it back if it's not a LoginCharacter action (shouldn't happen)
                    self.pending_auto_login = Some(action);
                }
            }
        }

        // Process all pending actions without blocking
        while let Ok(action) = self.action_rx.try_recv() {
            match action {
                gromnie_events::SimpleClientAction::Disconnect => {
                    info!(target: "events", "Action: Disconnecting");
                    // Disconnect action - transition to Error state
                    self.scene = Scene::Error(ErrorScene::new(
                        ClientError::ConnectionFailed("Disconnected by client action".to_string()),
                        true, // Can retry
                    ));
                }
                gromnie_events::SimpleClientAction::LoginCharacter {
                    character_id,
                    character_name,
                    account,
                } => {
                    debug!(target: "events", "Action: Logging in as character {} (ID: {})", character_name, character_id);
                    if let Err(e) =
                        self.attempt_character_login(character_id, character_name, account)
                    {
                        error!(target: "events", "Failed to attempt character login: {}", e);
                    }
                }
                gromnie_events::SimpleClientAction::SendLoginComplete => {
                    debug!(target: "events", "Action: Sending LoginComplete notification to server");
                    self.send_login_complete_notification();
                }
                gromnie_events::SimpleClientAction::SendChatSay { message } => {
                    debug!(target: "events", "Action: Sending chat say: {}", message);
                    self.send_chat_say(message);
                }
                gromnie_events::SimpleClientAction::SendChatTell {
                    recipient_name,
                    message,
                } => {
                    debug!(target: "events", "Action: Sending tell to {}: {}", recipient_name, message);
                    self.send_chat_tell(recipient_name, message);
                }
                gromnie_events::SimpleClientAction::ReloadScripts { script_dir } => {
                    debug!(target: "events", "Action: Reloading scripts from {:?}", script_dir);
                    // Note: This action is handled by ScriptRunner, not here
                    // The client just forwards it via the event channel
                    // We shouldn't see this here, but handle it gracefully
                    warn!(target: "events", "ReloadScripts action received in Client - this should be handled by ScriptRunner");
                }
                gromnie_events::SimpleClientAction::LogScriptMessage { script_id, message } => {
                    info!(target: "script", "[{}] {}", script_id, message);
                }
            }
        }
    }

    /// Check and send delayed messages based on login time
    /// Send a single outgoing message
    async fn send_outgoing_message(
        &mut self,
        message: OutgoingMessageContent,
    ) -> Result<(), std::io::Error> {
        match message {
            OutgoingMessageContent::CharacterCreation(char_gen) => {
                self.send_character_creation_internal(char_gen).await
            }
            OutgoingMessageContent::CharacterCreationAce(account, char_gen) => {
                self.send_character_creation_ace_internal(account, char_gen)
                    .await
            }
            OutgoingMessageContent::EnterWorldRequest => {
                self.send_enter_world_request_internal().await
            }
            OutgoingMessageContent::EnterWorld(enter_world) => {
                self.send_enter_world_internal(enter_world).await
            }
            OutgoingMessageContent::GameAction(message_data) => {
                self.send_fragmented_message(message_data, FragmentGroup::Object)
                    .await
            }
        }
    }

    /// Send a message wrapped in a BlobFragment
    async fn send_fragmented_message(
        &mut self,
        message_data: Vec<u8>,
        group: FragmentGroup,
    ) -> Result<(), std::io::Error> {
        info!(target: "net", "send_fragmented_message: message_data len={}, group={:?}", message_data.len(), group);

        // Get current fragment sequence and increment
        let frag_sequence = self.fragment_sequence;
        self.fragment_sequence += 1;

        // Create BlobFragment structure
        let fragment_size = (FRAGMENT_HEADER_SIZE + message_data.len()) as u16;
        let blob_fragment = BlobFragments {
            sequence: frag_sequence,
            id: 0x80000000, // Object ID (0x80000000 for game messages)
            count: 1,       // Single fragment
            size: fragment_size,
            index: 0, // First (and only) fragment
            group,
            data: message_data,
        };

        // Extract session values
        let (client_id, table) = {
            let connection = self
                .session
                .connection
                .as_ref()
                .expect("Session not established");
            (connection.client_id, connection.table)
        };

        // Increment send_count first, then use it (matches actestclient behavior)
        self.send_count += 1;
        let packet_sequence = self.send_count;

        // CRITICAL: Automatically include ACK if we have received packets that need acknowledging
        // This matches actestclient behavior and keeps the connection alive!
        let should_ack = self.recv_count > self.last_ack_sent;
        let ack_seq = if should_ack {
            let seq = self.recv_count;
            self.last_ack_sent = seq;
            debug!(target: "net", "Including ACK for sequence {} in fragmented message", seq);
            Some(seq)
        } else {
            None
        };

        // Create C2SPacket with BlobFragments flag
        let mut packet = C2SPacket {
            sequence: packet_sequence,
            flags: PacketHeaderFlags::BLOB_FRAGMENTS,
            checksum: 0, // Will be calculated
            recipient_id: client_id,
            time_since_last_packet: 0,
            size: 0, // Will be calculated during serialization
            iteration: table,
            server_switch: None,
            retransmit_sequences: None,
            reject_sequences: None,
            ack_sequence: None, // Will be set via with_ack_sequence if needed
            login_request: None,
            world_login_request: None,
            connect_response: None,
            cicmd_command: None,
            time: None,
            echo_time: None,
            flow: None,
            fragments: Some(blob_fragment),
        };

        // Safely add ACK if needed (sets both field and flag together)
        if let Some(seq) = ack_seq {
            packet = packet.with_ack_sequence(seq);
        }

        // Serialize packet to get size
        let mut buffer = Vec::new();
        {
            let mut cursor = Cursor::new(&mut buffer);
            packet
                .write(&mut cursor)
                .map_err(|e| std::io::Error::other(format!("Write error: {}", e)))?;
        }

        // Calculate payload size (everything after the header)
        let payload_size = buffer.len() - PACKET_HEADER_SIZE;

        // Update the size field in the header
        buffer[16..18].copy_from_slice(&(payload_size as u16).to_le_bytes());

        // Calculate checksum: header + fragment
        buffer[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4]
            .copy_from_slice(&CHECKSUM_PLACEHOLDER.to_le_bytes());

        // Checksum calculation:
        // 1. Header checksum (with placeholder)
        let header_checksum =
            get_magic_number(&buffer[0..PACKET_HEADER_SIZE], PACKET_HEADER_SIZE, true);

        // 2. Fragment checksum = fragment_header_checksum + fragment_data_checksum
        let fragment_header_checksum = get_magic_number(
            &buffer[PACKET_HEADER_SIZE..PACKET_HEADER_SIZE + FRAGMENT_HEADER_SIZE],
            FRAGMENT_HEADER_SIZE,
            true,
        );
        let fragment_data_size = buffer.len() - PACKET_HEADER_SIZE - FRAGMENT_HEADER_SIZE;
        let fragment_data_checksum = get_magic_number(
            &buffer[PACKET_HEADER_SIZE + FRAGMENT_HEADER_SIZE..],
            fragment_data_size,
            true,
        );
        let fragment_checksum = fragment_header_checksum.wrapping_add(fragment_data_checksum);

        // 3. Total checksum
        let total_checksum = header_checksum.wrapping_add(fragment_checksum);
        buffer[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4].copy_from_slice(&total_checksum.to_le_bytes());

        debug!(target: "net", "Sending fragmented message: seq={}, frag_seq={}, size={}, checksum=0x{:08X}",
            packet_sequence, frag_sequence, buffer.len(), total_checksum);

        // After connection, all packets with Id > 0 go to the login server port (9000)
        let login_addr = self.server.login_addr().await?;
        debug!(target: "net", "Sending fragmented message to login server at {}", login_addr);
        self.socket.send_to(&buffer, login_addr).await?;
        Ok(())
    }

    /// Send character creation request to the login server
    async fn send_character_creation_internal(
        &mut self,
        char_gen: CharacterSendCharGenResult,
    ) -> Result<(), std::io::Error> {
        info!(target: "net", "Sending Character Creation - Name: {}", char_gen.result.name);

        // Serialize the message using acprotocol's C2SMessage wrapper (handles opcode automatically)
        let mut message_data = Vec::new();
        {
            let mut cursor = Cursor::new(&mut message_data);
            C2SMessage::CharacterSendCharGenResult(char_gen)
                .write(&mut cursor)
                .map_err(|e| std::io::Error::other(format!("Write error: {}", e)))?;
        }

        // Send as a proper fragmented packet
        self.send_fragmented_message(message_data, FragmentGroup::Object)
            .await
    }

    /// Send character creation request with ACE-compatible serialization
    async fn send_character_creation_ace_internal(
        &mut self,
        account: String,
        char_gen: crate::client::ace_protocol::AceCharGenResult,
    ) -> Result<(), std::io::Error> {
        info!(target: "net", "Sending Character Creation (ACE format) - Name: {}", char_gen.name);

        // Serialize the character creation message with opcode prefix
        let mut message_data = Vec::new();
        {
            let mut cursor = Cursor::new(&mut message_data);
            // Write opcode first (0xF656 = Character_SendCharGenResult)
            write_u32(&mut cursor, 0xF656)
                .map_err(|e| std::io::Error::other(format!("Write error: {}", e)))?;
            // Write account string (outer wrapper field)
            write_string(&mut cursor, &account)
                .map_err(|e| std::io::Error::other(format!("Write error: {}", e)))?;
            // Write the ACE-formatted CharGenResult
            char_gen
                .write(&mut cursor)
                .map_err(|e| std::io::Error::other(format!("Write error: {}", e)))?;
        }

        // Send as a proper fragmented packet
        self.send_fragmented_message(message_data, FragmentGroup::Object)
            .await
    }

    /// Send character enter world request (0xF7C8) - Step 1 of character login
    /// Server will respond with CharacterEnterWorldServerReady (0xF7DF)
    async fn send_enter_world_request_internal(&mut self) -> Result<(), std::io::Error> {
        info!(target: "net", "Sending Login_SendEnterWorldRequest (0xF7C8)");

        // Serialize the message using acprotocol's C2SMessage wrapper (handles opcode automatically)
        let mut message_data = Vec::new();
        {
            let mut cursor = Cursor::new(&mut message_data);
            C2SMessage::LoginSendEnterWorldRequest(LoginSendEnterWorldRequest {})
                .write(&mut cursor)
                .map_err(|e| std::io::Error::other(format!("Write error: {}", e)))?;
        }

        // Send as a proper fragmented packet
        self.send_fragmented_message(message_data, FragmentGroup::Object)
            .await
    }

    /// Send character login (enter world) with character ID - Step 3 of character login
    /// This is sent after receiving CharacterEnterWorldServerReady (0xF7DF)
    async fn send_enter_world_internal(
        &mut self,
        enter_world: LoginSendEnterWorld,
    ) -> Result<(), std::io::Error> {
        info!(target: "net", "Sending Login_SendEnterWorld (0xF657) - Character ID: {}", enter_world.character_id.0);

        // Serialize the message using acprotocol's C2SMessage wrapper (handles opcode automatically)
        let mut message_data = Vec::new();
        {
            let mut cursor = Cursor::new(&mut message_data);
            C2SMessage::LoginSendEnterWorld(enter_world)
                .write(&mut cursor)
                .map_err(|e| std::io::Error::other(format!("Write error: {}", e)))?;
        }

        // Send as a proper fragmented packet
        self.send_fragmented_message(message_data, FragmentGroup::Object)
            .await
    }

    /// Handle a single parsed message
    fn handle_message(&mut self, message: RawMessage) {
        debug!(target: "net", "Received message: {} (0x{:08X})", message.message_type, message.opcode);

        let event_tx = self.raw_event_tx.clone();

        // Otherwise try to parse as S2CMessage
        match S2CMessage::try_from(message.opcode) {
            Ok(msg_type) => {
                info!(target: "net", "Client got S2CMessage: {:?} (0x{:04X})", msg_type, message.opcode);

                // TODO: NetworkMessage event removed in simplified event model
                // Could be added back to SimpleGameEvent if debug visibility is needed

                match msg_type {
                    S2CMessage::OrderedGameEvent => {
                        // Parse nested event type here instead of deferring
                        if message.data.len() >= 12 {
                            let mut cursor = Cursor::new(&message.data[8..]); // Skip outer opcode + seq
                            match u32::read(&mut cursor) {
                                Ok(event_opcode) => match GameEventType::try_from(event_opcode) {
                                    Ok(event_type) => self.handle_game_event(event_type, message),
                                    Err(_) => {
                                        debug!(target: "net", "Unknown game event opcode: 0x{:04X}", event_opcode)
                                    }
                                },
                                Err(e) => {
                                    error!(target: "net", "Failed to read game event opcode: {}", e)
                                }
                            }
                        } else {
                            error!(target: "net", "Game event message too short");
                        }
                    }
                    S2CMessage::LoginCreatePlayer => {
                        dispatch_message::<acprotocol::messages::s2c::LoginCreatePlayer, _>(
                            self, message, &event_tx,
                        )
                        .ok();
                    }
                    S2CMessage::LoginLoginCharacterSet => {
                        dispatch_message::<acprotocol::messages::s2c::LoginLoginCharacterSet, _>(
                            self, message, &event_tx,
                        )
                        .ok();
                    }
                    S2CMessage::DDDInterrogationMessage => {
                        dispatch_message::<acprotocol::messages::s2c::DDDInterrogationMessage, _>(
                            self, message, &event_tx,
                        )
                        .ok();
                    }
                    S2CMessage::CharacterCharGenVerificationResponse => {
                        dispatch_message::<
                            acprotocol::messages::s2c::CharacterCharGenVerificationResponse,
                            _,
                        >(self, message, &event_tx)
                        .ok();
                    }
                    S2CMessage::LoginEnterGameServerReady => {
                        self.handle_enter_game_server_ready(message)
                    }
                    S2CMessage::ItemCreateObject => {
                        dispatch_message::<acprotocol::messages::s2c::ItemCreateObject, _>(
                            self, message, &event_tx,
                        )
                        .ok();
                    }
                    S2CMessage::CommunicationTextboxString => self.handle_chat_message(message),
                    S2CMessage::CommunicationHearSpeech => {
                        dispatch_message::<acprotocol::messages::s2c::CommunicationHearSpeech, _>(
                            self, message, &event_tx,
                        )
                        .ok();
                    }
                    S2CMessage::CommunicationHearRangedSpeech => {
                        dispatch_message::<
                            acprotocol::messages::s2c::CommunicationHearRangedSpeech,
                            _,
                        >(self, message, &event_tx)
                        .ok();
                    }
                    S2CMessage::CharacterCharacterError => {
                        dispatch_message::<acprotocol::messages::s2c::CharacterCharacterError, _>(
                            self, message, &event_tx,
                        )
                        .ok();
                    }
                    // Add more handlers as needed
                    _ => {
                        info!(target: "net", "Unhandled S2CMessage: {:?} (0x{:04X})", msg_type, message.opcode);
                    }
                }
            }
            Err(_) => {
                info!(target: "net", "Unknown message opcode: 0x{:08X}. This indicates something unexpected is happening.", message.opcode);
            }
        }
    }

    /// Handle OrderedGameEvent (0xF7B0) messages
    fn handle_game_event(&mut self, event_type: GameEventType, message: RawMessage) {
        info!(target: "net", "Processing OrderedGameEvent message, data len={}", message.data.len());
        debug!(target: "net", "Game event type: {:?}", event_type);

        // Validate minimum packet length
        if message.data.len() < 8 {
            warn!(target: "net",
                  "OrderedGameEvent message too short: {} bytes (expected at least 8)",
                  message.data.len());
            return;
        }

        // Parse object_id and sequence from message header using acprotocol parsing
        // OrderedGameEvent format: object_id (4 bytes) + sequence (4 bytes) + event_data
        let mut cursor = Cursor::new(&message.data);

        let object_id = match u32::read(&mut cursor) {
            Ok(id) => id,
            Err(e) => {
                error!(target: "net", "Failed to read object_id from OrderedGameEvent: {}", e);
                return;
            }
        };

        let sequence = match u32::read(&mut cursor) {
            Ok(seq) => seq,
            Err(e) => {
                error!(target: "net", "Failed to read sequence from OrderedGameEvent: {}", e);
                return;
            }
        };

        let event_tx = self.raw_event_tx.clone();
        match event_type {
            GameEventType::CommunicationHearDirectSpeech => {
                dispatch_game_event::<CommunicationHearDirectSpeech, _, _>(
                    self,
                    &mut cursor,
                    &event_tx,
                    object_id,
                    sequence,
                    hear_direct_speech_to_game_event_msg,
                )
                .ok();
            }
            GameEventType::CommunicationTransientString => {
                dispatch_game_event::<CommunicationTransientString, _, _>(
                    self,
                    &mut cursor,
                    &event_tx,
                    object_id,
                    sequence,
                    transient_string_to_game_event_msg,
                )
                .ok();
            }
            _ => {
                debug!(target: "net", "Unhandled GameEvent: {:?}", event_type);
            }
        }
    }

    /// Handle LoginEnterGameServerReady (0xF7DF) - Step 2 of character login
    /// Server is ready to receive the character ID, so we send EnterWorld (0xF657)
    fn handle_enter_game_server_ready(&mut self, _message: RawMessage) {
        info!(target: "net", "Received LoginEnterGameServerReady (0xF7DF) - Server ready for character login");

        // Check if we're in the right state and extract the values we need
        if let Some(entering) = self
            .scene
            .as_character_select_mut()
            .and_then(|scene| scene.entering_world.as_ref().cloned())
        {
            let char_id = entering.character_id;
            let char_name = entering.character_name;
            let acc = entering.account;

            // Create the enter world message with character ID
            let enter_world = LoginSendEnterWorld {
                character_id: acprotocol::types::ObjectId(char_id),
                account: acc,
            };

            // Queue the message for sending
            self.outgoing_message_queue.push_back(OutgoingMessage::new(
                OutgoingMessageContent::EnterWorld(enter_world),
            ));

            info!(target: "net", "Queued EnterWorld (0xF657) for character: {} (ID: {})", char_name, char_id);
        } else {
            warn!(target: "net", "Received ServerReady but not in CharacterSelect with entering_world! Current scene: {:?}", self.scene);
        }
    }

    /// Handle chat message (Communication_TextboxString) from the server
    fn handle_chat_message(&mut self, message: RawMessage) {
        debug!(target: "net", "Processing chat message");

        // Parse the incoming message
        // Note: message.data includes the opcode as the first 4 bytes, skip it
        let mut cursor = Cursor::new(&message.data[4..]);

        // Communication_TextboxString format:
        // - String message (AC string format: i16 length + data + padding)
        // - i32 message_type (ChatMessageType enum)

        use acprotocol::readers::ACDataType;
        match String::read(&mut cursor) {
            Ok(chat_text) => {
                // Read the message type
                match u32::read(&mut cursor) {
                    Ok(message_type) => {
                        info!(target: "net", "Chat message received - Opcode: 0x{:04X}, Type: {}, Text: {}",
                              message.opcode, message_type, chat_text);

                        let game_event = GameEvent::ChatMessageReceived {
                            message: chat_text,
                            message_type,
                        };

                        // Send on channel (ignore error if no subscribers)
                        let _ = self.raw_event_tx.try_send(ClientEvent::Game(game_event));
                    }
                    Err(e) => {
                        error!(target: "net", "Failed to parse chat message type: {}", e);
                    }
                }
            }
            Err(e) => {
                error!(target: "net", "Failed to parse chat message text: {}", e);
            }
        }
    }

    /// Handle a fragment received from the server
    fn handle_fragment(&mut self, blob_fragment: acprotocol::types::BlobFragments) {
        let sequence = blob_fragment.sequence;
        let index = blob_fragment.index as usize;
        let count = blob_fragment.count;

        // Calculate the actual fragment data size (size includes the fragment header overhead)
        // In C#: fragLength = fragHeader.Size - FragmentHeader.SizeOf
        // FragmentHeader.SizeOf = 16 bytes (sequence:4 + id:4 + count:2 + size:2 + index:2 + group:2)
        let chunk_size = blob_fragment.data.len();

        // Get or create Fragment for this sequence
        let fragment = self
            .pending_fragments
            .entry(sequence)
            .or_insert_with(|| Fragment::new(sequence, count));

        // Set size and group metadata
        fragment.set_fragment_info(blob_fragment.size, blob_fragment.group as u16);

        // Add this chunk to the fragment
        fragment.add_chunk(&blob_fragment.data, index, chunk_size);

        // Check if fragment is complete
        if fragment.is_complete() {
            // Get the reassembled data
            let data = fragment.get_data();

            // Parse the reassembled data as an AC protocol message
            match RawMessage::from_fragment(data.to_vec(), sequence, blob_fragment.id) {
                Ok(message) => {
                    // Add to message queue for processing
                    self.message_queue.push_back(message);
                }
                Err(e) => {
                    error!(target: "net", "Error parsing message from fragment {}: {}", sequence, e);
                }
            }

            // Clean up metadata and remove from pending
            fragment.cleanup();
            self.pending_fragments.remove(&sequence);
        }
    }

    pub async fn process_packet(&mut self, buffer: &[u8], size: usize, peer: &SocketAddr) {
        // Pull out TransitHeader first and inspect
        let mut cursor = std::io::Cursor::new(buffer);
        let packet = PacketHeader::read(&mut cursor).unwrap();

        debug!(target: "net", "Received {} bytes from {}", size, peer);

        // Track server's packet sequence (for ACKing back to server)
        // Only update if this is a sequenced packet (sequence > 0) and it's newer than what we've seen
        if packet.sequence > 0 && packet.sequence > self.recv_count {
            debug!(target: "net", "📥 Received packet with seq={}, updating recv_count from {} to {}",
                packet.sequence, self.recv_count, packet.sequence);
            self.recv_count = packet.sequence;
            self.unacked_send_count = 0; // Reset unacked counter - server is responding
        } else if packet.sequence > 0 {
            debug!(target: "net", "📥 Received packet with seq={} (not newer than recv_count={})",
                packet.sequence, self.recv_count);
            self.unacked_send_count = 0; // Reset unacked counter - server is responding
        } else {
            // Non-sequenced packet, but server is still responding
            self.unacked_send_count = 0;
        }

        let flags = packet.flags;

        if flags.contains(PacketHeaderFlags::CONNECT_REQUEST) {
            debug!(target: "net", "Raw ConnectRequest bytes: {:02X?}", &buffer[..size]);
            let mut cursor = Cursor::new(&buffer[..size]);
            // Skip past the packet header (20 bytes) to read the ConnectRequest payload
            cursor.set_position(PACKET_HEADER_SIZE as u64);
            let connect_req_packet = ConnectRequestHeader::read(&mut cursor).unwrap();

            debug!(target: "net", "Received ConnectRequest from server");
            debug!(target: "net", "  Cookie: 0x{:016X}", connect_req_packet.cookie);
            debug!(target: "net", "  Server ID from header: {}", packet.id);
            debug!(target: "net", "  Client ID (our session index) from payload: {}", connect_req_packet.net_id);

            // Store session data from ConnectRequest
            // IMPORTANT: Use net_id from payload (our ClientId/session index), NOT packet.id (ServerId)!
            // The server uses packet.Header.Id for the SERVER's ID, not ours
            // Our session index is in the payload's net_id field
            debug!(target: "net", "🔑 Session established: client_id={}, table={}, server_id={}, cookie=0x{:016X}",
                connect_req_packet.net_id, packet.iteration, packet.id, connect_req_packet.cookie);
            self.session.set_connection(ConnectionState {
                cookie: connect_req_packet.cookie,
                client_id: connect_req_packet.net_id as u16, // Use net_id from payload - this is our session index!
                table: packet.iteration, // Use iteration from packet header as table value
                send_generator: std::sync::Mutex::new(CryptoSystem::new(
                    connect_req_packet.incoming_seed,
                )), // Client->Server seed
            });
            self.session
                .transition_to(SessionState::AuthConnectResponse);

            // Emit authentication success system event
            let _ = self
                .raw_event_tx
                .send(ClientEvent::System(
                    ClientSystemEvent::AuthenticationSucceeded,
                ))
                .await;

            info!(target: "net", "Authentication succeeded - received ConnectRequest from server");

            // Update progress to ConnectRequestReceived (66%)
            if let Some(connecting) = self.scene.as_connecting_mut() {
                connecting.connect_progress = ConnectingProgress::ConnectRequestReceived;
                let game_event = GameEvent::ConnectingSetProgress { progress: 0.66 };
                let _ = self.raw_event_tx.try_send(ClientEvent::Game(game_event));
                info!(target: "net", "Progress: ConnectRequest received (66%)");
            }

            // Delay before sending ConnectResponse (to make UI progress visible)
            tokio::time::sleep(tokio::time::Duration::from_millis(UI_DELAY_MS)).await;

            // Send ConnectResponse
            let _ = self.do_connect_response().await;

            // Update progress to ConnectResponseSent (100%) and transition to Patching phase
            if let Some(connecting) = self.scene.as_connecting_mut() {
                connecting.connect_progress = ConnectingProgress::ConnectResponseSent;
                connecting.patch_progress = PatchingProgress::WaitingForDDD;
                connecting.reset(); // Reset timing for patching phase
                let game_event = GameEvent::ConnectingSetProgress { progress: 1.0 };
                let _ = self.raw_event_tx.try_send(ClientEvent::Game(game_event));
                info!(target: "net", "Progress: ConnectResponse sent (100%)");
                info!(target: "net", "Scene transition: Connecting phase -> Patching phase");
            }

            // Emit Connected event to notify UI/scripts that connection is established
            let _ = self
                .raw_event_tx
                .try_send(ClientEvent::State(crate::client::ClientStateEvent::Connected));
            info!(target: "net", "Emitted Connected event - connection handshake complete");
        }

        if flags.contains(PacketHeaderFlags::ACK_SEQUENCE) {
            // Read the sequence number that the server is acknowledging
            let mut cursor = Cursor::new(&buffer[..size]);
            // Skip past the packet header to read the payload
            cursor.set_position(PACKET_HEADER_SIZE as u64);
            let acked_seq = u32::read(&mut cursor).unwrap();

            debug!(target: "net", "📨 Received ACK from server for our seq={} (packet.seq={}, recv_count={})",
                acked_seq, packet.sequence, self.recv_count);

            // Server is acknowledging our packets - we could track this to resend unacked packets
            // For now, we just note that we received the ACK
            // TODO: Track last_acked_by_server for retransmission logic
        }

        if flags.contains(PacketHeaderFlags::TIME_SYNC) {
            // Read the server time (8-byte double)
            let mut cursor = Cursor::new(&buffer[..size]);
            cursor.set_position(PACKET_HEADER_SIZE as u64);
            let server_time = f64::from_le_bytes([
                buffer[PACKET_HEADER_SIZE],
                buffer[PACKET_HEADER_SIZE + 1],
                buffer[PACKET_HEADER_SIZE + 2],
                buffer[PACKET_HEADER_SIZE + 3],
                buffer[PACKET_HEADER_SIZE + 4],
                buffer[PACKET_HEADER_SIZE + 5],
                buffer[PACKET_HEADER_SIZE + 6],
                buffer[PACKET_HEADER_SIZE + 7],
            ]);

            debug!(target: "net", "⏰ Received TIME_SYNC from server: time={:.3}, seq={}, recv_count={}",
                server_time, packet.sequence, self.recv_count);

            // TODO: Store server time if needed for future use
        }

        if flags.contains(PacketHeaderFlags::BLOB_FRAGMENTS) {
            // Parse the full S2CPacket to get fragment data
            let mut cursor = Cursor::new(&buffer[..size]);
            match S2CPacket::read(&mut cursor) {
                Ok(s2c_packet) => {
                    if let Some(blob_fragments) = s2c_packet.fragments {
                        self.handle_fragment(blob_fragments);
                    } else {
                        warn!(target: "net", "BLOB_FRAGMENTS flag set but no fragments in packet");
                    }
                }
                Err(e) => {
                    error!(target: "net", "Error parsing S2CPacket: {}", e);
                }
            }
        }
    }
    /// Send LoginRequest to server (part of Connecting state)
    pub async fn do_login(&mut self) -> Result<(), std::io::Error> {
        let account = self.account.name.to_lowercase();
        let password = self.account.password.clone();

        // Calculate sizes with proper AC protocol string encoding
        // For regular strings: 2 (i16 length) + string_len + padding to 4-byte alignment
        let account_size = {
            let str_len = account.len();
            let bytes_written = 2 + str_len;
            let padding = (4 - (bytes_written % 4)) % 4;
            bytes_written + padding
        };

        // account_to_login_as is empty, so it's just a u32(0) = 4 bytes
        let account_to_login_as_size = 4;

        // For password: 4 (u32 length) + 1 (packed word if len <= 255) + string_len + padding
        let password_size = {
            let str_len = password.len();
            let packed_word_size = if str_len > 255 { 2 } else { 1 };
            let data_len = packed_word_size + str_len;
            let padding = (4 - (data_len % 4)) % 4;
            4 + data_len + padding
        };

        // Total length = login_type (4) + unknown (4) + timestamp (4) + account + account_to_login_as + password
        let computed_length = 4 + 4 + 4 + account_size + account_to_login_as_size + password_size;

        // Create custom LoginRequest with correct structure
        let login_request = CustomLoginRequest {
            client_version: "1802".to_string(),
            length: computed_length as u32,
            login_type: AuthFlags::AdminAccountOverride as u32,
            unknown: 0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i32,
            account,
            password,
        };

        // Serialize the login request payload
        let mut payload_buffer = Vec::new();
        {
            let mut payload_cursor = Cursor::new(&mut payload_buffer);
            login_request
                .write(&mut payload_cursor)
                .map_err(|e| std::io::Error::other(format!("Write error: {}", e)))?;
        }

        let payload_size = payload_buffer.len();

        // Build packet manually: header + payload
        let mut buffer = Vec::with_capacity(PACKET_HEADER_SIZE + payload_size);

        // Write packet header (sequence will be overwritten by send_packet, but we need placeholder)
        buffer.extend_from_slice(&0u32.to_le_bytes()); // sequence (4) - will be set by send_packet
        buffer.extend_from_slice(&0x00010000u32.to_le_bytes()); // flags: LOGIN_REQUEST (4)
        buffer.extend_from_slice(&0u32.to_le_bytes()); // checksum placeholder (4)
        buffer.extend_from_slice(&0u16.to_le_bytes()); // recipient_id (2)
        buffer.extend_from_slice(&0u16.to_le_bytes()); // time_since_last_packet (2)
        buffer.extend_from_slice(&(payload_size as u16).to_le_bytes()); // size (2)
        buffer.extend_from_slice(&0u16.to_le_bytes()); // iteration (2)

        // Append payload
        buffer.extend_from_slice(&payload_buffer);

        // Compute checksum like C# does:
        // 1. Set checksum field to placeholder
        buffer[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4]
            .copy_from_slice(&CHECKSUM_PLACEHOLDER.to_le_bytes());

        // 2. Checksum header + payload
        let checksum1 = get_magic_number(&buffer[0..PACKET_HEADER_SIZE], PACKET_HEADER_SIZE, true);
        let checksum2 = get_magic_number(&payload_buffer, payload_size, true);
        let checksum = checksum1.wrapping_add(checksum2);

        // 3. Write final checksum back
        buffer[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4].copy_from_slice(&checksum.to_le_bytes());

        debug!(target: "net", "Sending LoginRequest for account: {}", self.account.name);

        // LoginRequest: includeSequence=false, incrementSequence=true (like actestclient line 164)
        // Sequence in header is 0, but send_count gets incremented
        // NOTE: We're building this packet manually, so we need to send it directly and manage sequence manually

        // Increment send_count for LoginRequest (matches actestclient)
        self.send_count += 1;

        // Send to login server port (9000)
        let login_addr = self.server.login_addr().await?;
        self.socket.send_to(&buffer, login_addr).await?;

        // Update progress to LoginRequestSent (33%)
        if let Some(connecting) = self.scene.as_connecting_mut()
            && connecting.connect_progress == ConnectingProgress::Initial
        {
            connecting.connect_progress = ConnectingProgress::LoginRequestSent;
            let game_event = GameEvent::ConnectingSetProgress { progress: 0.33 };
            let _ = self.raw_event_tx.send(ClientEvent::Game(game_event)).await;
            info!(target: "net", "Progress: LoginRequest sent (33%)");
        }

        Ok(())
    }

    pub async fn do_connect_response(&mut self) -> Result<(), std::io::Error> {
        // Get session data
        let connection = self
            .session
            .connection
            .as_ref()
            .expect("Session not established - ConnectRequest not received yet");

        let cookie = connection.cookie;
        let client_id = connection.client_id;
        let table = connection.table;

        // ConnectResponse payload: just a u64 cookie (8 bytes)
        let payload_size = 8;

        // Create C2SPacket with ConnectResponse
        let packet = C2SPacket {
            sequence: 0, // Will be set by send_packet
            flags: PacketHeaderFlags::CONNECT_RESPONSE,
            checksum: 0,
            recipient_id: client_id, // Must match the ClientId from ConnectRequest
            time_since_last_packet: 0,
            size: payload_size,
            iteration: table, // Must match the Table value from ConnectRequest header
            server_switch: None,
            retransmit_sequences: None,
            reject_sequences: None,
            ack_sequence: None,
            login_request: None,
            world_login_request: None,
            connect_response: Some(cookie),
            cicmd_command: None,
            time: None,
            echo_time: None,
            flow: None,
            fragments: None,
        };

        debug!(target: "net", "Sending ConnectResponse to complete handshake");
        debug!(target: "net", "  Cookie: 0x{:016X}", cookie);

        // ConnectResponse: includeSequence=false, incrementSequence=false (like actestclient line 558)
        self.send_packet(packet, false, false).await
    }

    // ========== Scene Transition Methods ==========

    /// Get the current scene
    pub fn get_scene(&self) -> &Scene {
        &self.scene
    }

    /// Transition to Connecting scene
    pub fn transition_to_connecting(&mut self) {
        self.session = ClientSession::new(SessionState::AuthLoginRequest);
        self.scene = Scene::Connecting(ConnectingScene::new());
        self.emit_scene_changed();
    }

    /// Update connecting progress within the Connecting scene
    pub fn update_connect_progress(&mut self, progress: SceneConnectingProgress) {
        if let Scene::Connecting(ref mut scene) = self.scene {
            scene.connect_progress = progress;
        }
    }

    /// Start patching phase (called after ConnectResponse is sent)
    pub fn start_patching(&mut self) {
        self.session.transition_to(SessionState::AuthConnected);
        if let Scene::Connecting(ref mut scene) = self.scene {
            scene.patch_progress = ScenePatchingProgress::WaitingForDDD;
        }
    }

    /// Update patching progress within the Connecting scene
    pub fn update_patch_progress(&mut self, progress: ScenePatchingProgress) {
        if let Scene::Connecting(ref mut scene) = self.scene {
            scene.patch_progress = progress;
        }
    }

    /// Transition to CharacterSelect scene
    pub fn transition_to_char_select(
        &mut self,
        characters: Vec<acprotocol::types::CharacterIdentity>,
    ) {
        // session.state should already be AuthConnected
        self.scene = Scene::CharacterSelect(CharacterSelectScene::new(
            self.account.name.clone(),
            characters,
        ));
        self.emit_scene_changed();
    }

    /// Begin entering world process (called when EnterWorldRequest is sent)
    pub fn begin_entering_world(&mut self, character_id: u32, character_name: String) {
        if let Scene::CharacterSelect(ref mut scene) = self.scene {
            scene.entering_world = Some(EnteringWorldState {
                character_id,
                character_name,
                account: self.account.name.clone(),
                login_complete: false,
            });
        }
    }

    /// Mark login as complete (called when Character_LoginCompleteNotification is received)
    pub fn mark_login_complete(&mut self) {
        if let Scene::CharacterSelect(ref mut scene) = self.scene
            && let Some(ref mut entering) = scene.entering_world
        {
            entering.login_complete = true;
        }
    }

    /// Transition to InWorld scene
    pub fn transition_to_in_world(&mut self, character_id: u32, character_name: String) {
        self.session.transition_to(SessionState::WorldConnected);
        self.scene = Scene::InWorld(InWorldScene::new(character_id, character_name));
        self.emit_scene_changed();
    }

    /// Transition to CharacterCreate scene
    pub fn transition_to_char_create(&mut self) {
        // session.state stays AuthConnected
        self.scene = Scene::CharacterCreate(CharacterCreateScene::new());
        self.emit_scene_changed();
    }

    /// Transition to Error scene
    pub fn transition_to_error(&mut self, error: ClientError, can_retry: bool) {
        self.scene = Scene::Error(ErrorScene::new(error, can_retry));
        self.emit_scene_changed();
    }

    /// Retry from error scene (transition back to Connecting)
    pub fn retry_from_error(&mut self) {
        if self.scene.can_retry() {
            self.transition_to_connecting();
        }
    }

    /// Emit scene changed event
    fn emit_scene_changed(&self) {
        // Emit appropriate event based on new scene
        match &self.scene {
            Scene::Connecting(_) => {
                let _ = self.raw_event_tx.try_send(ClientEvent::State(
                    crate::client::ClientStateEvent::Connecting,
                ));
            }
            Scene::CharacterSelect(_) => {
                let _ = self.raw_event_tx.try_send(ClientEvent::State(
                    crate::client::ClientStateEvent::CharacterSelect,
                ));
            }
            Scene::InWorld(_) => {
                let _ = self
                    .raw_event_tx
                    .try_send(ClientEvent::State(crate::client::ClientStateEvent::InWorld));
            }
            Scene::Error(_) => {
                let _ = self.raw_event_tx.try_send(ClientEvent::State(
                    crate::client::ClientStateEvent::CharacterError,
                ));
            }
            Scene::CharacterCreate(_) => {
                // No specific event for character create yet
            }
        }
    }
}

// ============================================================================
// GameEventHandler implementations
// ============================================================================

use crate::client::game_event_handler::GameEventHandler;

/// Handle Communication_HearDirectSpeech game events
impl GameEventHandler<acprotocol::gameevents::CommunicationHearDirectSpeech> for Client {
    fn handle(
        &mut self,
        event: acprotocol::gameevents::CommunicationHearDirectSpeech,
    ) -> Option<GameEvent> {
        let chat_text = format!("{} tells you, \"{}\"", event.sender_name, event.message);
        let message_type = event.type_ as u32;

        info!(target: "net", "Direct speech received - Type: {}, Text: {}", message_type, chat_text);

        Some(GameEvent::ChatMessageReceived {
            message: chat_text,
            message_type,
        })
    }
}

/// Handle Communication_TransientString game events
impl GameEventHandler<acprotocol::gameevents::CommunicationTransientString> for Client {
    fn handle(
        &mut self,
        event: acprotocol::gameevents::CommunicationTransientString,
    ) -> Option<GameEvent> {
        let message = event.message;
        info!(target: "net", "Transient string: {}", message);

        Some(GameEvent::ChatMessageReceived {
            message,
            message_type: 0x05, // System message type
        })
    }
}

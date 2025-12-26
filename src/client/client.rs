use std::collections::{HashMap, VecDeque};
use std::io::Cursor;
use std::net::SocketAddr;

use acprotocol::enums::{AuthFlags, FragmentGroup, PacketHeaderFlags, S2CMessage};
use acprotocol::messages::c2s::{CharacterSendCharGenResult, DDDInterrogationResponseMessage};
use acprotocol::messages::s2c::{DDDInterrogationMessage, LoginLoginCharacterSet};
use tokio::sync::{broadcast, mpsc};

/// Enum for outgoing messages to be sent in the network loop
#[derive(Debug)]
pub enum PendingOutgoingMessage {
    DDDInterrogationResponse(DDDInterrogationResponseMessage),
    CharacterCreation(CharacterSendCharGenResult),
}

use acprotocol::network::packet::PacketHeader;
use acprotocol::network::{Fragment, RawMessage};
use acprotocol::packets::c2s_packet::C2SPacket;
use acprotocol::packets::s2c_packet::S2CPacket;
use acprotocol::readers::ACDataType;
use acprotocol::types::{BlobFragments, ConnectRequestHeader, PackableList};
use acprotocol::writers::{write_i32, write_string, write_u32, ACWritable, ACWriter};
use tokio::net::UdpSocket;
use tracing::{debug, error, info, warn};

use crate::client::events::{ClientAction, GameEvent};
use crate::crypto::magic_number::get_magic_number;

// Protocol constants
const CHECKSUM_PLACEHOLDER: u32 = 0xbadd70dd;
const PACKET_HEADER_SIZE: usize = 20;
const CHECKSUM_OFFSET: usize = 8;
const FRAGMENT_HEADER_SIZE: usize = 16; // sequence(4) + id(4) + count(2) + size(2) + index(2) + group(2)

/// Server information tracking both login and world ports
#[derive(Clone, Debug)]
pub struct ServerInfo {
    pub host: String,
    pub login_port: u16, // Port 9000 - for LoginRequest and most traffic
    pub world_port: u16, // Port 9001 - for ConnectResponse and game data
}

impl ServerInfo {
    pub fn new(host: String, login_port: u16) -> Self {
        ServerInfo {
            host,
            login_port,
            world_port: login_port + 1,
        }
    }

    /// Check if the given peer address matches this server (either port)
    pub fn is_from(&self, peer: &SocketAddr) -> bool {
        let peer_ip = peer.ip().to_string();
        peer_ip == self.host || peer_ip == "127.0.0.1" || peer_ip == "::1"
    }

    /// Get the login server address for sending standard messages
    pub async fn login_addr(&self) -> Result<SocketAddr, std::io::Error> {
        let addr = format!("{}:{}", self.host, self.login_port);
        tokio::net::lookup_host(addr)
            .await?
            .find(|a| a.is_ipv4())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not resolve IPv4 address",
                )
            })
    }

    /// Get the world server address for sending ConnectResponse
    pub async fn world_addr(&self) -> Result<SocketAddr, std::io::Error> {
        let addr = format!("{}:{}", self.host, self.world_port);
        tokio::net::lookup_host(addr)
            .await?
            .find(|a| a.is_ipv4())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not resolve IPv4 address",
                )
            })
    }
}

/// Session state received from the server's ConnectRequest packet
#[derive(Clone, Debug)]
struct SessionState {
    cookie: u64,
    client_id: u16,
    table: u16,         // Table/iteration value from packet header
    outgoing_seed: u32, // Server->Client checksum seed
    incoming_seed: u32, // Client->Server checksum seed
}

/// Custom LoginRequest structure that matches the actual C# client implementation.
/// This is needed because acprotocol's LoginRequestHeaderType2 is missing the timestamp field.
#[derive(Clone, Debug)]
struct CustomLoginRequest {
    client_version: String,
    length: u32,
    login_type: u32, // Password authentication type
    unknown: u32,    // Always 0
    timestamp: i32,  // Unix timestamp
    account: String,
    password: String, // Raw password string (not WString)
}

impl ACWritable for CustomLoginRequest {
    fn write(&self, writer: &mut dyn ACWriter) -> Result<(), Box<dyn std::error::Error>> {
        // Write client_version string (AC format: i16 length + data + padding to 4-byte alignment)
        write_string(writer, &self.client_version)?;

        // Write length field
        write_u32(writer, self.length)?;

        // Write login type (2 for password)
        write_u32(writer, self.login_type)?;

        // Write unknown (always 0)
        write_u32(writer, self.unknown)?;

        // Write timestamp
        write_i32(writer, self.timestamp)?;

        // Write account name
        write_string(writer, &self.account)?;

        // Write account_to_login_as (always empty = 4 zero bytes for u32)
        write_u32(writer, 0)?;

        // Write password in C# format (NOT WString):
        // 1. 4-byte int: length of (packed_byte + string_data)
        // 2. 1-byte packed length
        // 3. char array data
        // 4. padding to 4-byte alignment
        let password_len = self.password.len();
        let packed_byte_size = if password_len > 255 { 2 } else { 1 };
        let total_data_len = packed_byte_size + password_len;

        write_u32(writer, total_data_len as u32)?;

        if password_len <= 255 {
            writer.write_all(&[password_len as u8])?;
        } else {
            // 2-byte packed length for strings > 255
            let high_byte = ((password_len >> 8) as u8) | 0x80;
            let low_byte = (password_len & 0xFF) as u8;
            writer.write_all(&[high_byte, low_byte])?;
        }

        // Write password chars
        writer.write_all(self.password.as_bytes())?;

        // Write alignment padding if needed
        let padding = (4 - (total_data_len % 4)) % 4;
        if padding > 0 {
            writer.write_all(&vec![0u8; padding])?;
        }

        Ok(())
    }
}

/// Extension trait for C2SPacket to add serialization with checksum
/// TODO: Consider putting this in acprotocol
trait C2SPacketExt {
    fn serialize(&self) -> Result<Vec<u8>, std::io::Error>;
}

impl C2SPacketExt for C2SPacket {
    fn serialize(&self) -> Result<Vec<u8>, std::io::Error> {
        let mut buffer = Vec::new();
        {
            let mut cursor = Cursor::new(&mut buffer);
            self.write(&mut cursor)
                .map_err(|e| std::io::Error::other(format!("Write error: {}", e)))?;
        }

        // Compute checksum on the entire serialized packet
        let checksum = get_magic_number(&buffer, buffer.len(), true);

        // Write checksum back into buffer
        if buffer.len() >= CHECKSUM_OFFSET + 4 {
            buffer[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4].copy_from_slice(&checksum.to_le_bytes());
        } else {
            panic!("At the time of writing the packet checksum, buffer was too small to be valid.");
        }

        Ok(buffer)
    }
}

// ClientConnectState
// TODO: Put this somewhere else
#[derive(Clone, Debug, PartialEq)]
pub enum ClientConnectState {
    #[allow(dead_code)]
    Error,
    Disconnected,
    Connecting,
    Connected,
}
#[derive(Clone, Debug, PartialEq)]
pub enum ClientLoginState {
    Error,
    NotLoggedIn,
    LoggingIn,
    LoggedIn,
}

// End state machine

struct Account {
    name: String,
    password: String,
}

// TODO: Don't require both bind_address and connect_address. I had to do this
// to get things to work but I should be able to listen on any random port so
// I'm not sure what I'm doing wrong
pub struct Client {
    pub id: u32,
    pub server: ServerInfo,
    pub socket: UdpSocket,
    account: Account,
    connect_state: ClientConnectState,
    pub login_state: ClientLoginState,
    pub send_count: u32,
    pub recv_count: u32,
    last_acked_to_server: u32, // Last sequence we ACKed to the server
    fragment_sequence: u32, // Counter for outgoing fragment sequences
    session: Option<SessionState>,
    pending_fragments: HashMap<u32, Fragment>, // Track incomplete fragment sequences
    message_queue: VecDeque<RawMessage>,       // Queue of parsed messages to process
    outgoing_message_queue: VecDeque<PendingOutgoingMessage>, // Queue of messages to send
    event_tx: broadcast::Sender<GameEvent>,    // Broadcast events to handlers
    action_rx: mpsc::UnboundedReceiver<ClientAction>, // Receive actions from handlers
}

impl Client {
    pub async fn new(
        id: u32,
        address: String,
        name: String,
        password: String,
    ) -> (
        Client,
        broadcast::Receiver<GameEvent>,
        mpsc::UnboundedSender<ClientAction>,
    ) {
        let sok = UdpSocket::bind("0.0.0.0:0").await.unwrap();

        // Parse address to extract host and port
        let parts: Vec<&str> = address.split(':').collect();
        let host = parts[0].to_string();
        let login_port = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(9000);

        // Create channels for event system
        // Event channel: Client broadcasts events to handlers
        let (event_tx, event_rx) = broadcast::channel(100);

        // Action channel: Handlers send actions back to client
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        let client = Client {
            id,
            server: ServerInfo::new(host, login_port),
            account: Account { name, password },
            socket: sok,
            connect_state: ClientConnectState::Disconnected,
            login_state: ClientLoginState::NotLoggedIn,
            send_count: 0,
            recv_count: 0,
            last_acked_to_server: 0,
            fragment_sequence: 1, // Start at 1 as per actestclient
            session: None,
            pending_fragments: HashMap::new(),
            message_queue: VecDeque::new(),
            outgoing_message_queue: VecDeque::new(),
            event_tx,
            action_rx,
        };

        (client, event_rx, action_tx)
    }

    /// Get the next sequence number for outgoing packets and increment the counter
    fn next_sequence(&mut self) -> u32 {
        let seq = self.send_count;
        self.send_count += 1;
        seq
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
        !self.outgoing_message_queue.is_empty()
    }

    /// Send all pending outgoing messages
    pub async fn send_pending_messages(&mut self) -> Result<(), std::io::Error> {
        while let Some(message) = self.outgoing_message_queue.pop_front() {
            self.send_outgoing_message(message).await?;
        }
        Ok(())
    }

    /// Send keep-alive packet (ACK or TimeSync) to maintain connection
    pub async fn send_keepalive(&mut self) -> Result<(), std::io::Error> {
        // If we have received new sequences from server that we haven't ACKed, send ACK
        if self.recv_count > self.last_acked_to_server && self.session.is_some() {
            debug!(target: "net", "Sending ACK for sequence {}", self.recv_count);
            self.send_ack(self.recv_count).await?;
            self.last_acked_to_server = self.recv_count;
        }
        // Otherwise, send TimeSync to keep connection alive
        else if self.session.is_some() {
            debug!(target: "net", "Sending TimeSync keep-alive");
            self.send_timesync().await?;
        }
        Ok(())
    }

    /// Send an ACK packet to acknowledge received server sequences
    async fn send_ack(&mut self, ack_sequence: u32) -> Result<(), std::io::Error> {
        let (client_id, table) = {
            let session = self.session.as_ref().ok_or_else(|| {
                std::io::Error::other("Session not established")
            })?;
            (session.client_id, session.table)
        };

        debug!(target: "net", "Creating ACK packet: client_id={}, table={}, ack_seq={}", client_id, table, ack_sequence);

        // ACK packets don't increment send_count
        let packet = C2SPacket {
            sequence: self.send_count, // Use current send_count without incrementing
            flags: PacketHeaderFlags::ACK_SEQUENCE,
            checksum: 0,
            recipient_id: client_id,
            time_since_last_packet: 0,
            size: 4, // ACK payload is just the u32 ack_sequence
            iteration: table,
            server_switch: None,
            retransmit_sequences: None,
            reject_sequences: None,
            ack_sequence: Some(ack_sequence),
            login_request: None,
            world_login_request: None,
            connect_response: None,
            cicmd_command: None,
            time: None,
            echo_time: None,
            flow: None,
            fragments: None,
        };

        let buffer = packet.serialize()?;
        // After connection, all packets with Id > 0 go to the login server port (9000), not world port
        let login_addr = self.server.login_addr().await?;
        debug!(target: "net", "Sending ACK to login server at {}", login_addr);
        self.socket.send_to(&buffer, login_addr).await?;
        Ok(())
    }

    /// Send a TimeSync packet to keep connection alive
    async fn send_timesync(&mut self) -> Result<(), std::io::Error> {
        let (client_id, table) = {
            let session = self.session.as_ref().ok_or_else(|| {
                std::io::Error::other("Session not established")
            })?;
            (session.client_id, session.table)
        };

        // Get current time as Unix timestamp (seconds since epoch)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // TimeSync packets don't increment send_count
        let packet = C2SPacket {
            sequence: self.send_count, // Use current send_count without incrementing
            flags: PacketHeaderFlags::TIME_SYNC,
            checksum: 0,
            recipient_id: client_id,
            time_since_last_packet: 0,
            size: 8, // TimeSync payload is a u64
            iteration: table,
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

        let buffer = packet.serialize()?;
        // After connection, all packets with Id > 0 go to the login server port (9000), not world port
        let login_addr = self.server.login_addr().await?;
        debug!(target: "net", "Sending TimeSync to login server at {}", login_addr);
        self.socket.send_to(&buffer, login_addr).await?;
        Ok(())
    }

    /// Get a new subscriber to client events
    pub fn subscribe_events(&self) -> broadcast::Receiver<GameEvent> {
        self.event_tx.subscribe()
    }

    /// Process actions sent from event handlers
    pub fn process_actions(&mut self) {
        // Process all pending actions without blocking
        while let Ok(action) = self.action_rx.try_recv() {
            match action {
                ClientAction::SendMessage(msg) => {
                    debug!(target: "events", "Action: Enqueueing message from event handler");
                    self.outgoing_message_queue.push_back(msg);
                }
                ClientAction::Disconnect => {
                    info!(target: "events", "Action: Disconnecting");
                    self.connect_state = ClientConnectState::Disconnected;
                }
            }
        }
    }

    /// Send a single outgoing message
    async fn send_outgoing_message(
        &mut self,
        message: PendingOutgoingMessage,
    ) -> Result<(), std::io::Error> {
        match message {
            PendingOutgoingMessage::DDDInterrogationResponse(response) => {
                self.send_ddd_response_internal(response).await
            }
            PendingOutgoingMessage::CharacterCreation(char_gen) => {
                self.send_character_creation_internal(char_gen).await
            }
        }
    }

    /// Send a message wrapped in a BlobFragment
    async fn send_fragmented_message(
        &mut self,
        message_data: Vec<u8>,
        group: FragmentGroup,
    ) -> Result<(), std::io::Error> {
        // Get current fragment sequence and increment
        let frag_sequence = self.fragment_sequence;
        self.fragment_sequence += 1;

        // Create BlobFragment structure
        let fragment_size = (FRAGMENT_HEADER_SIZE + message_data.len()) as u16;
        let blob_fragment = BlobFragments {
            sequence: frag_sequence,
            id: 0x80000000, // Object ID (0x80000000 for game messages)
            count: 1, // Single fragment
            size: fragment_size,
            index: 0, // First (and only) fragment
            group,
            data: message_data,
        };

        // Extract session values
        let (client_id, table) = {
            let session = self.session.as_ref().expect("Session not established");
            (session.client_id, session.table)
        };

        // Increment send_count first, then use it (matches actestclient behavior)
        self.send_count += 1;
        let packet_sequence = self.send_count;

        // Create C2SPacket with BlobFragments flag
        let packet = C2SPacket {
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
            ack_sequence: None,
            login_request: None,
            world_login_request: None,
            connect_response: None,
            cicmd_command: None,
            time: None,
            echo_time: None,
            flow: None,
            fragments: Some(blob_fragment),
        };

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
        let header_checksum = get_magic_number(&buffer[0..PACKET_HEADER_SIZE], PACKET_HEADER_SIZE, true);

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

    /// Send a DDD interrogation response to the login server
    async fn send_ddd_response_internal(
        &mut self,
        response: DDDInterrogationResponseMessage,
    ) -> Result<(), std::io::Error> {
        info!(target: "net", "Sending DDD Interrogation Response - Language: {}, Files: {:?}",
            response.language, response.files.list);

        // Serialize the response message (payload data without opcode)
        let mut message_data = Vec::new();
        {
            let mut cursor = Cursor::new(&mut message_data);
            response
                .write(&mut cursor)
                .map_err(|e| std::io::Error::other(format!("Write error: {}", e)))?;
        }

        // Send as a proper fragmented packet
        self.send_fragmented_message(message_data, FragmentGroup::Object).await
    }

    /// Send character creation request to the login server
    async fn send_character_creation_internal(
        &mut self,
        char_gen: CharacterSendCharGenResult,
    ) -> Result<(), std::io::Error> {
        info!(target: "net", "Sending Character Creation - Name: {}", char_gen.result.name);

        // Serialize the character creation message with opcode prefix
        let mut message_data = Vec::new();
        {
            let mut cursor = Cursor::new(&mut message_data);
            // Write opcode first (0xF656 = Character_SendCharGenResult)
            write_u32(&mut cursor, 0xF656)
                .map_err(|e| std::io::Error::other(format!("Write error: {}", e)))?;
            // Then write the message payload
            char_gen
                .write(&mut cursor)
                .map_err(|e| std::io::Error::other(format!("Write error: {}", e)))?;
        }

        // Send as a proper fragmented packet
        self.send_fragmented_message(message_data, FragmentGroup::Object).await
    }

    /// Handle a single parsed message
    fn handle_message(&mut self, message: RawMessage) {
        debug!(target: "net", "Received message: {} (0x{:08X})", message.message_type, message.opcode);

        match S2CMessage::try_from(message.opcode) {
            Ok(msg_type) => {
                match msg_type {
                    S2CMessage::LoginLoginCharacterSet => self.handle_character_list(message),
                    S2CMessage::DDDInterrogationMessage => self.handle_ddd_interrogation(message),
                    // Add more handlers as needed
                    _ => {
                        warn!(target: "net", "Unhandled S2CMessage: {:?} (0x{:04X})", msg_type, message.opcode);
                    }
                }
            }
            Err(_) => {
                warn!(target: "net", "Unknown message opcode: 0x{:04X}", message.opcode);
            }
        }
    }

    /// Handle the character list message from the server
    fn handle_character_list(&mut self, message: RawMessage) {
        debug!(target: "net", "Processing character list message");

        // Parse using acprotocol's generated parser
        // Note: message.data includes the opcode as the first 4 bytes, skip it
        let payload = &message.data[4..];
        let mut cursor = Cursor::new(payload);
        match LoginLoginCharacterSet::read(&mut cursor) {
            Ok(char_list) => {
                info!(target: "net", "=== Character List for Account: {} ===", char_list.account);
                info!(target: "net", "Available character slots: {}", char_list.num_allowed_characters);
                info!(target: "net", "Characters on account: {}", char_list.characters.list.len());

                for character in &char_list.characters.list {
                    if character.seconds_greyed_out > 0 {
                        info!(target: "net", "  - {} (ID: {:?}) [PENDING DELETION in {} seconds]",
                            character.name, character.character_id, character.seconds_greyed_out);
                    } else {
                        info!(target: "net", "  - {} (ID: {:?})", character.name, character.character_id);
                    }
                }

                if !char_list.deleted_characters.list.is_empty() {
                    info!(target: "net", "Characters pending deletion: {}", char_list.deleted_characters.list.len());
                }

                // Emit event to broadcast channel
                use crate::client::events::CharacterInfo;
                let characters = char_list
                    .characters
                    .list
                    .iter()
                    .map(|c| {
                        CharacterInfo {
                            name: c.name.clone(),
                            id: c.character_id.0, // Extract u32 from ObjectId wrapper
                            delete_pending: c.seconds_greyed_out > 0,
                        }
                    })
                    .collect();

                let event = GameEvent::CharacterListReceived {
                    account: char_list.account.clone(),
                    characters,
                    num_slots: char_list.num_allowed_characters,
                };

                // Send on channel (ignore error if no subscribers)
                let _ = self.event_tx.send(event);
            }
            Err(e) => {
                error!(target: "net", "Failed to parse character list: {}", e);
            }
        }
    }

    /// Handle DDD Interrogation message from the server
    fn handle_ddd_interrogation(&mut self, message: RawMessage) {
        debug!(target: "net", "Processing DDD interrogation message");

        // Parse the incoming message
        // Note: message.data includes the opcode as the first 4 bytes, skip it
        let mut cursor = Cursor::new(&message.data[4..]);
        match DDDInterrogationMessage::read(&mut cursor) {
            Ok(ddd_msg) => {
                info!(target: "net", "Received DDD Interrogation - Language: {}, Region: {}, Product: {}",
                    ddd_msg.name_rule_language, ddd_msg.servers_region, ddd_msg.product_id);

                // Prepare response with language 1 and the file list from the pcap
                let files = vec![4294967296, -8899172235240, 4294967297];
                let response = DDDInterrogationResponseMessage {
                    language: 1,
                    files: PackableList {
                        count: files.len() as u32,
                        list: files,
                    },
                };

                // Queue the response to be sent in the next send cycle
                self.outgoing_message_queue
                    .push_back(PendingOutgoingMessage::DDDInterrogationResponse(response));
                info!(target: "net", "DDD response queued for sending");
            }
            Err(e) => {
                error!(target: "net", "Failed to parse DDD interrogation message: {}", e);
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
            self.recv_count = packet.sequence;
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
            self.session = Some(SessionState {
                cookie: connect_req_packet.cookie,
                client_id: connect_req_packet.net_id as u16, // Use net_id from payload - this is our session index!
                table: packet.iteration, // Use iteration from packet header as table value
                outgoing_seed: connect_req_packet.outgoing_seed,
                incoming_seed: connect_req_packet.incoming_seed,
            });

            // Small delay to avoid race condition with server's async authentication
            // The server validates the password asynchronously, so we need to wait
            // for it to transition to AuthConnectResponse state before sending ConnectResponse
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            let _ = self.do_connect_response().await;
        }

        if flags.contains(PacketHeaderFlags::ACK_SEQUENCE) {
            // Read the sequence number that the server is acknowledging
            let mut cursor = Cursor::new(&buffer[..size]);
            // Skip past the packet header to read the payload
            cursor.set_position(PACKET_HEADER_SIZE as u64);
            let _acked_seq = u32::read(&mut cursor).unwrap();

            // Server is acknowledging our packets - we could track this to resend unacked packets
            // For now, we just note that we received the ACK
            // TODO: Track last_acked_by_server for retransmission logic
        }

        if flags.contains(PacketHeaderFlags::TIME_SYNC) {
            // Read the server time (8-byte double)
            let mut cursor = Cursor::new(&buffer[..size]);
            cursor.set_position(PACKET_HEADER_SIZE as u64);
            let _server_time = f64::from_le_bytes([
                buffer[PACKET_HEADER_SIZE],
                buffer[PACKET_HEADER_SIZE + 1],
                buffer[PACKET_HEADER_SIZE + 2],
                buffer[PACKET_HEADER_SIZE + 3],
                buffer[PACKET_HEADER_SIZE + 4],
                buffer[PACKET_HEADER_SIZE + 5],
                buffer[PACKET_HEADER_SIZE + 6],
                buffer[PACKET_HEADER_SIZE + 7],
            ]);

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
    // TODO: Should return a Result with a success or failure
    pub async fn connect(&mut self) -> Result<(), std::io::Error> {
        self.connect_state = ClientConnectState::Connecting;

        // Note: We don't use socket.connect() here because we need to send to different
        // ports (9000 for login, 9001 for world). Instead, we use send_to() with explicit addresses.

        self.connect_state = ClientConnectState::Connected;

        Ok(())
    }

    pub async fn do_login(&mut self) -> Result<(), std::io::Error> {
        self.login_state = ClientLoginState::LoggingIn;

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

        // Write packet header
        let sequence = self.next_sequence();
        buffer.extend_from_slice(&sequence.to_le_bytes()); // sequence (4)
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
        debug!(target: "net", "LoginRequest data: {:2X?}", buffer);

        // Send to login server port (9000)
        let login_addr = format!("{}:{}", self.server.host, self.server.login_port);
        let login_sockaddr = tokio::net::lookup_host(&login_addr)
            .await?
            .find(|addr| addr.is_ipv4())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not resolve IPv4 address",
                )
            })?;
        match self.socket.send_to(&buffer, login_sockaddr).await {
            Ok(_) => {}
            Err(e) => {
                error!(target: "net", "Send error: {}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    pub async fn do_connect_response(&mut self) -> Result<(), std::io::Error> {
        // Get session data
        let session = self
            .session
            .as_ref()
            .expect("Session not established - ConnectRequest not received yet");

        let cookie = session.cookie;
        let client_id = session.client_id;
        let table = session.table;

        // ConnectResponse payload: just a u64 cookie (8 bytes)
        let payload_size = 8;

        // Create C2SPacket with ConnectResponse
        // NOTE: We use sequence 0 and increment manually per C# code (includeSequence=false, incrementSequence=false)
        let packet = C2SPacket {
            sequence: 0, // ConnectResponse doesn't use sequence numbers
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

        // Serialize packet with checksum
        let buffer = packet.serialize()?;

        debug!(target: "net", "Sending ConnectResponse to complete handshake");
        debug!(target: "net", "  Cookie: 0x{:016X}", cookie);
        debug!(target: "net", "ConnectResponse data: {:2X?}", buffer);

        // CRITICAL: ConnectResponse must be sent to world server port (9001), not login port (9000)
        // This matches the C# client behavior where ConnectResponse uses "ReadAddress"
        let world_addr = format!("{}:{}", self.server.host, self.server.world_port);

        let world_sockaddr = tokio::net::lookup_host(&world_addr)
            .await?
            .find(|addr| addr.is_ipv4())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not resolve IPv4 address",
                )
            })?;
        match self.socket.send_to(&buffer, world_sockaddr).await {
            Ok(_) => {}
            Err(e) => {
                error!(target: "net", "Send error: {}", e);
                return Err(e);
            }
        }
        Ok(())
    }

    pub async fn do_ack_response(&mut self, value: u32) -> Result<(), std::io::Error> {
        // AckSequence payload: just a u32 value (4 bytes)
        let payload_size = 4;

        // Create C2SPacket with AckSequence
        let packet = C2SPacket {
            sequence: self.next_sequence(),
            flags: PacketHeaderFlags::ACK_SEQUENCE,
            checksum: 0,
            recipient_id: 0,
            time_since_last_packet: 0,
            size: payload_size,
            iteration: 0,
            server_switch: None,
            retransmit_sequences: None,
            reject_sequences: None,
            ack_sequence: Some(value),
            login_request: None,
            world_login_request: None,
            connect_response: None,
            cicmd_command: None,
            time: None,
            echo_time: None,
            flow: None,
            fragments: None,
        };

        // Serialize packet with checksum
        let buffer = packet.serialize()?;

        debug!(target: "net", "Sending AckResponse with data: {:2X?}", buffer);

        // Send ACK to login server port (9000)
        let login_addr = format!("{}:{}", self.server.host, self.server.login_port);
        let login_sockaddr = tokio::net::lookup_host(&login_addr)
            .await?
            .find(|addr| addr.is_ipv4())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not resolve IPv4 address",
                )
            })?;
        match self.socket.send_to(&buffer, login_sockaddr).await {
            Ok(_) => {}
            Err(e) => {
                error!(target: "net", "Send error: {}", e);
                return Err(e);
            }
        }
        Ok(())
    }
}

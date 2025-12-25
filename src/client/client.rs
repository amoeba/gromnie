use std::collections::{HashMap, VecDeque};
use std::io::Cursor;
use std::net::SocketAddr;

use acprotocol::enums::{AuthFlags, PacketHeaderFlags, S2CMessage};
use acprotocol::network::{Fragment, RawMessage};
use acprotocol::network::packet::PacketHeader;
use acprotocol::packets::c2s_packet::C2SPacket;
use acprotocol::packets::s2c_packet::S2CPacket;
use acprotocol::readers::ACDataType;
use acprotocol::types::ConnectRequestHeader;
use acprotocol::writers::{
    write_i32, write_string, write_u32, ACWritable, ACWriter,
};
use tokio::net::UdpSocket;
use tracing::{debug, error, warn};

use crate::crypto::magic_number::get_magic_number;

// Protocol constants
const CHECKSUM_PLACEHOLDER: u32 = 0xbadd70dd;
const PACKET_HEADER_SIZE: usize = 20;
const CHECKSUM_OFFSET: usize = 8;

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

/// Server connection information tracking both login and world server ports
#[derive(Clone, Debug)]
struct ServerInfo {
    login_port: u16,  // Port 9000 - for LoginRequest and most traffic
    world_port: u16,  // Port 9001 - for ConnectResponse and game data
    host: String,
}

// TODO: Don't require both bind_address and connect_address. I had to do this
// to get things to work but I should be able to listen on any random port so
// I'm not sure what I'm doing wrong
pub struct Client {
    pub id: u32,
    server: ServerInfo,
    pub socket: UdpSocket,
    account: Account,
    connect_state: ClientConnectState,
    pub login_state: ClientLoginState,
    pub send_count: u32,
    pub recv_count: u32,
    session: Option<SessionState>,
    pending_fragments: HashMap<u32, Fragment>,  // Track incomplete fragment sequences
    message_queue: VecDeque<RawMessage>,        // Queue of parsed messages to process
}

impl Client {
    pub async fn new(id: u32, address: String, name: String, password: String) -> Client {
        let sok = UdpSocket::bind("0.0.0.0:0").await.unwrap();

        // Parse address to extract host and port
        let parts: Vec<&str> = address.split(':').collect();
        let host = parts[0].to_string();
        let login_port = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(9000);

        Client {
            id,
            server: ServerInfo {
                login_port,
                world_port: login_port + 1,  // Typically 9001
                host,
            },
            account: Account { name, password },
            socket: sok,
            connect_state: ClientConnectState::Disconnected,
            login_state: ClientLoginState::NotLoggedIn,
            send_count: 0,
            recv_count: 0,
            session: None,
            pending_fragments: HashMap::new(),
            message_queue: VecDeque::new(),
        }
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

    /// Handle a single parsed message
    fn handle_message(&mut self, message: RawMessage) {
        debug!(target: "net", "Received message: {} (0x{:04X})", message.message_type, message.opcode);

        // Handle message based on S2C message type
        // Panic on unhandled messages so we know what needs to be implemented
        match S2CMessage::try_from(message.opcode) {
            Ok(msg_type) => {
                // Panic to force us to implement the handler
                panic!("Unhandled S2CMessage: {:?} (0x{:04X})\nImplement handler for this message type!",
                       msg_type, message.opcode);
            }
            Err(_) => {
                panic!("Unknown message opcode: 0x{:04X}\nThis message type is not in the S2CMessage enum!",
                       message.opcode);
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
        let fragment = self.pending_fragments
            .entry(sequence)
            .or_insert_with(|| {
                Fragment::new(sequence, count)
            });

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

        let flags = packet.flags;

        if flags.contains(PacketHeaderFlags::CONNECT_REQUEST) {
            let mut cursor = Cursor::new(&buffer[..size]);
            let connect_req_packet = ConnectRequestHeader::read(&mut cursor).unwrap();

            // Store session data from ConnectRequest
            // Note: table/iteration comes from the packet header, not the ConnectRequest payload
            self.session = Some(SessionState {
                cookie: connect_req_packet.cookie,
                client_id: connect_req_packet.net_id as u16,
                table: packet.iteration,  // Use iteration from packet header as table value
                outgoing_seed: connect_req_packet.outgoing_seed,
                incoming_seed: connect_req_packet.incoming_seed,
            });

            let _ = self.do_connect_response().await;
        }

        if flags.contains(PacketHeaderFlags::ACK_SEQUENCE) {
            // Read the sequence number that the server is acknowledging
            let mut cursor = Cursor::new(&buffer[..size]);
            // Skip past the packet header to read the payload
            cursor.set_position(PACKET_HEADER_SIZE as u64);
            let acked_seq = u32::read(&mut cursor).unwrap();

            // Store the last acked sequence - we don't send a response to this
            self.recv_count = acked_seq;
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
            login_request.write(&mut payload_cursor)
                .map_err(|e| std::io::Error::other(format!("Write error: {}", e)))?;
        }

        let payload_size = payload_buffer.len();

        // Build packet manually: header + payload
        let mut buffer = Vec::with_capacity(PACKET_HEADER_SIZE + payload_size);

        // Write packet header
        let sequence = self.next_sequence();
        buffer.extend_from_slice(&sequence.to_le_bytes());                      // sequence (4)
        buffer.extend_from_slice(&0x00010000u32.to_le_bytes());                 // flags: LOGIN_REQUEST (4)
        buffer.extend_from_slice(&0u32.to_le_bytes());                          // checksum placeholder (4)
        buffer.extend_from_slice(&0u16.to_le_bytes());                          // recipient_id (2)
        buffer.extend_from_slice(&0u16.to_le_bytes());                          // time_since_last_packet (2)
        buffer.extend_from_slice(&(payload_size as u16).to_le_bytes());         // size (2)
        buffer.extend_from_slice(&0u16.to_le_bytes());                          // iteration (2)

        // Append payload
        buffer.extend_from_slice(&payload_buffer);

        // Compute checksum like C# does:
        // 1. Set checksum field to placeholder
        buffer[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4].copy_from_slice(&CHECKSUM_PLACEHOLDER.to_le_bytes());

        // 2. Checksum header + payload
        let checksum1 = get_magic_number(&buffer[0..PACKET_HEADER_SIZE], PACKET_HEADER_SIZE, true);
        let checksum2 = get_magic_number(&payload_buffer, payload_size, true);
        let checksum = checksum1.wrapping_add(checksum2);

        // 3. Write final checksum back
        buffer[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4].copy_from_slice(&checksum.to_le_bytes());

        debug!(target: "net", "Sending LoginRequest with data: {:2X?}", buffer);

        // Send to login server port (9000)
        let login_addr = format!("{}:{}", self.server.host, self.server.login_port);
        let login_sockaddr = tokio::net::lookup_host(&login_addr)
            .await?
            .find(|addr| addr.is_ipv4())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Could not resolve IPv4 address"))?;
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
            sequence: 0,  // ConnectResponse doesn't use sequence numbers
            flags: PacketHeaderFlags::CONNECT_RESPONSE,
            checksum: 0,
            recipient_id: client_id,  // Must match the ClientId from ConnectRequest
            time_since_last_packet: 0,
            size: payload_size,
            iteration: table,  // Must match the Table value from ConnectRequest header
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

        debug!(target: "net", "Sending ConnectResponse with data: {:2X?}", buffer);

        // CRITICAL: ConnectResponse must be sent to world server port (9001), not login port (9000)
        // This matches the C# client behavior where ConnectResponse uses "ReadAddress"
        let world_addr = format!("{}:{}", self.server.host, self.server.world_port);

        let world_sockaddr = tokio::net::lookup_host(&world_addr)
            .await?
            .find(|addr| addr.is_ipv4())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Could not resolve IPv4 address"))?;
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
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Could not resolve IPv4 address"))?;
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

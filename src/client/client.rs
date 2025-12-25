use std::io::Cursor;
use std::net::SocketAddr;

use acprotocol::enums::PacketHeaderFlags;
use acprotocol::network::packet::PacketHeader;
use acprotocol::packets::c2s_packet::C2SPacket;
use acprotocol::readers::ACDataType;
use acprotocol::types::ConnectRequestHeader;
use acprotocol::writers::{
    write_i32, write_string, write_u32, ACWritable, ACWriter,
};
use tokio::net::UdpSocket;

use crate::crypto::magic_number::get_magic_number;

/// Custom LoginRequest structure that matches the actual C# client implementation.
/// This is needed because acprotocol's LoginRequestHeaderType2 is missing the timestamp field.
#[derive(Clone, Debug)]
struct CustomLoginRequest {
    client_version: String,
    length: u32,
    login_type: u32,       // 0x00000002 for password authentication
    unknown: u32,          // Always 0
    timestamp: i32,        // Unix timestamp
    account: String,
    account_to_login_as: String,
    password: String,      // Raw password string (not WString)
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

        // Write account_to_login_as (empty string = 4 zero bytes for u32)
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
/// TODO: Concisder putting this in acprotocol
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

        // Write checksum back into buffer (checksum is at offset 8-11)
        if buffer.len() >= 12 {
            buffer[8..12].copy_from_slice(&checksum.to_le_bytes());
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
    address: String,
    pub socket: UdpSocket,
    account: Account,
    connect_state: ClientConnectState,
    pub login_state: ClientLoginState,
    pub send_count: u32,
    pub recv_count: u32,
    // Session state from ConnectRequest
    cookie: Option<u64>,
    client_id: Option<u16>,
    outgoing_seed: Option<u32>, // Server->Client checksum seed
    incoming_seed: Option<u32>, // Client->Server checksum seed
}

impl Client {
    pub async fn new(id: u32, address: String, name: String, password: String) -> Client {
        let sok = UdpSocket::bind("0.0.0.0:0").await.unwrap();

        // Debug
        let local_addr = sok.local_addr().unwrap();
        println!(
            "[Client::create] client listening on {}:{}",
            local_addr.ip(),
            local_addr.port()
        );

        Client {
            id,
            address,
            account: Account { name, password },
            socket: sok,
            connect_state: ClientConnectState::Disconnected,
            login_state: ClientLoginState::NotLoggedIn,
            send_count: 0,
            recv_count: 0,
            cookie: None,
            client_id: None,
            outgoing_seed: None,
            incoming_seed: None,
        }
    }

    /// Get the next sequence number for outgoing packets and increment the counter
    fn next_sequence(&mut self) -> u32 {
        let seq = self.send_count;
        self.send_count += 1;
        seq
    }

    pub async fn process_packet(&mut self, buffer: &[u8], size: usize, peer: &SocketAddr) {
        // Pull out TransitHeader first and inspect
        let mut cursor = std::io::Cursor::new(buffer);
        let packet = PacketHeader::read(&mut cursor).unwrap();

        println!(
            "[NET/RECV] [client: {} on port: {} recv'd {} bytes from {:?}]",
            self.id,
            self.socket.local_addr().unwrap().port(),
            size,
            peer
        );

        let flags = packet.flags;
        println!(
            "[RECVLOOP] Processing packet with PacketHeaderFlags: {:?}",
            flags
        );

        if flags.contains(PacketHeaderFlags::CONNECT_REQUEST) {
            let mut cursor = Cursor::new(&buffer[..size]);
            let packet = ConnectRequestHeader::read(&mut cursor).unwrap();
            println!("        -> packet: {:?}", packet);

            // Store session data from ConnectRequest
            self.cookie = Some(packet.cookie);
            self.client_id = Some(packet.net_id as u16);
            self.outgoing_seed = Some(packet.outgoing_seed);
            self.incoming_seed = Some(packet.incoming_seed);

            println!(
                "[SESSION] Stored: Cookie={:016X}, ClientId={:04X}, OutgoingSeed={:08X}, IncomingSeed={:08X}",
                packet.cookie, packet.net_id, packet.outgoing_seed, packet.incoming_seed
            );

            let _ = self.do_connect_response().await;
        }

        if flags.contains(PacketHeaderFlags::ACK_SEQUENCE) {
            // Read the sequence number that the server is acknowledging
            let mut cursor = Cursor::new(&buffer[..size]);
            // Skip past the packet header (20 bytes) to read the payload
            cursor.set_position(20);
            let acked_seq = u32::read(&mut cursor).unwrap();

            println!(
                "[ACK_SEQUENCE] Server acknowledged our sequence: {}",
                acked_seq
            );
            // Store the last acked sequence - we don't send a response to this
            self.recv_count = acked_seq;
        }
    }
    // TODO: Should return a Result with a success or failure
    pub async fn connect(&mut self) -> Result<(), std::io::Error> {
        self.connect_state = ClientConnectState::Connecting;

        // TODO: Should handle this with pattern matching
        self.socket
            .connect(self.address.clone())
            .await
            .expect("connect failed");

        self.connect_state = ClientConnectState::Connected;
        let peer = self.socket.peer_addr().unwrap();
        println!("[Client::connect] Client connected to {:?}!", peer);

        Ok(())
    }

    pub async fn do_login(&mut self) -> Result<(), std::io::Error> {
        self.login_state = ClientLoginState::LoggingIn;

        let account = self.account.name.to_lowercase();
        let account_to_login_as = String::new();
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
            login_type: 0x00000002, // Password authentication
            unknown: 0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i32,
            account,
            account_to_login_as,
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

        // Build packet manually: header (20 bytes) + payload
        let mut buffer = Vec::with_capacity(20 + payload_size);

        // Write packet header (20 bytes)
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
        // 1. Set checksum field to 0xbadd70dd
        buffer[8..12].copy_from_slice(&0xbadd70ddu32.to_le_bytes());

        // 2. Checksum header (20 bytes) + payload
        let checksum1 = get_magic_number(&buffer[0..20], 20, true);
        let checksum2 = get_magic_number(&payload_buffer, payload_size, true);
        let checksum = checksum1.wrapping_add(checksum2);

        // 3. Write final checksum back
        buffer[8..12].copy_from_slice(&checksum.to_le_bytes());

        println!(
            "Sending LoginRequest data for account {}:{}",
            self.account.name, self.account.password
        );
        println!("          -> raw: {:02X?}", buffer);

        match self.socket.send(&buffer).await {
            Ok(n) => {
                println!(
                    "[NET/SEND] Sent {} bytes to {:?}",
                    n,
                    self.socket.peer_addr()?
                );
            }
            Err(e) => {
                eprintln!("[NET/SEND] ERROR: {}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    pub async fn do_connect_response(&mut self) -> Result<(), std::io::Error> {
        // Get the cookie from session state
        let cookie = self
            .cookie
            .expect("Cookie not set - ConnectRequest not received yet");

        // ConnectResponse payload: just a u64 cookie (8 bytes)
        let payload_size = 8;

        // Create C2SPacket with ConnectResponse
        let packet = C2SPacket {
            sequence: self.next_sequence(),
            flags: PacketHeaderFlags::CONNECT_RESPONSE,
            checksum: 0,
            recipient_id: 0,
            time_since_last_packet: 0,
            size: payload_size,
            iteration: 0,
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

        println!(
            "[NET/SEND] Sending ConnectResponse with data: {:2X?}",
            buffer
        );
        println!("           -> raw: {:02X?}", buffer);
        println!("           -> packet: {:?}", packet);

        match self.socket.send(&buffer).await {
            Ok(_) => {}
            Err(_) => panic!(),
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

        println!("[NET/SEND] Sending AckResponse with data: {:2X?}", buffer);

        match self.socket.send(&buffer).await {
            Ok(_) => {}
            Err(_) => panic!(),
        }
        Ok(())
    }
}

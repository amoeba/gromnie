use std::io::Cursor;
use std::net::SocketAddr;

use acprotocol::enums::{AuthFlags, PacketHeaderFlags};
use acprotocol::network::packet::PacketHeader;
use acprotocol::packets::c2s_packet::C2SPacket;
use acprotocol::readers::ACDataType;
use acprotocol::types::{
    ConnectRequestHeader, LoginRequestHeader, LoginRequestHeaderType2, WString,
};
use acprotocol::writers::ACWritable;
use tokio::net::UdpSocket;

use crate::crypto::magic_number::get_magic_number;

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

/// Helper function to serialize a C2SPacket and compute its checksum
fn serialize_c2s_packet(packet: &C2SPacket) -> Result<Vec<u8>, std::io::Error> {
    // Write packet to buffer with checksum=0
    let mut buffer = Vec::new();
    {
        let mut cursor = Cursor::new(&mut buffer);
        packet
            .write(&mut cursor)
            .map_err(|e| std::io::Error::other(format!("Write error: {}", e)))?;
    }

    // Compute checksum on the entire serialized packet
    let checksum = get_magic_number(&buffer, buffer.len(), true);

    // Write checksum back into buffer (checksum is at offset 8-11)
    if buffer.len() >= 12 {
        buffer[8..12].copy_from_slice(&checksum.to_le_bytes());
    }

    Ok(buffer)
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
        }
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
        println!("           -> raw: {:02X?}", &buffer[..size]);
        println!("           -> packet: {:?}", packet);

        let flags = packet.flags;
        println!(
            "[RECVLOOP] Processing packet with PacketHeaderFlags: {:?}",
            flags
        );

        if flags.contains(PacketHeaderFlags::CONNECT_REQUEST) {
            let mut cursor = Cursor::new(&buffer[..size]);
            let packet = ConnectRequestHeader::read(&mut cursor).unwrap();
            println!("        -> packet: {:?}", packet);

            let _ = self.do_connect_response(packet.cookie).await;
        }

        if flags.contains(PacketHeaderFlags::ACK_SEQUENCE) {
            println!("TODO: Send AckResponse");
            let _ = self.do_ack_response(0x02).await;
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

        // Compute the length field for LoginRequestHeaderType2
        // The length field contains the size of all data after client_version and length itself
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

        let account_to_login_as_size = {
            let str_len = account_to_login_as.len();
            let bytes_written = 2 + str_len;
            let padding = (4 - (bytes_written % 4)) % 4;
            bytes_written + padding
        };

        // For string32l: 4 (u32 length) + 1 (packed word if len <= 255) + string_len
        let password_size = {
            let str_len = password.len();
            let packed_word_size = if str_len > 255 { 2 } else { 1 };
            4 + packed_word_size + str_len
        };

        // Total length = auth_type (4) + flags (4) + sequence (4) + account + account_to_login_as + password
        let computed_length = 4 + 4 + 4 + account_size + account_to_login_as_size + password_size;

        // Create LoginRequestHeaderType2 for password authentication
        let login_header = LoginRequestHeaderType2 {
            client_version: "1802".to_string(),
            length: computed_length as u32,
            flags: AuthFlags::None,
            sequence: 0,
            account,
            account_to_login_as,
            password: WString(password),
        };

        // Pre-compute the payload size (excluding the 20-byte C2SPacket header)
        // LoginRequest payload: client_version string + length field + computed_length
        let client_version_size = {
            let str_len = "1802".len();
            let bytes_written = 2 + str_len;
            let padding = (4 - (bytes_written % 4)) % 4;
            bytes_written + padding
        };
        let login_payload_size = client_version_size + 4 + computed_length; // client_version + length field (u32) + rest

        // Create C2SPacket with LoginRequest
        let packet = C2SPacket {
            sequence: 0,
            flags: PacketHeaderFlags::LOGIN_REQUEST,
            checksum: 0, // Will compute this below
            recipient_id: 0,
            time_since_last_packet: 0,
            size: login_payload_size as u16, // Size field = payload size only (not including header)
            iteration: 0,
            server_switch: None,
            retransmit_sequences: None,
            reject_sequences: None,
            ack_sequence: None,
            login_request: Some(LoginRequestHeader::Type2(login_header)),
            world_login_request: None,
            connect_response: None,
            cicmd_command: None,
            time: None,
            echo_time: None,
            flow: None,
            fragments: None,
        };

        println!(
            "DEBUG: packet flags = {:?} (bits: 0x{:x})",
            packet.flags,
            packet.flags.bits()
        );
        println!(
            "DEBUG: computed size = {} (payload only), computed length = {}",
            login_payload_size, computed_length
        );

        // Serialize packet with checksum
        let buffer = serialize_c2s_packet(&packet)?;

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

    pub async fn do_connect_response(&mut self, cookie: u64) -> Result<(), std::io::Error> {
        // ConnectResponse payload: just a u64 cookie (8 bytes)
        let payload_size = 8;

        // Create C2SPacket with ConnectResponse
        let packet = C2SPacket {
            sequence: 0,
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
        let buffer = serialize_c2s_packet(&packet)?;

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
            sequence: 0,
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
        let buffer = serialize_c2s_packet(&packet)?;

        println!("[NET/SEND] Sending AckResponse with data: {:2X?}", buffer);
        println!("           -> raw: {:02X?}", buffer);
        println!("           -> packet: {:?}", packet);

        match self.socket.send(&buffer).await {
            Ok(_) => {}
            Err(_) => panic!(),
        }
        Ok(())
    }
}

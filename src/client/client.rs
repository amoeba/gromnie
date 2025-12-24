use std::io::Cursor;
use std::net::SocketAddr;

use deku::prelude::*;
use tokio::net::UdpSocket;

use crate::net::packet::PacketHeaderFlags;
use crate::net::packets::connect_request::ConnectRequestHeader;
use crate::net::transit_header::TransitHeader;

// ClientConnectState
// TODO: Put this somewhere else
#[derive(Clone, Debug, PartialEq)]
pub enum ClientConnectState {
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
        let packet = TransitHeader::parse(&mut cursor).unwrap();

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
            let packet = ConnectRequestHeader::from_bytes((&buffer[..size], size)).unwrap();
            println!("        -> packet: {:?}", packet.1);

            let _ = self.do_connect_response(packet.1.cookie).await;
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
        println!("[Client::connect] Client connected!");

        Ok(())
    }

    pub async fn do_login(&mut self) -> Result<(), std::io::Error> {
        use acprotocol::enums::{AuthFlags, PacketHeaderFlags};
        use acprotocol::packets::c2s_packet::C2SPacket;
        use acprotocol::types::{LoginRequestHeader, LoginRequestHeaderType2, WString};
        use acprotocol::writers::ACWritable;

        self.login_state = ClientLoginState::LoggingIn;

        // Create LoginRequestHeaderType2 for password authentication
        let login_header = LoginRequestHeaderType2 {
            client_version: "1802".to_string(),
            length: 0, // Will be computed during serialization
            flags: AuthFlags::None,
            sequence: 0,
            account: self.account.name.to_lowercase(),
            account_to_login_as: String::new(),
            password: WString(self.account.password.clone()),
        };

        // Create C2SPacket with LoginRequest
        let packet = C2SPacket {
            sequence: 0,
            flags: PacketHeaderFlags::LOGIN_REQUEST,
            checksum: 0, // Will need to compute this
            recipient_id: 0,
            time_since_last_packet: 0,
            size: 0, // Will be set after writing
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

        // Write packet to buffer
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);
        packet.write(&mut cursor).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Write error: {}", e))
        })?;

        println!(
            "Sending LoginRequest data for account {}:{}",
            self.account.name, self.account.password
        );
        println!("          -> raw: {:02X?}", buffer);

        match self.socket.send(&buffer).await {
            Ok(_) => {}
            Err(_) => panic!(),
        }

        Ok(())
    }

    pub async fn do_connect_response(&mut self, cookie: u64) -> Result<(), std::io::Error> {
        use acprotocol::enums::PacketHeaderFlags;
        use acprotocol::packets::c2s_packet::C2SPacket;
        use acprotocol::writers::ACWritable;

        // Create C2SPacket with ConnectResponse
        let packet = C2SPacket {
            sequence: 0,
            flags: PacketHeaderFlags::CONNECT_RESPONSE,
            checksum: 0, // Will need to compute this
            recipient_id: 0,
            time_since_last_packet: 0,
            size: 0, // Will be set after writing
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

        // Write packet to buffer
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);
        packet.write(&mut cursor).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Write error: {}", e))
        })?;

        println!(
            "[NET/SEND] Sending ConnectResponse with data: {:2X?}",
            buffer
        );
        println!("           -> raw: {:02X?}", buffer);
        println!("           -> packet: {:?}", packet);
        // TODO: Handle here with match
        match self.socket.send(&buffer).await {
            Ok(_) => {}
            Err(_) => panic!(),
        }
        Ok(())
    }

    pub async fn do_ack_response(&mut self, value: u32) -> Result<(), std::io::Error> {
        use acprotocol::enums::PacketHeaderFlags;
        use acprotocol::packets::c2s_packet::C2SPacket;
        use acprotocol::writers::ACWritable;

        // Create C2SPacket with AckSequence
        let packet = C2SPacket {
            sequence: 0,
            flags: PacketHeaderFlags::ACK_SEQUENCE,
            checksum: 0, // Will need to compute this
            recipient_id: 0,
            time_since_last_packet: 0,
            size: 0, // Will be set after writing
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

        // Write packet to buffer
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);
        packet.write(&mut cursor).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Write error: {}", e))
        })?;

        println!(
            "[NET/SEND] Sending AckResponse with data: {:2X?}",
            buffer
        );
        println!("           -> raw: {:02X?}", buffer);
        println!("           -> packet: {:?}", packet);
        // TODO: Handle here with match
        match self.socket.send(&buffer).await {
            Ok(_) => {}
            Err(_) => panic!(),
        }
        Ok(())
    }
}

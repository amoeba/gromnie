use std::io::{self, Cursor, Seek};

use deku::prelude::*;
use std::string::ToString;
use strum_macros::Display;
use tokio::net::UdpSocket;

use crate::messages::{login_request::login_request, packet::{ConnectRequestHeader, S2CPacket}};

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
}

impl Client {
    pub async fn create(id: u32, address: String, name: String, password: String) -> Client {
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
        self.login_state = ClientLoginState::LoggingIn;

        // TODO: Wrap this up in a nicer way
        let mut buffer: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        login_request(&mut buffer, &self.account.name, &self.account.password);
        let serialized_data: Vec<u8> = buffer.into_inner();

        // TODO: Handle here with match
        self.socket.send(&serialized_data).await;

        Ok(())
    }

    // Response... which packet is this?
    // 0000   02 00 00 00 45 00 00 50 cc c2 00 00 40 11 00 00
    // 0010   7f 00 00 01 7f 00 00 01 23 28 c9 10 00 3c fe 4f
    // 0020   00 00 00 00 00 00 04 00 0a b9 8c 2c 0b 00 4a 88
    // 0030   20 00 01 00 ad aa f2 94 10 a7 aa 41 f9 93 86 68
    // 0040   82 d4 43 a8 00 00 00 00 75 05 81 e9 55 88 43 18
    // 0050   00 00 00 00

    // ame="S2CPacket" text="Server to Client AC packet.">
    // 		<field type="uint" name="Sequence" text="Packet Sequence / Order" />
    // 		<field type="PacketHeaderFlags" name="Flags" text="Flags that dictate the content / purpose of this packet" />
    // 		<field type="uint" name="Checksum" text="Packet Checksum" />
    // 		<field type="ushort" name="RecipientId" />
    // 		<field type="ushort" name="TimeSinceLastPacket" />
    // 		<field type="ushort" name="Size" text="Packet length, excluding this header" />
    // 		<field type="ushort" name="Iteration" />

    // PacketHeaderFlags
    // <value value="0x00080000" name="ConnectResponse" />

    // <type name="ConnectRequestHeader" proto="true" text="Optional header data when PacketHeaderFlags includes ConnectRequest">
    // <field name="ServerTime" type="double" />
    // <field name="Cookie" type="ulong" />
    // <field name="NetID" type="int" />
    // <field name="OutgoingSeed" type="uint" />
    // <field name="IncomingSeed" type="uint" />
    // <field name="Unknown" type="DWORD" />
    // </type>
}

// TODO: this is a total hack but it looks like it works. Can we wrap this up
// better?
pub fn parse_response(buffer: &[u8]) {
    let mut cursor = Cursor::new(&buffer);

    // Temporarily: Handle this tolerantly as we figure out the protocol
    let result = S2CPacket::from_bytes((&cursor.get_ref(), 0));

    let hdr = match result {
        Ok(val) => val,
        Err(e) => {
            println!("[WARN] {}", e);
            return;
        }
    };
    println!("[parse_response/header] {:?}", hdr.1);

    // Skip to remainder
    cursor.seek(io::SeekFrom::Start(hdr.1.size as u64));

    let data: ((&[u8], usize), ConnectRequestHeader) =
        ConnectRequestHeader::from_bytes((&cursor.get_ref(), 32)).unwrap();

    println!("[parse_response/data] {:?}", data.1);
}

use std::io::Cursor;

use tokio::net::UdpSocket;

use crate::net::packets::login_request::LoginRequestPacket;
use crate::net::packets::connect_response::ConnectResponsePacket;

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
        let mut packet = LoginRequestPacket::new(&self.account.name, &self.account.password);
        packet.serialize(&mut buffer);
        let serialized_data: Vec<u8> = buffer.into_inner();

        // TODO: Handle here with match
        println!("Sending LoginRequest data for account {}:{}", self.account.name, self.account.password);
        println!("          -> raw: {:02X?}", serialized_data);

        match self.socket.send(&serialized_data).await {
            Ok(_) => {},
            Err(_) => panic!(),
        }

        Ok(())
    }

    pub async fn do_connect_response(&mut self, cookie: u64) -> Result<(), std::io::Error> {
        // TODO: Wrap this up in a nicer way
        let mut buffer: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        let mut packet = ConnectResponsePacket::new(cookie);
        packet.serialize(&mut buffer);
        let serialized_data: Vec<u8> = buffer.into_inner();

        println!("[NET/SEND] Sending ConnectResponse with data: {:2X?}", serialized_data);
        println!("           -> raw: {:02X?}", serialized_data);
        println!("           -> packet: {:?}", packet);
        // TODO: Handle here with match
        match self.socket.send(&serialized_data).await {
            Ok(_) => {},
            Err(_) => panic!(),
        }
        Ok(())
    }
}

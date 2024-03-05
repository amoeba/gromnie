
use std::{io::Cursor, net::UdpSocket};

use libgromnie::on_serialize;

struct Account {
  name: String,
  password: String
}

// TODO: Don't require both bind_address and connect_address. I had to do this
// to get things to work but I should be able to listen on any random port so
// I'm not sure what I'm doing wrong
pub struct Client {
  bind_address: String,
  connect_address: String,
  socket: Option<UdpSocket>,
  account: Account,
}

impl Client {
  pub fn create(bind_address: String, connect_address: String, account_name: String, password: String) -> Client {
    Client {
      bind_address, connect_address, account: Account { name: account_name, password: password }, socket: None
    }
  }

  pub fn connect(&self) -> Result<usize, std::io::Error> {
    let socket: UdpSocket = UdpSocket::bind(self.bind_address.clone()).expect("Failed to bind");
    let _ = socket.connect(self.connect_address.clone());

    let mut buffer = Cursor::new(Vec::new());
    on_serialize(&mut buffer);

    let serialized_data: Vec<u8> = buffer.into_inner();
    let _ = socket.send(&serialized_data).unwrap();

    let mut recv_buffer = [0u8; 1024];

    return socket.recv(&mut recv_buffer);
  }
}

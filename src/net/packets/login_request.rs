use std::io::{Write, Seek};

use crate::net::packet::{Packet};

#[derive(Debug, PartialEq)]
pub struct LoginRequestPacket {
  pub packet: Packet,
  pub protocol_version: String,
  pub account_name: String,
  pub password: String,
}

impl LoginRequestPacket {
  pub fn new(account_name: &str, password: &str) -> LoginRequestPacket {
    LoginRequestPacket {
      packet: Packet::new(),
      protocol_version: "1802".to_owned(),
      account_name: account_name.to_lowercase().to_owned(),
      password: password.to_owned()
    }
  }
}

impl LoginRequestPacket {
  pub fn serialize<W: Write + Seek>(&mut self, writer: &mut W) {
    // Calculate lengths ahead of time
    let account_len: u16 = (self.account_name.len() + self.password.len() + 1) as u16;
    let remaining: u32 = (account_len as u32) + 24; // +24 comes from: u32 + u32 + u32 + u16 + 10
    let packet_len: u16 = 8 + (remaining as u16) + account_len - 24;

    // Sequence
    writer.write(&0x0u32.to_le_bytes()).unwrap();

    // Flags
    writer.write(&0x00010000u32.to_le_bytes()).unwrap();

    // Checksum
    writer.write(&0x05d00093u32.to_le_bytes()).unwrap();

    // RecipientId
    writer.write(&0x0u16.to_le_bytes()).unwrap();

    // TimeSinceLastPacket
    writer.write(&0x0u16.to_le_bytes()).unwrap();

    // Size
    writer.write(&packet_len.to_le_bytes()).unwrap();

    // Iteration
    writer.write(&0x0u16.to_le_bytes()).unwrap();

    // Begin LoginRequest
    // ClientVersion
    writer.write(&0x04u16.to_le_bytes()).unwrap();
    let client_version: [u8; 6] = [0x31, 0x38, 0x30, 0x32, 0x00, 0x00];
    writer.write(&client_version).unwrap();

    // Length
    writer.write(&remaining.to_le_bytes()).unwrap();

    // AuthType
    writer.write(&0x01u32.to_le_bytes()).unwrap();

    // Flags
    writer.write(&0x0u32.to_le_bytes()).unwrap();

    // Sequence
    writer.write(&0x58a8b83eu32.to_le_bytes()).unwrap();

    // Account
    let mut token: String = self.account_name.to_owned();
    token.push_str(":");
    token.push_str(&self.password.to_owned());

    writer.write(&(token.len() as u16).to_le_bytes()).unwrap();
    writer.write(&token.as_bytes()).unwrap();

    // TODO: Not sure about the remainder but this works for now
    writer
        .write(vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0].as_ref())
        .unwrap();
  }
}

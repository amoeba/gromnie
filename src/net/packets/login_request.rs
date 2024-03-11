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

    // sequence
    writer.write(&0x0u32.to_le_bytes()).unwrap();

    // packetheaderflags
    writer.write(&0x00010000u32.to_le_bytes()).unwrap();

    // checksum
    writer.write(&0x05d00093u32.to_le_bytes()).unwrap();

    // recipient
    writer.write(&0x0u16.to_le_bytes()).unwrap();

    // timesincelastpacket
    writer.write(&0x0u16.to_le_bytes()).unwrap();

    // size
    writer.write(&packet_len.to_le_bytes()).unwrap();

    // iteration
    writer.write(&0x0u16.to_le_bytes()).unwrap();

    // ClientVersion = packet.DataReader.ReadString16L();      // should be "1802" for end of retail client
    writer.write(&0x04u16.to_le_bytes()).unwrap();
    let client_version: [u8; 6] = [0x31, 0x38, 0x30, 0x32, 0x00, 0x00];
    writer.write(&client_version).unwrap();

    // uint len = packet.DataReader.ReadUInt32();                     // data length left in packet including ticket
    writer.write(&remaining.to_le_bytes()).unwrap();

    // NetAuthType = (NetAuthType)packet.DataReader.ReadUInt32();
    writer.write(&0x01u32.to_le_bytes()).unwrap();

    // var authFlags = (AuthFlags)packet.DataReader.ReadUInt32();
    writer.write(&0x0u32.to_le_bytes()).unwrap();

    // Timestamp = packet.DataReader.ReadUInt32();                    // sequence
    writer.write(&0x58a8b83eu32.to_le_bytes()).unwrap();

    // TODO: Encapsulte string writing better, but here it is
    let mut token: String = self.account_name.to_owned();
    token.push_str(":");
    token.push_str(&self.password.to_owned());

    writer.write(&(token.len() as u16).to_le_bytes()).unwrap();
    writer.write(&token.as_bytes()).unwrap();

    writer
        .write(vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0].as_ref())
        .unwrap();
  }
}

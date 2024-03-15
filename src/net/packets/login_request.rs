use std::{io::{Seek, Write}, mem};

use deku::DekuContainerWrite;

use crate::net::{packet::{Packet, PacketHeaderFlags}, transit_header::TransitHeader};

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
    println!("LoginRequestPacket: serialize");

    // Seek to just after TransitHeader
    println!("Seeking to beyond TransitHeader...");
    let offset = mem::size_of::<TransitHeader>() as u64;
    writer.seek(std::io::SeekFrom::Start(offset)).unwrap();
    println!("Seeked to {}", writer.stream_position().unwrap());

    // Calculate lengths ahead of time
    let account_len: u16 = (self.account_name.len() + self.password.len() + 1) as u16;
    let remaining: u32 = (account_len as u32) + 24; // +24 comes from: u32 + u32 + u32 + u16 + 10
    let packet_len: u16 = 8 + (remaining as u16) + account_len - 24;

    // Begin LoginRequest

    // ClientVersion
    writer.write(&0x04u16.to_le_bytes()).unwrap();
    let client_version: [u8; 4] = [0x31, 0x38, 0x30, 0x32];
    writer.write(&client_version).unwrap();

    // Align to byte boundary, in this case this doesn't change
    writer.write(&0x0u16.to_le_bytes()).unwrap();

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

    // Align to byte boundary since we just wrote a string
    let pos = writer.stream_position().unwrap() as i64;
    writer.seek(std::io::SeekFrom::Current(pos % 4)).unwrap();

    // TODO: Not sure about the remainder but this works for now
    writer
        .write(vec![0, 0, 0, 0, 0, 0, 0, 0].as_ref())
        .unwrap();

    let bytes_written = writer.stream_position().unwrap() - offset;
    println!("Wrote {} bytes of packet data", bytes_written);

    // WIP: Set size now, but see how actestclient does it
    println!("stream position is {}", writer.stream_position().unwrap());
    println!("size of transit header is {}", mem::size_of::<TransitHeader>());

    // WIP: Set header size as a side-effect
    self.packet.header.size = (writer.stream_position().unwrap() - mem::size_of::<TransitHeader>() as u64) as u16;

    self.packet.serialize(writer, bytes_written);

  }
}

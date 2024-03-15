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

    // Calculate lengths and paddings ahead of time
    let mut packet_len = 20;

    let mut username_pad = (self.account_name.len() + 2) % 4;

    if username_pad > 0 {
      username_pad = 4 - username_pad;
    }

    let mut password_pad = (self.password.len() + 5) % 4;

    if password_pad > 0 {
      password_pad = 4 - password_pad;
    }

    packet_len += self.account_name.len() + 2 + username_pad;
    packet_len += self.password.len() + 5 + password_pad;

    // Begin LoginRequest

    // ClientVersion
    writer.write(&0x04u16.to_le_bytes()).unwrap();
    let client_version: [u8; 4] = [0x31, 0x38, 0x30, 0x32];
    writer.write(&client_version).unwrap();

    // Align to byte boundary, in this case this doesn't change so write two
    // bytes (6 % 4 = 2)
    writer.write(&0x0u16.to_le_bytes()).unwrap();

    // Length
    writer.write(&(packet_len as u32).to_le_bytes()).unwrap();

    // AuthType
    writer.write(&0x00000002u32.to_le_bytes()).unwrap();

    // Flags
    writer.write(&0x0u32.to_le_bytes()).unwrap();

    // Sequence
    writer.write(&0x58a8b83eu32.to_le_bytes()).unwrap();

    // Account
    writer.write(&(self.account_name.len() as u16).to_le_bytes()).unwrap();
    writer.write(&self.account_name.as_bytes()).unwrap();
    writer.seek(std::io::SeekFrom::Current(username_pad as i64)).unwrap();

    // AccountToLoginAs (admin only)
    writer.write(&0x0u32.to_le_bytes()).unwrap();

    // Password
    let password_length = self.password.len() as u32;
    writer.write(&(password_length + 1).to_le_bytes()).unwrap();
    writer.write(&(password_length as u8).to_le_bytes()).unwrap();
    writer.write(&self.password.as_bytes()).unwrap();
    writer.seek(std::io::SeekFrom::Current(password_pad as i64)).unwrap();

    // Debug
    let bytes_written = writer.stream_position().unwrap() - offset;
    println!("Wrote {} bytes of packet data", bytes_written);
    println!("stream position is {}", writer.stream_position().unwrap());
    println!("size of transit header is {}", mem::size_of::<TransitHeader>());
    // End Debug

    // WIP: Set header size as a side-effect
    self.packet.header.size = (writer.stream_position().unwrap() - mem::size_of::<TransitHeader>() as u64) as u16;

    self.packet.serialize(writer, bytes_written);

  }
}

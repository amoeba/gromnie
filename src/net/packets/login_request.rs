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
    println!("LoginRequestPacket.serialize");
    self.packet.serialize(writer);

    // sequence
    writer.write(&0x0u32.to_le_bytes()).unwrap();

    // packetheaderflags
    writer.write(&0x00010000u32.to_le_bytes()).unwrap();

    // checksum (packet checksum)
    // TODO: Need to make this dynamic so we can use any login
    // ACE has this code:
    //  uint fragmentChecksum = Hash32.Calculate(buffer, buffer.Length) + Hash32.Calculate(Data, Data.Length);
    writer.write(&0x05d00093u32.to_le_bytes()).unwrap();

    // recipient
    writer.write(&0x0u16.to_le_bytes()).unwrap();

    // timesincelastpacket
    writer.write(&0x0u16.to_le_bytes()).unwrap();

    // Various ahead of time length calculations
    // TODO: This might be a little weird right now
    let account_len: u16 = (self.account_name.len() + self.password.len() + 1) as u16;
    let remaining: u32 = (account_len as u32) + 24; // +24 comes from: u32 + u32 + u32 + u16 + 10
    let packet_len: u16 = 8 + (remaining as u16) + account_len - 24;

    // size
    writer.write(&packet_len.to_le_bytes()).unwrap();

    // iteration
    writer.write(&0x0u16.to_le_bytes()).unwrap();

    // ClientVersion
    writer.write(&0x04u16.to_le_bytes()).unwrap();
    let client_version: [u8; 6] = [0x31, 0x38, 0x30, 0x32, 0x00, 0x00];
    writer.write(&client_version).unwrap();

    // Length
    writer.write(&remaining.to_le_bytes()).unwrap();

    // AuthType
    // TODO: This is hardcoded as Password and GlsTicket mode isn't supported
    writer.write(&0x01u32.to_le_bytes()).unwrap();

    // Flags
    writer.write(&0x0u32.to_le_bytes()).unwrap();

    // Sequence
    // TODO: I think this is supposed to be a timestamp
    writer.write(&0x58a8b83eu32.to_le_bytes()).unwrap();

    // Account
    // AccountToLoginAs (admin only)
    // AuthType.Password | AuthType.GlsTicket
    // writer.write(&self.account_len.to_le_bytes()).unwrap();
    writer.write(&self.account_name.as_bytes()).unwrap();
    writer.write(&":".as_bytes()).unwrap();
    writer.write(&self.password.as_bytes()).unwrap();

    // TODO: Not sure what this extra is yet
    writer
        .write(vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0].as_ref())
        .unwrap();
  }
}

use std::{
    io::{Cursor, Seek, Write},
    mem,
};

use crate::net::{
    packet::{Packet, PacketHeaderFlags},
    transit_header::TransitHeader,
};

#[derive(Debug)]
pub struct LoginRequestPacket {
    pub packet: Packet,
    pub protocol_version: String,
    pub account_name: String,
    pub password: String,
}

impl LoginRequestPacket {
    pub fn new(account_name: &str, password: &str) -> LoginRequestPacket {
        LoginRequestPacket {
            packet: Packet::new(PacketHeaderFlags::LOGIN_REQUEST.bits()),
            protocol_version: "1802".to_owned(),
            account_name: account_name.to_lowercase().to_owned(),
            password: password.to_owned(),
        }
    }
}

impl LoginRequestPacket {
    pub fn serialize(&mut self, writer: &mut Cursor<Vec<u8>>) {
        // Seek to just after TransitHeader
        let offset = mem::size_of::<TransitHeader>() as u64;
        writer.seek(std::io::SeekFrom::Start(offset)).unwrap();

        // Calculate lengths and paddings ahead of time
        let mut username_pad = (self.account_name.len() + 2) % 4;

        if username_pad > 0 {
            username_pad = 4 - username_pad;
        }

        let packet_len = 20 +
      self.account_name.len() +
      2 + // len (short)
      username_pad +
      self.password.len() +
      1; // len (byte)

        // Begin LoginRequest fields

        // ClientVersion
        writer.write_all(&0x04u16.to_le_bytes()).unwrap();
        let client_version: [u8; 4] = [0x31, 0x38, 0x30, 0x32];
        writer.write_all(&client_version).unwrap();

        // Align to byte boundary, in this case this doesn't change so write two
        // bytes (6 % 4 = 2)
        writer.write_all(&0x0u16.to_le_bytes()).unwrap();

        // Length
        writer
            .write_all(&(packet_len as u32).to_le_bytes())
            .unwrap();

        // AuthType
        writer.write_all(&0x00000002u32.to_le_bytes()).unwrap();

        // Flags
        writer.write_all(&0x0u32.to_le_bytes()).unwrap();

        // Sequence
        writer.write_all(&0x0u32.to_le_bytes()).unwrap();

        // Account
        writer
            .write_all(&(self.account_name.len() as u16).to_le_bytes())
            .unwrap();
        writer
            .write_all(self.account_name.as_bytes())
            .unwrap();
        writer
            .seek(std::io::SeekFrom::Current(username_pad as i64))
            .unwrap();

        // AccountToLoginAs (admin only)
        // This seems to always be zeroes
        writer.write_all(&0x0u32.to_le_bytes()).unwrap();

        // Password
        let password_length = self.password.len() as u32;
        writer
            .write_all(&(password_length + 1).to_le_bytes())
            .unwrap();
        writer
            .write_all(&(password_length as u8).to_le_bytes())
            .unwrap();
        writer.write_all(self.password.as_bytes()).unwrap();
        // WIP: Skipping adding a pad here because you don't see the retail client
        // do it
        // writer.seek(std::io::SeekFrom::Current(password_pad as i64)).unwrap();

        let bytes_written = writer.stream_position().unwrap() - offset;

        self.packet.serialize(writer, bytes_written);
    }
}

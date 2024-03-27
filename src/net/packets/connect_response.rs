use std::{io::{Cursor, Seek, Write}, mem};

use crate::net::{packet::{Packet, PacketHeaderFlags}, transit_header::TransitHeader};

// TODO: Not right yet
#[derive(Debug, PartialEq)]
pub struct ConnectResponseBody {
    cookie: u64,
}


#[derive(Debug, PartialEq)]
pub struct ConnectResponsePacket {
  pub packet: Packet,
  body: ConnectResponseBody,
}

impl ConnectResponsePacket {
  pub fn new(cookie: u64) -> ConnectResponsePacket {
    ConnectResponsePacket {
      packet: Packet::new(PacketHeaderFlags::ConnectResponse.as_u32()),
      body: ConnectResponseBody {cookie: cookie}
    }
  }
}

impl ConnectResponsePacket {
  pub fn serialize(&mut self, writer: &mut Cursor<Vec<u8>>) {
    let offset = mem::size_of::<TransitHeader>() as u64;
    writer.seek(std::io::SeekFrom::Start(offset)).unwrap();

    writer.write(&self.body.cookie.to_le_bytes()).unwrap();

    let bytes_written = writer.stream_position().unwrap() - offset;
    self.packet.serialize(writer, bytes_written);
  }
}

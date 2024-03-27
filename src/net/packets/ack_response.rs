use std::{io::{Cursor, Seek, Write}, mem};

use crate::net::{packet::{Packet, PacketHeaderFlags}, transit_header::TransitHeader};

// TODO: Not right yet
#[derive(Debug, PartialEq)]
pub struct AckResponseBody {
  value: u32
}

#[derive(Debug, PartialEq)]
pub struct AckResponsePacket {
  pub packet: Packet,
  body: AckResponseBody,
}

impl AckResponsePacket {
  pub fn new(value: u32) -> AckResponsePacket {
    AckResponsePacket {
      packet: Packet::new(PacketHeaderFlags::ConnectResponse.as_u32()),
      body: AckResponseBody {value: value}
    }
  }
}

impl AckResponsePacket {
  pub fn serialize(&mut self, writer: &mut Cursor<Vec<u8>>) {
    let offset = mem::size_of::<TransitHeader>() as u64;
    writer.seek(std::io::SeekFrom::Start(offset)).unwrap();

    writer.write(&self.body.value.to_le_bytes()).unwrap();

    let bytes_written = writer.stream_position().unwrap() - offset;
    self.packet.serialize(writer, bytes_written);
  }
}

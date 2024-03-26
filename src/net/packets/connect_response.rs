use std::{io::{Cursor, Seek, Write}, mem};

use crate::net::{packet::{Packet, PacketHeaderFlags}, transit_header::TransitHeader};

#[derive(Debug, PartialEq)]
pub struct ConnectResponsePacket {
  pub packet: Packet,
}

impl ConnectResponsePacket {
  pub fn new() -> ConnectResponsePacket {
    ConnectResponsePacket {
      packet: Packet::new(PacketHeaderFlags::ConnectResponse.as_u32()),
    }
  }
}

impl ConnectResponsePacket {
  pub fn serialize(&mut self, writer: &mut Cursor<Vec<u8>>) {
    println!("TODO: ConnectResponsePacket: serialize");

    // TODO
    // writer.write(TODO).unwrap();

    self.packet.serialize(writer, 0);
  }
}

use std::{
    io::{Cursor, Seek, Write},
    mem,
};

use super::transit_header::{PacketHeaderExt, TransitHeader};
use crate::crypto::magic_number::get_magic_number;
pub use acprotocol::network::packet::PacketHeaderFlags;

#[derive(Debug)]
pub struct Packet {
    pub header: TransitHeader,
    option_size: i32,
    sequence_ack: u32,
    connect_token: i64,
    timestamp: i64,
}

impl Packet {
    pub fn new(flags: u32) -> Packet {
        Packet {
            header: TransitHeader::new(flags),
            option_size: 0,
            sequence_ack: 0,
            connect_token: 0,
            timestamp: 0,
        }
    }

    pub fn set_ack(&mut self, sequence: u32) {
        self.sequence_ack = sequence;
        self.option_size += 4;

        // TODO: Update flags
        // Header.Flags |= PacketFlags.AckSequence;
        self.header.flags |= PacketHeaderFlags::ACK_SEQUENCE;
    }

    pub fn set_token(&mut self, token: i64, world: Option<bool>) {
        self.connect_token = token;
        self.option_size += 8;

        // TODO: Update flags
        // Header.Flags |= world ? PacketFlags.LoginWorld : PacketFlags.ConnectResponse;
        match world {
            Some(_world) => {
                self.header.flags |= PacketHeaderFlags::WORLD_LOGIN_REQUEST;
            }
            None => {
                self.header.flags |= PacketHeaderFlags::CONNECT_RESPONSE;
            }
        }
    }

    pub fn set_timestamp(&mut self, timestamp: i64) {
        self.timestamp = timestamp;
        self.option_size += 8;

        // TODO: Update flags
        // Header.Flags |= PacketFlags.TimeSync;
        self.header.flags |= PacketHeaderFlags::TIME_SYNC;
    }

    // This hashes a buffer containing [empty transitheader, packet data]
    pub fn compute_checksum(&mut self, buffer: &[u8]) -> u32 {
        // TODO: Pass in include_size or determine whether to set it
        get_magic_number(buffer, buffer.len(), true)
    }

    // VERY WIP
    // TODO: Can I avoid passing a mut Write+Seek?
    pub fn set_checksum(&mut self, writer: &Cursor<Vec<u8>>) {
        // Create a copy of the buffer
        // TODO: Not sure if I want clone here
        let buffer_copy = writer.clone().into_inner();
        let body = &buffer_copy[(mem::size_of::<TransitHeader>())..buffer_copy.len()];

        let header_checksum = self.header.compute_checksum();
        let body_checksum = self.compute_checksum(body);

        // TODO: Eventually needs to include the ISAAC XOR
        //self.checksum = header_checksum + (body_checksum ^ issac_xor);
        self.header.checksum = header_checksum.wrapping_add(body_checksum);
    }

    pub fn serialize(&mut self, writer: &mut Cursor<Vec<u8>>, size: u64) {
        // Set size
        self.header.size = size as u16;

        // We can set the checksum now
        self.set_checksum(writer);

        writer.seek(std::io::SeekFrom::Start(0)).unwrap();

        writer
            .write_all(&self.header.to_bytes().unwrap())
            .unwrap();
    }
}

// PacketHeaderFlags is now provided by acprotocol

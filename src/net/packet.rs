use std::{io::{Cursor, Seek, Write}, mem};

use deku::prelude::*;
use bitflags::bitflags;

use crate::crypto::magic_number::get_magic_number;

use super::transit_header::TransitHeader;

#[derive(Debug, PartialEq)]
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
        self.header.flags |= PacketHeaderFlags::AckSequence.as_u32();
    }

    pub fn set_token(&mut self, token: i64, world: Option<bool>)
    {
        self.connect_token = token;
        self.option_size += 8;

        // TODO: Update flags
        // Header.Flags |= world ? PacketFlags.LoginWorld : PacketFlags.ConnectResponse;
        match world {
            Some(_world) => {
                self.header.flags |= PacketHeaderFlags::WorldLoginRequest.as_u32();
            }
            None => {
                self.header.flags |= PacketHeaderFlags::ConnectResponse.as_u32();

            }
        }
    }

    pub fn set_timestamp(&mut self, timestamp: i64)
    {
        self.timestamp = timestamp;
        self.option_size += 8;

        // TODO: Update flags
        // Header.Flags |= PacketFlags.TimeSync;
        self.header.flags |= PacketHeaderFlags::TimeSync.as_u32();
    }

    // This hashes a buffer containing [empty transitheader, packet data]
    pub fn compute_checksum(&mut self, buffer: &[u8]) -> u32{
        // TODO: Pass in include_size or determine whether to set it
        return get_magic_number(buffer, buffer.len(), true);
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

        println!("header_checksum is 0x{:02x?}", header_checksum);
        println!("body_checksum is 0x{:02x?}", body_checksum);
        println!("combined checksum is 0x{:02x?}", header_checksum.wrapping_add(body_checksum));

        // TODO: Eventually needs to include the ISAAC XOR
        //self.checksum = header_checksum + (body_checksum ^ issac_xor);
        self.header.checksum = header_checksum.wrapping_add(body_checksum);
    }

    pub fn serialize(&mut self, writer: &mut Cursor<Vec<u8>>, size: u64) {
        // Once we're called, we have a writer that has the packet data in it
        // with no header information (yet)
        println!("Packet.serialize(), size is {} bytes", size);

        // Set size
        self.header.size = size as u16;

        // We can set the checksum now
        self.set_checksum(writer);

        println!("Jumping to start of stream");
        writer.seek(std::io::SeekFrom::Start(0)).unwrap();

        println!("Writing TransitHeader");
        writer.write(&self.header.to_bytes().unwrap()).unwrap();
    }
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct ConnectRequestHeader {
    server_time: f64,
    pub cookie: u8,
    net_id: i32,
    outgoing_seed: u32,
    unknown: u32,
}

// TODO: Probably remove this. We need to use these values as bitflags but
// Rust enums can't be used that way. The recommended way to do bitflags in Rust
// is to use the bitflags crate. Unfortunately, I couldn't figure out how to
// get the bitflags! macro to work with Deku. It almost works except the deku
// macro can't be resolved inside the bitflags! macro.
//
// #[derive(Debug, PartialEq, DekuRead, DekuWrite, Display)]
// #[deku(type = "u32", endian = "endian", ctx = "endian: deku::ctx::Endian")]
// pub enum PacketHeaderFlags {
//     #[deku(id = "0x00000000")]
//     None,
//     #[deku(id = "0x00000001")]
//     Retransmission,
//     #[deku(id = "0x00000002")]
//     EncryptedChecksum,
//     #[deku(id = "0x00000004")]
//     BlobFragments,
//     #[deku(id = "0x00000100")]
//     ServerSwitch,
//     #[deku(id = "0x00000200")]
//     LogonServerAddr,
//     #[deku(id = "0x00000400")]
//     EmptyHeader1,
//     #[deku(id = "0x00000800")]
//     Referral,
//     #[deku(id = "0x00001000")]
//     RequestRetransmit,
//     #[deku(id = "0x00002000")]
//     RejectRetransmit,
//     #[deku(id = "0x00004000")]
//     AckSequence,
//     #[deku(id = "0x00008000")]
//     Disconnect,
//     #[deku(id = "0x00010000")]
//     LoginRequest,
//     #[deku(id = "0x00020000")]
//     WorldLoginRequest,
//     #[deku(id = "0x00040000")]
//     ConnectRequest,
//     #[deku(id = "0x00080000")]
//     ConnectResponse,
//     #[deku(id = "0x00100000")]
//     NetError,
//     #[deku(id = "0x00200000")]
//     NetErrorDisconnect,
//     #[deku(id = "0x00400000")]
//     CICMDCommand,
//     #[deku(id = "0x01000000")]
//     TimeSync,
//     #[deku(id = "0x02000000")]
//     EchoRequest,
//     #[deku(id = "0x04000000")]
//     EchoResponse,
//     #[deku(id = "0x08000000")]
//     Flow,
// }

// TODO: I'm not sure if there's a way to use Deku with the bitflags crate
// so I duplicated the flags. Use this one just for bitwise operations.
// TODO: This might be it, see below...
#[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
pub struct PacketHeaderFlags(u32);

bitflags! {
    impl PacketHeaderFlags: u32 {
        const None = 0x00000000;
        const Retransmission = 0x00000001;
        const EncryptedChecksum = 0x00000002;
        const BlobFragments = 0x00000004;
        const ServerSwitch = 0x00000100;
        const LogonServerAddr = 0x00000200;
        const EmptyHeader1 = 0x00000400;
        const Referral = 0x00000800;
        const RequestRetransmit = 0x00001000;
        const RejectRetransmit = 0x00002000;
        const AckSequence = 0x00004000;
        const Disconnect = 0x00008000;
        const LoginRequest = 0x00010000;
        const WorldLoginRequest = 0x00020000;
        const ConnectRequest = 0x00040000;
        const ConnectResponse = 0x00080000;
        const NetError = 0x00100000;
        const NetErrorDisconnect = 0x00200000;
        const CICMDCommand = 0x00400000;
        const TimeSync = 0x01000000;
        const EchoRequest = 0x02000000;
        const EchoResponse = 0x04000000;
        const Flow = 0x08000000;
    }
}

impl PacketHeaderFlags {
    pub fn as_u32(&self) -> u32 {
        self.bits() as u32
    }
}

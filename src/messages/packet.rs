use deku::prelude::*;
use strum_macros::Display;
use bitflags::bitflags;

use crate::checksum::get_magic_number;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct TransitHeader {
    sequence: u32,
    pub flags: u32, // Weakly typed here because deku and bitflags don't work
                    // togetherr
    checksum: u32,
    recipient_id: u16,
    time_since_last_packet: u16,
    pub size: u16,
    table: u16,
}

#[derive(Debug, PartialEq)]
pub struct Packet {
    pub header: TransitHeader,
    pub fragment: Fragment,
    // TODO: Document each of these
    option_size: i32,
    sequence_ack: u32,
    connect_token: i64,
    timestamp: i64,
}

impl Packet {
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
            Some(world) => {
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

    fn hash(&mut self, seed : u32, data : Vec<u8>) -> u32{
        let orig = 0; // should be header.Checksum, whatever that is
        let mut result = 0;

        if self.option_size > 0 {
            result += get_magic_number(&data[..(self.option_size as usize)], self.option_size, true)
        }

        result
  }


}

#[derive(Debug, PartialEq)]
pub struct Fragment {
    pub header: TransitHeader,
    // TODO: WRong type here but for now it works
    pub body : ConnectRequestHeader,
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
bitflags! {
    pub struct PacketHeaderFlags: u32 {
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

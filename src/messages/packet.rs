use deku::prelude::*;
use std::string::ToString;
use strum_macros::Display;


#[derive(Debug, PartialEq)]
pub struct Fragment {
    pub header: S2CPacket,
    // TODO: WRong type here but for now it works
    pub body : ConnectRequestHeader,
}

// 20 bytes?
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct S2CPacket {
    sequence: u32,
    pub flags: PacketHeaderFlags,
    checksum: u32,
    recipient_id: u16,
    time_since_last_packet: u16,
    pub size: u16,
    iteration: u16,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct ConnectRequestHeader {
    server_time: f64,
    cookie: u8,
    net_id: i32,
    outgoing_seed: u32,
    unknown: u32,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite, Display)]
#[deku(type = "u32", endian = "endian", ctx = "endian: deku::ctx::Endian")]
pub enum PacketHeaderFlags {
    #[deku(id = "0x00000000")]
    None,
    #[deku(id = "0x00000001")]
    Retransmission,
    #[deku(id = "0x00000002")]
    EncryptedChecksum,
    #[deku(id = "0x00000004")]
    BlobFragments,
    #[deku(id = "0x00000100")]
    ServerSwitch,
    #[deku(id = "0x00000200")]
    LogonServerAddr,
    #[deku(id = "0x00000400")]
    EmptyHeader1,
    #[deku(id = "0x00000800")]
    Referral,
    #[deku(id = "0x00001000")]
    RequestRetransmit,
    #[deku(id = "0x00002000")]
    RejectRetransmit,
    #[deku(id = "0x00004000")]
    AckSequence,
    #[deku(id = "0x00008000")]
    Disconnect,
    #[deku(id = "0x00010000")]
    LoginRequest,
    #[deku(id = "0x00020000")]
    WorldLoginRequest,
    #[deku(id = "0x00040000")]
    ConnectRequest,
    #[deku(id = "0x00080000")]
    ConnectResponse,
    #[deku(id = "0x00100000")]
    NetError,
    #[deku(id = "0x00200000")]
    NetErrorDisconnect,
    #[deku(id = "0x00400000")]
    CICMDCommand,
    #[deku(id = "0x01000000")]
    TimeSync,
    #[deku(id = "0x02000000")]
    EchoRequest,
    #[deku(id = "0x04000000")]
    EchoResponse,
    #[deku(id = "0x08000000")]
    Flow,
}

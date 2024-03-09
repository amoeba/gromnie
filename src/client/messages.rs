// use deku::prelude::*;
use deku::prelude::*;
use std::string::ToString;
use strum_macros::Display;

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

// #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
// pub struct StringWithLength {
//   length: u16,
//   value: Vec<u8>,
// }

// /// LoginRequestPacket
// /// TODO: Re-do this to match
// /// ```text
// ///     0                   1                   2                   3
// ///     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// ///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// ///    |Version|  IHL  |    DSCP   |ECN|         Total Length          |
// ///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// ///    |         Identification        |Flags|      Fragment Offset    |
// ///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// ///    |  Time to Live |    Protocol   |         Header Checksum       |
// ///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// ///    |                       Source Address                          |
// ///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// ///    |                    Destination Address                        |
// ///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// ///    |                    Options                    |    Padding    |
// ///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// /// ```

// // TODO: strings are 4byte aligned including length
// #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
// #[deku(endian = "big")]
// pub struct LoginRequestPacket {
//   protocol_version: StringWithLength,
//   // AccountName: u8,
//   // Password: u8,
//   // PacketLength: u8,
//   // LoginType: u8,
//   // Unknown: u8,
//   // Timestamp: u8,
//   // AccountName: StringWithLength,
//   // UserNamePad: u8,
//   // AnotherUknown: u8,
//   // DatVersion: u8,
//   // Engine: u8,
//   // Game: u8,
//   // Major: u8,
//   // Minor: u8,
// }

use std::io::{Seek, Write};

pub fn login_request<W: Write + Seek>(writer: &mut W, name: &str, password: &str) {
    let protocol_version = "1802";
    let account_name = "acservertracker";
    let password = "jj9h26hcsggc";

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
    let account_len: u16 = (account_name.len() + password.len() + 1) as u16;
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
    writer.write(&account_len.to_le_bytes()).unwrap();
    writer.write(&account_name.as_bytes()).unwrap();
    writer.write(&":".as_bytes()).unwrap();
    writer.write(&password.as_bytes()).unwrap();

    // TODO: Not sure what this extra is yet
    writer
        .write(vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0].as_ref())
        .unwrap();
}

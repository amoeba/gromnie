// use deku::prelude::*;

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

use std::io::{Seek, Write};

pub fn login_request<W: Write + Seek>(writer: &mut W) {
    let protocol_version = "1802";
    let account_name = "acservertracker";
    let password = "jj9h26hcsggc";

    // sequence
    writer.write(&0x0u32.to_le_bytes()).unwrap();

    // packetheaderflags
    writer.write(&0x00010000u32.to_le_bytes()).unwrap();

    // checksum
    writer.write(&0x05d00093u32.to_le_bytes()).unwrap();

    // recipient
    writer.write(&0x0u16.to_le_bytes()).unwrap();

    // timesincelastpacket
    writer.write(&0x0u16.to_le_bytes()).unwrap();

    // size
    writer.write(&0x40u16.to_le_bytes()).unwrap();

    // iteration
    writer.write(&0x0u16.to_le_bytes()).unwrap();

    // ClientVersion = packet.DataReader.ReadString16L();      // should be "1802" for end of retail client
    writer.write(&0x04u16.to_le_bytes()).unwrap();
    let client_version: [u8; 6] = [0x31, 0x38, 0x30, 0x32, 0x00, 0x00];
    writer.write(&client_version).unwrap();

    // uint len = packet.DataReader.ReadUInt32();                     // data length left in packet including ticket
    writer.write(&0x34u32.to_le_bytes()).unwrap();

    // NetAuthType = (NetAuthType)packet.DataReader.ReadUInt32();
    writer.write(&0x01u32.to_le_bytes()).unwrap();

    // var authFlags = (AuthFlags)packet.DataReader.ReadUInt32();
    writer.write(&0x0u32.to_le_bytes()).unwrap();

    // Timestamp = packet.DataReader.ReadUInt32();                    // sequence
    writer.write(&0x58a8b83eu32.to_le_bytes()).unwrap();

    // Account = packet.DataReader.ReadString16L();
    // string accountToLoginAs = packet.DataReader.ReadString16L();   // special admin only, AuthFlags has 2

    // if (NetAuthType == NetAuthType.AccountPassword)
    //     Password = packet.DataReader.ReadString32L();
    // else if (NetAuthType == NetAuthType.GlsTicket)
    //     GlsTicket = packet.DataReader.ReadString32L();

    let account: [u8; 30] = [
        0x1c, 0x00, 0x61, 0x63, 0x73, 0x65, 0x72, 0x76, 0x65, 0x72, 0x74, 0x72, 0x61, 0x63, 0x6b,
        0x65, 0x72, 0x3a, 0x6a, 0x6a, 0x39, 0x68, 0x32, 0x36, 0x68, 0x63, 0x73, 0x67, 0x67, 0x63,
    ];
    writer.write(&account).unwrap();
    writer
        .write(vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0].as_ref())
        .unwrap();
}

use std::{fmt::write, io::{Seek, SeekFrom, Write}, time::{SystemTime, UNIX_EPOCH}};

use byteorder::{LittleEndian, WriteBytesExt};

// trait Packet {
//   fn serialize(&self) -> Vec<u8>;
//   fn common_data() -> Vec<u8> {
//     vec![0x01, 0x02, 0x03]
// }
// }

// struct LoginPacketMessage {
//   // Define your message fields here
// }

// impl Packet for LoginPacketMessage {
//   fn serialize(&self) -> Vec<u8> {
//       // Implement serialization logic specific to MessageType1
//       // Add common prefix to the buffer
//       let mut buffer = Self::common_data();
//       // buffer.extend(&self.data);
//       buffer
//   }
// }

pub fn on_serialize<W: Write + Seek>(writer: &mut W) {
  // TODO: We should tolower the account value

  // TODO: Factor these out into a struct
  let protocol_version = "1802";
  let account_name = "acservertracker";
  let password = "jj9h26hcsggc";

  // TODO: This bit should come from the parent packet class
  let preamble = vec![0; 20];
  writer.write(&preamble).unwrap();

  writer.write(&(protocol_version.len() as u16).to_le_bytes()).unwrap();
  writer.write(protocol_version.as_bytes()).unwrap();

  let padding = 4 - (protocol_version.len() + 2) % 4;
  println!("Padding is {}", padding);
  writer.seek(SeekFrom::Current(padding as i64)).unwrap();

  //

  let mut user_name_pad = 0;
  let mut password_pad = 0;
  let mut packet_len: usize = 20;
  let mut _login_type : u8 = 0;

  user_name_pad = (account_name.len() + 2) % 4;
  let login_type : u8;

  if user_name_pad > 0 {
    user_name_pad = 4 - user_name_pad;
  }

  packet_len += account_name.len() + 2 + user_name_pad;

  match Some(password) {
    Some(value) => {
      println!("0x02 login type (match case)");

      login_type = 0x0000002;
      password_pad = (value.len() + 5) % 4;

      if password_pad > 0 {
        password_pad = 4 - password_pad
      };

      packet_len += value.len() + 5 + password_pad;
    }
    _ => {
      println!("0x01 login type (else case)");
      login_type = 0x0000001;
    }
  }

  println!("packet_len is {packet_len} but should be 52");
  writer.write(&(packet_len as u32).to_le_bytes()).unwrap();
  // should be 4
  writer.write(&(login_type as u32).to_le_bytes()).unwrap();
  // writer.write_u32::<LittleEndian>(login_type.into()).unwrap();
  // should be 4
  writer.write(&(0x0 as u32).to_le_bytes()).unwrap();

  let unix_time = SystemTime::now()
  .duration_since(UNIX_EPOCH)
  .expect("Time went backwards")
  .as_secs() as i32;

  // TODO: This isn't right
  writer.write(&unix_time.to_le_bytes()).unwrap();

  // Account Name
  writer.write(&(account_name.len() as u16).to_le_bytes()).unwrap();
  writer.write(account_name.as_bytes()).unwrap();

  // Padding for alignment
  let user_name_pad = 4 - (account_name.len() + 2) % 4;

  if user_name_pad > 0 {
      writer.seek(SeekFrom::Current(user_name_pad as i64)).unwrap();
  }

  // Empty string
  writer.write(&0u32.to_le_bytes()).unwrap();

  // Password
  let maybe_password = Some(password);

  match maybe_password {
    Some(password) => {
      let password_len = password.len() + 1;
      // this is good
      writer.write(&(password_len as u32).to_le_bytes()).unwrap();
      writer.write(&(password.len() as u8).to_le_bytes()).unwrap();
      writer.write(password.as_bytes()).unwrap();

      // Padding for alignment
      let password_pad = 4 - (password_len as usize) % 4;
      if password_pad > 0 {
          writer.seek(SeekFrom::Current(password_pad as i64)).unwrap();
      }
    },
    _ => {
      writer.write(&0u32.to_le_bytes()).unwrap();
    }
  }

  // Data versions
  writer.write(&0x0000001Cu32.to_le_bytes()).unwrap(); // Length
  writer.write(&0x00000016u32.to_le_bytes()).unwrap(); // Engine
  writer.write(&0x00000000u32.to_le_bytes()).unwrap(); // Game

  // Major versions
  writer.write(&0x4C46722F34A7D7D2u64.to_le_bytes()).unwrap();
  writer.write(&0xFD6F854F51EFB48Au64.to_le_bytes()).unwrap();

  // Minor version
  writer.write(&0x00001A01u32.to_le_bytes()).unwrap();
}

pub fn on_serialize_alt<W: Write + Seek>(writer: &mut W) {

  let protocol_version = "1802";
  let account_name = "acservertracker";
  let password = "jj9h26hcsggc";

  // sequence
  writer.write(&0x0u32.to_le_bytes()).unwrap();

  // packetheaderflags
  writer.write(&0x000100u32.to_le_bytes()).unwrap();

  // checksum
  writer.write(&0x9300d005u32.to_le_bytes()).unwrap();

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
  let client_version : [u8; 6] = [
    0x31, 0x38, 0x30, 0x32,
    0x00, 0x00];
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

  let account:[u8; 30] = [0x1c, 0x00,
  0x61, 0x63, 0x73, 0x65, 0x72, 0x76, 0x65, 0x72, 0x74, 0x72, 0x61, 0x63, 0x6b, 0x65, 0x72,
  0x3a,
  0x6a, 0x6a, 0x39, 0x68, 0x32, 0x36, 0x68, 0x63, 0x73, 0x67, 0x67, 0x63,
  ];
  writer.write(&account).unwrap();
  writer.write(vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, ].as_ref()).unwrap();

}

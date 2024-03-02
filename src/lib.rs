use std::{io::{Cursor, Seek, SeekFrom, Write}, time::{SystemTime, UNIX_EPOCH}};

fn on_serialize<W: Write + Seek>(writer: &mut W) {
  let protocol_version = "1802";

  writer.write(&(protocol_version.len() as u16).to_le_bytes()).unwrap();
  writer.write(protocol_version.as_bytes()).unwrap();

  let padding = 4 - (protocol_version.len() + 2) % 4;
  println!("Padding is {}", padding);
  writer.seek(SeekFrom::Current(padding as i64)).unwrap();

  //
  let account_name = "acservertracker";
  let password = "jj9h26hcsggc";

  let mut packet_len = 0;
  let mut user_name_pad = (account_name.len() + 2) % 4;
  let mut password_pad = 0;
  let login_type : u8;

  if (user_name_pad > 0){
    user_name_pad = 4 - user_name_pad;
  }

  packet_len += account_name.len() + 2 + user_name_pad;


  if password.len() == 0 {
      login_type = 0x0000001;
  } else {
      login_type = 0x0000002;
      password_pad = (password.len() + 5) % 4;

      if password_pad > 0 {
        password_pad = 4 - password_pad
      };

      packet_len += password.len() + 5 + password_pad;
  }

  println!("packet_len is {packet_len} but should be 52");
  writer.write(&(packet_len as u8).to_le_bytes()).unwrap();
  writer.write(&(login_type).to_le_bytes()).unwrap();
  writer.write(&(0x0 as u8).to_le_bytes()).unwrap();

  let unix_time = SystemTime::now()
  .duration_since(UNIX_EPOCH)
  .expect("Time went backwards")
  .as_secs() as i32;

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

pub fn test() {
  let mut buffer = Cursor::new(Vec::new());

  on_serialize(&mut buffer);
  let serialized_data = buffer.into_inner();

  println!("len is {}", serialized_data.len());
  print!("[");
  for i in 0..(serialized_data.len()) {
      print!("{:#04X}, ", &serialized_data[i]);
  }
  print!("]");
}

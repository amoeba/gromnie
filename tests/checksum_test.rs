use std::io::Cursor;

use gromnie::{messages::packet::{Packet, TransitHeader}, packets::login_request::LoginRequestPacket};

#[test]
fn test_checksum() {
  let buf = [0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 48, 0, 0, 0, 4, 0, 49, 56, 48, 50, 0, 0, 40, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 173, 239, 237, 101, 4, 0, 116, 101, 115, 116, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 4, 116, 101, 115, 116];
  let expect = 840662028;//0x321B7C0C

  let mut lrp = LoginRequestPacket::create("account", "password");

  assert_eq!(0, lrp.packet.hash());
}

// #[test]
// fn test_hash() {
//   let input = vec![1, 2, 3, 4];
//   let magic = hash(0, &input);

//   assert_eq!(0, magic);
// }

// #[test]
// fn test_message_login_request() {
//     let account_name = "acservertracker";
//     let password = "jj9h26hcsggc";
//     // TODO: Find out how I can test that the above gives me this checksum
//     // from my PCAP;
//     // 0x05d00093u32

//     let mut buf = Cursor::new(Vec::new());
//     login_request(&mut buf, &account_name.to_owned(), &password.to_owned());

//     let expected = vec![0, 1, 2, 3];
//     assert_eq!(expected, buf.into_inner());
// }

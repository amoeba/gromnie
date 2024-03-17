use std::io::Cursor;

use byteorder::{ByteOrder, LittleEndian};
use gromnie::{crypto::magic_number::get_magic_number, net::packets::login_request::LoginRequestPacket};

#[test]
fn test_get_magic_number() {
  // Expected value came from debugging actestclient
  let input = vec![1, 2, 3];
  assert_eq!(0x1020300, get_magic_number(&input, input.len(), false));
}

#[test]
fn test_get_magic_number_with_known_slice() {
  let input = vec![4, 0, 49, 56, 48, 50, 0, 0, 40, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 229, 192, 243, 101, 4, 0, 116, 101, 115, 116, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 4, 116, 101, 115, 116, 0, 0, 0];
  // actually equal to 0x772EDD29
  assert_eq!(0x772EDD29, get_magic_number(&input, input.len(), true));

}

#[test]
fn test_hash_wip() {
  // Example LoginRequest packet
  let mut packet = LoginRequestPacket::new("test", "test");

  let mut buffer = Cursor::new(Vec::new());
  packet.serialize(&mut buffer);

  // Size should be 48, from debugger
  assert_eq!(48, packet.packet.header.size);

  // Captured this from NetworkManager.Send
  let input = vec![0,0,0,0,0,0,1,0,0,0,0,0,0,0,0,0,48,0,0,0,4,0,49,56,48,50,0,0,40,0,0,0,2,0,0,0,0,0,0,0,211,202,243,101,4,0,116,101,115,116,0,0,0,0,0,0,5,0,0,0,4,116,101,115,116,0,0,0];
  let expected = 841045810;

  assert_eq!(expected, 2);
}

#[test]
fn test_casting_as_u32() {
  assert_eq!(0, LittleEndian::read_u32(&vec![0, 0, 0, 0][0..4]));
  assert_eq!(1, LittleEndian::read_u32(&vec![1, 0, 0, 0][0..4]));
  assert_eq!(255, LittleEndian::read_u32(&vec![255, 0, 0, 0][0..4]));
  assert_eq!(65280, LittleEndian::read_u32(&vec![0, 255, 0, 0][0..4]));
  assert_eq!(16711680, LittleEndian::read_u32(&vec![0, 0, 255, 0][0..4]));
  assert_eq!(4278190080, LittleEndian::read_u32(&vec![0, 0, 0, 255][0..4]));
  assert_eq!(4294967295, LittleEndian::read_u32(&vec![255, 255, 255, 255][0..4]));
}

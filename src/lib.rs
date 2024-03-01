use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct DekuTest {
    protocol_version_length: u8,
    #[deku(count = "protocol_version_length")]
    protocol_version: Vec<u8>,
}

pub fn test() {
  let data: Vec<u8> = vec![0x04, 0x00, 0x00, 0x00, 0x00];

  let (_rest, val) = DekuTest::from_bytes((data.as_ref(), 0)).unwrap();

  assert_eq!(DekuTest {
    protocol_version_length: 0x04,
    protocol_version: vec![0x00, 0x00, 0x00, 0x00],
  }, val);

  let data_out = val.to_bytes().unwrap();
  assert_eq!(vec![0x04, 0x00, 0x00, 0x00, 0x00], data_out);
}

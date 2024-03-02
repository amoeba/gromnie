// use std::convert::TryInto;

// use deku::bitvec::{BitVec, Msb0};
// use deku::ctx::BitSize;
// use deku::prelude::*;

// fn my_pad_read<R: std::io::Read>(
//   field_length: u8,
//   field_value: Vec<u8>,
//   reader: &mut Reader<R>,
//   bit_size: BitSize,
// ) -> Result<u8, DekuError> {
//   // // Access to previously read fields
//   // println!("field_a = 0x{:X}", field_length);

//   // // Size of the current field
//   // println!("bit_size: {:?}", bit_size);

//   // // read field_b, calling original func
//   let value = u8::from_reader_with_ctx(reader, bit_size)?;

//   // flip the bits on value if field_length is 0x01
//   // let value = if field_length == 0x01 { !value } else { value };

//   Ok(value)
// }

// fn my_pad_write(
//   field_length: u8,
//   field_value: Vec<u8>,
//   output: &mut BitVec<u8, Msb0>,
//   bit_size: BitSize,
// ) -> Result<(), DekuError> {
//   // // Access to previously written fields
//   // println!("field_a = 0x{:X}", field_length);

//   // // value of field_value
//   // println!("field_value = 0x{:X}", field_value);

//   // // Size of the current field
//   // println!("bit_size: {:?}", bit_size);

//   // // flip the bits on value if field_length is 0x01
//   // let value = if field_length == 0x01 { !field_value } else { field_value };

//   value.write(output, bit_size)
// }

// #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
// #[deku(endian = "big")]
// struct DekuTest {
//     protocol_version_length: u8,
//     #[deku(count = "protocol_version_length")]
//     protocol_version: Vec<u8>,
//     #[deku(
//       count = "protocol_version_length",
//       reader = "my_pad(*protocol_version_length, *protocol_version, deku::reader, BitSize(8))",
//       writer = "my_pad_write(*protocol_version_length, *protocol_version, deku::output, BitSize(8))")
//     ]
//     protocol_pad: Vec<u8>
// }

// pub fn test() {
//   let data: Vec<u8> = vec![0x04, 0x00, 0x00, 0x00, 0x00];

//   let (_rest, val) = DekuTest::from_bytes((data.as_ref(), 0)).unwrap();

//   assert_eq!(DekuTest {
//     protocol_version_length: 0x04,
//     protocol_version: vec![0x00, 0x00, 0x00, 0x00],
//   }, val);

//   let data_out = val.to_bytes().unwrap();
//   assert_eq!(vec![0x04, 0x00, 0x00, 0x00, 0x00], data_out);
// }

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

pub mod client;
pub mod checksum;
pub mod net;

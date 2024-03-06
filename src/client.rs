
use std::{io::{Cursor, Seek}, net::UdpSocket};

use deku::prelude::*;
use libgromnie::on_serialize;

struct Account {
  name: String,
  password: String
}

// TODO: Don't require both bind_address and connect_address. I had to do this
// to get things to work but I should be able to listen on any random port so
// I'm not sure what I'm doing wrong
pub struct Client {
  bind_address: String,
  connect_address: String,
  socket: Option<UdpSocket>,
  account: Account,
}

impl Client {
  pub fn create(bind_address: String, connect_address: String, account_name: String, password: String) -> Client {
    Client {
      bind_address, connect_address, account: Account { name: account_name, password: password }, socket: None
    }
  }

  pub fn connect(&mut self) -> Result<usize, std::io::Error> {
    // TODO: Possibly set this elsewhere
    self.socket = Some(UdpSocket::bind(self.bind_address.clone()).expect("Failed to bind"));

    let socket = Option::expect(self.socket.as_ref(), "socket not set");

    let _ = socket.connect(self.connect_address.clone());

    let mut buffer = Cursor::new(Vec::new());
    on_serialize(&mut buffer);
    let serialized_data: Vec<u8> = buffer.into_inner();

    let _ = socket.send(&serialized_data).unwrap();

    let mut recv_buffer = [0u8; 1024];

    let nbytes = socket.recv(&mut recv_buffer);

    // TODO: Temporary code to parse response. Move this elsewhere when it's ready.
    let mut recv_cursor = Cursor::new(&recv_buffer);
    // parse_response(&mut recv_cursor);
    parse_response(&recv_buffer);

    nbytes
  }

// Response... which packet is this?
// 0000   02 00 00 00 45 00 00 50 cc c2 00 00 40 11 00 00
// 0010   7f 00 00 01 7f 00 00 01 23 28 c9 10 00 3c fe 4f
// 0020   00 00 00 00 00 00 04 00 0a b9 8c 2c 0b 00 4a 88
// 0030   20 00 01 00 ad aa f2 94 10 a7 aa 41 f9 93 86 68
// 0040   82 d4 43 a8 00 00 00 00 75 05 81 e9 55 88 43 18
// 0050   00 00 00 00

  // ame="S2CPacket" text="Server to Client AC packet.">
	// 		<field type="uint" name="Sequence" text="Packet Sequence / Order" />
	// 		<field type="PacketHeaderFlags" name="Flags" text="Flags that dictate the content / purpose of this packet" />
	// 		<field type="uint" name="Checksum" text="Packet Checksum" />
	// 		<field type="ushort" name="RecipientId" />
	// 		<field type="ushort" name="TimeSinceLastPacket" />
	// 		<field type="ushort" name="Size" text="Packet length, excluding this header" />
	// 		<field type="ushort" name="Iteration" />

// PacketHeaderFlags
// <value value="0x00080000" name="ConnectResponse" />

// <type name="ConnectRequestHeader" proto="true" text="Optional header data when PacketHeaderFlags includes ConnectRequest">
// <field name="ServerTime" type="double" />
// <field name="Cookie" type="ulong" />
// <field name="NetID" type="int" />
// <field name="OutgoingSeed" type="uint" />
// <field name="IncomingSeed" type="uint" />
// <field name="Unknown" type="DWORD" />
// </type>

}

// 20 bytes?
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct S2CPacket {
  sequence: u32,
  flags: u32,
  checksum: u32,
  recipient_id: u16,
  time_since_last_packet: u16,
  size: u16,
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

// TODO: this is a total hack but it looks like it works. Can we wrap this up
// better?
pub fn parse_response(buffer: &[u8]) {
  let mut cursor = Cursor::new(&buffer);

  let hdr = S2CPacket::from_bytes((&cursor.get_ref(), 0)).unwrap();
  println!("{:?}", hdr.1);

  // Skip to remainder
  cursor.seek(std::io::SeekFrom::Start(hdr.1.size as u64));

  let data: ((&[u8], usize), ConnectRequestHeader) = ConnectRequestHeader::from_bytes((&cursor.get_ref(), 32)).unwrap();
  println!("{:?}", data.1);

}

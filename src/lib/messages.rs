#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct StringWithLength {
  Length: u16,
  Value: string,
}

// TODO: strings are 4byte aligned including length
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct LoginRequestPacket {
  ProtocolVersion: StringWithLength,
  AccountName: u8,
  Password: u8,
  PacketLength: u8,
  LoginType: u8,
  Unknown: u8,
  Timestamp: u8,
  AccountName: StringWithLength,
  UserNamePad: u8,
  AnotherUknown: u8,
  DatVersion: u8,
  Engine: u8,
  Game: u8,
  Major: u8,
  Minor: u8,
}

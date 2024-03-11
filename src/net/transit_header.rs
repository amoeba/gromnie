use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct TransitHeader {
    pub sequence: u32,
    pub flags: u32, // Weakly typed here because deku and bitflags don't work
                    // together. Or do they? TODO
    pub checksum: u32,
    pub recipient_id: u16,
    pub time_since_last_packet: u16,
    pub size: u16,
    pub iteration: u16,
}

impl TransitHeader {
    pub fn new() -> TransitHeader {
        TransitHeader {
            sequence: 0,
            flags: 0,
            checksum: 0,
            recipient_id: 0,
            time_since_last_packet: 0,
            size: 0,
            iteration: 0,
        }
    }
}

use deku::prelude::*;

use crate::crypto::magic_number::get_magic_number;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian", ctx_default = "deku::ctx::Endian::Little")]
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
    pub fn new(flags : u32) -> TransitHeader {
        TransitHeader {
            sequence: 0,
            flags: flags,
            checksum: 0,
            recipient_id: 0,
            time_since_last_packet: 0,
            size: 0,
            iteration: 0,
        }
    }

    pub fn compute_checksum(&mut self) -> u32 {
        let orig = self.checksum; // should be header.Checksum, whatever that is

        self.checksum = 0xBADD70DD;

        let buf = self.to_bytes().unwrap();
        let result = get_magic_number(&buf, buf.len(), true);

        self.checksum = orig;

        // TODO: See if I can avoid the cast here
        result as u32
    }
}

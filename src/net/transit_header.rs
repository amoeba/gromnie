// Re-export PacketHeader from acprotocol as TransitHeader for compatibility
pub use acprotocol::network::packet::PacketHeader as TransitHeader;
pub use acprotocol::network::packet::PacketHeaderFlags;

use crate::crypto::magic_number::get_magic_number;

// Extension trait to add checksum computation to PacketHeader
pub trait PacketHeaderExt {
    fn new(flags: u32) -> Self;
    fn compute_checksum(&self) -> u32;
    fn to_bytes(&self) -> Result<Vec<u8>, std::io::Error>;
}

impl PacketHeaderExt for TransitHeader {
    fn new(flags: u32) -> Self {
        TransitHeader {
            sequence: 0,
            flags: PacketHeaderFlags::from_bits_truncate(flags),
            checksum: 0,
            id: 0,   // formerly recipient_id
            time: 0, // formerly time_since_last_packet
            size: 0,
            iteration: 0,
        }
    }

    fn compute_checksum(&self) -> u32 {
        // Create a copy for checksum computation
        let temp = TransitHeader {
            sequence: self.sequence,
            flags: self.flags,
            checksum: 0xBADD70DD, // Magic constant
            id: self.id,
            time: self.time,
            size: self.size,
            iteration: self.iteration,
        };

        let buf = temp.to_bytes().unwrap();
        get_magic_number(&buf, buf.len(), true)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, std::io::Error> {
        use std::io::Write;

        let mut buf = Vec::with_capacity(20);

        // Write fields in little-endian order (20 bytes total)
        buf.write_all(&self.sequence.to_le_bytes())?;
        buf.write_all(&self.flags.bits().to_le_bytes())?;
        buf.write_all(&self.checksum.to_le_bytes())?;
        buf.write_all(&self.id.to_le_bytes())?;
        buf.write_all(&self.time.to_le_bytes())?;
        buf.write_all(&self.size.to_le_bytes())?;
        buf.write_all(&self.iteration.to_le_bytes())?;

        Ok(buf)
    }
}

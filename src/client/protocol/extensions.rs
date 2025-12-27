use std::io::Cursor;

use acprotocol::enums::PacketHeaderFlags;
use acprotocol::packets::c2s_packet::C2SPacket;
use acprotocol::writers::ACWritable;

use crate::client::constants::*;
use crate::client::session::SessionState;
use crate::crypto::magic_number::get_magic_number;

/// Extension trait for C2SPacket to add serialization with checksum
/// TODO: Consider putting this in acprotocol
pub trait C2SPacketExt {
    fn serialize(&self, session: Option<&SessionState>) -> Result<Vec<u8>, std::io::Error>;
    fn calculate_option_size(&self) -> usize;
}

impl C2SPacketExt for C2SPacket {
    /// Calculate the size of optional headers based on which fields are present
    fn calculate_option_size(&self) -> usize {
        let mut option_size = 0usize;

        // Order matches actestclient's Serialize() method (Packet.cs lines 126-176)
        if self.ack_sequence.is_some() {
            option_size += 4; // u32
        }
        if self.world_login_request.is_some() {
            option_size += 8; // u64
        }
        if self.connect_response.is_some() {
            option_size += 8; // u64
        }
        // Note: LoginRequest is handled in C2SPacket.write(), so it contributes to size
        // but is not part of optional headers in the same way
        if self.time.is_some() {
            option_size += 8; // u64
        }
        if self.echo_time.is_some() {
            option_size += 4; // f32
        }
        if self.flow.is_some() {
            option_size += 6; // u32 + u16
        }

        option_size
    }

    fn serialize(&self, session: Option<&SessionState>) -> Result<Vec<u8>, std::io::Error> {
        let mut buffer = Vec::new();
        {
            let mut cursor = Cursor::new(&mut buffer);
            self.write(&mut cursor)
                .map_err(|e| std::io::Error::other(format!("Write error: {}", e)))?;
        }

        // If this is a fragmented packet with session established, set ENCRYPTED_CHECKSUM flag
        if self.flags.contains(PacketHeaderFlags::BLOB_FRAGMENTS) && session.is_some() {
            // Set the ENCRYPTED_CHECKSUM flag (0x2) in the flags field (bytes 4-7)
            let mut flags = u32::from_le_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);
            flags |= 0x2; // ENCRYPTED_CHECKSUM flag
            buffer[4..8].copy_from_slice(&flags.to_le_bytes());
        }

        // Calculate option size before we modify the buffer
        let option_size = self.calculate_option_size();

        // Get the size field from the header (bytes 16-17)
        // This is the total size of payload + optional headers
        let header_size = u16::from_le_bytes([buffer[16], buffer[17]]) as usize;
        let mut checksum_result = 0u32;

        if header_size > 0 {
            // Step 1: Checksum optional headers if present
            if option_size > 0 && option_size <= header_size {
                let option_checksum = get_magic_number(
                    &buffer[PACKET_HEADER_SIZE..PACKET_HEADER_SIZE + option_size],
                    option_size,
                    true,
                );
                checksum_result = checksum_result.wrapping_add(option_checksum);
            }

            // Step 2: Checksum remaining payload (fragments or message data)
            let remaining = header_size - option_size;
            if remaining > 0 {
                let payload_start = PACKET_HEADER_SIZE + option_size;
                let payload_checksum = get_magic_number(
                    &buffer[payload_start..payload_start + remaining],
                    remaining,
                    true,
                );
                checksum_result = checksum_result.wrapping_add(payload_checksum);
            }

            // Step 3: XOR with seed if this is a fragmented packet with session established
            // Fragmented packets use encrypted checksums (ENCRYPTED_CHECKSUM flag 0x2)
            if self.flags.contains(PacketHeaderFlags::BLOB_FRAGMENTS) {
                if let Some(sess) = session {
                    let encryption_key = sess.send_generator.borrow_mut().get_send_key();
                    checksum_result ^= encryption_key;
                }
            }
        }

        // Step 4: Set header checksum to placeholder (0xbadd70dd)
        buffer[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4]
            .copy_from_slice(&CHECKSUM_PLACEHOLDER.to_le_bytes());

        // Step 5: Checksum the entire packet header (with placeholder in checksum field)
        let header_checksum =
            get_magic_number(&buffer[0..PACKET_HEADER_SIZE], PACKET_HEADER_SIZE, true);
        checksum_result = checksum_result.wrapping_add(header_checksum);

        // Step 6: Write final checksum back to the packet
        buffer[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4]
            .copy_from_slice(&checksum_result.to_le_bytes());

        Ok(buffer)
    }
}

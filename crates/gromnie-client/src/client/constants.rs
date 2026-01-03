// Protocol constants used throughout the client module

pub const PACKET_HEADER_SIZE: usize = 20;
pub const CHECKSUM_OFFSET: usize = 8;
pub const CHECKSUM_PLACEHOLDER: u32 = 0xbadd70dd;
pub const FRAGMENT_HEADER_SIZE: usize = 16; // sequence(4) + id(4) + count(2) + size(2) + index(2) + group(2)

// UI delay for connection flow to make progress visible (1 second)
pub const UI_DELAY_MS: u64 = 1000;

/// DDD Interrogation Response - indicates client is up-to-date with all DAT files
/// Format: [Opcode (0xF7E6), Language (1), CAllIterationList count (0)]
///
/// This is a static response that tells the ACE server the client doesn't need any DAT patches.
/// The server expects:
/// - Opcode: 0xF7E6 (u32, little-endian)
/// - Language: 1 (u32, little-endian)
/// - CAllIterationList count: 0 (i32, little-endian) - empty list means "up-to-date"
pub const DDD_RESPONSE_UP_TO_DATE: [u8; 12] = [
    0xE6, 0xF7, 0x00, 0x00, // Opcode: 0xF7E6 (little-endian u32)
    0x01, 0x00, 0x00, 0x00, // Language: 1 (little-endian u32)
    0x00, 0x00, 0x00, 0x00, // CAllIterationList count: 0 (little-endian i32)
];

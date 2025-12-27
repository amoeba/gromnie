// Protocol constants used throughout the client module

pub const PACKET_HEADER_SIZE: usize = 20;
pub const CHECKSUM_OFFSET: usize = 8;
pub const CHECKSUM_PLACEHOLDER: u32 = 0xbadd70dd;
pub const FRAGMENT_HEADER_SIZE: usize = 16; // sequence(4) + id(4) + count(2) + size(2) + index(2) + group(2)

// UI delay for connection flow to make progress visible (1 second)
pub const UI_DELAY_MS: u64 = 1000;

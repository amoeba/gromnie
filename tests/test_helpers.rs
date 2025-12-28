use acprotocol::enums::PacketHeaderFlags;
/// Shared helpers for regression testing
///
/// This module provides utilities for building test packets and validating their structure.
use acprotocol::packets::c2s_packet::C2SPacket;
use byteorder::{ByteOrder, LittleEndian};
use gromnie::client::C2SPacketExt;

// ============================================================================
// Field Extraction Helpers
// ============================================================================

/// Extract sequence field from packet header
pub fn extract_sequence(buffer: &[u8]) -> u32 {
    assert!(buffer.len() >= 4, "Buffer too small for sequence field");
    LittleEndian::read_u32(&buffer[0..4])
}

/// Extract flags field from packet header
pub fn extract_flags(buffer: &[u8]) -> u32 {
    assert!(buffer.len() >= 8, "Buffer too small for flags field");
    LittleEndian::read_u32(&buffer[4..8])
}

/// Extract checksum field from packet header
pub fn extract_checksum(buffer: &[u8]) -> u32 {
    assert!(buffer.len() >= 12, "Buffer too small for checksum field");
    LittleEndian::read_u32(&buffer[8..12])
}

/// Extract recipient_id field from packet header
pub fn extract_recipient_id(buffer: &[u8]) -> u16 {
    assert!(
        buffer.len() >= 14,
        "Buffer too small for recipient_id field"
    );
    LittleEndian::read_u16(&buffer[12..14])
}

/// Extract time_since_last_packet field from packet header
pub fn extract_time_since_last_packet(buffer: &[u8]) -> u16 {
    assert!(
        buffer.len() >= 16,
        "Buffer too small for time_since_last_packet field"
    );
    LittleEndian::read_u16(&buffer[14..16])
}

/// Extract size field from packet header (payload size, not including header)
pub fn extract_size(buffer: &[u8]) -> u16 {
    assert!(buffer.len() >= 18, "Buffer too small for size field");
    LittleEndian::read_u16(&buffer[16..18])
}

/// Extract iteration field from packet header
pub fn extract_iteration(buffer: &[u8]) -> u16 {
    assert!(buffer.len() >= 20, "Buffer too small for iteration field");
    LittleEndian::read_u16(&buffer[18..20])
}

/// Extract ACK sequence from optional headers (if present)
pub fn extract_ack_sequence(buffer: &[u8], flags: u32) -> Option<u32> {
    if (flags & PacketHeaderFlags::ACK_SEQUENCE.bits()) == 0 {
        return None;
    }
    assert!(buffer.len() >= 24, "Buffer too small for ACK sequence");
    Some(LittleEndian::read_u32(&buffer[20..24]))
}

/// Extract payload portion of packet (everything after header and optional headers)
pub fn extract_payload(buffer: &[u8]) -> &[u8] {
    assert!(buffer.len() >= 20, "Buffer must have at least header");
    let size = extract_size(buffer) as usize;
    let payload_offset = 20; // Just after header, optional headers are part of size
    assert!(
        buffer.len() >= payload_offset + size,
        "Buffer too small for payload"
    );
    &buffer[payload_offset..]
}

// ============================================================================
// Packet Structure Validation
// ============================================================================

/// Verify basic packet structure invariants
pub fn verify_basic_structure(buffer: &[u8], expected_flags: u32) {
    assert!(
        buffer.len() >= 20,
        "Packet must have at least 20-byte header"
    );
    assert_eq!(buffer.len() % 4, 0, "Packet must be 4-byte aligned");

    let flags = extract_flags(buffer);
    assert_eq!(flags, expected_flags, "Flags mismatch");

    let size = extract_size(buffer) as usize;
    assert_eq!(buffer.len(), 20 + size, "Total length = header + size");
}

/// Verify that checksum field is calculated (not placeholder, not zero)
pub fn verify_checksum_calculated(buffer: &[u8]) {
    let checksum = extract_checksum(buffer);
    assert_ne!(checksum, 0, "Checksum should not be zero");
    assert_ne!(
        checksum, 0xbadd70dd,
        "Checksum should be calculated, not placeholder"
    );
}

/// Verify packet structure for packets with payload
pub fn verify_structure_with_payload(
    buffer: &[u8],
    expected_flags: u32,
    expected_payload_size: usize,
) {
    verify_basic_structure(buffer, expected_flags);

    let size = extract_size(buffer) as usize;
    assert_eq!(size, expected_payload_size, "Payload size mismatch");
}

// ============================================================================
// Packet Builders for Tests
// ============================================================================

/// Build a minimal TimeSync packet for testing
pub fn build_timesync_packet(sequence: u32, timestamp: u64) -> C2SPacket {
    C2SPacket {
        sequence,
        flags: PacketHeaderFlags::TIME_SYNC,
        checksum: 0,
        recipient_id: 0,
        time_since_last_packet: 0,
        size: 8,
        iteration: 0,
        server_switch: None,
        retransmit_sequences: None,
        reject_sequences: None,
        ack_sequence: None,
        login_request: None,
        world_login_request: None,
        connect_response: None,
        cicmd_command: None,
        time: Some(timestamp),
        echo_time: None,
        flow: None,
        fragments: None,
    }
}

/// Build an empty packet (header only, no payload or optional headers)
pub fn build_empty_packet(sequence: u32, flags: PacketHeaderFlags) -> C2SPacket {
    C2SPacket {
        sequence,
        flags,
        checksum: 0,
        recipient_id: 0,
        time_since_last_packet: 0,
        size: 0,
        iteration: 0,
        server_switch: None,
        retransmit_sequences: None,
        reject_sequences: None,
        ack_sequence: None,
        login_request: None,
        world_login_request: None,
        connect_response: None,
        cicmd_command: None,
        time: None,
        echo_time: None,
        flow: None,
        fragments: None,
    }
}

/// Build a packet with ACK sequence for testing optional headers
pub fn build_packet_with_ack(sequence: u32, ack_seq: u32) -> C2SPacket {
    build_empty_packet(sequence, PacketHeaderFlags::empty()).with_ack_sequence(ack_seq)
}

// ============================================================================
// Hex Formatting Helpers (for test output)
// ============================================================================

/// Format a buffer as hex string for test output/debugging
pub fn format_hex(buffer: &[u8]) -> String {
    buffer
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Format a buffer as hex string with line breaks for readability
pub fn format_hex_pretty(buffer: &[u8], bytes_per_line: usize) -> String {
    buffer
        .chunks(bytes_per_line)
        .enumerate()
        .map(|(i, chunk)| {
            let hex = chunk
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" ");
            format!("{:04X}: {}", i * bytes_per_line, hex)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ============================================================================
// Comparison Helpers
// ============================================================================

/// Compare packet headers, ignoring sequence and checksum fields
/// Useful for regression testing when sequence is dynamic
pub fn compare_headers_ignore_sequence_checksum(buffer1: &[u8], buffer2: &[u8]) -> bool {
    // Bytes 0-4: sequence (skip)
    // Bytes 4-8: flags (compare)
    // Bytes 8-12: checksum (skip)
    // Bytes 12-20: rest of header (compare)

    if buffer1.len() < 20 || buffer2.len() < 20 {
        return false;
    }

    // Compare flags
    if buffer1[4..8] != buffer2[4..8] {
        return false;
    }

    // Compare rest of header (recipient_id, time_since_last_packet, size, iteration)
    if buffer1[12..20] != buffer2[12..20] {
        return false;
    }

    true
}

/// Compare packet payloads (everything after 20-byte header)
pub fn compare_payloads(buffer1: &[u8], buffer2: &[u8]) -> bool {
    if buffer1.len() < 20 || buffer2.len() < 20 {
        return false;
    }

    let payload1 = &buffer1[20..];
    let payload2 = &buffer2[20..];
    payload1 == payload2
}

/// Regression tests for packet checksum calculation
/// 
/// These tests verify that packet serialization produces correct checksums.
/// Checksums are computed from actual packet data, so they're stable regression indicators.
/// 
/// Golden values in these tests should be verified against:
/// - Known good packets from pcaps
/// - Server responses that accept the packets
/// - Cross-validation with actestclient behavior

use gromnie::client::C2SPacketExt;
use acprotocol::packets::c2s_packet::C2SPacket;
use acprotocol::enums::PacketHeaderFlags;
use byteorder::{ByteOrder, LittleEndian};

// ============================================================================
// Test Helpers
// ============================================================================

/// Extract checksum from a serialized packet buffer
fn extract_checksum(buffer: &[u8]) -> u32 {
    assert!(buffer.len() >= 12, "Buffer too small for checksum field");
    LittleEndian::read_u32(&buffer[8..12])
}

/// Extract flags from a serialized packet buffer
fn extract_flags(buffer: &[u8]) -> u32 {
    assert!(buffer.len() >= 8, "Buffer too small for flags field");
    LittleEndian::read_u32(&buffer[4..8])
}

/// Extract size field from a serialized packet buffer (payload size, not including header)
fn extract_size(buffer: &[u8]) -> u16 {
    assert!(buffer.len() >= 18, "Buffer too small for size field");
    LittleEndian::read_u16(&buffer[16..18])
}

/// Verify basic packet structure invariants
fn verify_packet_structure(buffer: &[u8], expected_flags: u32, expected_payload_size: Option<usize>) {
    assert!(buffer.len() >= 20, "Packet must have at least 20-byte header");
    assert_eq!(buffer.len() % 4, 0, "Packet must be 4-byte aligned");
    
    let flags = extract_flags(buffer);
    assert_eq!(flags, expected_flags, "Flags mismatch");
    
    let size = extract_size(buffer) as usize;
    assert_eq!(buffer.len(), 20 + size, "Total length = header + size");
    
    if let Some(expected_size) = expected_payload_size {
        assert_eq!(size, expected_size, "Payload size mismatch");
    }
}

// ============================================================================
// Test Cases: Simple Packets (No Session Required)
// ============================================================================

#[test]
fn test_timesync_packet_checksum() {
    // TimeSync is a simple packet: just a u64 timestamp in the payload
    // No session, no fragments, no encryption
    
    let packet = C2SPacket {
        sequence: 0,                           // TimeSync uses seq=0
        flags: PacketHeaderFlags::TIME_SYNC,
        checksum: 0,                           // Will be calculated
        recipient_id: 0,
        time_since_last_packet: 0,
        size: 8,                               // u64 timestamp
        iteration: 0,
        server_switch: None,
        retransmit_sequences: None,
        reject_sequences: None,
        ack_sequence: None,
        login_request: None,
        world_login_request: None,
        connect_response: None,
        cicmd_command: None,
        time: Some(1704067200),                // Fixed timestamp for reproducibility
        echo_time: None,
        flow: None,
        fragments: None,
    };
    
    let buffer = packet.serialize(None).expect("Failed to serialize");
    
    verify_packet_structure(&buffer, 
        PacketHeaderFlags::TIME_SYNC.bits(),
        Some(8)  // u64 timestamp
    );
    
    let checksum = extract_checksum(&buffer);
    assert_ne!(checksum, 0, "Checksum should not be zero");
    assert_ne!(checksum, 0xbadd70dd, "Checksum should be calculated, not placeholder");
    
    // Verify this packet serializes deterministically (same input = same output)
    let buffer2 = packet.serialize(None).expect("Failed to serialize second time");
    assert_eq!(buffer, buffer2, "Serialization should be deterministic");
}

#[test]
fn test_timesync_checksum_reproducible() {
    // Verify that TimeSync packets with the same timestamp produce the same checksum
    let make_packet = |time| C2SPacket {
        sequence: 0,
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
        time: Some(time),
        echo_time: None,
        flow: None,
        fragments: None,
    };
    
    let buffer1 = make_packet(1234567890).serialize(None).unwrap();
    let buffer2 = make_packet(1234567890).serialize(None).unwrap();
    
    let checksum1 = extract_checksum(&buffer1);
    let checksum2 = extract_checksum(&buffer2);
    
    assert_eq!(checksum1, checksum2, "Same timestamp should produce same checksum");
}

#[test]
fn test_timesync_checksum_changes_with_different_timestamp() {
    // Verify that different timestamps produce different checksums
    
    let make_packet = |time| C2SPacket {
        sequence: 0,
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
        time: Some(time),
        echo_time: None,
        flow: None,
        fragments: None,
    };
    
    let buffer1 = make_packet(1000).serialize(None).unwrap();
    let buffer2 = make_packet(2000).serialize(None).unwrap();
    
    let checksum1 = extract_checksum(&buffer1);
    let checksum2 = extract_checksum(&buffer2);
    
    assert_ne!(checksum1, checksum2, "Different timestamps should produce different checksums");
}

// ============================================================================
// Test Cases: Packets with Optional Headers
// ============================================================================

#[test]
fn test_packet_with_ack_sequence_checksum() {
    // Test that ACK sequences are included in checksum calculation
    
    let make_packet = |ack_seq: Option<u32>| {
        let mut packet = C2SPacket {
            sequence: 1,
            flags: PacketHeaderFlags::empty(),
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
        };
        
        if let Some(ack) = ack_seq {
            packet = packet.with_ack_sequence(ack);
        }
        
        packet
    };
    
    let buffer_without_ack = make_packet(None).serialize(None).unwrap();
    let buffer_with_ack = make_packet(Some(42)).serialize(None).unwrap();
    
    let checksum_without = extract_checksum(&buffer_without_ack);
    let checksum_with = extract_checksum(&buffer_with_ack);
    
    // Checksums should differ because the optional header changes the packet
    assert_ne!(checksum_without, checksum_with, 
        "Adding ACK sequence should change checksum");
    
    // Packet with ACK should be larger
    assert!(buffer_with_ack.len() > buffer_without_ack.len(), 
        "Packet with ACK should be larger (4 extra bytes)");
}

#[test]
fn test_packet_with_multiple_optional_headers_checksum() {
    // Test that multiple optional headers are checksummed correctly
    // Optional header order matters: ack, world_login, connect_response, time, echo, flow
    
    let packet = C2SPacket {
        sequence: 1,
        flags: PacketHeaderFlags::empty(),
        checksum: 0,
        recipient_id: 0,
        time_since_last_packet: 0,
        size: 0,
        iteration: 0,
        server_switch: None,
        retransmit_sequences: None,
        reject_sequences: None,
        ack_sequence: Some(100),
        login_request: None,
        world_login_request: Some(0xDEADBEEFCAFEBABE),
        connect_response: None,
        cicmd_command: None,
        time: Some(1704067200),
        echo_time: None,
        flow: None,
        fragments: None,
    };
    
    // Update flags to match the fields we set
    let mut packet = packet;
    packet.flags |= PacketHeaderFlags::ACK_SEQUENCE;
    packet.flags |= PacketHeaderFlags::WORLD_LOGIN_REQUEST;
    packet.flags |= PacketHeaderFlags::TIME_SYNC;
    
    let buffer = packet.serialize(None).expect("Failed to serialize");
    
    // Total optional header size should be:
    // ack_sequence: 4, world_login_request: 8, time: 8 = 20 bytes
    let expected_optional_size = 4 + 8 + 8;
    let size = extract_size(&buffer) as usize;
    assert_eq!(size, expected_optional_size, 
        "Size should match sum of optional headers (no payload)");
    
    let checksum = extract_checksum(&buffer);
    assert_ne!(checksum, 0, "Checksum should be calculated");
    assert_ne!(checksum, 0xbadd70dd, "Checksum should not be placeholder");
}

// ============================================================================
// Test Cases: Size Field Regression
// ============================================================================

#[test]
fn test_size_field_auto_calculation() {
    // Verify that the size field is automatically calculated from actual buffer length
    // (not from manually set size field)
    
    let packet = C2SPacket {
        sequence: 0,
        flags: PacketHeaderFlags::TIME_SYNC,
        checksum: 0,
        recipient_id: 0,
        time_since_last_packet: 0,
        size: 9999,  // Intentionally wrong!
        iteration: 0,
        server_switch: None,
        retransmit_sequences: None,
        reject_sequences: None,
        ack_sequence: None,
        login_request: None,
        world_login_request: None,
        connect_response: None,
        cicmd_command: None,
        time: Some(1704067200),
        echo_time: None,
        flow: None,
        fragments: None,
    };
    
    let buffer = packet.serialize(None).expect("Failed to serialize");
    
    // The serialize method should have corrected the size field
    let actual_size = extract_size(&buffer);
    assert_eq!(actual_size, 8, "Size should be auto-corrected to 8 (u64 timestamp)");
    assert_eq!(buffer.len(), 20 + 8, "Total length should be header + corrected size");
}

// ============================================================================
// Test Cases: Checksum Placeholder Detection
// ============================================================================

#[test]
fn test_checksum_never_placeholder() {
    // Regression test: ensure checksum is never left as the placeholder value
    
    let packet = C2SPacket {
        sequence: 0,
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
        time: Some(1704067200),
        echo_time: None,
        flow: None,
        fragments: None,
    };
    
    let buffer = packet.serialize(None).expect("Serialize failed");
    let checksum = extract_checksum(&buffer);
    assert_ne!(checksum, 0xbadd70dd, 
        "Checksum must never be left as placeholder 0xbadd70dd");
}

// ============================================================================
// Test Cases: Deterministic Serialization
// ============================================================================

#[test]
fn test_serialization_determinism() {
    // Verify that serializing the same packet twice produces identical output
    // This is critical for regression testing: same input = same bytes = same checksum
    
    let packet = C2SPacket {
        sequence: 42,
        flags: PacketHeaderFlags::TIME_SYNC | PacketHeaderFlags::ACK_SEQUENCE,
        checksum: 0,
        recipient_id: 5,
        time_since_last_packet: 100,
        size: 8,
        iteration: 3,
        server_switch: None,
        retransmit_sequences: None,
        reject_sequences: None,
        ack_sequence: Some(37),
        login_request: None,
        world_login_request: None,
        connect_response: None,
        cicmd_command: None,
        time: Some(1704067200),
        echo_time: None,
        flow: None,
        fragments: None,
    };
    
    let buffer1 = packet.serialize(None).unwrap();
    let buffer2 = packet.serialize(None).unwrap();
    let buffer3 = packet.serialize(None).unwrap();
    
    assert_eq!(buffer1, buffer2, "First and second serialization should be identical");
    assert_eq!(buffer2, buffer3, "Second and third serialization should be identical");
}

// Note: Tests requiring session/encryption keys are in regression_session_tests.rs

use asheron_rs::enums::PacketHeaderFlags;
use asheron_rs::packets::c2s_packet::C2SPacket;
use gromnie_client::client::C2SPacketExt;

#[allow(dead_code)]
mod common;

use common::{extract_checksum, extract_size, verify_packet_structure};

#[test]
fn test_timesync_packet_checksum() {
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

    let buffer = packet.serialize(None).expect("Failed to serialize");

    verify_packet_structure(&buffer, PacketHeaderFlags::TIME_SYNC.bits(), Some(8));

    let checksum = extract_checksum(&buffer);
    assert_ne!(checksum, 0, "Checksum should not be zero");
    assert_ne!(
        checksum, 0xbadd70dd,
        "Checksum should be calculated, not placeholder"
    );

    let buffer2 = packet
        .serialize(None)
        .expect("Failed to serialize second time");
    assert_eq!(buffer, buffer2, "Serialization should be deterministic");
}

#[test]
fn test_timesync_checksum_reproducible() {
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

    assert_eq!(
        checksum1, checksum2,
        "Same timestamp should produce same checksum"
    );
}

#[test]
fn test_timesync_checksum_changes_with_different_timestamp() {
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

    assert_ne!(
        checksum1, checksum2,
        "Different timestamps should produce different checksums"
    );
}

#[test]
fn test_packet_with_ack_sequence_checksum() {
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

    assert_ne!(
        checksum_without, checksum_with,
        "Adding ACK sequence should change checksum"
    );

    assert!(
        buffer_with_ack.len() > buffer_without_ack.len(),
        "Packet with ACK should be larger (4 extra bytes)"
    );
}

#[test]
fn test_packet_with_multiple_optional_headers_checksum() {
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

    let mut packet = packet;
    packet.flags |= PacketHeaderFlags::ACK_SEQUENCE;
    packet.flags |= PacketHeaderFlags::WORLD_LOGIN_REQUEST;
    packet.flags |= PacketHeaderFlags::TIME_SYNC;

    let buffer = packet.serialize(None).expect("Failed to serialize");

    let expected_optional_size = 4 + 8 + 8;
    let size = extract_size(&buffer) as usize;
    assert_eq!(
        size, expected_optional_size,
        "Size should match sum of optional headers (no payload)"
    );

    let checksum = extract_checksum(&buffer);
    assert_ne!(checksum, 0, "Checksum should be calculated");
    assert_ne!(checksum, 0xbadd70dd, "Checksum should not be placeholder");
}

#[test]
fn test_size_field_auto_calculation() {
    let packet = C2SPacket {
        sequence: 0,
        flags: PacketHeaderFlags::TIME_SYNC,
        checksum: 0,
        recipient_id: 0,
        time_since_last_packet: 0,
        size: 9999,
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

    let actual_size = extract_size(&buffer);
    assert_eq!(
        actual_size, 8,
        "Size should be auto-corrected to 8 (u64 timestamp)"
    );
    assert_eq!(
        buffer.len(),
        20 + 8,
        "Total length should be header + corrected size"
    );
}

#[test]
fn test_checksum_never_placeholder() {
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
    assert_ne!(
        checksum, 0xbadd70dd,
        "Checksum must never be left as placeholder 0xbadd70dd"
    );
}

#[test]
fn test_serialization_determinism() {
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

    assert_eq!(
        buffer1, buffer2,
        "First and second serialization should be identical"
    );
    assert_eq!(
        buffer2, buffer3,
        "Second and third serialization should be identical"
    );
}

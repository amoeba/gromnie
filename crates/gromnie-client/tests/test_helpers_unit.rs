#[allow(dead_code)]
mod common;

use asheron_rs::enums::PacketHeaderFlags;

#[test]
fn test_field_extraction_helpers() {
    let mut buffer = vec![0u8; 24];

    buffer[0..4].copy_from_slice(&42u32.to_le_bytes());
    buffer[4..8].copy_from_slice(&PacketHeaderFlags::TIME_SYNC.bits().to_le_bytes());
    buffer[8..12].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());
    buffer[12..14].copy_from_slice(&5u16.to_le_bytes());
    buffer[14..16].copy_from_slice(&100u16.to_le_bytes());
    buffer[16..18].copy_from_slice(&4u16.to_le_bytes());
    buffer[18..20].copy_from_slice(&3u16.to_le_bytes());

    assert_eq!(common::extract_sequence(&buffer), 42);
    assert_eq!(
        common::extract_flags(&buffer),
        PacketHeaderFlags::TIME_SYNC.bits()
    );
    assert_eq!(common::extract_checksum(&buffer), 0xDEADBEEF);
    assert_eq!(common::extract_recipient_id(&buffer), 5);
    assert_eq!(common::extract_time_since_last_packet(&buffer), 100);
    assert_eq!(common::extract_size(&buffer), 4);
    assert_eq!(common::extract_iteration(&buffer), 3);
}

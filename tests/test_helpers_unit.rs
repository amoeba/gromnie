/// Unit tests for test helpers
///
/// These tests verify that the helper functions used in other tests work correctly.
mod helpers {
    use acprotocol::enums::PacketHeaderFlags;
    use byteorder::{ByteOrder, LittleEndian};

    // ============================================================================
    // Field Extraction Helpers (copied from test_helpers.rs for testing)
    // ============================================================================

    fn extract_sequence(buffer: &[u8]) -> u32 {
        assert!(buffer.len() >= 4, "Buffer too small for sequence field");
        LittleEndian::read_u32(&buffer[0..4])
    }

    fn extract_flags(buffer: &[u8]) -> u32 {
        assert!(buffer.len() >= 8, "Buffer too small for flags field");
        LittleEndian::read_u32(&buffer[4..8])
    }

    fn extract_checksum(buffer: &[u8]) -> u32 {
        assert!(buffer.len() >= 12, "Buffer too small for checksum field");
        LittleEndian::read_u32(&buffer[8..12])
    }

    fn extract_recipient_id(buffer: &[u8]) -> u16 {
        assert!(
            buffer.len() >= 14,
            "Buffer too small for recipient_id field"
        );
        LittleEndian::read_u16(&buffer[12..14])
    }

    fn extract_time_since_last_packet(buffer: &[u8]) -> u16 {
        assert!(
            buffer.len() >= 16,
            "Buffer too small for time_since_last_packet field"
        );
        LittleEndian::read_u16(&buffer[14..16])
    }

    fn extract_size(buffer: &[u8]) -> u16 {
        assert!(buffer.len() >= 18, "Buffer too small for size field");
        LittleEndian::read_u16(&buffer[16..18])
    }

    fn extract_iteration(buffer: &[u8]) -> u16 {
        assert!(buffer.len() >= 20, "Buffer too small for iteration field");
        LittleEndian::read_u16(&buffer[18..20])
    }

    #[test]
    fn test_field_extraction_helpers() {
        // Build a test buffer
        let mut buffer = vec![0u8; 24];

        // Write sequence (0-4)
        buffer[0..4].copy_from_slice(&42u32.to_le_bytes());

        // Write flags (4-8)
        buffer[4..8].copy_from_slice(&PacketHeaderFlags::TIME_SYNC.bits().to_le_bytes());

        // Write checksum (8-12)
        buffer[8..12].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());

        // Write recipient_id (12-14)
        buffer[12..14].copy_from_slice(&5u16.to_le_bytes());

        // Write time_since_last_packet (14-16)
        buffer[14..16].copy_from_slice(&100u16.to_le_bytes());

        // Write size (16-18)
        buffer[16..18].copy_from_slice(&4u16.to_le_bytes());

        // Write iteration (18-20)
        buffer[18..20].copy_from_slice(&3u16.to_le_bytes());

        // Verify extractions
        assert_eq!(extract_sequence(&buffer), 42);
        assert_eq!(extract_flags(&buffer), PacketHeaderFlags::TIME_SYNC.bits());
        assert_eq!(extract_checksum(&buffer), 0xDEADBEEF);
        assert_eq!(extract_recipient_id(&buffer), 5);
        assert_eq!(extract_time_since_last_packet(&buffer), 100);
        assert_eq!(extract_size(&buffer), 4);
        assert_eq!(extract_iteration(&buffer), 3);
    }
}

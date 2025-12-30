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

    /// Safely set the ACK sequence, ensuring both the field and flag are set together.
    fn with_ack_sequence(self, ack_seq: u32) -> Self;

    /// Safely set the server switch header, ensuring both the field and flag are set together.
    fn with_server_switch(self, header: acprotocol::types::ServerSwitchHeader) -> Self;

    /// Safely set retransmit sequences, ensuring both the field and flag are set together.
    fn with_retransmit_sequences(self, sequences: acprotocol::types::PackableList<u32>) -> Self;

    /// Safely set reject sequences, ensuring both the field and flag are set together.
    fn with_reject_sequences(self, sequences: acprotocol::types::PackableList<u32>) -> Self;

    /// Safely set the login request header, ensuring both the field and flag are set together.
    fn with_login_request(self, header: acprotocol::types::LoginRequestHeader) -> Self;

    /// Safely set the world login request, ensuring both the field and flag are set together.
    fn with_world_login_request(self, value: u64) -> Self;

    /// Safely set the connect response (cookie), ensuring both the field and flag are set together.
    fn with_connect_response(self, cookie: u64) -> Self;

    /// Safely set the CICMD command header, ensuring both the field and flag are set together.
    fn with_cicmd_command(self, header: acprotocol::types::CICMDCommandHeader) -> Self;

    /// Safely set the time sync value, ensuring both the field and flag are set together.
    fn with_time_sync(self, time: u64) -> Self;

    /// Safely set the echo request time, ensuring both the field and flag are set together.
    fn with_echo_request(self, time: f32) -> Self;

    /// Safely set the echo response time, ensuring both the field and flag are set together.
    fn with_echo_response(self, time: f32) -> Self;

    /// Safely set the flow header, ensuring both the field and flag are set together.
    fn with_flow(self, flow: acprotocol::types::FlowHeader) -> Self;

    /// Safely set the blob fragments, ensuring both the field and flag are set together.
    fn with_fragments(self, fragments: acprotocol::types::BlobFragments) -> Self;

    /// Validate that the packet is properly formed (optional fields match their flags).
    fn validate(&self) -> Result<(), &'static str>;

    /// Helper to set a field and its corresponding flag atomically
    fn set_field_with_flag<T>(
        self,
        field_setter: impl FnOnce(&mut Self) -> &mut Option<T>,
        value: T,
        flag: PacketHeaderFlags,
    ) -> Self;
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

    /// Safely set the ACK sequence, ensuring both the field and flag are set together.
    /// This prevents bugs where ack_sequence is Some but the ACK_SEQUENCE flag is missing.
    ///
    /// # Example
    /// ```ignore
    /// let packet = C2SPacket::default()
    ///     .with_ack_sequence(1234);
    /// ```
    fn with_ack_sequence(self, ack_seq: u32) -> Self {
        self.set_field_with_flag(
            |p| &mut p.ack_sequence,
            ack_seq,
            PacketHeaderFlags::ACK_SEQUENCE,
        )
    }

    fn with_server_switch(self, header: acprotocol::types::ServerSwitchHeader) -> Self {
        self.set_field_with_flag(
            |p| &mut p.server_switch,
            header,
            PacketHeaderFlags::SERVER_SWITCH,
        )
    }

    fn with_retransmit_sequences(self, sequences: acprotocol::types::PackableList<u32>) -> Self {
        self.set_field_with_flag(
            |p| &mut p.retransmit_sequences,
            sequences,
            PacketHeaderFlags::REQUEST_RETRANSMIT,
        )
    }

    fn with_reject_sequences(self, sequences: acprotocol::types::PackableList<u32>) -> Self {
        self.set_field_with_flag(
            |p| &mut p.reject_sequences,
            sequences,
            PacketHeaderFlags::REJECT_RETRANSMIT,
        )
    }

    fn with_login_request(self, header: acprotocol::types::LoginRequestHeader) -> Self {
        self.set_field_with_flag(
            |p| &mut p.login_request,
            header,
            PacketHeaderFlags::LOGIN_REQUEST,
        )
    }

    fn with_world_login_request(self, value: u64) -> Self {
        self.set_field_with_flag(
            |p| &mut p.world_login_request,
            value,
            PacketHeaderFlags::WORLD_LOGIN_REQUEST,
        )
    }

    fn with_connect_response(self, cookie: u64) -> Self {
        self.set_field_with_flag(
            |p| &mut p.connect_response,
            cookie,
            PacketHeaderFlags::CONNECT_RESPONSE,
        )
    }

    fn with_cicmd_command(self, header: acprotocol::types::CICMDCommandHeader) -> Self {
        self.set_field_with_flag(
            |p| &mut p.cicmd_command,
            header,
            PacketHeaderFlags::CICMDCOMMAND,
        )
    }

    fn with_time_sync(self, time: u64) -> Self {
        self.set_field_with_flag(|p| &mut p.time, time, PacketHeaderFlags::TIME_SYNC)
    }

    fn with_echo_request(self, time: f32) -> Self {
        self.set_field_with_flag(|p| &mut p.echo_time, time, PacketHeaderFlags::ECHO_REQUEST)
    }

    fn with_echo_response(self, time: f32) -> Self {
        self.set_field_with_flag(|p| &mut p.echo_time, time, PacketHeaderFlags::ECHO_RESPONSE)
    }

    fn with_flow(self, flow: acprotocol::types::FlowHeader) -> Self {
        self.set_field_with_flag(|p| &mut p.flow, flow, PacketHeaderFlags::FLOW)
    }

    fn with_fragments(self, fragments: acprotocol::types::BlobFragments) -> Self {
        self.set_field_with_flag(
            |p| &mut p.fragments,
            fragments,
            PacketHeaderFlags::BLOB_FRAGMENTS,
        )
    }

    /// Helper to set a field and its corresponding flag atomically
    #[inline]
    fn set_field_with_flag<T>(
        mut self,
        field_setter: impl FnOnce(&mut Self) -> &mut Option<T>,
        value: T,
        flag: PacketHeaderFlags,
    ) -> Self {
        *field_setter(&mut self) = Some(value);
        self.flags |= flag;
        self
    }

    /// Validate that the packet is properly formed.
    /// Checks that ack_sequence and ACK_SEQUENCE flag are in sync.
    ///
    /// Returns an error if:
    /// - ack_sequence is Some but ACK_SEQUENCE flag is not set
    /// - ack_sequence is None but ACK_SEQUENCE flag is set
    fn validate(&self) -> Result<(), &'static str> {
        let has_ack_field = self.ack_sequence.is_some();
        let has_ack_flag = self.flags.contains(PacketHeaderFlags::ACK_SEQUENCE);

        if has_ack_field && !has_ack_flag {
            return Err("ack_sequence is Some but ACK_SEQUENCE flag is not set");
        }
        if !has_ack_field && has_ack_flag {
            return Err("ACK_SEQUENCE flag is set but ack_sequence is None");
        }

        Ok(())
    }

    fn serialize(&self, session: Option<&SessionState>) -> Result<Vec<u8>, std::io::Error> {
        // Validate packet before serialization (debug only - catches programming errors)
        debug_assert!(
            self.validate().is_ok(),
            "Packet validation failed: {:?}",
            self.validate().unwrap_err()
        );

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

        // IMPORTANT: Automatically recalculate size field from actual buffer contents
        // This ensures the size is always correct regardless of what was set during packet construction
        // Size = total payload after header (includes optional headers + any message data)
        let actual_payload_size = buffer.len().saturating_sub(PACKET_HEADER_SIZE);
        buffer[16..18].copy_from_slice(&(actual_payload_size as u16).to_le_bytes());

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
            let remaining = header_size.saturating_sub(option_size);
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
            if self.flags.contains(PacketHeaderFlags::BLOB_FRAGMENTS)
                && let Some(sess) = session
            {
                let encryption_key = sess.send_generator.borrow_mut().get_send_key();
                checksum_result ^= encryption_key;
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

use acprotocol::writers::{write_i32, write_string, write_u32, ACWritable, ACWriter};

/// Custom LoginRequest structure that matches the actual C# client implementation.
/// This is needed because acprotocol's LoginRequestHeaderType2 is missing the timestamp field.
#[derive(Clone, Debug)]
pub struct CustomLoginRequest {
    pub client_version: String,
    pub length: u32,
    pub login_type: u32, // Password authentication type
    pub unknown: u32,    // Always 0
    pub timestamp: i32,  // Unix timestamp
    pub account: String,
    pub password: String, // Raw password string (not WString)
}

impl ACWritable for CustomLoginRequest {
    fn write(&self, writer: &mut dyn ACWriter) -> Result<(), Box<dyn std::error::Error>> {
        // Write client_version string (AC format: i16 length + data + padding to 4-byte alignment)
        write_string(writer, &self.client_version)?;

        // Write length field
        write_u32(writer, self.length)?;

        // Write login type (2 for password)
        write_u32(writer, self.login_type)?;

        // Write unknown (always 0)
        write_u32(writer, self.unknown)?;

        // Write timestamp
        write_i32(writer, self.timestamp)?;

        // Write account name
        write_string(writer, &self.account)?;

        // Write account_to_login_as (always empty = 4 zero bytes for u32)
        write_u32(writer, 0)?;

        // Write password in C# format (NOT WString):
        // 1. 4-byte int: length of (packed_byte + string_data)
        // 2. 1-byte packed length
        // 3. char array data
        // 4. padding to 4-byte alignment
        let password_len = self.password.len();
        let packed_byte_size = if password_len > 255 { 2 } else { 1 };
        let total_data_len = packed_byte_size + password_len;

        write_u32(writer, total_data_len as u32)?;

        if password_len <= 255 {
            writer.write_all(&[password_len as u8])?;
        } else {
            // 2-byte packed length for strings > 255
            let high_byte = ((password_len >> 8) as u8) | 0x80;
            let low_byte = (password_len & 0xFF) as u8;
            writer.write_all(&[high_byte, low_byte])?;
        }

        // Write password chars
        writer.write_all(self.password.as_bytes())?;

        // Write alignment padding if needed
        let padding = (4 - (total_data_len % 4)) % 4;
        if padding > 0 {
            writer.write_all(&vec![0u8; padding])?;
        }

        Ok(())
    }
}

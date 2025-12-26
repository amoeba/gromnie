use rand::SeedableRng;
use rand_isaac::IsaacRng;

/// Wrapper around ISAAC random number generator for checksum encryption
/// Mirrors the C# CryptoSystem class used in actestclient
#[derive(Clone, Debug)]
pub struct CryptoSystem {
    rng: IsaacRng,
}

impl CryptoSystem {
    /// Create a new CryptoSystem with the given seed
    /// The seed is used to initialize the ISAAC RNG
    pub fn new(seed: u32) -> Self {
        // Convert u32 seed to [u8; 32] for IsaacRng::from_seed
        // Replicate the seed across the byte array
        let mut seed_array = [0u8; 32];
        seed_array[0..4].copy_from_slice(&seed.to_le_bytes());
        seed_array[4..8].copy_from_slice(&seed.to_le_bytes());
        seed_array[8..12].copy_from_slice(&seed.to_le_bytes());
        seed_array[12..16].copy_from_slice(&seed.to_le_bytes());
        seed_array[16..20].copy_from_slice(&seed.to_le_bytes());
        seed_array[20..24].copy_from_slice(&seed.to_le_bytes());
        seed_array[24..28].copy_from_slice(&seed.to_le_bytes());
        seed_array[28..32].copy_from_slice(&seed.to_le_bytes());
        
        let rng = IsaacRng::from_seed(seed_array);
        CryptoSystem { rng }
    }

    /// Get the next encryption key from the ISAAC sequence
    /// This advances the RNG state, so each call produces a different value
    pub fn get_send_key(&mut self) -> u32 {
        use rand::Rng;
        self.rng.gen::<u32>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypto_system_sequence() {
        let mut crypto = CryptoSystem::new(0x12345678);
        
        // Each call should produce a different value
        let key1 = crypto.get_send_key();
        let key2 = crypto.get_send_key();
        let key3 = crypto.get_send_key();
        
        assert_ne!(key1, key2);
        assert_ne!(key2, key3);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_crypto_system_deterministic() {
        let mut crypto1 = CryptoSystem::new(0x12345678);
        let mut crypto2 = CryptoSystem::new(0x12345678);
        
        // Same seed should produce same sequence
        assert_eq!(crypto1.get_send_key(), crypto2.get_send_key());
        assert_eq!(crypto1.get_send_key(), crypto2.get_send_key());
        assert_eq!(crypto1.get_send_key(), crypto2.get_send_key());
    }
}

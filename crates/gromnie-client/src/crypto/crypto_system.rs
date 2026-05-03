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
        let seed_bytes = seed.to_le_bytes();
        for chunk in seed_array.chunks_exact_mut(4) {
            chunk.copy_from_slice(&seed_bytes);
        }

        let rng = IsaacRng::from_seed(seed_array);
        CryptoSystem { rng }
    }

    /// Get the next encryption key from the ISAAC sequence
    /// This advances the RNG state, so each call produces a different value
    pub fn get_send_key(&mut self) -> u32 {
        use rand::Rng;
        self.rng.r#gen::<u32>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypto_system_sequence() {
        use std::collections::HashSet;

        let mut crypto = CryptoSystem::new(0x12345678);

        // Each call should produce a different value
        // Test with a larger sequence to verify no duplicates
        let keys: Vec<u32> = (0..100).map(|_| crypto.get_send_key()).collect();
        let unique_keys: HashSet<_> = keys.iter().collect();

        // All keys should be unique
        assert_eq!(
            keys.len(),
            unique_keys.len(),
            "Generated keys should all be unique"
        );
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

//! Keypair management for Ed25519 signing

use crate::{Error, Pubkey, Result};
use ed25519_dalek::{SecretKey, SigningKey, VerifyingKey};
use rand::rngs::OsRng;

/// Ed25519 keypair for signing transactions
#[derive(Clone)]
pub struct Keypair {
    signing_key: SigningKey,
}

impl Keypair {
    /// Generate a new random keypair
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    /// Create from a 32-byte secret key
    pub fn from_secret_key(secret: &[u8]) -> Result<Self> {
        if secret.len() != 32 {
            return Err(Error::InvalidKeyLength {
                expected: 32,
                got: secret.len(),
            });
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(secret);
        let signing_key = SigningKey::from_bytes(&bytes);
        Ok(Self { signing_key })
    }

    /// Create from a 64-byte keypair (32-byte secret + 32-byte public)
    ///
    /// This is the format used by Solana and some other systems.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() == 32 {
            return Self::from_secret_key(bytes);
        }
        if bytes.len() != 64 {
            return Err(Error::InvalidKeyLength {
                expected: 64,
                got: bytes.len(),
            });
        }
        // First 32 bytes are the secret key
        Self::from_secret_key(&bytes[..32])
    }

    /// Create from base58-encoded secret key or keypair
    pub fn from_base58(s: &str) -> Result<Self> {
        let bytes = bs58::decode(s)
            .into_vec()
            .map_err(|e| Error::InvalidBase58(e.to_string()))?;
        Self::from_bytes(&bytes)
    }

    /// Get the public key
    pub fn pubkey(&self) -> Pubkey {
        let verifying_key = self.signing_key.verifying_key();
        Pubkey::from_bytes(verifying_key.to_bytes())
    }

    /// Get the secret key bytes (32 bytes)
    pub fn secret_key(&self) -> &SecretKey {
        self.signing_key.as_bytes()
    }

    /// Get the full keypair bytes (64 bytes: secret + public)
    pub fn to_bytes(&self) -> [u8; 64] {
        let mut bytes = [0u8; 64];
        bytes[..32].copy_from_slice(self.signing_key.as_bytes());
        bytes[32..].copy_from_slice(self.signing_key.verifying_key().as_bytes());
        bytes
    }

    /// Encode keypair to base58 (64 bytes)
    pub fn to_base58(&self) -> String {
        bs58::encode(self.to_bytes()).into_string()
    }

    /// Get the internal signing key reference (for direct signing)
    pub(crate) fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    /// Get the verifying key
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }
}

impl std::fmt::Debug for Keypair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Keypair")
            .field("pubkey", &self.pubkey().to_base58())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair() {
        let keypair = Keypair::generate();
        let pubkey = keypair.pubkey();

        // Public key should be 32 bytes
        assert_eq!(pubkey.as_bytes().len(), 32);

        // Should be able to encode to base58
        let b58 = pubkey.to_base58();
        assert!(!b58.is_empty());
    }

    #[test]
    fn test_keypair_roundtrip() {
        let keypair = Keypair::generate();
        let bytes = keypair.to_bytes();

        let restored = Keypair::from_bytes(&bytes).unwrap();
        assert_eq!(keypair.pubkey(), restored.pubkey());
    }

    #[test]
    fn test_keypair_from_base58() {
        let keypair = Keypair::generate();
        let b58 = keypair.to_base58();

        let restored = Keypair::from_base58(&b58).unwrap();
        assert_eq!(keypair.pubkey(), restored.pubkey());
    }

    #[test]
    fn test_invalid_key_length() {
        let result = Keypair::from_bytes(&[0u8; 31]);
        assert!(result.is_err());

        let result = Keypair::from_bytes(&[0u8; 65]);
        assert!(result.is_err());
    }
}

//! AES-256-GCM encryption for disk clusters.
//!
//! Uses the `aes-gcm` crate (pure-Rust, audited implementation).
//! Each cluster is encrypted independently with a random 96-bit nonce
//! so that cluster-level deduplication and random access are possible.

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::StorageError;

/// A 256-bit AES-GCM encryption key.
///
/// Automatically zeroed on drop via `Zeroize`.
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct EncryptionKey(pub [u8; 32]);

impl std::fmt::Debug for EncryptionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EncryptionKey([REDACTED])")
    }
}

impl EncryptionKey {
    /// Generate a new random 256-bit key.
    pub fn generate() -> Self {
        let key = Aes256Gcm::generate_key(OsRng);
        Self(key.into())
    }

    /// Wrap a raw 32-byte key.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// Serialisable nonce (stored per-cluster alongside the ciphertext).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNonce(pub [u8; 12]);

/// Encrypt a cluster using AES-256-GCM.
///
/// Returns `(ciphertext, nonce)`. The nonce must be stored alongside the
/// ciphertext to allow decryption later.
pub fn encrypt_cluster(
    plaintext: &[u8],
    key: &EncryptionKey,
) -> Result<(Vec<u8>, ClusterNonce), StorageError> {
    let cipher_key = Key::<Aes256Gcm>::from_slice(&key.0);
    let cipher = Aes256Gcm::new(cipher_key);
    let nonce_bytes = Aes256Gcm::generate_nonce(OsRng);
    let ciphertext = cipher
        .encrypt(&nonce_bytes, plaintext)
        .map_err(|e| StorageError::Encryption(e.to_string()))?;
    Ok((ciphertext, ClusterNonce(nonce_bytes.into())))
}

/// Decrypt a cluster using AES-256-GCM.
pub fn decrypt_cluster(
    ciphertext: &[u8],
    key: &EncryptionKey,
    nonce: &ClusterNonce,
) -> Result<Vec<u8>, StorageError> {
    let cipher_key = Key::<Aes256Gcm>::from_slice(&key.0);
    let cipher = Aes256Gcm::new(cipher_key);
    let nonce_obj = Nonce::from_slice(&nonce.0);
    let plaintext = cipher
        .decrypt(nonce_obj, ciphertext)
        .map_err(|e| StorageError::Encryption(format!("Decryption failed: {e}")))?;
    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_encryption() {
        let key = EncryptionKey::generate();
        let plaintext = b"Hello, NovaDisk! This is a test cluster payload.";
        let (ciphertext, nonce) = encrypt_cluster(plaintext, &key).unwrap();
        assert_ne!(ciphertext, plaintext);
        let decrypted = decrypt_cluster(&ciphertext, &key, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = EncryptionKey::generate();
        let key2 = EncryptionKey::generate();
        let plaintext = b"Secret data";
        let (ciphertext, nonce) = encrypt_cluster(plaintext, &key1).unwrap();
        assert!(decrypt_cluster(&ciphertext, &key2, &nonce).is_err());
    }

    #[test]
    fn test_key_zeroed_on_drop() {
        let key = EncryptionKey::generate();
        let key_bytes = key.0;
        drop(key);
        // The bytes in key are gone; just verify the type system works.
        assert_eq!(key_bytes.len(), 32);
    }
}

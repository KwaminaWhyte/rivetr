//! Cryptographic utilities for encrypting sensitive data.
//!
//! This module provides AES-256-GCM encryption for environment variables
//! and other sensitive data stored in the database.
//!
//! The encryption format is: base64(nonce || ciphertext || tag)
//! where nonce is 12 bytes, and tag is 16 bytes (AES-GCM authentication tag).

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ring::pbkdf2;
use std::num::NonZeroU32;

/// The length of the AES-256 key in bytes
const KEY_LENGTH: usize = 32;

/// The length of the AES-GCM nonce in bytes
const NONCE_LENGTH: usize = 12;

/// Number of PBKDF2 iterations for key derivation
const PBKDF2_ITERATIONS: u32 = 100_000;

/// Salt for PBKDF2 key derivation (fixed salt is acceptable here since we have a unique secret)
const PBKDF2_SALT: &[u8] = b"rivetr-env-var-encryption-v1";

/// Prefix added to encrypted values to identify them as encrypted
/// This allows backwards compatibility with unencrypted values
pub const ENCRYPTED_PREFIX: &str = "ENC:";

/// Derive a 256-bit encryption key from a secret string using PBKDF2.
///
/// This function takes a human-readable secret (like a passphrase or API key)
/// and derives a cryptographically strong 256-bit key suitable for AES-256-GCM.
///
/// # Arguments
/// * `secret` - The secret string to derive the key from
///
/// # Returns
/// A 32-byte array suitable for use as an AES-256 key
pub fn derive_key(secret: &str) -> [u8; KEY_LENGTH] {
    let mut key = [0u8; KEY_LENGTH];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        NonZeroU32::new(PBKDF2_ITERATIONS).unwrap(),
        PBKDF2_SALT,
        secret.as_bytes(),
        &mut key,
    );
    key
}

/// Encrypt a plaintext string using AES-256-GCM.
///
/// The output format is: `ENC:` prefix + base64(nonce || ciphertext || tag)
///
/// # Arguments
/// * `plaintext` - The string to encrypt
/// * `key` - The 32-byte AES-256 key (use `derive_key` to generate from a secret)
///
/// # Returns
/// A base64-encoded string containing the nonce and ciphertext, prefixed with "ENC:"
///
/// # Errors
/// Returns an error if encryption fails
pub fn encrypt(plaintext: &str, key: &[u8; KEY_LENGTH]) -> Result<String> {
    use rand::RngCore;

    // Generate a random 12-byte nonce
    let mut nonce_bytes = [0u8; NONCE_LENGTH];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Create the cipher
    let cipher = Aes256Gcm::new_from_slice(key).context("Failed to create cipher")?;

    // Encrypt the plaintext
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    // Combine nonce and ciphertext: nonce || ciphertext
    let mut combined = Vec::with_capacity(NONCE_LENGTH + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    // Base64 encode and add prefix
    let encoded = BASE64.encode(&combined);
    Ok(format!("{}{}", ENCRYPTED_PREFIX, encoded))
}

/// Decrypt a ciphertext string using AES-256-GCM.
///
/// # Arguments
/// * `ciphertext` - The encrypted string (with "ENC:" prefix and base64-encoded)
/// * `key` - The 32-byte AES-256 key (must be the same key used for encryption)
///
/// # Returns
/// The decrypted plaintext string
///
/// # Errors
/// Returns an error if:
/// - The ciphertext doesn't have the expected "ENC:" prefix
/// - The base64 decoding fails
/// - The ciphertext is too short
/// - The decryption or authentication fails
pub fn decrypt(ciphertext: &str, key: &[u8; KEY_LENGTH]) -> Result<String> {
    // Remove the prefix
    let encoded = ciphertext
        .strip_prefix(ENCRYPTED_PREFIX)
        .context("Ciphertext doesn't have expected prefix")?;

    // Base64 decode
    let combined = BASE64
        .decode(encoded)
        .context("Failed to decode base64")?;

    // Extract nonce and ciphertext
    if combined.len() < NONCE_LENGTH + 1 {
        anyhow::bail!("Ciphertext too short");
    }

    let (nonce_bytes, ciphertext_bytes) = combined.split_at(NONCE_LENGTH);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Create the cipher and decrypt
    let cipher = Aes256Gcm::new_from_slice(key).context("Failed to create cipher")?;
    let plaintext = cipher
        .decrypt(nonce, ciphertext_bytes)
        .map_err(|e| anyhow::anyhow!("Decryption failed (wrong key or corrupted data): {}", e))?;

    String::from_utf8(plaintext).context("Decrypted data is not valid UTF-8")
}

/// Check if a value is encrypted (has the ENC: prefix).
///
/// This is used for backwards compatibility - existing unencrypted values
/// will not have this prefix and can be used as-is.
pub fn is_encrypted(value: &str) -> bool {
    value.starts_with(ENCRYPTED_PREFIX)
}

/// Decrypt a value if it's encrypted, otherwise return as-is.
///
/// This provides backwards compatibility for existing unencrypted values.
///
/// # Arguments
/// * `value` - The value to potentially decrypt
/// * `key` - Optional encryption key. If None, returns value as-is.
///
/// # Returns
/// The decrypted value if it was encrypted and key is provided, otherwise the original value
pub fn decrypt_if_encrypted(value: &str, key: Option<&[u8; KEY_LENGTH]>) -> Result<String> {
    match (is_encrypted(value), key) {
        (true, Some(k)) => decrypt(value, k),
        (true, None) => {
            // Value is encrypted but no key provided - this is an error
            anyhow::bail!("Value is encrypted but no encryption key is configured")
        }
        (false, _) => Ok(value.to_string()),
    }
}

/// Encrypt a value if encryption key is provided, otherwise return as-is.
///
/// # Arguments
/// * `value` - The plaintext value to encrypt
/// * `key` - Optional encryption key. If None, returns value as-is.
///
/// # Returns
/// The encrypted value if key is provided, otherwise the original value
pub fn encrypt_if_key_available(value: &str, key: Option<&[u8; KEY_LENGTH]>) -> Result<String> {
    match key {
        Some(k) => encrypt(value, k),
        None => Ok(value.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key_consistent() {
        let key1 = derive_key("my-secret-key");
        let key2 = derive_key("my-secret-key");
        assert_eq!(key1, key2, "Same secret should derive same key");
    }

    #[test]
    fn test_derive_key_different_secrets() {
        let key1 = derive_key("secret1");
        let key2 = derive_key("secret2");
        assert_ne!(key1, key2, "Different secrets should derive different keys");
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = derive_key("test-encryption-key");
        let plaintext = "my-database-password-123!@#";

        let encrypted = encrypt(plaintext, &key).unwrap();
        assert!(encrypted.starts_with(ENCRYPTED_PREFIX));
        assert_ne!(encrypted, plaintext);

        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_produces_different_ciphertext() {
        // Due to random nonce, encrypting the same plaintext twice should produce different ciphertext
        let key = derive_key("test-key");
        let plaintext = "same-plaintext";

        let encrypted1 = encrypt(plaintext, &key).unwrap();
        let encrypted2 = encrypt(plaintext, &key).unwrap();

        assert_ne!(encrypted1, encrypted2, "Random nonce should produce different ciphertext");

        // But both should decrypt to the same value
        assert_eq!(decrypt(&encrypted1, &key).unwrap(), plaintext);
        assert_eq!(decrypt(&encrypted2, &key).unwrap(), plaintext);
    }

    #[test]
    fn test_decrypt_with_wrong_key_fails() {
        let key1 = derive_key("correct-key");
        let key2 = derive_key("wrong-key");
        let plaintext = "secret-value";

        let encrypted = encrypt(plaintext, &key1).unwrap();
        let result = decrypt(&encrypted, &key2);

        assert!(result.is_err(), "Decryption with wrong key should fail");
    }

    #[test]
    fn test_is_encrypted() {
        assert!(is_encrypted("ENC:some-base64-data"));
        assert!(!is_encrypted("plain-text-value"));
        assert!(!is_encrypted(""));
    }

    #[test]
    fn test_decrypt_if_encrypted_with_plaintext() {
        let key = derive_key("test-key");
        let plaintext = "not-encrypted-value";

        let result = decrypt_if_encrypted(plaintext, Some(&key)).unwrap();
        assert_eq!(result, plaintext);
    }

    #[test]
    fn test_decrypt_if_encrypted_with_encrypted() {
        let key = derive_key("test-key");
        let plaintext = "secret-value";
        let encrypted = encrypt(plaintext, &key).unwrap();

        let result = decrypt_if_encrypted(&encrypted, Some(&key)).unwrap();
        assert_eq!(result, plaintext);
    }

    #[test]
    fn test_decrypt_if_encrypted_no_key_unencrypted() {
        let plaintext = "plain-value";
        let result = decrypt_if_encrypted(plaintext, None).unwrap();
        assert_eq!(result, plaintext);
    }

    #[test]
    fn test_decrypt_if_encrypted_no_key_encrypted() {
        let key = derive_key("test-key");
        let encrypted = encrypt("secret", &key).unwrap();

        let result = decrypt_if_encrypted(&encrypted, None);
        assert!(result.is_err(), "Should fail when encrypted value has no key");
    }

    #[test]
    fn test_encrypt_if_key_available() {
        let key = derive_key("test-key");
        let plaintext = "my-secret";

        // With key - should encrypt
        let result = encrypt_if_key_available(plaintext, Some(&key)).unwrap();
        assert!(result.starts_with(ENCRYPTED_PREFIX));

        // Without key - should pass through
        let result = encrypt_if_key_available(plaintext, None).unwrap();
        assert_eq!(result, plaintext);
    }

    #[test]
    fn test_empty_string_encryption() {
        let key = derive_key("test-key");
        let encrypted = encrypt("", &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, "");
    }

    #[test]
    fn test_unicode_encryption() {
        let key = derive_key("test-key");
        let plaintext = "Hello, World! Привет мир! 你好世界! 🚀🎉";

        let encrypted = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_large_value_encryption() {
        let key = derive_key("test-key");
        let plaintext = "x".repeat(10000);

        let encrypted = encrypt(&plaintext, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}

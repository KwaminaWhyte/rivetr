//! Cryptographic utilities for encrypting sensitive data.
//!
//! This module provides AES-256-GCM encryption for environment variables
//! and other sensitive data stored in the database.
//!
//! Two ciphertext formats are understood:
//! - `ENC:` (legacy v1): base64(nonce || ciphertext || tag), AES-256-GCM keyed directly
//!   with the output of [`derive_key`].
//! - `ENC2:` (v2): base64(nonce || ciphertext || tag), AES-256-GCM keyed with a subkey
//!   derived by HKDF-SHA256 from the master key and a per-install random salt
//!   (`<data_dir>/.encryption_salt`, see [`key_file::resolve_install_salt`]).
//!
//! New values are written as v2 once the install salt is set ([`set_install_salt`]); until
//! then (and in unit tests) `encrypt` still writes v1. Decryption dispatches on the prefix,
//! and values without a prefix are legacy plaintext and pass through. v1 values are
//! re-encrypted as v2 lazily, the next time the owning row is written.
//!
//! KDF choice (SEC-M10): the master key is `PBKDF2-HMAC-SHA256(secret, 100k)` (unchanged,
//! it must stay for v1) and the v2 subkey is `HKDF-SHA256(master, install_salt, info)`.
//! Generated and documented keys are random 256-bit values (`openssl rand -hex 32`), so a
//! memory-hard KDF such as Argon2id adds nothing against them; what v2 adds is a per-install
//! salt (no cross-install key reuse for identical secrets), domain separation from any other
//! use of the master key, and an AAD-bound version tag. A weak passphrase is still
//! stretched by the PBKDF2 step, but operators should use a random key.

pub mod key_file;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ring::{hkdf, pbkdf2};
use std::num::NonZeroU32;
use std::sync::OnceLock;

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

/// Prefix of the versioned (v2) format: HKDF subkey with a per-install salt.
pub const ENCRYPTED_PREFIX_V2: &str = "ENC2:";

/// Length of the per-install salt in bytes.
pub const SALT_LENGTH: usize = 32;

/// HKDF `info` for the v2 AES-256-GCM subkey (domain separation).
const HKDF_INFO_V2: &[u8] = b"rivetr/aes-256-gcm/v2";

/// AEAD associated data binding v2 ciphertexts to their format version.
const AAD_V2: &[u8] = b"rivetr-enc-v2";

static INSTALL_SALT: OnceLock<[u8; SALT_LENGTH]> = OnceLock::new();

/// Install the per-install salt for this process. After this, [`encrypt`] writes v2.
/// Returns false if a salt was already set (the first one wins).
pub fn set_install_salt(salt: [u8; SALT_LENGTH]) -> bool {
    INSTALL_SALT.set(salt).is_ok()
}

struct Len32;

impl hkdf::KeyType for Len32 {
    fn len(&self) -> usize {
        KEY_LENGTH
    }
}

/// HKDF-SHA256 subkey for v2 from the master key and install salt.
fn derive_subkey_v2(master: &[u8; KEY_LENGTH], salt: &[u8; SALT_LENGTH]) -> [u8; KEY_LENGTH] {
    let prk = hkdf::Salt::new(hkdf::HKDF_SHA256, salt).extract(master);
    let mut out = [0u8; KEY_LENGTH];
    prk.expand(&[HKDF_INFO_V2], Len32)
        .and_then(|okm| okm.fill(&mut out))
        .expect("HKDF-SHA256 expand of 32 bytes cannot fail");
    out
}

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
/// Writes the v2 format (`ENC2:` + base64(nonce || ciphertext || tag)) when the install
/// salt has been set, otherwise the legacy `ENC:` format.
///
/// # Arguments
/// * `plaintext` - The string to encrypt
/// * `key` - The 32-byte master key (use `derive_key` to generate from a secret)
///
/// # Errors
/// Returns an error if encryption fails
pub fn encrypt(plaintext: &str, key: &[u8; KEY_LENGTH]) -> Result<String> {
    encrypt_with_salt(plaintext, key, INSTALL_SALT.get())
}

/// Encrypt with an explicit salt: `Some` writes v2, `None` writes legacy v1.
fn encrypt_with_salt(
    plaintext: &str,
    key: &[u8; KEY_LENGTH],
    salt: Option<&[u8; SALT_LENGTH]>,
) -> Result<String> {
    use aes_gcm::aead::Payload;
    use rand::RngCore;

    // Generate a random 12-byte nonce
    let mut nonce_bytes = [0u8; NONCE_LENGTH];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let (prefix, aes_key, aad) = match salt {
        Some(s) => (ENCRYPTED_PREFIX_V2, derive_subkey_v2(key, s), AAD_V2),
        None => (ENCRYPTED_PREFIX, *key, &b""[..]),
    };

    let cipher = Aes256Gcm::new_from_slice(&aes_key).context("Failed to create cipher")?;
    let ciphertext = cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext.as_bytes(),
                aad,
            },
        )
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    // Combine nonce and ciphertext: nonce || ciphertext
    let mut combined = Vec::with_capacity(NONCE_LENGTH + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(format!("{}{}", prefix, BASE64.encode(&combined)))
}

/// Decrypt a ciphertext string (v2 `ENC2:` or legacy `ENC:`) using AES-256-GCM.
///
/// # Arguments
/// * `ciphertext` - The encrypted string (with prefix and base64-encoded)
/// * `key` - The 32-byte master key (must be the one used for encryption)
///
/// # Errors
/// Returns an error if the prefix is unknown, a v2 value is read before the install
/// salt is set, the base64 or length is invalid, or authentication fails (wrong key,
/// wrong salt or tampered data).
pub fn decrypt(ciphertext: &str, key: &[u8; KEY_LENGTH]) -> Result<String> {
    decrypt_with_salt(ciphertext, key, INSTALL_SALT.get())
}

fn decrypt_with_salt(
    ciphertext: &str,
    key: &[u8; KEY_LENGTH],
    salt: Option<&[u8; SALT_LENGTH]>,
) -> Result<String> {
    use aes_gcm::aead::Payload;

    // Version prefix first (ENC2: does not start with ENC:), then legacy.
    let (encoded, aes_key, aad) = if let Some(rest) = ciphertext.strip_prefix(ENCRYPTED_PREFIX_V2) {
        let salt = salt.context("ENC2 value but the install salt is not initialised")?;
        (rest, derive_subkey_v2(key, salt), AAD_V2)
    } else if let Some(rest) = ciphertext.strip_prefix(ENCRYPTED_PREFIX) {
        (rest, *key, &b""[..])
    } else {
        anyhow::bail!("Ciphertext doesn't have expected prefix");
    };

    let combined = BASE64.decode(encoded).context("Failed to decode base64")?;

    // Extract nonce and ciphertext
    if combined.len() < NONCE_LENGTH + 1 {
        anyhow::bail!("Ciphertext too short");
    }

    let (nonce_bytes, ciphertext_bytes) = combined.split_at(NONCE_LENGTH);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new_from_slice(&aes_key).context("Failed to create cipher")?;
    let plaintext = cipher
        .decrypt(
            nonce,
            Payload {
                msg: ciphertext_bytes,
                aad,
            },
        )
        .map_err(|e| anyhow::anyhow!("Decryption failed (wrong key or corrupted data): {}", e))?;

    String::from_utf8(plaintext).context("Decrypted data is not valid UTF-8")
}

/// Check if a value is encrypted (has the `ENC:` or `ENC2:` prefix).
///
/// This is used for backwards compatibility - existing unencrypted values
/// will not have this prefix and can be used as-is.
pub fn is_encrypted(value: &str) -> bool {
    value.starts_with(ENCRYPTED_PREFIX) || value.starts_with(ENCRYPTED_PREFIX_V2)
}

/// True for values still in the legacy `ENC:` format (candidates for lazy upgrade).
pub fn is_legacy_format(value: &str) -> bool {
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

        assert_ne!(
            encrypted1, encrypted2,
            "Random nonce should produce different ciphertext"
        );

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
    fn test_legacy_plaintext_passthrough_with_and_without_key() {
        // Rows written before a key was configured must stay readable after one is enabled.
        let key = derive_key("newly-enabled-key");
        for legacy in ["hunter2", "", "postgres://u:p@db/x", "-----BEGIN KEY-----"] {
            assert_eq!(decrypt_if_encrypted(legacy, Some(&key)).unwrap(), legacy);
            assert_eq!(decrypt_if_encrypted(legacy, None).unwrap(), legacy);
        }
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
        assert!(
            result.is_err(),
            "Should fail when encrypted value has no key"
        );
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

    // ---- v2 / versioned format ----

    fn salt(b: u8) -> [u8; SALT_LENGTH] {
        [b; SALT_LENGTH]
    }

    #[test]
    fn v2_roundtrip() {
        let key = derive_key("k");
        let s = salt(7);
        let ct = encrypt_with_salt("hunter2", &key, Some(&s)).unwrap();
        assert!(ct.starts_with(ENCRYPTED_PREFIX_V2));
        assert!(!ct.starts_with(ENCRYPTED_PREFIX));
        assert!(is_encrypted(&ct));
        assert!(!is_legacy_format(&ct));
        assert_eq!(decrypt_with_salt(&ct, &key, Some(&s)).unwrap(), "hunter2");
        // fresh nonce each time
        assert_ne!(ct, encrypt_with_salt("hunter2", &key, Some(&s)).unwrap());
    }

    #[test]
    fn v2_roundtrip_empty_unicode_large() {
        let key = derive_key("k");
        let s = salt(1);
        for p in ["", "Привет 你好 🚀", &"x".repeat(10_000)] {
            let ct = encrypt_with_salt(p, &key, Some(&s)).unwrap();
            assert_eq!(decrypt_with_salt(&ct, &key, Some(&s)).unwrap(), p);
        }
    }

    #[test]
    fn legacy_v1_still_decrypts_with_and_without_salt() {
        let key = derive_key("k");
        let legacy = encrypt_with_salt("old-secret", &key, None).unwrap();
        assert!(legacy.starts_with(ENCRYPTED_PREFIX));
        assert!(is_legacy_format(&legacy));
        // Salt presence must not affect reading v1 values.
        assert_eq!(
            decrypt_with_salt(&legacy, &key, None).unwrap(),
            "old-secret"
        );
        let s = salt(9);
        assert_eq!(
            decrypt_with_salt(&legacy, &key, Some(&s)).unwrap(),
            "old-secret"
        );
    }

    #[test]
    fn legacy_v1_known_vector_format_is_unchanged() {
        // Build a v1 value by hand exactly as the pre-v2 code did (no AAD, key used directly).
        use aes_gcm::aead::Aead;
        let key = derive_key("k");
        let nonce = [3u8; NONCE_LENGTH];
        let ct = Aes256Gcm::new_from_slice(&key)
            .unwrap()
            .encrypt(Nonce::from_slice(&nonce), "legacy".as_bytes())
            .unwrap();
        let mut raw = nonce.to_vec();
        raw.extend_from_slice(&ct);
        let stored = format!("ENC:{}", BASE64.encode(raw));
        assert_eq!(decrypt_with_salt(&stored, &key, None).unwrap(), "legacy");
    }

    #[test]
    fn plaintext_passes_through_even_when_it_looks_versioned() {
        let key = derive_key("k");
        for p in ["hunter2", "", "v2:abc", "enc2:abc", "ENC", "ENC2"] {
            assert_eq!(decrypt_if_encrypted(p, Some(&key)).unwrap(), p);
            assert_eq!(decrypt_if_encrypted(p, None).unwrap(), p);
            assert!(!is_encrypted(p));
        }
    }

    #[test]
    fn v2_wrong_key_fails() {
        let s = salt(7);
        let ct = encrypt_with_salt("x", &derive_key("right"), Some(&s)).unwrap();
        assert!(decrypt_with_salt(&ct, &derive_key("wrong"), Some(&s)).is_err());
    }

    #[test]
    fn v2_wrong_salt_fails() {
        let key = derive_key("k");
        let ct = encrypt_with_salt("x", &key, Some(&salt(1))).unwrap();
        assert!(decrypt_with_salt(&ct, &key, Some(&salt(2))).is_err());
    }

    #[test]
    fn v2_without_salt_fails_cleanly() {
        let key = derive_key("k");
        let ct = encrypt_with_salt("x", &key, Some(&salt(1))).unwrap();
        let err = decrypt_with_salt(&ct, &key, None).unwrap_err().to_string();
        assert!(err.contains("salt"), "{err}");
    }

    #[test]
    fn v2_tamper_fails() {
        let key = derive_key("k");
        let s = salt(5);
        let ct = encrypt_with_salt("sensitive", &key, Some(&s)).unwrap();
        let raw = BASE64
            .decode(ct.strip_prefix(ENCRYPTED_PREFIX_V2).unwrap())
            .unwrap();
        // Flip one bit in every region: nonce, ciphertext body, tag.
        for idx in [0, NONCE_LENGTH, raw.len() - 1] {
            let mut t = raw.clone();
            t[idx] ^= 1;
            let tampered = format!("{}{}", ENCRYPTED_PREFIX_V2, BASE64.encode(&t));
            assert!(
                decrypt_with_salt(&tampered, &key, Some(&s)).is_err(),
                "idx {idx}"
            );
        }
        // Truncated and garbage payloads.
        assert!(decrypt_with_salt("ENC2:", &key, Some(&s)).is_err());
        assert!(decrypt_with_salt("ENC2:!!!notbase64", &key, Some(&s)).is_err());
    }

    #[test]
    fn v2_is_bound_to_version_prefix() {
        // A v1 ciphertext relabelled as v2 (or the reverse) must not decrypt.
        let key = derive_key("k");
        let s = salt(5);
        let v1 = encrypt_with_salt("x", &key, None).unwrap();
        let relabelled = v1.replacen("ENC:", "ENC2:", 1);
        assert!(decrypt_with_salt(&relabelled, &key, Some(&s)).is_err());
        let v2 = encrypt_with_salt("x", &key, Some(&s)).unwrap();
        let relabelled = v2.replacen("ENC2:", "ENC:", 1);
        assert!(decrypt_with_salt(&relabelled, &key, Some(&s)).is_err());
    }

    #[test]
    fn subkey_is_salt_and_key_dependent() {
        let k = derive_key("k");
        assert_ne!(
            derive_subkey_v2(&k, &salt(1)),
            derive_subkey_v2(&k, &salt(2))
        );
        assert_ne!(derive_subkey_v2(&k, &salt(1)), k);
        assert_ne!(
            derive_subkey_v2(&k, &salt(1)),
            derive_subkey_v2(&derive_key("other"), &salt(1))
        );
        assert_eq!(
            derive_subkey_v2(&k, &salt(1)),
            derive_subkey_v2(&k, &salt(1))
        );
    }
}

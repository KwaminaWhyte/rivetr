use crate::AppState;
use crate::crypto;

/// Key length for AES-256 encryption
pub const KEY_LENGTH: usize = 32;

/// Get the derived encryption key from the config if configured
pub fn get_encryption_key(state: &AppState) -> Option<[u8; KEY_LENGTH]> {
    state
        .config
        .auth
        .encryption_key
        .as_ref()
        .map(|secret| crypto::derive_key(secret))
}

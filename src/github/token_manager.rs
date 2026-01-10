//! Token management for GitHub App authentication.
//!
//! GitHub Apps use two types of authentication:
//! 1. App JWT - Short-lived JWT signed with the app's private key (for app-level operations)
//! 2. Installation Access Token - Token for a specific installation (for repo operations)

use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

/// JWT claims for GitHub App authentication.
/// GitHub requires: iat (issued at), exp (expiration), iss (issuer = app_id)
#[derive(Debug, Serialize, Deserialize)]
struct GitHubAppClaims {
    /// Issued at time (Unix timestamp)
    iat: i64,
    /// Expiration time (Unix timestamp) - max 10 minutes
    exp: i64,
    /// Issuer - the GitHub App ID
    iss: String,
}

/// Generate a JWT for GitHub App authentication.
///
/// The JWT is signed with RS256 (RSA-SHA256) using the app's private key.
/// It's valid for 10 minutes (GitHub's maximum).
///
/// # Arguments
/// * `app_id` - The GitHub App ID (numeric)
/// * `private_key_pem` - The private key in PEM format
///
/// # Returns
/// A signed JWT string that can be used in the Authorization header
///
/// # Example
/// ```ignore
/// let jwt = generate_app_jwt(12345, &private_key)?;
/// // Use: Authorization: Bearer {jwt}
/// ```
pub fn generate_app_jwt(app_id: i64, private_key_pem: &str) -> Result<String> {
    let now = Utc::now();
    // Issue time: 60 seconds in the past to account for clock drift
    let iat = now - Duration::seconds(60);
    // Expiration: 10 minutes (GitHub's maximum)
    let exp = now + Duration::minutes(10);

    let claims = GitHubAppClaims {
        iat: iat.timestamp(),
        exp: exp.timestamp(),
        iss: app_id.to_string(),
    };

    let header = Header::new(Algorithm::RS256);

    let encoding_key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .context("Failed to parse private key PEM")?;

    let token = encode(&header, &claims, &encoding_key).context("Failed to encode JWT")?;

    Ok(token)
}

/// Response from GitHub's installation access token endpoint.
#[derive(Debug, Deserialize)]
pub struct InstallationTokenResponse {
    pub token: String,
    pub expires_at: String,
    pub permissions: serde_json::Value,
    pub repository_selection: Option<String>,
}

/// Get an installation access token for a specific GitHub App installation.
///
/// This exchanges an app JWT for an installation-specific access token
/// that can be used to interact with repositories the installation has access to.
///
/// # Arguments
/// * `app_id` - The GitHub App ID
/// * `private_key_pem` - The app's private key in PEM format
/// * `installation_id` - The installation ID to get a token for
///
/// # Returns
/// An access token that can be used for repository operations
pub async fn get_installation_token(
    app_id: i64,
    private_key_pem: &str,
    installation_id: i64,
) -> Result<InstallationTokenResponse> {
    // First, generate an app JWT
    let jwt = generate_app_jwt(app_id, private_key_pem)?;

    // Exchange the JWT for an installation access token
    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "https://api.github.com/app/installations/{}/access_tokens",
            installation_id
        ))
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "Rivetr")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .context("Failed to request installation access token")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!(
            "GitHub API error getting installation token: {} - {}",
            status,
            body
        );
    }

    let token_response: InstallationTokenResponse = response
        .json()
        .await
        .context("Failed to parse installation token response")?;

    Ok(token_response)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a valid private key to pass.
    // In CI, you'd typically mock the external calls or use a test key.

    #[test]
    fn test_generate_jwt_invalid_key() {
        let result = generate_app_jwt(12345, "not-a-valid-key");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_jwt_with_malformed_pem() {
        // Test that malformed PEM structures are rejected
        let malformed_pem = "-----BEGIN RSA PRIVATE KEY-----\ninvalid-base64-content\n-----END RSA PRIVATE KEY-----";
        let result = generate_app_jwt(12345, malformed_pem);
        assert!(result.is_err());
    }
}

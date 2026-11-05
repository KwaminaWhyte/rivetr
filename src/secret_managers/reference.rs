//! The secret reference syntax stored in env var values.
//!
//! ```text
//! secret://<connection>/<path>#<key>
//! ```
//!
//! - `<connection>` is the name of a secret manager connection in the resource's team
//!   (lowercase letters, digits, `-` and `_`, starting with a letter or digit).
//! - `<path>` is the provider path: a Vault/OpenBao KV v2 secret path inside the
//!   connection's mount, an Infisical folder, an optional `project/config` for Doppler,
//!   or an AWS Secrets Manager secret name or ARN. It may be empty.
//! - `#<key>` picks one key from the secret. It is optional only for AWS Secrets
//!   Manager, where leaving it out uses the whole `SecretString`.
//!
//! Whole paths (all keys, optional prefix) are linked with `secret_imports` rows rather
//! than a value syntax, see [`super::store`].

use std::fmt;

/// Scheme every reference starts with.
pub const SCHEME: &str = "secret://";

/// A parsed `secret://` reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretRef {
    pub connection: String,
    pub path: String,
    pub key: Option<String>,
}

/// Why a value that starts with `secret://` is not a valid reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefError {
    MissingConnection,
    InvalidConnection(String),
    EmptyKey,
    InvalidCharacter,
}

impl fmt::Display for RefError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingConnection => write!(f, "the reference has no connection name"),
            Self::InvalidConnection(n) => write!(
                f,
                "'{}' is not a valid connection name (lowercase letters, digits, '-' and '_')",
                n
            ),
            Self::EmptyKey => write!(f, "the key after '#' is empty"),
            Self::InvalidCharacter => {
                write!(f, "the reference contains whitespace or control characters")
            }
        }
    }
}

impl std::error::Error for RefError {}

/// Does this value look like a reference (starts with the scheme)?
pub fn is_reference(value: &str) -> bool {
    value.starts_with(SCHEME)
}

/// Is `name` a valid connection name?
pub fn is_valid_connection_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => return false,
    }
    name.len() <= 63
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// Parse a reference. Returns `Ok(None)` for values that are not references at all.
pub fn parse(value: &str) -> Result<Option<SecretRef>, RefError> {
    let Some(rest) = value.strip_prefix(SCHEME) else {
        return Ok(None);
    };
    if rest.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(RefError::InvalidCharacter);
    }
    let (location, key) = match rest.split_once('#') {
        Some((loc, key)) => {
            if key.is_empty() {
                return Err(RefError::EmptyKey);
            }
            (loc, Some(key.to_string()))
        }
        None => (rest, None),
    };
    let (connection, path) = match location.split_once('/') {
        Some((c, p)) => (c, p),
        None => (location, ""),
    };
    if connection.is_empty() {
        return Err(RefError::MissingConnection);
    }
    if !is_valid_connection_name(connection) {
        return Err(RefError::InvalidConnection(connection.to_string()));
    }
    Ok(Some(SecretRef {
        connection: connection.to_string(),
        path: path.trim_matches('/').to_string(),
        key,
    }))
}

impl fmt::Display for SecretRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}/{}", SCHEME, self.connection, self.path)?;
        if let Some(k) = &self.key {
            write!(f, "#{}", k)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(conn: &str, path: &str, key: Option<&str>) -> SecretRef {
        SecretRef {
            connection: conn.into(),
            path: path.into(),
            key: key.map(Into::into),
        }
    }

    #[test]
    fn plain_values_are_not_references() {
        assert_eq!(parse("postgres://u:p@h/db").unwrap(), None);
        assert_eq!(parse("").unwrap(), None);
        assert_eq!(parse("Secret://vault/a#b").unwrap(), None);
        assert!(!is_reference("secret:/vault/a"));
    }

    #[test]
    fn parses_vault_style_reference() {
        assert_eq!(
            parse("secret://vault-prod/myapp/config#DATABASE_URL").unwrap(),
            Some(r("vault-prod", "myapp/config", Some("DATABASE_URL")))
        );
    }

    #[test]
    fn trims_slashes_and_allows_empty_path() {
        assert_eq!(
            parse("secret://doppler/#API_KEY").unwrap(),
            Some(r("doppler", "", Some("API_KEY")))
        );
        assert_eq!(
            parse("secret://doppler#API_KEY").unwrap(),
            Some(r("doppler", "", Some("API_KEY")))
        );
        assert_eq!(
            parse("secret://inf//backend/api/#K").unwrap(),
            Some(r("inf", "backend/api", Some("K")))
        );
    }

    #[test]
    fn key_is_optional_and_arns_survive() {
        let arn = "arn:aws:secretsmanager:eu-west-1:123456789012:secret:prod/db-AbCdEf";
        assert_eq!(
            parse(&format!("secret://aws/{arn}")).unwrap(),
            Some(r("aws", arn, None))
        );
        assert_eq!(
            parse(&format!("secret://aws/{arn}#password")).unwrap(),
            Some(r("aws", arn, Some("password")))
        );
    }

    #[test]
    fn rejects_malformed_references() {
        assert_eq!(parse("secret://").unwrap_err(), RefError::MissingConnection);
        assert_eq!(
            parse("secret:///a#b").unwrap_err(),
            RefError::MissingConnection
        );
        assert_eq!(parse("secret://v/a#").unwrap_err(), RefError::EmptyKey);
        assert!(matches!(
            parse("secret://Vault/a#b").unwrap_err(),
            RefError::InvalidConnection(_)
        ));
        assert_eq!(
            parse("secret://v/a b#c").unwrap_err(),
            RefError::InvalidCharacter
        );
    }

    #[test]
    fn display_round_trips() {
        for s in [
            "secret://vault/a/b#K",
            "secret://aws/prod/db",
            "secret://doppler/#X",
        ] {
            assert_eq!(parse(s).unwrap().unwrap().to_string(), s);
        }
    }

    #[test]
    fn connection_name_rules() {
        assert!(is_valid_connection_name("vault-prod_1"));
        assert!(is_valid_connection_name("1password"));
        assert!(!is_valid_connection_name("-x"));
        assert!(!is_valid_connection_name("a.b"));
        assert!(!is_valid_connection_name(&"a".repeat(64)));
    }
}

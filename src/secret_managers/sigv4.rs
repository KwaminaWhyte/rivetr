//! Minimal AWS Signature Version 4 signer for the two Secrets Manager calls Rivetr makes.
//!
//! Implemented here instead of pulling in the AWS SDK for one JSON-RPC endpoint. Checked
//! against the vectors from the AWS SigV4 test suite and the IAM example in the AWS
//! documentation (see the tests).

use ring::{digest, hmac};

/// AWS credentials used to sign a request.
#[derive(Clone)]
pub struct AwsCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

impl std::fmt::Debug for AwsCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwsCredentials")
            .field("access_key_id", &self.access_key_id)
            .field("secret_access_key", &"********")
            .field(
                "session_token",
                &self.session_token.as_ref().map(|_| "********"),
            )
            .finish()
    }
}

/// The parts of an HTTP request that go into the signature.
pub struct SignableRequest<'a> {
    pub method: &'a str,
    pub host: &'a str,
    /// Already URI-encoded absolute path, `/` for the root.
    pub path: &'a str,
    /// Raw (not yet encoded) query parameters.
    pub query: &'a [(&'a str, &'a str)],
    /// Extra headers to sign, besides `host` and `x-amz-date` (added here).
    pub headers: &'a [(&'a str, &'a str)],
    pub payload: &'a [u8],
}

/// Headers to add to the request: `x-amz-date`, `x-amz-security-token` when a session
/// token is set, and `authorization`.
pub fn sign(
    creds: &AwsCredentials,
    region: &str,
    service: &str,
    amz_date: &str,
    req: &SignableRequest<'_>,
) -> Vec<(String, String)> {
    let date = &amz_date[..8.min(amz_date.len())];

    let mut headers: Vec<(String, String)> = req
        .headers
        .iter()
        .map(|(k, v)| (k.to_ascii_lowercase(), normalize_header_value(v)))
        .collect();
    headers.push(("host".into(), req.host.to_string()));
    headers.push(("x-amz-date".into(), amz_date.to_string()));
    if let Some(token) = &creds.session_token {
        headers.push(("x-amz-security-token".into(), token.clone()));
    }
    headers.sort();

    let canonical_headers: String = headers
        .iter()
        .map(|(k, v)| format!("{}:{}\n", k, v))
        .collect();
    let signed_headers = headers
        .iter()
        .map(|(k, _)| k.as_str())
        .collect::<Vec<_>>()
        .join(";");

    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        req.method,
        req.path,
        canonical_query(req.query),
        canonical_headers,
        signed_headers,
        hex_sha256(req.payload)
    );

    let scope = format!("{}/{}/{}/aws4_request", date, region, service);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        amz_date,
        scope,
        hex_sha256(canonical_request.as_bytes())
    );

    let key = signing_key(&creds.secret_access_key, date, region, service);
    let signature = hex::encode(hmac::sign(&key, string_to_sign.as_bytes()).as_ref());

    let mut out = vec![("x-amz-date".to_string(), amz_date.to_string())];
    if let Some(token) = &creds.session_token {
        out.push(("x-amz-security-token".to_string(), token.clone()));
    }
    out.push((
        "authorization".to_string(),
        format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            creds.access_key_id, scope, signed_headers, signature
        ),
    ));
    out
}

/// `kSigning = HMAC(HMAC(HMAC(HMAC("AWS4" + secret, date), region), service), "aws4_request")`.
pub fn signing_key(secret: &str, date: &str, region: &str, service: &str) -> hmac::Key {
    hmac::Key::new(
        hmac::HMAC_SHA256,
        &signing_key_bytes(secret, date, region, service),
    )
}

fn signing_key_bytes(secret: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac_bytes(format!("AWS4{}", secret).as_bytes(), date.as_bytes());
    let k_region = hmac_bytes(&k_date, region.as_bytes());
    let k_service = hmac_bytes(&k_region, service.as_bytes());
    hmac_bytes(&k_service, b"aws4_request")
}

fn hmac_bytes(key: &[u8], data: &[u8]) -> Vec<u8> {
    let key = hmac::Key::new(hmac::HMAC_SHA256, key);
    hmac::sign(&key, data).as_ref().to_vec()
}

/// Lowercase hex SHA-256.
pub fn hex_sha256(data: &[u8]) -> String {
    hex::encode(digest::digest(&digest::SHA256, data).as_ref())
}

/// Trim and collapse runs of spaces, as the canonical header rules require.
fn normalize_header_value(v: &str) -> String {
    v.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn canonical_query(query: &[(&str, &str)]) -> String {
    let mut pairs: Vec<(String, String)> = query
        .iter()
        .map(|(k, v)| (uri_encode(k), uri_encode(v)))
        .collect();
    pairs.sort();
    pairs
        .into_iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&")
}

/// RFC 3986 encoding: everything but unreserved characters is percent-encoded.
pub fn uri_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

/// `YYYYMMDDTHHMMSSZ` for now.
pub fn amz_date_now() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Credentials and date used by the AWS SigV4 test suite and documentation.
    fn suite_creds() -> AwsCredentials {
        AwsCredentials {
            access_key_id: "AKIDEXAMPLE".into(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".into(),
            session_token: None,
        }
    }
    const DATE: &str = "20150830T123600Z";

    fn auth(headers: &[(String, String)]) -> String {
        headers
            .iter()
            .find(|(k, _)| k == "authorization")
            .map(|(_, v)| v.clone())
            .unwrap()
    }

    #[test]
    fn signing_key_matches_aws_documentation() {
        // docs.aws.amazon.com "Examples of how to derive a signing key for SigV4".
        let raw = signing_key_bytes(
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            "20150830",
            "us-east-1",
            "iam",
        );
        assert_eq!(
            hex::encode(raw),
            "c4afb1cc5771d871763a393e44b703571b55cc28424d1a5e86da6ed3c154a4b9"
        );
    }

    #[test]
    fn get_vanilla() {
        let h = sign(
            &suite_creds(),
            "us-east-1",
            "service",
            DATE,
            &SignableRequest {
                method: "GET",
                host: "example.amazonaws.com",
                path: "/",
                query: &[],
                headers: &[],
                payload: b"",
            },
        );
        assert_eq!(
            auth(&h),
            "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/service/aws4_request, \
             SignedHeaders=host;x-amz-date, \
             Signature=5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31"
        );
    }

    #[test]
    fn post_vanilla() {
        let h = sign(
            &suite_creds(),
            "us-east-1",
            "service",
            DATE,
            &SignableRequest {
                method: "POST",
                host: "example.amazonaws.com",
                path: "/",
                query: &[],
                headers: &[],
                payload: b"",
            },
        );
        assert!(auth(&h).ends_with(
            "Signature=5da7c1a2acd57cee7505fc6676e4e544621c30862966e37dddb68e92efbe5d6b"
        ));
    }

    #[test]
    fn get_vanilla_query_order_key_case() {
        let h = sign(
            &suite_creds(),
            "us-east-1",
            "service",
            DATE,
            &SignableRequest {
                method: "GET",
                host: "example.amazonaws.com",
                path: "/",
                query: &[("Param2", "value2"), ("Param1", "value1")],
                headers: &[],
                payload: b"",
            },
        );
        assert!(auth(&h).ends_with(
            "Signature=b97d918cfa904a5beff61c982a1b6f458b799221646efd99d3219ec94cdf2500"
        ));
    }

    #[test]
    fn iam_list_users_documentation_example() {
        let h = sign(
            &suite_creds(),
            "us-east-1",
            "iam",
            DATE,
            &SignableRequest {
                method: "GET",
                host: "iam.amazonaws.com",
                path: "/",
                query: &[("Action", "ListUsers"), ("Version", "2010-05-08")],
                headers: &[(
                    "Content-Type",
                    "application/x-www-form-urlencoded; charset=utf-8",
                )],
                payload: b"",
            },
        );
        assert_eq!(
            auth(&h),
            "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/iam/aws4_request, \
             SignedHeaders=content-type;host;x-amz-date, \
             Signature=5d672d79c15b13162d9279b0855cfba6789a8edb4c82c400e06b5924a6f2b5d7"
        );
    }

    #[test]
    fn session_token_is_signed_and_returned() {
        let mut c = suite_creds();
        c.session_token = Some("tok".into());
        let h = sign(
            &c,
            "us-east-1",
            "secretsmanager",
            DATE,
            &SignableRequest {
                method: "POST",
                host: "secretsmanager.us-east-1.amazonaws.com",
                path: "/",
                query: &[],
                headers: &[],
                payload: b"{}",
            },
        );
        assert!(h
            .iter()
            .any(|(k, v)| k == "x-amz-security-token" && v == "tok"));
        assert!(auth(&h).contains("SignedHeaders=host;x-amz-date;x-amz-security-token"));
    }

    #[test]
    fn debug_hides_secrets() {
        let mut c = suite_creds();
        c.session_token = Some("tok-secret".into());
        let s = format!("{:?}", c);
        assert!(!s.contains("EXAMPLEKEY"));
        assert!(!s.contains("tok-secret"));
    }

    #[test]
    fn uri_encoding() {
        assert_eq!(uri_encode("a b/c~"), "a%20b%2Fc~");
    }
}

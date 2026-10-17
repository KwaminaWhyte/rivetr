//! Content-Security-Policy and HSTS helpers (SEC-M6, SEC-L3).
//!
//! The dashboard is a React Router SPA prerendered to static HTML that contains
//! several inline `<script>` blocks (theme bootstrap, router context, streaming
//! bootstrap). Their content changes with every frontend build, so instead of
//! allowing `'unsafe-inline'` for scripts we hash every inline script found in the
//! embedded HTML at serve time and emit `'sha256-...'` sources for exactly those.
//!
//! Styles keep `'unsafe-inline'`: white-label custom CSS is injected as a
//! `<style>` element and component libraries set inline style attributes. Inline
//! styles cannot execute script, so this is the narrowest workable policy.

use axum::http::HeaderMap;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use sha2::{Digest, Sha256};

/// Policy for non-HTML responses (JSON API, downloads): nothing may load or frame.
pub const API_CSP: &str =
    "default-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'";

/// HSTS value sent on HTTPS requests. No `includeSubDomains`: app subdomains
/// deployed under the same parent domain may legitimately be served over HTTP.
pub const HSTS_VALUE: &str = "max-age=31536000";

/// Build the CSP for an HTML document.
///
/// `host` is the request `Host` header; it is used to allow same-host
/// WebSocket connections (`ws://host` / `wss://host`) explicitly, since not
/// every browser maps `'self'` to the ws/wss schemes.
pub fn html_csp(html: &str, host: Option<&str>) -> String {
    let mut script_src = String::from("script-src 'self'");
    for hash in inline_script_hashes(html) {
        script_src.push_str(" 'sha256-");
        script_src.push_str(&hash);
        script_src.push('\'');
    }

    let mut connect_src = String::from("connect-src 'self'");
    if let Some(host) = host.filter(|h| is_safe_host(h)) {
        connect_src.push_str(&format!(" ws://{host} wss://{host}"));
    }

    [
        "default-src 'self'",
        &script_src,
        "style-src 'self' 'unsafe-inline'",
        "img-src 'self' data: blob: https:",
        "font-src 'self' data:",
        &connect_src,
        "worker-src 'self' blob:",
        "media-src 'self' blob:",
        "object-src 'none'",
        "base-uri 'self'",
        "form-action 'self'",
        "frame-ancestors 'none'",
    ]
    .join("; ")
}

/// Whether the request reached us over HTTPS (via a TLS-terminating proxy).
pub fn is_https_request(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("https"))
}

/// Only plain host[:port] / IPv6 literals are echoed into the policy, so a
/// crafted Host header cannot inject extra directives.
fn is_safe_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 255
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'))
}

/// Base64 SHA-256 digests of every inline (no `src`) `<script>` body in `html`.
fn inline_script_hashes(html: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let mut hashes: Vec<String> = Vec::new();
    let mut pos = 0;

    while let Some(rel) = lower[pos..].find("<script") {
        let start = pos + rel;
        let after = start + "<script".len();
        // Must be exactly "<script" followed by whitespace, '>' or '/'
        match lower.as_bytes().get(after) {
            Some(b) if b.is_ascii_whitespace() || *b == b'>' || *b == b'/' => {}
            _ => {
                pos = after;
                continue;
            }
        }
        let Some(tag_end_rel) = lower[after..].find('>') else {
            break;
        };
        let tag_end = after + tag_end_rel;
        let attrs = &lower[after..tag_end];
        let body_start = tag_end + 1;
        let Some(close_rel) = lower[body_start..].find("</script") else {
            break;
        };
        let body_end = body_start + close_rel;
        pos = body_end;

        let has_src = attrs
            .split(|c: char| c.is_ascii_whitespace() || c == '/')
            .any(|a| a == "src" || a.starts_with("src="));
        let body = &html[body_start..body_end];
        if has_src || body.is_empty() {
            continue;
        }
        let digest = BASE64.encode(Sha256::digest(body.as_bytes()));
        if !hashes.contains(&digest) {
            hashes.push(digest);
        }
    }
    hashes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(body: &str) -> String {
        BASE64.encode(Sha256::digest(body.as_bytes()))
    }

    #[test]
    fn hashes_only_inline_scripts() {
        let html = r#"<html><head>
<script>var a = 1;</script>
<script src="/assets/app.js"></script>
<script type="module" async="">import "/x.js";</script>
<script id="_R_">b()</script>
</head></html>"#;
        let hashes = inline_script_hashes(html);
        assert_eq!(
            hashes,
            vec![sha("var a = 1;"), sha("import \"/x.js\";"), sha("b()"),]
        );
    }

    #[test]
    fn ignores_scriptish_tags_and_empty_bodies() {
        let html = "<scripty>x</scripty><script></script><script  >z</script>";
        assert_eq!(inline_script_hashes(html), vec![sha("z")]);
    }

    #[test]
    fn html_csp_has_required_directives_and_hashes() {
        let csp = html_csp("<script>hello()</script>", Some("rivetr.example.com:8080"));
        assert!(csp.contains("default-src 'self'"));
        assert!(csp.contains("frame-ancestors 'none'"));
        assert!(csp.contains("object-src 'none'"));
        assert!(csp.contains("base-uri 'self'"));
        assert!(csp.contains("style-src 'self' 'unsafe-inline'"));
        assert!(csp.contains("img-src 'self' data: blob: https:"));
        assert!(csp.contains(
            "connect-src 'self' ws://rivetr.example.com:8080 wss://rivetr.example.com:8080"
        ));
        assert!(csp.contains(&format!("script-src 'self' 'sha256-{}'", sha("hello()"))));
        assert!(!csp.contains("script-src 'self' 'unsafe-inline'"));
        assert!(!csp.contains("'unsafe-eval'"));
    }

    #[test]
    fn unsafe_host_header_is_not_echoed() {
        let csp = html_csp("", Some("evil.com; script-src *"));
        assert!(!csp.contains("evil.com"));
        assert!(csp.contains("connect-src 'self'"));
    }

    #[test]
    fn detects_https_from_forwarded_proto() {
        let mut h = HeaderMap::new();
        assert!(!is_https_request(&h));
        h.insert("x-forwarded-proto", "http".parse().unwrap());
        assert!(!is_https_request(&h));
        h.insert("x-forwarded-proto", "HTTPS".parse().unwrap());
        assert!(is_https_request(&h));
    }
}

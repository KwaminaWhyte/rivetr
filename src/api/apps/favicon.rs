//! App icons: fetch the favicon an application serves and hand it to the
//! dashboard, so an app is recognisable by the same icon as its site.
//!
//! The domain comes from the app's own settings, but it is still user input, so
//! every hop goes through the SSRF guard (`super::super::ssrf`) and redirects
//! are followed by hand to re-check each one. Responses are capped and snified
//! by content, not by what the server claims, and delivered with `nosniff` plus
//! a restrictive CSP because the bytes are third-party (an SVG could carry
//! script).

use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use futures::StreamExt;

use crate::db::User;
use crate::AppState;

use super::super::authz;
use super::super::error::ApiError;
use super::super::validation::validate_uuid;

/// Favicons are small. Anything larger is not an icon.
const MAX_BYTES: usize = 512 * 1024;
/// How long a found icon is reused.
const HIT_TTL: Duration = Duration::from_secs(6 * 60 * 60);
/// How long "this app has no icon" is remembered.
const MISS_TTL: Duration = Duration::from_secs(15 * 60);
const MAX_REDIRECTS: usize = 3;

#[derive(Debug, Clone)]
struct Icon {
    bytes: Bytes,
    content_type: String,
}

struct Entry {
    fetched: Instant,
    icon: Option<Arc<Icon>>,
}

static CACHE: LazyLock<DashMap<String, Entry>> = LazyLock::new(DashMap::new);

/// GET /api/apps/:id/favicon
pub async fn get_app_favicon(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }
    let app = authz::authorize_app(&state, &user, &id).await?;
    let Some(domain) = app.get_primary_domain() else {
        return Err(ApiError::not_found("This app has no domain"));
    };
    let icon = icon_for(&domain)
        .await
        .ok_or_else(|| ApiError::not_found("No favicon found for this app"))?;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&icon.content_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=3600"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    // The icon is a third-party image; make sure it can never run as a document.
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; style-src 'unsafe-inline'"),
    );
    Ok((StatusCode::OK, headers, icon.bytes.clone()).into_response())
}

/// Cached lookup. A miss is cached briefly so a domain without an icon is not
/// re-fetched on every page load.
async fn icon_for(domain: &str) -> Option<Arc<Icon>> {
    let key = domain
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_string();
    // A domain with a path or port is not something to fetch an icon from
    // blindly; keep the host part only.
    let key = key
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .trim_end_matches('/')
        .to_string();
    if key.is_empty() {
        return None;
    }

    if let Some(entry) = CACHE.get(&key) {
        let ttl = if entry.icon.is_some() {
            HIT_TTL
        } else {
            MISS_TTL
        };
        if entry.fetched.elapsed() < ttl {
            return entry.icon.clone();
        }
    }

    let icon = fetch_icon(&key).await.map(Arc::new);
    CACHE.insert(
        key,
        Entry {
            fetched: Instant::now(),
            icon: icon.clone(),
        },
    );
    icon
}

async fn fetch_icon(domain: &str) -> Option<Icon> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(6))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .ok()?;

    // The proxy usually serves both; https first, plain http for apps without a
    // certificate.
    for scheme in ["https", "http"] {
        let Ok(base) = reqwest::Url::parse(&format!("{scheme}://{domain}/")) else {
            continue;
        };
        if super::super::ssrf::validate_external_url(base.as_str())
            .await
            .is_err()
        {
            return None;
        }

        if let Ok(icon_url) = base.join("favicon.ico") {
            if let Some(icon) = fetch_image(&client, icon_url).await {
                return Some(icon);
            }
        }

        if let Some(html) = fetch_text(&client, base.clone()).await {
            for href in extract_icon_hrefs(&html).into_iter().take(5) {
                let Ok(url) = base.join(&href) else { continue };
                if let Some(icon) = fetch_image(&client, url).await {
                    return Some(icon);
                }
            }
        }
    }
    None
}

/// Follow up to [`MAX_REDIRECTS`] redirects by hand, validating every hop.
async fn get_following(client: &reqwest::Client, url: reqwest::Url) -> Option<reqwest::Response> {
    let mut url = url;
    for _ in 0..=MAX_REDIRECTS {
        if super::super::ssrf::validate_external_url(url.as_str())
            .await
            .is_err()
        {
            return None;
        }
        let res = client.get(url.clone()).send().await.ok()?;
        if res.status().is_redirection() {
            let location = res
                .headers()
                .get(header::LOCATION)?
                .to_str()
                .ok()?
                .to_string();
            url = url.join(&location).ok()?;
            continue;
        }
        return Some(res);
    }
    None
}

/// Fetch a URL if the response really is an image.
async fn fetch_image(client: &reqwest::Client, url: reqwest::Url) -> Option<Icon> {
    let res = get_following(client, url).await?;
    if !res.status().is_success() {
        return None;
    }
    if let Some(len) = res.content_length() {
        if len as usize > MAX_BYTES {
            return None;
        }
    }
    let header_type = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let bytes = read_capped(res).await?;
    let content_type = image_content_type(&bytes, header_type.as_deref())?;
    Some(Icon {
        bytes,
        content_type,
    })
}

async fn fetch_text(client: &reqwest::Client, url: reqwest::Url) -> Option<String> {
    let res = get_following(client, url).await?;
    if !res.status().is_success() {
        return None;
    }
    if let Some(len) = res.content_length() {
        if len as usize > MAX_BYTES {
            return None;
        }
    }
    let bytes = read_capped(res).await?;
    String::from_utf8(bytes.to_vec()).ok()
}

async fn read_capped(res: reqwest::Response) -> Option<Bytes> {
    let mut stream = res.bytes_stream();
    let mut out = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.ok()?;
        if out.len() + chunk.len() > MAX_BYTES {
            return None;
        }
        out.extend_from_slice(&chunk);
    }
    if out.is_empty() {
        None
    } else {
        Some(Bytes::from(out))
    }
}

/// Identify the image from its first bytes; the header alone is not reliable
/// (many servers answer `application/octet-stream` for `.ico`).
fn image_content_type(bytes: &[u8], header_type: Option<&str>) -> Option<String> {
    if bytes.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
        return Some("image/x-icon".into());
    }
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return Some("image/png".into());
    }
    if bytes.starts_with(b"GIF8") {
        return Some("image/gif".into());
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg".into());
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("image/webp".into());
    }
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]);
    let head = head.trim_start();
    if head.starts_with("<svg") || (head.starts_with("<?xml") && head.contains("<svg")) {
        return Some("image/svg+xml".into());
    }

    let declared = header_type?.split(';').next()?.trim().to_ascii_lowercase();
    match declared.as_str() {
        "image/x-icon"
        | "image/vnd.microsoft.icon"
        | "image/png"
        | "image/gif"
        | "image/jpeg"
        | "image/webp"
        | "image/svg+xml"
        | "image/avif" => Some(declared),
        _ => None,
    }
}

/// `href` values from `<link rel="... icon ...">` tags, in document order.
///
/// A hand-rolled parser is enough here: the goal is the handful of link tags in
/// a page head, not general HTML.
fn extract_icon_hrefs(html: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let mut primary = Vec::new();
    let mut fallback = Vec::new();
    let mut from = 0;
    while let Some(rel_pos) = lower[from..].find("<link") {
        let start = from + rel_pos;
        let Some(end_rel) = lower[start..].find('>') else {
            break;
        };
        let tag = &html[start..start + end_rel + 1];
        from = start + end_rel + 1;

        let Some(rel) = attr(tag, "rel") else {
            continue;
        };
        let rel = rel.to_ascii_lowercase();
        let tokens: Vec<&str> = rel.split_whitespace().collect();
        let is_icon = tokens
            .iter()
            .any(|t| matches!(*t, "icon" | "apple-touch-icon" | "fluid-icon"));
        if !is_icon || tokens.contains(&"mask-icon") {
            continue;
        }
        let Some(href) = attr(tag, "href") else {
            continue;
        };
        let href = href.trim();
        if href.is_empty() || href.starts_with("data:") {
            continue;
        }
        let entry = href.to_string();
        // Plain icons first; touch icons are usually a large square fallback.
        if rel.contains("apple-touch-icon") || rel.contains("fluid-icon") {
            fallback.push(entry);
        } else {
            primary.push(entry);
        }
    }
    primary.extend(fallback);
    primary
}

/// Read `name="value"` (or single quoted, or unquoted) from a tag.
fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(pos) = lower[from..].find(name) {
        let start = from + pos;
        from = start + name.len();
        // Must be a whole attribute name: preceded by whitespace.
        let preceded_ok = start == 0
            || tag[..start]
                .chars()
                .last()
                .is_some_and(|c| c.is_whitespace());
        if !preceded_ok {
            continue;
        }
        let after = &tag[from..];
        let after = after.trim_start();
        let Some(after) = after.strip_prefix('=') else {
            continue;
        };
        let after = after.trim_start();
        let mut chars = after.chars();
        return match chars.next()? {
            q @ ('"' | '\'') => {
                let rest = &after[1..];
                rest.find(q).map(|end| rest[..end].to_string())
            }
            _ => {
                // Unquoted values run to whitespace or '>'. A trailing '/' is
                // the self-closing slash, not part of the value. Splitting on
                // '/' itself would break every href that starts with one.
                let end = after
                    .find(|c: char| c.is_whitespace() || c == '>')
                    .unwrap_or(after.len());
                let value = after[..end].trim_end_matches('/');
                Some(value.to_string())
            }
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_icon_links_in_priority_order() {
        let html = r#"
            <head>
              <link rel="apple-touch-icon" href="/apple.png">
              <link rel="stylesheet" href="/app.css">
              <link rel="shortcut icon" href="/favicon.png">
              <link rel="icon" type="image/svg+xml" href="/logo.svg">
            </head>"#;
        assert_eq!(
            extract_icon_hrefs(html),
            vec!["/favicon.png", "/logo.svg", "/apple.png"]
        );
    }

    #[test]
    fn ignores_data_uris_and_masks() {
        let html = r#"
            <link rel="icon" href="data:image/png;base64,AAAA">
            <link rel="mask-icon" href="/mask.svg">
            <link rel="icon" href="//cdn.example.com/i.png">
        "#;
        assert_eq!(extract_icon_hrefs(html), vec!["//cdn.example.com/i.png"]);
    }

    #[test]
    fn reads_single_quoted_and_unquoted_attributes() {
        let html = "<link rel='icon' href='/a.png'><link rel=icon href=/b.png>";
        assert_eq!(extract_icon_hrefs(html), vec!["/a.png", "/b.png"]);
    }

    #[test]
    fn sniffs_image_types_regardless_of_the_header() {
        assert_eq!(
            image_content_type(&[0x00, 0x00, 0x01, 0x00, 9], None).as_deref(),
            Some("image/x-icon")
        );
        assert_eq!(
            image_content_type(&[0x89, b'P', b'N', b'G', 0], None).as_deref(),
            Some("image/png")
        );
        assert_eq!(
            image_content_type(b"<?xml version=\"1.0\"?><svg xmlns=\"...\"></svg>", None)
                .as_deref(),
            Some("image/svg+xml")
        );
        // A page that is not an image is rejected even if it claims to be one.
        assert_eq!(
            image_content_type(b"<!DOCTYPE html><html></html>", Some("text/html")),
            None
        );
        assert_eq!(
            image_content_type(b"not an image", Some("image/png")).as_deref(),
            Some("image/png")
        );
    }
}

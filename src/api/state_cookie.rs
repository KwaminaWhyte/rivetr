//! Browser-binding cookie for login CSRF `state` values (SEC-M3).
//!
//! Used by both the social OAuth login and the SSO/OIDC login. The `state` sent to
//! the identity provider must equal the cookie set when this browser started the
//! login, so a state minted by an attacker cannot be replayed into a victim's browser.

use axum::http::HeaderMap;

/// Same lifetime as the DB state rows.
const MAX_AGE_SECS: u32 = 600;

/// A named, path-scoped state cookie.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StateCookie {
    pub name: &'static str,
    /// Only sent to the login endpoints under this path.
    pub path: &'static str,
}

impl StateCookie {
    /// `Set-Cookie` value. `Secure` only when the request arrived over HTTPS,
    /// otherwise browsers would drop it on plain-http installs.
    pub fn header(&self, value: &str, secure: bool) -> String {
        format!(
            "{}={value}; Path={}; Max-Age={MAX_AGE_SECS}; HttpOnly; SameSite=Lax{}",
            self.name,
            self.path,
            if secure { "; Secure" } else { "" }
        )
    }

    /// `Set-Cookie` value that expires the cookie.
    pub fn clear_header(&self, secure: bool) -> String {
        format!(
            "{}=; Path={}; Max-Age=0; HttpOnly; SameSite=Lax{}",
            self.name,
            self.path,
            if secure { "; Secure" } else { "" }
        )
    }

    /// The cookie value from the request headers, if present.
    pub fn read<'a>(&self, headers: &'a HeaderMap) -> Option<&'a str> {
        headers
            .get_all(axum::http::header::COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .find_map(|h| cookie_value(h, self.name))
    }
}

/// Find a cookie by name in a `Cookie` request header value.
fn cookie_value<'a>(cookie_header: &'a str, name: &str) -> Option<&'a str> {
    cookie_header.split(';').find_map(|pair| {
        let (k, v) = pair.trim().split_once('=')?;
        (k == name).then_some(v)
    })
}

/// Constant-time check that the callback `state` equals the browser's cookie value.
/// A missing or empty cookie never matches.
pub(crate) fn state_matches_cookie(state_param: &str, cookie: Option<&str>) -> bool {
    use subtle::ConstantTimeEq;
    match cookie {
        Some(c) if !c.is_empty() && c.len() == state_param.len() => {
            c.as_bytes().ct_eq(state_param.as_bytes()).into()
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OAUTH: StateCookie = StateCookie {
        name: "rivetr_oauth_state",
        path: "/api/auth/oauth-login",
    };
    const SSO: StateCookie = StateCookie {
        name: "rivetr_sso_state",
        path: "/auth/sso",
    };

    #[test]
    fn cookie_attributes() {
        let c = OAUTH.header("abc", false);
        assert!(c.starts_with("rivetr_oauth_state=abc;"));
        for attr in [
            "HttpOnly",
            "SameSite=Lax",
            "Max-Age=600",
            "Path=/api/auth/oauth-login",
        ] {
            assert!(c.contains(attr), "{c}");
        }
        assert!(!c.contains("Secure"));
        assert!(OAUTH.header("abc", true).ends_with("; Secure"));
    }

    #[test]
    fn sso_cookie_is_scoped_and_named_separately() {
        let c = SSO.header("abc", false);
        assert!(c.starts_with("rivetr_sso_state=abc;"));
        assert!(c.contains("Path=/auth/sso;"));
        assert!(c.contains("HttpOnly") && c.contains("SameSite=Lax"));
    }

    #[test]
    fn clear_cookie_expires_immediately() {
        let c = OAUTH.clear_header(true);
        assert!(c.starts_with("rivetr_oauth_state=;"));
        assert!(c.contains("Max-Age=0") && c.ends_with("; Secure"));
        assert!(!OAUTH.clear_header(false).contains("Secure"));
    }

    #[test]
    fn parses_cookie_among_others() {
        let n = OAUTH.name;
        assert_eq!(
            cookie_value("a=1; rivetr_oauth_state=xyz; b=2", n),
            Some("xyz")
        );
        assert_eq!(cookie_value("a=1", n), None);
        assert_eq!(cookie_value("", n), None);
        // Name must match exactly, not as a suffix/prefix.
        assert_eq!(cookie_value("x_rivetr_oauth_state=1", n), None);
        assert_eq!(cookie_value("rivetr_oauth_state2=1", n), None);
    }

    #[test]
    fn reads_cookie_from_header_map() {
        let mut h = HeaderMap::new();
        h.append(axum::http::header::COOKIE, "a=1".parse().unwrap());
        h.append(
            axum::http::header::COOKIE,
            "rivetr_oauth_state=s1".parse().unwrap(),
        );
        assert_eq!(OAUTH.read(&h), Some("s1"));
        // The OAuth cookie is not mistaken for the SSO one.
        assert_eq!(SSO.read(&h), None);
        assert_eq!(OAUTH.read(&HeaderMap::new()), None);
    }

    #[test]
    fn state_must_match_cookie() {
        assert!(state_matches_cookie("s-1", Some("s-1")));
        assert!(!state_matches_cookie("s-1", Some("s-2")));
        assert!(!state_matches_cookie("s-1", Some("s-12")));
        assert!(!state_matches_cookie("s-1", Some("")));
        assert!(!state_matches_cookie("s-1", None));
        assert!(!state_matches_cookie("", Some("")));
    }
}

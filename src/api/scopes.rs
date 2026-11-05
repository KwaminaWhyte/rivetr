//! API token scopes and the central route-to-scope table.
//!
//! Every route under `/api` that an API token can reach is classified here by
//! HTTP method and matched path template (the `MatchedPath` axum records, e.g.
//! `/api/apps/{id}/deploy`). The classification is an [`ActionKind`]; a token
//! needs the matching [`TokenScope`] to call the route. The same classification
//! drives the team RBAC ability checks in `authz.rs`, so a route is mapped once.
//!
//! Rules are matched top to bottom, first match wins. Every top-level API area
//! is listed explicitly and a request whose route matches no rule is refused, so
//! a new area cannot silently become reachable by every token. The unit test
//! `every_api_route_has_a_scope` parses `api/mod.rs` and fails when a route is
//! not covered by a rule.
//!
//! Scope set (see `docs/reference/api.md#api-token-scopes`):
//! - `read`: GET requests that do not reveal secrets.
//! - `deploy`: deploy, start, stop, restart, roll back, cancel, approve.
//! - `write`: create and change (POST/PUT), container terminals.
//! - `delete`: DELETE requests.
//! - `secrets`: reveal secret values (`?reveal=true`, resolved env, container
//!   files, backup downloads and exports).
//! - `admin`: instance and team administration, host access (server files and
//!   terminal, reboot, patching); also implies every other scope.

use std::collections::BTreeSet;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{MatchedPath, State},
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use super::api_tokens::TokenContext;
use crate::AppState;

/// What a request does, independent of who sends it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Read,
    Deploy,
    Write,
    Delete,
    /// Reveals secret values.
    Secrets,
    /// Instance or team administration, or host-level access.
    Admin,
}

/// A permission an API token can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenScope {
    Read,
    Deploy,
    Write,
    Delete,
    Secrets,
    Admin,
}

impl TokenScope {
    pub const ALL: [TokenScope; 6] = [
        TokenScope::Read,
        TokenScope::Deploy,
        TokenScope::Write,
        TokenScope::Delete,
        TokenScope::Secrets,
        TokenScope::Admin,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            TokenScope::Read => "read",
            TokenScope::Deploy => "deploy",
            TokenScope::Write => "write",
            TokenScope::Delete => "delete",
            TokenScope::Secrets => "secrets",
            TokenScope::Admin => "admin",
        }
    }

    pub fn parse(s: &str) -> Option<TokenScope> {
        TokenScope::ALL.into_iter().find(|t| t.as_str() == s.trim())
    }

    /// The scope a route of this action kind requires.
    pub fn for_action(action: ActionKind) -> TokenScope {
        match action {
            ActionKind::Read => TokenScope::Read,
            ActionKind::Deploy => TokenScope::Deploy,
            ActionKind::Write => TokenScope::Write,
            ActionKind::Delete => TokenScope::Delete,
            ActionKind::Secrets => TokenScope::Secrets,
            ActionKind::Admin => TokenScope::Admin,
        }
    }
}

pub type ScopeSet = BTreeSet<TokenScope>;

/// Every scope; what tokens created before scopes existed carry.
pub fn full_scopes() -> ScopeSet {
    TokenScope::ALL.into_iter().collect()
}

/// Parse the comma separated `api_tokens.scopes` column. Unknown entries are
/// dropped, so a typo can only remove access, never add it.
pub fn parse_scopes(csv: &str) -> ScopeSet {
    csv.split(',').filter_map(TokenScope::parse).collect()
}

pub fn scopes_to_csv(set: &ScopeSet) -> String {
    set.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(",")
}

/// Does a token with `held` scopes satisfy `needed`? `admin` implies all.
pub fn scope_allows(held: &ScopeSet, needed: TokenScope) -> bool {
    held.contains(&TokenScope::Admin) || held.contains(&needed)
}

/// How a rule turns a method into an action.
#[derive(Debug, Clone, Copy)]
enum Class {
    /// GET/HEAD read, DELETE delete, anything else write.
    Crud,
    Fixed(ActionKind),
}

/// Which methods a rule applies to.
#[derive(Debug, Clone, Copy)]
enum Methods {
    Any,
    Get,
    Post,
    /// Anything but GET/HEAD/OPTIONS.
    Mutating,
}

impl Methods {
    fn matches(self, m: &Method) -> bool {
        match self {
            Methods::Any => true,
            Methods::Get => *m == Method::GET || *m == Method::HEAD,
            Methods::Post => *m == Method::POST,
            Methods::Mutating => !matches!(*m, Method::GET | Method::HEAD | Method::OPTIONS),
        }
    }
}

/// One row of the route table. `pattern` is a path under `/api`; `*` matches
/// one segment and a trailing `**` matches zero or more segments.
struct Rule {
    methods: Methods,
    pattern: &'static str,
    class: Class,
}

const fn r(methods: Methods, pattern: &'static str, class: Class) -> Rule {
    Rule {
        methods,
        pattern,
        class,
    }
}

use ActionKind::{Admin, Deploy, Secrets, Write};
use Class::{Crud, Fixed};
use Methods::{Any, Get, Mutating, Post};

/// THE route table. Order matters: specific rules first, then one CRUD rule per
/// top-level area. Adding a route under a new top-level path needs a new area
/// row here or the coverage test fails (and tokens are refused at runtime).
#[rustfmt::skip]
static RULES: &[Rule] = &[
    // ── Instance administration and account security: admin only ─────────────
    r(Any, "/tokens/**", Fixed(Admin)),
    r(Any, "/settings/**", Fixed(Admin)),
    r(Any, "/sso/**", Fixed(Admin)),
    r(Any, "/system/**", Fixed(Admin)),
    r(Any, "/audit/**", Fixed(Admin)),
    r(Any, "/server-ops/**", Fixed(Admin)),
    r(Any, "/swarm/**", Fixed(Admin)),
    r(Any, "/auth/**", Fixed(Admin)),
    r(Any, "/passkeys/**", Fixed(Admin)),
    r(Any, "/backups/**", Fixed(Admin)),
    // DNS provider credentials and the instance wildcard certificate (admin only in the API too)
    r(Any, "/dns-providers/**", Fixed(Admin)),
    r(Any, "/certificates/**", Fixed(Admin)),
    r(Any, "/invitations/**", Fixed(Admin)),
    r(Any, "/analytics/settings", Fixed(Admin)),
    r(Mutating, "/white-label", Fixed(Admin)),
    // Host-level access on servers.
    r(Any, "/servers/*/files/**", Fixed(Admin)),
    r(Any, "/servers/*/terminal", Fixed(Admin)),
    r(Any, "/servers/*/reboot", Fixed(Admin)),
    r(Any, "/servers/*/patching/update", Fixed(Admin)),
    r(Any, "/servers/*/install-docker", Fixed(Admin)),
    r(Any, "/servers/*/cleanup", Fixed(Admin)),
    r(Any, "/servers/*/images/*", Fixed(Admin)),
    // Team administration.
    r(Mutating, "/teams/*", Fixed(Admin)),
    r(Mutating, "/teams/*/2fa-enforcement", Fixed(Admin)),
    r(Mutating, "/teams/*/members/**", Fixed(Admin)),
    r(Mutating, "/teams/*/invitations/**", Fixed(Admin)),
    r(Mutating, "/teams/*/roles/**", Fixed(Admin)),
    r(Get, "/teams/*/audit-logs", Fixed(Admin)),

    // ── Secrets: reading values that are otherwise masked ─────────────────────
    r(Get, "/apps/*/env-vars/resolved", Fixed(Secrets)),
    r(Get, "/apps/*/linked-env-vars", Fixed(Secrets)),
    r(Get, "/services/*/generated-vars", Fixed(Secrets)),
    r(Get, "/services/*/export-db", Fixed(Secrets)),
    r(Get, "/databases/*/backups/*/download", Fixed(Secrets)),
    r(Get, "/projects/*/export", Fixed(Secrets)),
    r(Get, "/apps/*/files/**", Fixed(Secrets)),
    r(Get, "/databases/*/files/**", Fixed(Secrets)),
    r(Get, "/services/*/files/**", Fixed(Secrets)),

    // ── Container terminals: exec into a running container ────────────────────
    r(Get, "/apps/*/terminal", Fixed(Write)),
    r(Get, "/services/*/containers/*/terminal", Fixed(Write)),

    // ── Deploy actions ─────────────────────────────────────────────────────────
    r(Post, "/apps/*/deploy/**", Fixed(Deploy)),
    r(Post, "/apps/*/start", Fixed(Deploy)),
    r(Post, "/apps/*/stop", Fixed(Deploy)),
    r(Post, "/apps/*/restart", Fixed(Deploy)),
    r(Post, "/apps/*/replicas/*/restart", Fixed(Deploy)),
    r(Post, "/apps/*/deployments/*/cancel", Fixed(Deploy)),
    r(Post, "/apps/*/jobs/*/run", Fixed(Deploy)),
    r(Post, "/deployments/*/rollback", Fixed(Deploy)),
    r(Post, "/deployments/*/approve", Fixed(Deploy)),
    r(Post, "/deployments/*/reject", Fixed(Deploy)),
    r(Post, "/previews/*/redeploy", Fixed(Deploy)),
    r(Post, "/services/*/deploy", Fixed(Deploy)),
    r(Post, "/services/*/start", Fixed(Deploy)),
    r(Post, "/services/*/stop", Fixed(Deploy)),
    r(Post, "/services/*/restart", Fixed(Deploy)),
    r(Post, "/services/*/containers/*/restart", Fixed(Deploy)),
    r(Post, "/services/*/deployments/*/rollback", Fixed(Deploy)),
    r(Post, "/databases/*/start", Fixed(Deploy)),
    r(Post, "/databases/*/stop", Fixed(Deploy)),
    r(Post, "/bulk/deploy", Fixed(Deploy)),
    r(Post, "/bulk/restart", Fixed(Deploy)),
    r(Post, "/bulk/start", Fixed(Deploy)),
    r(Post, "/bulk/stop", Fixed(Deploy)),
    r(Post, "/tags/*/deploy", Fixed(Deploy)),

    // ── Areas: CRUD by method ──────────────────────────────────────────────────
    r(Any, "/apps/**", Crud),
    r(Any, "/services/**", Crud),
    r(Any, "/databases/**", Crud),
    r(Any, "/deployments/**", Crud),
    r(Any, "/previews/**", Crud),
    r(Any, "/volumes/**", Crud),
    r(Any, "/projects/**", Crud),
    r(Any, "/environments/**", Crud),
    r(Any, "/servers/**", Crud),
    r(Any, "/build-servers/**", Crud),
    r(Any, "/build/**", Crud),
    r(Any, "/tunnels/**", Crud),
    r(Any, "/github-apps/**", Crud),
    r(Any, "/git-providers/**", Crud),
    r(Any, "/ssh-keys/**", Crud),
    r(Any, "/ca-certificates/**", Crud),
    r(Any, "/destinations/**", Crud),
    r(Any, "/notification-channels/**", Crud),
    r(Any, "/notification-subscriptions/**", Crud),
    r(Any, "/s3/**", Crud),
    r(Any, "/tags/**", Crud),
    r(Any, "/templates/**", Crud),
    r(Any, "/routes/**", Crud),
    r(Any, "/domains/**", Crud),
    r(Any, "/clone-jobs/**", Crud),
    r(Any, "/events/**", Crud),
    r(Any, "/proxy/**", Crud),
    r(Any, "/security/**", Crud),
    r(Any, "/webhook-events/**", Crud),
    r(Any, "/analytics/**", Crud),
    r(Any, "/preferences/**", Crud),
    r(Any, "/ai/**", Crud),
    r(Any, "/teams/**", Crud),
    r(Get, "/white-label", Crud),
];

fn pattern_matches(pattern: &str, path: &str) -> bool {
    let pat: Vec<&str> = pattern.trim_matches('/').split('/').collect();
    let segs: Vec<&str> = path
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    let mut i = 0;
    for (pi, p) in pat.iter().enumerate() {
        if *p == "**" {
            debug_assert!(pi == pat.len() - 1, "`**` must be the last segment");
            return true;
        }
        match segs.get(i) {
            Some(s) if *p == "*" || p == s => i += 1,
            _ => return false,
        }
    }
    i == segs.len()
}

/// Classify a request by method and route template (with or without the
/// `/api` prefix). `None` means the route is not in the table.
pub fn classify(method: &Method, template: &str) -> Option<ActionKind> {
    let path = template.strip_prefix("/api").unwrap_or(template);
    let rule = RULES
        .iter()
        .find(|rule| rule.methods.matches(method) && pattern_matches(rule.pattern, path))?;
    Some(match rule.class {
        Fixed(a) => a,
        Crud => match *method {
            Method::GET | Method::HEAD | Method::OPTIONS => ActionKind::Read,
            Method::DELETE => ActionKind::Delete,
            _ => ActionKind::Write,
        },
    })
}

/// `?reveal=true` (or `reveal=1`) turns a read into a secrets read.
pub fn query_reveals_secrets(query: Option<&str>) -> bool {
    query.is_some_and(|q| {
        q.split('&').any(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            k == "reveal" && matches!(v, "true" | "1")
        })
    })
}

/// Action of a live request: the table classification plus the reveal upgrade.
pub fn classify_request(
    method: &Method,
    template: &str,
    query: Option<&str>,
) -> Option<ActionKind> {
    let action = classify(method, template)?;
    if action == ActionKind::Read && query_reveals_secrets(query) {
        return Some(ActionKind::Secrets);
    }
    Some(action)
}

fn forbidden(code: &str, message: String) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({ "error": { "code": code, "message": message } })),
    )
        .into_response()
}

/// Enforce API token scopes and restrictions on the protected API router.
///
/// Requests without an API token (dashboard sessions, the admin token from the
/// config) pass through unchanged. For `rvt_` tokens the route must be in the
/// table, the token must hold the scope, and team/resource restrictions are
/// checked by `authz::check_token_restrictions`.
///
/// On the main API group the token was already resolved by `auth_middleware`
/// (a `TokenContext` extension). WebSocket routes do their own auth, so here the
/// token is resolved from the request when the extension is missing.
pub async fn token_scope_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let ctx = match request.extensions().get::<TokenContext>().cloned() {
        Some(ctx) => ctx,
        None => match super::authz::request_token(&request) {
            Some(t) if t.starts_with("rvt_") => {
                match super::api_tokens::resolve_api_token(&state.db, &t).await {
                    Ok(Some(ctx)) => ctx,
                    Ok(None) => return StatusCode::UNAUTHORIZED.into_response(),
                    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                }
            }
            _ => return next.run(request).await,
        },
    };

    let Some(template) = request
        .extensions()
        .get::<MatchedPath>()
        .map(|m| m.as_str().to_string())
    else {
        return forbidden(
            "insufficient_scope",
            "This route is not available to API tokens".into(),
        );
    };
    let Some(action) = classify_request(request.method(), &template, request.uri().query()) else {
        return forbidden(
            "insufficient_scope",
            "This route has no API token scope mapping".into(),
        );
    };
    let needed = TokenScope::for_action(action);
    if !scope_allows(&ctx.scopes, needed) {
        return forbidden(
            "insufficient_scope",
            format!("This API token needs the `{}` scope", needed.as_str()),
        );
    }

    if ctx.is_restricted() {
        match super::authz::check_token_restrictions(&state, &ctx, action, request).await {
            Ok(req) => request = req,
            Err(resp) => return resp,
        }
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(path: &str) -> Option<ActionKind> {
        classify(&Method::GET, path)
    }
    fn post(path: &str) -> Option<ActionKind> {
        classify(&Method::POST, path)
    }

    #[test]
    fn scope_parsing_and_admin_implies_all() {
        let s = parse_scopes("read, deploy,bogus");
        assert_eq!(s.len(), 2);
        assert!(scope_allows(&s, TokenScope::Read));
        assert!(scope_allows(&s, TokenScope::Deploy));
        assert!(!scope_allows(&s, TokenScope::Write));
        let admin = parse_scopes("admin");
        for t in TokenScope::ALL {
            assert!(scope_allows(&admin, t));
        }
        assert_eq!(
            scopes_to_csv(&full_scopes()),
            "read,deploy,write,delete,secrets,admin"
        );
    }

    #[test]
    fn route_classes() {
        // Reads
        assert_eq!(get("/api/apps"), Some(ActionKind::Read));
        assert_eq!(get("/api/apps/{id}/deployments"), Some(ActionKind::Read));
        assert_eq!(get("/api/deployments/{id}/logs"), Some(ActionKind::Read));
        // Deploy
        assert_eq!(post("/api/apps/{id}/deploy"), Some(ActionKind::Deploy));
        assert_eq!(
            post("/api/apps/{id}/deploy/upload"),
            Some(ActionKind::Deploy)
        );
        assert_eq!(post("/api/apps/{id}/restart"), Some(ActionKind::Deploy));
        assert_eq!(
            post("/api/deployments/{id}/rollback"),
            Some(ActionKind::Deploy)
        );
        assert_eq!(post("/api/services/{id}/deploy"), Some(ActionKind::Deploy));
        assert_eq!(post("/api/bulk/deploy"), Some(ActionKind::Deploy));
        // Write
        assert_eq!(post("/api/apps"), Some(ActionKind::Write));
        assert_eq!(
            classify(&Method::PUT, "/api/apps/{id}"),
            Some(ActionKind::Write)
        );
        assert_eq!(post("/api/apps/{id}/env-vars"), Some(ActionKind::Write));
        assert_eq!(get("/api/apps/{id}/terminal"), Some(ActionKind::Write));
        // Delete
        assert_eq!(
            classify(&Method::DELETE, "/api/apps/{id}"),
            Some(ActionKind::Delete)
        );
        assert_eq!(
            classify(&Method::DELETE, "/api/databases/{id}"),
            Some(ActionKind::Delete)
        );
        // Secrets
        assert_eq!(
            get("/api/apps/{id}/env-vars/resolved"),
            Some(ActionKind::Secrets)
        );
        assert_eq!(
            get("/api/databases/{id}/backups/{backup_id}/download"),
            Some(ActionKind::Secrets)
        );
        assert_eq!(
            classify_request(&Method::GET, "/api/apps/{id}/env-vars", Some("reveal=true")),
            Some(ActionKind::Secrets)
        );
        assert_eq!(
            classify_request(
                &Method::GET,
                "/api/apps/{id}/env-vars",
                Some("reveal=false")
            ),
            Some(ActionKind::Read)
        );
        // Admin
        assert_eq!(get("/api/tokens"), Some(ActionKind::Admin));
        assert_eq!(get("/api/settings/advanced"), Some(ActionKind::Admin));
        assert_eq!(
            get("/api/servers/{id}/files/content"),
            Some(ActionKind::Admin)
        );
        assert_eq!(get("/api/servers/{id}/terminal"), Some(ActionKind::Admin));
        assert_eq!(post("/api/teams/{id}/members"), Some(ActionKind::Admin));
        assert_eq!(get("/api/teams/{id}/members"), Some(ActionKind::Read));
        assert_eq!(post("/api/teams"), Some(ActionKind::Write));
        assert_eq!(get("/api/servers/{id}"), Some(ActionKind::Read));
        // Not in the table.
        assert_eq!(get("/api/nope"), None);
    }

    #[test]
    fn scope_enforcement_per_route_class() {
        let read = parse_scopes("read");
        let ci = parse_scopes("read,deploy");
        let writer = parse_scopes("read,write");
        let need = |m: Method, p: &str| TokenScope::for_action(classify(&m, p).unwrap());

        // A read token can list but not deploy, change, delete or administer.
        assert!(scope_allows(&read, need(Method::GET, "/api/apps")));
        assert!(!scope_allows(
            &read,
            need(Method::POST, "/api/apps/{id}/deploy")
        ));
        assert!(!scope_allows(&read, need(Method::PUT, "/api/apps/{id}")));
        assert!(!scope_allows(&read, need(Method::DELETE, "/api/apps/{id}")));
        assert!(!scope_allows(&read, need(Method::GET, "/api/tokens")));
        assert!(!scope_allows(
            &read,
            need(Method::GET, "/api/apps/{id}/env-vars/resolved")
        ));
        // A CI token deploys but cannot edit or delete.
        assert!(scope_allows(
            &ci,
            need(Method::POST, "/api/apps/{id}/deploy")
        ));
        assert!(!scope_allows(&ci, need(Method::POST, "/api/apps")));
        assert!(!scope_allows(&ci, need(Method::DELETE, "/api/apps/{id}")));
        // Write does not imply deploy or delete.
        assert!(scope_allows(&writer, need(Method::POST, "/api/apps")));
        assert!(!scope_allows(
            &writer,
            need(Method::POST, "/api/apps/{id}/restart")
        ));
        assert!(!scope_allows(
            &writer,
            need(Method::DELETE, "/api/apps/{id}")
        ));
    }

    /// Every `(method, path)` registered on the protected API router and the
    /// WebSocket router in `api/mod.rs`.
    fn registered_api_routes() -> Vec<(Method, String)> {
        let src = include_str!("mod.rs");
        let start = src
            .find("let ws_routes = Router::new()")
            .expect("ws_routes block");
        let end = src.find(".merge(ws_routes);").expect("end of api_routes");
        let block = &src[start..end];
        let method_re = regex::Regex::new(r"\b(get|post|put|delete|patch)\(").unwrap();
        let mut out = Vec::new();
        let mut rest = block;
        while let Some(pos) = rest.find(".route(") {
            let after = &rest[pos + ".route(".len()..];
            // Find the matching close paren of `.route(`.
            let mut depth = 1;
            let mut len = 0;
            for (i, c) in after.char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            len = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let expr = &after[..len];
            let q1 = expr.find('"').expect("route path literal");
            let q2 = q1 + 1 + expr[q1 + 1..].find('"').unwrap();
            let path = format!("/api{}", &expr[q1 + 1..q2]);
            for cap in method_re.captures_iter(&expr[q2 + 1..]) {
                let m = match &cap[1] {
                    "get" => Method::GET,
                    "post" => Method::POST,
                    "put" => Method::PUT,
                    "delete" => Method::DELETE,
                    _ => Method::PATCH,
                };
                out.push((m, path.clone()));
            }
            rest = &after[len..];
        }
        out
    }

    #[test]
    fn every_api_route_has_a_scope() {
        let routes = registered_api_routes();
        assert!(
            routes.len() > 400,
            "route parser found only {} routes; did api/mod.rs change shape?",
            routes.len()
        );
        let missing: Vec<String> = routes
            .iter()
            .filter(|(m, p)| classify(m, p).is_none())
            .map(|(m, p)| format!("{m} {p}"))
            .collect();
        assert!(
            missing.is_empty(),
            "routes without an API token scope mapping in api/scopes.rs RULES:\n{}",
            missing.join("\n")
        );
    }

    /// The middleware relies on `MatchedPath` holding the full template
    /// (including the `/api` nest prefix) inside a `Router::layer`.
    #[tokio::test]
    async fn matched_path_is_the_nested_template_inside_layers() {
        use axum::{routing::post, Router};
        use tower::ServiceExt;

        async fn capture(req: Request<Body>, next: Next) -> Response {
            let template = req
                .extensions()
                .get::<MatchedPath>()
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let action = classify_request(req.method(), &template, req.uri().query());
            let _ = next.run(req).await;
            format!("{template} {action:?}").into_response()
        }
        let inner = Router::new()
            .route("/apps/{id}/deploy", post(|| async { "ok" }))
            .layer(axum::middleware::from_fn(capture));
        let app: Router = Router::new().nest("/api", inner);
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/apps/123/deploy")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        assert_eq!(&body[..], b"/api/apps/{id}/deploy Some(Deploy)");
    }

    #[test]
    fn pattern_matching() {
        assert!(pattern_matches("/apps/**", "/apps"));
        assert!(pattern_matches("/apps/**", "/apps/{id}/deploy"));
        assert!(!pattern_matches("/apps/**", "/applications"));
        assert!(pattern_matches("/apps/*/deploy/**", "/apps/{id}/deploy"));
        assert!(pattern_matches(
            "/apps/*/deploy/**",
            "/apps/{id}/deploy/upload"
        ));
        assert!(!pattern_matches("/apps/*/start", "/apps/{id}/start/now"));
        assert!(!pattern_matches("/teams/*", "/teams"));
    }
}

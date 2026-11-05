//! Deploy-time resolution of `secret://` references and path imports.
//!
//! Values are fetched from the providers when a deployment, restart or service start
//! needs them, kept in memory for that run only ([`super::session`]), registered for log
//! masking, and never written to the database in plaintext. With the `last_known`
//! failure policy the last successful values are stored encrypted and used, with a
//! warning, when a provider cannot be reached.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;

use super::providers::{ProviderError, SecretData, SecretProvider, DEFAULT_TIMEOUT};
use super::reference::{self, SecretRef};
use super::session;
use super::store::{self, LastValues, SecretImportRow, POLICY_LAST_KNOWN};
use crate::db::{App, Service};
use crate::DbPool;

/// A resource whose env is being resolved (for policy and last-values lookups).
#[derive(Debug, Clone, Copy)]
pub struct Owner<'a> {
    pub kind: &'a str,
    pub id: &'a str,
}

/// Result of resolving one owner's variables.
#[derive(Debug, Default)]
pub struct Outcome {
    pub vars: Vec<(String, String)>,
    /// Every value that came from a secret manager (to mask in logs).
    pub secret_values: Vec<String>,
    /// Non-fatal problems to show in the deployment log.
    pub warnings: Vec<String>,
}

impl Outcome {
    fn passthrough(vars: Vec<(String, String)>) -> Self {
        Self {
            vars,
            ..Default::default()
        }
    }
}

struct Connection {
    id: String,
    name: String,
    provider: Arc<dyn SecretProvider>,
}

/// Fetches secrets for one resolution run, caching each (connection, path) read.
pub struct Resolver<'a> {
    db: &'a DbPool,
    key: Option<&'a [u8; 32]>,
    team_id: Option<String>,
    deployment_id: Option<String>,
    timeout: Duration,
    by_name: HashMap<String, Connection>,
    by_id: HashMap<String, Connection>,
    cache: HashMap<(String, String), SecretData>,
}

impl<'a> Resolver<'a> {
    pub fn new(db: &'a DbPool, key: Option<&'a [u8; 32]>, team_id: Option<&str>) -> Self {
        Self {
            db,
            key,
            team_id: team_id.map(str::to_string),
            deployment_id: None,
            timeout: DEFAULT_TIMEOUT,
            by_name: HashMap::new(),
            by_id: HashMap::new(),
            cache: HashMap::new(),
        }
    }

    /// Share fetched secrets with the rest of this deployment (build and start phases).
    pub fn for_deployment(mut self, deployment_id: Option<&str>) -> Self {
        self.deployment_id = deployment_id.map(str::to_string);
        self
    }

    /// Use this provider for connection `name` instead of loading it (tests).
    #[cfg(test)]
    pub fn with_provider(mut self, name: &str, id: &str, p: Arc<dyn SecretProvider>) -> Self {
        let make = || Connection {
            id: id.to_string(),
            name: name.to_string(),
            provider: p.clone(),
        };
        self.by_name.insert(name.to_string(), make());
        self.by_id.insert(id.to_string(), make());
        self
    }

    async fn connection_by_name(&mut self, name: &str) -> Result<&Connection, String> {
        if !self.by_name.contains_key(name) {
            let row = store::find_connection_by_name(self.db, self.team_id.as_deref(), name)
                .await
                .map_err(|e| format!("could not load connection '{}': {}", name, e))?
                .ok_or_else(|| format!("no secret manager connection named '{}'", name))?;
            let provider = row
                .provider(self.key, self.timeout)
                .map_err(|e| format!("connection '{}': {}", name, e))?;
            self.by_name.insert(
                name.to_string(),
                Connection {
                    id: row.id.clone(),
                    name: row.name.clone(),
                    provider,
                },
            );
        }
        Ok(&self.by_name[name])
    }

    async fn connection_by_id(&mut self, id: &str) -> Result<&Connection, String> {
        if !self.by_id.contains_key(id) {
            let row = store::get_connection(self.db, id)
                .await
                .map_err(|e| format!("could not load connection: {}", e))?
                .ok_or_else(|| "the linked secret manager connection was deleted".to_string())?;
            if !store::connection_usable_by(row.team_id.as_deref(), self.team_id.as_deref()) {
                return Err(format!("connection '{}' belongs to another team", row.name));
            }
            let provider = row
                .provider(self.key, self.timeout)
                .map_err(|e| format!("connection '{}': {}", row.name, e))?;
            self.by_id.insert(
                id.to_string(),
                Connection {
                    id: row.id.clone(),
                    name: row.name.clone(),
                    provider,
                },
            );
        }
        Ok(&self.by_id[id])
    }

    async fn read(
        &mut self,
        conn_id: &str,
        provider: Arc<dyn SecretProvider>,
        path: &str,
    ) -> Result<SecretData, ProviderError> {
        let k = (conn_id.to_string(), path.to_string());
        if let Some(d) = self.cache.get(&k) {
            return Ok(d.clone());
        }
        if let Some(dep) = &self.deployment_id {
            if let Some(d) = session::cached(dep, conn_id, path) {
                self.cache.insert(k, d.clone());
                return Ok(d);
            }
        }
        let data = provider.read(path).await?;
        if let Some(dep) = &self.deployment_id {
            session::store(dep, conn_id, path, &data);
        }
        self.cache.insert(k, data.clone());
        Ok(data)
    }

    /// Resolve one reference to its value.
    pub async fn resolve_ref(&mut self, r: &SecretRef) -> Result<String, String> {
        let (id, provider) = {
            let c = self.connection_by_name(&r.connection).await?;
            (c.id.clone(), c.provider.clone())
        };
        let data = self
            .read(&id, provider, &r.path)
            .await
            .map_err(|e| format!("{}: {}", r, e))?;
        match &r.key {
            Some(k) => data
                .values
                .get(k)
                .cloned()
                .ok_or_else(|| format!("{}: key '{}' not found at that path", r, k)),
            None => data
                .raw
                .clone()
                .ok_or_else(|| format!("{}: add '#<key>' to pick a key from this secret", r)),
        }
    }

    /// All keys of an imported path, prefixed. Keys that are not valid env var names are
    /// skipped with a warning.
    pub async fn resolve_import(
        &mut self,
        imp: &SecretImportRow,
        warnings: &mut Vec<String>,
    ) -> Result<Vec<(String, String)>, String> {
        let (name, provider, id) = {
            let c = self.connection_by_id(&imp.connection_id).await?;
            (c.name.clone(), c.provider.clone(), c.id.clone())
        };
        let label = format!("import of '{}' from {}", imp.path, name);
        let data = self
            .read(&id, provider, &imp.path)
            .await
            .map_err(|e| format!("{}: {}", label, e))?;
        if data.values.is_empty() && data.raw.is_some() {
            return Err(format!(
                "{}: the secret is not a JSON object, reference it with a key instead",
                label
            ));
        }
        let mut out = Vec::with_capacity(data.values.len());
        for (k, v) in data.values {
            let env_key = format!("{}{}", imp.prefix, k);
            if is_valid_env_key(&env_key) {
                out.push((env_key, v));
            } else {
                warnings.push(format!(
                    "{}: skipped key '{}' (not a valid variable name)",
                    label, env_key
                ));
            }
        }
        Ok(out)
    }
}

/// Letters, digits and `_`, not starting with a digit.
pub fn is_valid_env_key(k: &str) -> bool {
    let mut chars = k.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Resolve every reference in `vars` and append the keys of `imports` that are not set
/// already. Applies the failure policy of `policy_owner` and keeps the last values of
/// `cache_owner`.
pub async fn resolve_vars(
    resolver: &mut Resolver<'_>,
    policy_owner: Owner<'_>,
    cache_owner: Owner<'_>,
    vars: Vec<(String, String)>,
    imports: &[SecretImportRow],
) -> anyhow::Result<Outcome> {
    let has_refs = vars.iter().any(|(_, v)| reference::is_reference(v));
    if !has_refs && imports.is_empty() {
        return Ok(Outcome::passthrough(vars));
    }

    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut fresh = LastValues::default();
    let mut out: Vec<(String, String)> = Vec::with_capacity(vars.len());

    for (k, v) in &vars {
        match reference::parse(v) {
            Ok(None) => out.push((k.clone(), v.clone())),
            Ok(Some(r)) => match resolver.resolve_ref(&r).await {
                Ok(val) => {
                    fresh.refs.insert(k.clone(), val.clone());
                    out.push((k.clone(), val));
                }
                Err(e) => errors.push(format!("{}: {}", k, e)),
            },
            Err(e) => errors.push(format!("{}: invalid secret reference: {}", k, e)),
        }
    }

    for imp in imports {
        match resolver.resolve_import(imp, &mut warnings).await {
            Ok(pairs) => {
                for (k, v) in pairs {
                    if !vars.iter().any(|(ek, _)| ek == &k) && !fresh.imports.contains_key(&k) {
                        fresh.imports.insert(k, v);
                    }
                }
            }
            Err(e) => errors.push(e),
        }
    }

    if errors.is_empty() {
        let added: Vec<(String, String)> = fresh
            .imports
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        out.extend(added);
        let secret_values = fresh
            .refs
            .values()
            .chain(fresh.imports.values())
            .cloned()
            .collect();
        let policy = store::get_policy(resolver.db, policy_owner.kind, policy_owner.id).await;
        if policy == POLICY_LAST_KNOWN {
            match store::save_last_values(
                resolver.db,
                resolver.key,
                cache_owner.kind,
                cache_owner.id,
                &fresh,
            )
            .await
            {
                Ok(true) => {}
                Ok(false) => warnings.push(
                    "No encryption key is configured, so resolved secrets are not kept for the 'use last values' policy"
                        .to_string(),
                ),
                Err(e) => warnings.push(format!("Could not store the resolved secrets: {}", e)),
            }
        }
        return Ok(Outcome {
            vars: out,
            secret_values,
            warnings,
        });
    }

    let summary = errors.join("; ");
    let policy = store::get_policy(resolver.db, policy_owner.kind, policy_owner.id).await;
    if policy != POLICY_LAST_KNOWN {
        anyhow::bail!("Secret manager resolution failed: {}", summary);
    }
    let Some(last) =
        store::load_last_values(resolver.db, resolver.key, cache_owner.kind, cache_owner.id).await
    else {
        anyhow::bail!(
            "Secret manager resolution failed and there are no previously resolved values to fall back to: {}",
            summary
        );
    };
    let when = store::last_resolved_at(resolver.db, cache_owner.kind, cache_owner.id)
        .await
        .unwrap_or_else(|| "an earlier deploy".to_string());

    // Rebuild from the stored values: every reference must have a stored value.
    let mut out = Vec::with_capacity(vars.len());
    let mut used: BTreeMap<String, String> = BTreeMap::new();
    for (k, v) in &vars {
        if reference::is_reference(v) {
            match last.refs.get(k) {
                Some(val) => {
                    used.insert(k.clone(), val.clone());
                    out.push((k.clone(), val.clone()));
                }
                None => anyhow::bail!(
                    "Secret manager resolution failed and {} has no previously resolved value: {}",
                    k,
                    summary
                ),
            }
        } else {
            out.push((k.clone(), v.clone()));
        }
    }
    for (k, v) in &last.imports {
        if !vars.iter().any(|(ek, _)| ek == k) {
            used.insert(k.clone(), v.clone());
            out.push((k.clone(), v.clone()));
        }
    }
    warnings.push(format!(
        "Secret manager unavailable ({}); using the values resolved at {}",
        summary, when
    ));
    Ok(Outcome {
        vars: out,
        secret_values: used.into_values().collect(),
        warnings,
    })
}

/// Resolve an app's collected env (app, environment, project and team layers already
/// merged) plus the app's and its project's path imports. Resolved values are masked in
/// `deployment_id`'s logs.
pub async fn resolve_app_env(
    db: &DbPool,
    key: Option<&[u8; 32]>,
    app: &App,
    deployment_id: Option<&str>,
    vars: Vec<(String, String)>,
) -> anyhow::Result<Outcome> {
    let mut imports = store::imports_for(db, "app", &app.id).await?;
    if let Some(project_id) = &app.project_id {
        imports.extend(store::imports_for(db, "project", project_id).await?);
    }
    let mut resolver = Resolver::new(db, key, app.team_id.as_deref()).for_deployment(deployment_id);
    let owner = Owner {
        kind: "app",
        id: &app.id,
    };
    let outcome = resolve_vars(&mut resolver, owner, owner, vars, &imports).await?;
    if let Some(dep) = deployment_id {
        session::register_values(dep, outcome.secret_values.iter().map(String::as_str));
    }
    Ok(outcome)
}

/// Resolve references in an app's BuildKit build secrets (no imports).
pub async fn resolve_app_build_secrets(
    db: &DbPool,
    key: Option<&[u8; 32]>,
    app: &App,
    deployment_id: &str,
    secrets: Vec<(String, String)>,
) -> anyhow::Result<Outcome> {
    let mut resolver =
        Resolver::new(db, key, app.team_id.as_deref()).for_deployment(Some(deployment_id));
    let outcome = resolve_vars(
        &mut resolver,
        Owner {
            kind: "app",
            id: &app.id,
        },
        Owner {
            kind: "app_build",
            id: &app.id,
        },
        secrets,
        &[],
    )
    .await?;
    session::register_values(
        deployment_id,
        outcome.secret_values.iter().map(String::as_str),
    );
    Ok(outcome)
}

/// Resolve a compose service's env vars plus its path imports.
pub async fn resolve_service_env(
    db: &DbPool,
    key: Option<&[u8; 32]>,
    service: &Service,
    vars: Vec<(String, String)>,
) -> anyhow::Result<Outcome> {
    let imports = store::imports_for(db, "service", &service.id).await?;
    let mut resolver = Resolver::new(db, key, service.team_id.as_deref());
    let owner = Owner {
        kind: "service",
        id: &service.id,
    };
    resolve_vars(&mut resolver, owner, owner, vars, &imports).await
}

#[cfg(test)]
mod tests {
    use super::super::providers::Listing;
    use super::super::store::test_db;
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    /// Provider with fixed data that can be switched off to simulate an outage.
    struct Fake {
        data: HashMap<String, SecretData>,
        down: AtomicBool,
        reads: AtomicUsize,
    }

    impl Fake {
        fn new() -> Arc<Self> {
            let mut data = HashMap::new();
            data.insert(
                "app/config".to_string(),
                SecretData {
                    values: [
                        (
                            "DATABASE_URL".to_string(),
                            "postgres://u:hunter22@db/app".to_string(),
                        ),
                        ("API_KEY".to_string(), "key-123456".to_string()),
                        ("bad-key".to_string(), "x".to_string()),
                    ]
                    .into(),
                    raw: None,
                },
            );
            data.insert(
                "plain".to_string(),
                SecretData {
                    values: Default::default(),
                    raw: Some("raw-token-value".to_string()),
                },
            );
            Arc::new(Self {
                data,
                down: AtomicBool::new(false),
                reads: AtomicUsize::new(0),
            })
        }
    }

    #[async_trait]
    impl SecretProvider for Fake {
        async fn read(&self, path: &str) -> Result<SecretData, ProviderError> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            if self.down.load(Ordering::SeqCst) {
                return Err(ProviderError::Timeout);
            }
            self.data
                .get(path)
                .cloned()
                .ok_or_else(|| ProviderError::NotFound(path.to_string()))
        }
        async fn list(&self, _: &str) -> Result<Listing, ProviderError> {
            Ok(Listing::default())
        }
        async fn test(&self) -> Result<String, ProviderError> {
            Ok("ok".into())
        }
    }

    fn import(prefix: &str) -> SecretImportRow {
        SecretImportRow {
            id: "i1".into(),
            owner_type: "app".into(),
            owner_id: "a1".into(),
            connection_id: "c1".into(),
            path: "app/config".into(),
            prefix: prefix.into(),
            created_at: String::new(),
        }
    }

    const OWNER: Owner<'static> = Owner {
        kind: "app",
        id: "a1",
    };

    fn v(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[tokio::test]
    async fn plain_env_passes_through_untouched() {
        let db = test_db::pool().await;
        let mut r = Resolver::new(&db, None, None);
        let vars = v(&[("A", "1"), ("B", "postgres://x")]);
        let out = resolve_vars(&mut r, OWNER, OWNER, vars.clone(), &[])
            .await
            .unwrap();
        assert_eq!(out.vars, vars);
        assert!(out.secret_values.is_empty());
    }

    #[tokio::test]
    async fn resolves_refs_and_imports_with_precedence() {
        let db = test_db::pool().await;
        let fake = Fake::new();
        let mut r = Resolver::new(&db, None, None).with_provider("vault", "c1", fake.clone());
        let vars = v(&[
            ("DB", "secret://vault/app/config#DATABASE_URL"),
            ("TOKEN", "secret://vault/plain"),
            ("APP_API_KEY", "explicit-wins"),
            ("PLAIN", "1"),
        ]);
        let out = resolve_vars(&mut r, OWNER, OWNER, vars, &[import("APP_")])
            .await
            .unwrap();
        let get = |k: &str| {
            out.vars
                .iter()
                .find(|(x, _)| x == k)
                .map(|(_, v)| v.clone())
        };
        assert_eq!(get("DB").unwrap(), "postgres://u:hunter22@db/app");
        assert_eq!(get("TOKEN").unwrap(), "raw-token-value");
        assert_eq!(get("APP_API_KEY").unwrap(), "explicit-wins");
        assert_eq!(
            get("APP_DATABASE_URL").unwrap(),
            "postgres://u:hunter22@db/app"
        );
        assert_eq!(get("PLAIN").unwrap(), "1");
        assert!(get("APP_bad-key").is_none());
        assert!(out.warnings.iter().any(|w| w.contains("bad-key")));
        // One fetch per path for the whole run.
        assert_eq!(fake.reads.load(Ordering::SeqCst), 2);
        assert!(out
            .secret_values
            .contains(&"postgres://u:hunter22@db/app".to_string()));
    }

    #[tokio::test]
    async fn failure_fails_by_default_without_leaking_values() {
        let db = test_db::pool().await;
        let fake = Fake::new();
        let mut r = Resolver::new(&db, None, None).with_provider("vault", "c1", fake.clone());
        let vars = v(&[("DB", "secret://vault/app/config#MISSING")]);
        let err = resolve_vars(&mut r, OWNER, OWNER, vars, &[])
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("key 'MISSING' not found"), "{err}");
        assert!(!err.contains("hunter22"));

        let mut r = Resolver::new(&db, None, None);
        let err = resolve_vars(&mut r, OWNER, OWNER, v(&[("X", "secret://nope/a#b")]), &[])
            .await
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("no secret manager connection named 'nope'"),
            "{err}"
        );

        let err = resolve_vars(&mut r, OWNER, OWNER, v(&[("X", "secret://Bad/a#b")]), &[])
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("invalid secret reference"), "{err}");
    }

    #[tokio::test]
    async fn last_known_policy_falls_back_to_encrypted_values() {
        let db = test_db::pool().await;
        let key = crate::crypto::derive_key("k");
        store::set_policy(&db, "app", "a1", POLICY_LAST_KNOWN)
            .await
            .unwrap();
        let fake = Fake::new();
        let vars = v(&[("DB", "secret://vault/app/config#DATABASE_URL"), ("P", "1")]);

        // Outage before any success: nothing to fall back to.
        fake.down.store(true, Ordering::SeqCst);
        let mut r = Resolver::new(&db, Some(&key), None).with_provider("vault", "c1", fake.clone());
        let err = resolve_vars(&mut r, OWNER, OWNER, vars.clone(), &[import("")])
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("no previously resolved values"), "{err}");

        // A success stores the values encrypted.
        fake.down.store(false, Ordering::SeqCst);
        let mut r = Resolver::new(&db, Some(&key), None).with_provider("vault", "c1", fake.clone());
        resolve_vars(&mut r, OWNER, OWNER, vars.clone(), &[import("")])
            .await
            .unwrap();
        let raw: String = sqlx::query_scalar(
            "SELECT last_values FROM secret_resolution_state WHERE owner_id = 'a1'",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert!(!raw.contains("hunter22"));

        // Next outage uses them, with a warning.
        fake.down.store(true, Ordering::SeqCst);
        let mut r = Resolver::new(&db, Some(&key), None).with_provider("vault", "c1", fake.clone());
        let out = resolve_vars(&mut r, OWNER, OWNER, vars.clone(), &[import("")])
            .await
            .unwrap();
        let get = |k: &str| {
            out.vars
                .iter()
                .find(|(x, _)| x == k)
                .map(|(_, v)| v.clone())
        };
        assert_eq!(get("DB").unwrap(), "postgres://u:hunter22@db/app");
        assert_eq!(get("API_KEY").unwrap(), "key-123456");
        assert_eq!(get("P").unwrap(), "1");
        assert!(out
            .warnings
            .iter()
            .any(|w| w.contains("using the values resolved at")));
        assert!(out.secret_values.contains(&"key-123456".to_string()));

        // A reference added since the last success has no stored value: fail.
        let mut more = vars.clone();
        more.push(("NEW".into(), "secret://vault/app/config#API_KEY".into()));
        let mut r = Resolver::new(&db, Some(&key), None).with_provider("vault", "c1", fake.clone());
        assert!(resolve_vars(&mut r, OWNER, OWNER, more, &[]).await.is_err());
    }

    #[tokio::test]
    async fn default_policy_does_not_store_values() {
        let db = test_db::pool().await;
        let key = crate::crypto::derive_key("k");
        let fake = Fake::new();
        let mut r = Resolver::new(&db, Some(&key), None).with_provider("vault", "c1", fake);
        resolve_vars(
            &mut r,
            OWNER,
            OWNER,
            v(&[("DB", "secret://vault/app/config#DATABASE_URL")]),
            &[],
        )
        .await
        .unwrap();
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM secret_resolution_state")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn deployment_session_shares_reads_and_masks_values() {
        let db = test_db::pool().await;
        let fake = Fake::new();
        let dep = "dep-resolve-session";
        let _guard = session::SessionGuard::new(dep);
        for _ in 0..2 {
            let mut r = Resolver::new(&db, None, None)
                .with_provider("vault", "c1", fake.clone())
                .for_deployment(Some(dep));
            let out = resolve_vars(
                &mut r,
                OWNER,
                OWNER,
                v(&[("DB", "secret://vault/app/config#DATABASE_URL")]),
                &[],
            )
            .await
            .unwrap();
            session::register_values(dep, out.secret_values.iter().map(String::as_str));
        }
        assert_eq!(fake.reads.load(Ordering::SeqCst), 1);
        assert_eq!(
            session::redact(dep, "connecting to postgres://u:hunter22@db/app failed"),
            "connecting to ******** failed"
        );
    }

    #[tokio::test]
    async fn imports_of_other_teams_connections_are_refused() {
        let db = test_db::pool().await;
        sqlx::query("INSERT INTO teams (id) VALUES ('t-other')")
            .execute(&db)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO secret_connections (id, team_id, name, provider) VALUES ('c9', 't-other', 'x', 'doppler')",
        )
        .execute(&db)
        .await
        .unwrap();
        let mut r = Resolver::new(&db, None, Some("t-mine"));
        let mut imp = import("");
        imp.connection_id = "c9".into();
        let err = resolve_vars(&mut r, OWNER, OWNER, vec![], &[imp])
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("belongs to another team"), "{err}");
    }

    #[test]
    fn env_key_rules() {
        assert!(is_valid_env_key("A_1"));
        assert!(is_valid_env_key("_X"));
        assert!(!is_valid_env_key("1A"));
        assert!(!is_valid_env_key("a-b"));
        assert!(!is_valid_env_key(""));
    }
}

//! Reading which image each compose service runs, by digest.

use anyhow::Result;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

use super::runner::{CommandRunner, CommandSpec};
use crate::db::{ServiceImageMap, ServiceImageState};

/// One container as reported by `docker compose ps --format json`.
#[derive(Debug, Clone, Deserialize, Default, serde::Serialize)]
#[serde(default)]
pub struct ComposePsEntry {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Image")]
    pub image: String,
    #[serde(rename = "Service")]
    pub service: String,
    #[serde(rename = "State")]
    pub state: String,
    #[serde(rename = "Health")]
    pub health: String,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "CreatedAt")]
    pub created_at: String,
    #[serde(rename = "ExitCode")]
    pub exit_code: i64,
    #[serde(rename = "Publishers")]
    pub publishers: Option<Vec<ComposePublisher>>,
}

#[derive(Debug, Clone, Deserialize, Default, serde::Serialize)]
#[serde(default)]
pub struct ComposePublisher {
    #[serde(rename = "URL")]
    pub url: String,
    #[serde(rename = "TargetPort")]
    pub target_port: u32,
    #[serde(rename = "PublishedPort")]
    pub published_port: u32,
    #[serde(rename = "Protocol")]
    pub protocol: String,
}

/// Parse `docker compose ps --format json`. Compose v2.21+ prints one object
/// per line; older versions print a single JSON array. Both are accepted.
pub fn parse_compose_ps(output: &str) -> Vec<ComposePsEntry> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return vec![];
    }
    if trimmed.starts_with('[') {
        return serde_json::from_str(trimmed).unwrap_or_default();
    }
    trimmed
        .lines()
        .filter_map(|l| serde_json::from_str::<ComposePsEntry>(l.trim()).ok())
        .collect()
}

/// `repo` part of an image reference: strips `@digest` and a trailing `:tag`
/// (a colon after the last slash), keeping registry ports like `localhost:5000`.
pub fn image_repo(image: &str) -> &str {
    let no_digest = image.split('@').next().unwrap_or(image);
    match no_digest.rfind(':') {
        Some(colon) if no_digest[colon..].find('/').is_none() => &no_digest[..colon],
        _ => no_digest,
    }
}

/// Normalize Docker Hub short names so `postgres` matches `docker.io/library/postgres`.
fn normalize_repo(repo: &str) -> String {
    let r = repo
        .strip_prefix("docker.io/")
        .or_else(|| repo.strip_prefix("index.docker.io/"))
        .unwrap_or(repo);
    r.strip_prefix("library/").unwrap_or(r).to_string()
}

/// Pick the repo digest that belongs to `image` out of an image's RepoDigests.
pub fn pick_digest(image: &str, repo_digests: &[String]) -> Option<String> {
    if image.contains("@sha256:") {
        return Some(image.to_string());
    }
    let want = normalize_repo(image_repo(image));
    repo_digests
        .iter()
        .find(|d| normalize_repo(image_repo(d)) == want)
        .or_else(|| repo_digests.first())
        .cloned()
}

/// List the containers of a compose project (running and stopped).
pub async fn compose_ps(
    runner: &dyn CommandRunner,
    compose_dir: &Path,
    project: &str,
) -> Result<Vec<ComposePsEntry>> {
    let spec = CommandSpec::new(
        "docker",
        &["compose", "-p", project, "ps", "-a", "--format", "json"],
    )
    .cwd(compose_dir);
    let out = runner.run(&spec, None).await?;
    if !out.success {
        anyhow::bail!("docker compose ps failed: {}", out.error_tail());
    }
    Ok(parse_compose_ps(&out.stdout))
}

/// Image ID and registry digest of every compose service in the project.
/// Services without a container (never created) are absent from the map.
pub async fn capture_images(
    runner: &dyn CommandRunner,
    compose_dir: &Path,
    project: &str,
) -> Result<ServiceImageMap> {
    let containers = compose_ps(runner, compose_dir, project).await?;
    let mut map: ServiceImageMap = BTreeMap::new();
    if containers.is_empty() {
        return Ok(map);
    }

    // container id -> image id
    let mut args = vec![
        "inspect".to_string(),
        "--format".to_string(),
        "{{.Id}} {{.Image}}".to_string(),
    ];
    args.extend(containers.iter().map(|c| c.id.clone()));
    let out = runner.run(&CommandSpec::args("docker", args), None).await?;
    let mut image_ids: BTreeMap<String, String> = BTreeMap::new();
    for line in out.stdout.lines() {
        let mut parts = line.split_whitespace();
        if let (Some(cid), Some(iid)) = (parts.next(), parts.next()) {
            image_ids.insert(cid.to_string(), iid.to_string());
        }
    }

    // image id -> repo digests
    let mut unique: Vec<String> = image_ids.values().cloned().collect();
    unique.sort();
    unique.dedup();
    let mut digests: BTreeMap<String, Vec<String>> = BTreeMap::new();
    if !unique.is_empty() {
        let mut args = vec![
            "image".to_string(),
            "inspect".to_string(),
            "--format".to_string(),
            "{{.Id}} {{json .RepoDigests}}".to_string(),
        ];
        args.extend(unique);
        let out = runner.run(&CommandSpec::args("docker", args), None).await?;
        for line in out.stdout.lines() {
            if let Some((iid, json)) = line.split_once(' ') {
                let list: Vec<String> = serde_json::from_str(json).unwrap_or_default();
                digests.insert(iid.to_string(), list);
            }
        }
    }

    for c in containers {
        // Container IDs from `compose ps` may be shortened; match by prefix.
        let image_id = image_ids
            .iter()
            .find(|(full, _)| full.starts_with(&c.id) || c.id.starts_with(full.as_str()))
            .map(|(_, iid)| iid.clone());
        let digest = image_id
            .as_ref()
            .and_then(|iid| digests.get(iid))
            .and_then(|list| pick_digest(&c.image, list));
        // Several containers of one service (scale > 1) share an image; first wins.
        map.entry(c.service.clone()).or_insert(ServiceImageState {
            image: c.image.clone(),
            image_id,
            digest,
        });
    }
    Ok(map)
}

/// Compose services whose image changed between two captures (or are new).
pub fn changed_services(before: &ServiceImageMap, after: &ServiceImageMap) -> Vec<String> {
    after
        .iter()
        .filter(|(svc, state)| {
            before
                .get(*svc)
                .map(|b| b.image_id != state.image_id)
                .unwrap_or(true)
        })
        .map(|(svc, _)| svc.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ndjson_and_array_ps_output() {
        let nd = r#"{"ID":"abc","Name":"p-web-1","Image":"localhost:5000/app:latest","Service":"web","State":"running","Health":"healthy","Status":"Up 2 minutes (healthy)"}
{"ID":"def","Name":"p-db-1","Image":"postgres:18","Service":"db","State":"running","Health":"","Status":"Up"}"#;
        let a = parse_compose_ps(nd);
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].service, "web");
        assert_eq!(a[0].health, "healthy");

        let arr = r#"[{"ID":"abc","Name":"p-web-1","Image":"x","Service":"web","State":"exited"}]"#;
        let b = parse_compose_ps(arr);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].state, "exited");
        assert!(parse_compose_ps("").is_empty());
    }

    #[test]
    fn image_repo_keeps_registry_port() {
        assert_eq!(
            image_repo("localhost:5000/megasign-app:latest"),
            "localhost:5000/megasign-app"
        );
        assert_eq!(
            image_repo("localhost:5000/megasign-app"),
            "localhost:5000/megasign-app"
        );
        assert_eq!(image_repo("postgres:18.3-alpine"), "postgres");
        assert_eq!(image_repo("ghcr.io/o/r@sha256:abc"), "ghcr.io/o/r");
    }

    #[test]
    fn picks_matching_digest() {
        let digests = vec![
            "other/repo@sha256:111".to_string(),
            "localhost:5000/megasign-app@sha256:222".to_string(),
        ];
        assert_eq!(
            pick_digest("localhost:5000/megasign-app:latest", &digests).as_deref(),
            Some("localhost:5000/megasign-app@sha256:222")
        );
        assert_eq!(
            pick_digest("postgres:18", &["postgres@sha256:333".to_string()]).as_deref(),
            Some("postgres@sha256:333")
        );
        assert_eq!(pick_digest("local-only:latest", &[]), None);
        // An image already pinned by digest is its own pin.
        assert_eq!(
            pick_digest("r@sha256:444", &[]).as_deref(),
            Some("r@sha256:444")
        );
    }

    #[test]
    fn detects_changed_services() {
        let mut before = ServiceImageMap::new();
        before.insert(
            "web".into(),
            ServiceImageState {
                image: "a".into(),
                image_id: Some("sha256:1".into()),
                digest: None,
            },
        );
        before.insert(
            "db".into(),
            ServiceImageState {
                image: "pg".into(),
                image_id: Some("sha256:9".into()),
                digest: None,
            },
        );
        let mut after = before.clone();
        after.get_mut("web").unwrap().image_id = Some("sha256:2".into());
        after.insert("worker".into(), ServiceImageState::default());
        assert_eq!(changed_services(&before, &after), vec!["web", "worker"]);
    }
}

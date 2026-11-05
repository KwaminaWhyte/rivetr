//! Container image inventory for a server: which images exist, how big they
//! are, which containers use them, and removal of unused ones.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

use super::{parse_human_size, shell_quote, HostTarget};
use crate::DbPool;

/// One image row (an image with several tags shows up once per tag).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ServerImage {
    /// Full image id without the `sha256:` prefix.
    pub id: String,
    pub short_id: String,
    pub repository: String,
    pub tag: String,
    /// RFC 3339 creation time when the runtime reported one.
    pub created_at: Option<String>,
    pub size_bytes: u64,
    /// Names of containers (running or stopped) created from this image.
    pub used_by: Vec<String>,
    /// Referenced by a Rivetr deployment and kept for rollbacks.
    pub retained: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageInventory {
    pub images: Vec<ServerImage>,
    /// Sum of unique image sizes.
    pub total_bytes: u64,
    /// Sum of unique image sizes not used by any container.
    pub reclaimable_bytes: u64,
}

/// Script printing images as JSON lines, then `<image id>|<container name>`
/// for every container (running or stopped).
pub(crate) const LIST_SCRIPT: &str = r#"echo '---IMAGES---'
"$D" image ls --no-trunc --format '{{json .}}' || exit 1
echo '---CONTAINERS---'
ids=$("$D" ps -aq --no-trunc 2>/dev/null)
if [ -n "$ids" ]; then "$D" inspect --format '{{.Image}}|{{.Name}}' $ids 2>/dev/null; fi
exit 0"#;

/// Strip `sha256:` and lowercase an image id.
pub fn normalize_id(id: &str) -> String {
    id.trim().trim_start_matches("sha256:").to_ascii_lowercase()
}

fn str_field<'a>(v: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|k| v.get(*k).and_then(|x| x.as_str()))
        .filter(|s| !s.is_empty())
}

fn parse_created(v: &Value) -> Option<String> {
    if let Some(s) = str_field(v, &["CreatedAt", "Created", "created"]) {
        // Docker: "2024-05-01 10:00:00 +0000 UTC"
        let trimmed = s
            .rsplit_once(' ')
            .filter(|(_, zone)| zone.chars().all(|c| c.is_ascii_alphabetic()))
            .map(|(rest, _)| rest)
            .unwrap_or(s);
        if let Ok(dt) = chrono::DateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S %z") {
            return Some(dt.with_timezone(&chrono::Utc).to_rfc3339());
        }
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            return Some(dt.with_timezone(&chrono::Utc).to_rfc3339());
        }
        if let Ok(secs) = s.parse::<i64>() {
            return chrono::DateTime::from_timestamp(secs, 0).map(|d| d.to_rfc3339());
        }
        return None;
    }
    // Podman: unix seconds as a number
    v.get("Created")
        .and_then(|x| x.as_i64())
        .and_then(|secs| chrono::DateTime::from_timestamp(secs, 0))
        .map(|d| d.to_rfc3339())
}

fn parse_size(v: &Value) -> u64 {
    for key in ["Size", "VirtualSize", "size"] {
        match v.get(key) {
            Some(Value::Number(n)) => return n.as_u64().unwrap_or(0),
            Some(Value::String(s)) => {
                if let Some(b) = parse_human_size(s) {
                    return b;
                }
            }
            _ => {}
        }
    }
    0
}

fn image_from_json(v: &Value) -> Option<ServerImage> {
    let id = normalize_id(str_field(v, &["ID", "Id", "id"])?);
    let (mut repository, mut tag) = (
        str_field(v, &["Repository", "repository"])
            .unwrap_or("<none>")
            .to_string(),
        str_field(v, &["Tag", "tag"])
            .unwrap_or("<none>")
            .to_string(),
    );
    // Podman JSON arrays carry "Names": ["docker.io/library/nginx:latest"].
    if repository == "<none>" {
        if let Some(name) = v
            .get("Names")
            .and_then(|n| n.as_array())
            .and_then(|a| a.first())
            .and_then(|n| n.as_str())
        {
            match name.rsplit_once(':') {
                Some((r, t)) if !t.contains('/') => {
                    repository = r.to_string();
                    tag = t.to_string();
                }
                _ => repository = name.to_string(),
            }
        }
    }
    Some(ServerImage {
        short_id: id.chars().take(12).collect(),
        id,
        repository,
        tag,
        created_at: parse_created(v),
        size_bytes: parse_size(v),
        used_by: Vec::new(),
        retained: false,
    })
}

/// Parse the image section: one JSON object per line (docker, podman with
/// `{{json .}}`) or a single JSON array (podman `--format json`).
pub fn parse_image_lines(text: &str) -> Vec<ServerImage> {
    let trimmed = text.trim();
    if trimmed.starts_with('[') {
        return serde_json::from_str::<Vec<Value>>(trimmed)
            .map(|arr| arr.iter().filter_map(image_from_json).collect())
            .unwrap_or_default();
    }
    trimmed
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok())
        .filter_map(|v| image_from_json(&v))
        .collect()
}

/// Parse `<image id>|/<container name>` lines into image id -> container names.
pub fn parse_container_lines(text: &str) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for line in text.lines() {
        let Some((image, name)) = line.trim().split_once('|') else {
            continue;
        };
        let image = normalize_id(image);
        let name = name.trim().trim_start_matches('/').to_string();
        if image.is_empty() || name.is_empty() {
            continue;
        }
        map.entry(image).or_default().push(name);
    }
    for names in map.values_mut() {
        names.sort();
        names.dedup();
    }
    map
}

/// Combine the output of [`LIST_SCRIPT`] into an inventory.
pub fn build_inventory(output: &str, retained_refs: &HashSet<String>) -> ImageInventory {
    let (images_part, containers_part) = match output.split_once("---CONTAINERS---") {
        Some((a, b)) => (a, b),
        None => (output, ""),
    };
    let images_part = images_part
        .split_once("---IMAGES---")
        .map(|(_, b)| b)
        .unwrap_or(images_part);

    let mut images = parse_image_lines(images_part);
    let users = parse_container_lines(containers_part);

    for img in &mut images {
        if let Some(names) = users.get(&img.id) {
            img.used_by = names.clone();
        }
        let reference = format!("{}:{}", img.repository, img.tag);
        img.retained = retained_refs.contains(&reference)
            || retained_refs.contains(&img.id)
            || (img.tag == "latest" && retained_refs.contains(&img.repository));
    }
    images.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| a.repository.cmp(&b.repository))
    });

    let mut seen = HashSet::new();
    let (mut total, mut reclaimable) = (0u64, 0u64);
    for img in &images {
        if seen.insert(img.id.clone()) {
            total += img.size_bytes;
            if img.used_by.is_empty() {
                reclaimable += img.size_bytes;
            }
        }
    }
    ImageInventory {
        images,
        total_bytes: total,
        reclaimable_bytes: reclaimable,
    }
}

/// Image references Rivetr keeps for rollbacks (deployment image tags).
pub async fn retained_refs(db: &DbPool) -> HashSet<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT image_tag FROM deployments WHERE image_tag IS NOT NULL AND image_tag != ''",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default()
    .into_iter()
    .collect()
}

/// List images on the target.
pub async fn list_images(db: &DbPool, target: &HostTarget) -> Result<ImageInventory> {
    let out = target.exec(LIST_SCRIPT, Duration::from_secs(60)).await?;
    if !out.success() {
        anyhow::bail!("listing images failed: {}", out.combined());
    }
    let retained = retained_refs(db).await;
    Ok(build_inventory(&out.stdout, &retained))
}

/// True for a plausible image id (hex, 12 to 64 chars, optional `sha256:`).
pub fn is_valid_image_id(id: &str) -> bool {
    let id = id.trim_start_matches("sha256:");
    (12..=64).contains(&id.len()) && id.chars().all(|c| c.is_ascii_hexdigit())
}

#[derive(Debug)]
pub enum DeleteImageOutcome {
    Deleted,
    NotFound,
    InUse(Vec<String>),
    Failed(String),
}

/// Remove an image that no container uses. Refuses images in use.
pub async fn delete_unused_image(
    db: &DbPool,
    target: &HostTarget,
    image_id: &str,
) -> Result<DeleteImageOutcome> {
    let wanted = normalize_id(image_id);
    let inventory = list_images(db, target).await?;
    let matches: Vec<&ServerImage> = inventory
        .images
        .iter()
        .filter(|i| i.id.starts_with(&wanted))
        .collect();
    let Some(first) = matches.first() else {
        return Ok(DeleteImageOutcome::NotFound);
    };
    let full_id = first.id.clone();
    if matches.iter().any(|i| i.id != full_id) {
        return Ok(DeleteImageOutcome::Failed(
            "Image id prefix is ambiguous".to_string(),
        ));
    }
    let used: Vec<String> = matches
        .iter()
        .flat_map(|i| i.used_by.iter().cloned())
        .collect();
    if !used.is_empty() {
        return Ok(DeleteImageOutcome::InUse(used));
    }
    // -f untags every tag of the image; safe because no container uses it.
    let script = format!("\"$D\" rmi -f {}", shell_quote(&full_id));
    let out = target.exec(&script, Duration::from_secs(120)).await?;
    if out.success() {
        Ok(DeleteImageOutcome::Deleted)
    } else {
        Ok(DeleteImageOutcome::Failed(out.combined()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOCKER_OUTPUT: &str = r#"---IMAGES---
{"Containers":"N/A","CreatedAt":"2024-05-01 10:00:00 +0000 UTC","CreatedSince":"6 months ago","Digest":"<none>","ID":"sha256:aaaaaaaaaaaa1111111111111111111111111111111111111111111111111111","Repository":"nginx","SharedSize":"N/A","Size":"187MB","Tag":"latest","UniqueSize":"N/A","VirtualSize":"187.1MB"}
{"Containers":"N/A","CreatedAt":"2024-06-01 10:00:00 +0000 UTC","CreatedSince":"5 months ago","Digest":"<none>","ID":"sha256:bbbbbbbbbbbb2222222222222222222222222222222222222222222222222222","Repository":"rivetr-myapp","SharedSize":"N/A","Size":"1.2GB","Tag":"abc123","UniqueSize":"N/A","VirtualSize":"1.2GB"}
{"Containers":"N/A","CreatedAt":"2024-06-01 10:00:00 +0000 UTC","CreatedSince":"5 months ago","Digest":"<none>","ID":"sha256:bbbbbbbbbbbb2222222222222222222222222222222222222222222222222222","Repository":"rivetr-myapp","SharedSize":"N/A","Size":"1.2GB","Tag":"latest","UniqueSize":"N/A","VirtualSize":"1.2GB"}
{"Containers":"N/A","CreatedAt":"2023-01-01 00:00:00 +0000 UTC","CreatedSince":"2 years ago","Digest":"<none>","ID":"sha256:cccccccccccc3333333333333333333333333333333333333333333333333333","Repository":"<none>","SharedSize":"N/A","Size":"50MB","Tag":"<none>","UniqueSize":"N/A","VirtualSize":"50MB"}
---CONTAINERS---
sha256:aaaaaaaaaaaa1111111111111111111111111111111111111111111111111111|/rivetr-web
sha256:aaaaaaaaaaaa1111111111111111111111111111111111111111111111111111|/proxy
"#;

    #[test]
    fn parses_docker_image_ls_json() {
        let inv = build_inventory(DOCKER_OUTPUT, &HashSet::new());
        assert_eq!(inv.images.len(), 4);
        let nginx = inv.images.iter().find(|i| i.repository == "nginx").unwrap();
        assert_eq!(nginx.short_id, "aaaaaaaaaaaa");
        assert_eq!(nginx.tag, "latest");
        assert_eq!(nginx.size_bytes, 187_000_000);
        assert_eq!(nginx.used_by, vec!["proxy", "rivetr-web"]);
        assert_eq!(
            nginx.created_at.as_deref(),
            Some("2024-05-01T10:00:00+00:00")
        );
        let dangling = inv
            .images
            .iter()
            .find(|i| i.repository == "<none>")
            .unwrap();
        assert!(dangling.used_by.is_empty());
    }

    #[test]
    fn totals_count_each_image_id_once() {
        let inv = build_inventory(DOCKER_OUTPUT, &HashSet::new());
        assert_eq!(inv.total_bytes, 187_000_000 + 1_200_000_000 + 50_000_000);
        assert_eq!(inv.reclaimable_bytes, 1_200_000_000 + 50_000_000);
    }

    #[test]
    fn marks_retained_rollback_images() {
        let retained: HashSet<String> = ["rivetr-myapp:abc123".to_string()].into();
        let inv = build_inventory(DOCKER_OUTPUT, &retained);
        let tagged = inv
            .images
            .iter()
            .find(|i| i.repository == "rivetr-myapp" && i.tag == "abc123")
            .unwrap();
        assert!(tagged.retained);
        let nginx = inv.images.iter().find(|i| i.repository == "nginx").unwrap();
        assert!(!nginx.retained);
    }

    #[test]
    fn parses_podman_json_array() {
        let text = r#"[{"Id":"dddddddddddd4444","Names":["docker.io/library/redis:7"],"Created":1700000000,"Size":120000000}]"#;
        let images = parse_image_lines(text);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].repository, "docker.io/library/redis");
        assert_eq!(images[0].tag, "7");
        assert_eq!(images[0].size_bytes, 120_000_000);
        assert!(images[0].created_at.is_some());
    }

    #[test]
    fn ignores_garbage_lines() {
        let images = parse_image_lines("WARNING: something\n{not json}\n");
        assert!(images.is_empty());
        let users = parse_container_lines("garbage\n|/x\nabc|\n");
        assert!(users.is_empty());
    }

    #[test]
    fn image_id_validation() {
        assert!(is_valid_image_id("aaaaaaaaaaaa"));
        assert!(is_valid_image_id(
            "sha256:aaaaaaaaaaaa1111111111111111111111111111111111111111111111111111"
        ));
        assert!(!is_valid_image_id("abc"));
        assert!(!is_valid_image_id("aaaaaaaaaaaa; rm -rf /"));
        assert!(!is_valid_image_id("nginx:latest"));
    }
}

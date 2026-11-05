//! Docker cleanup runs: prune dangling images, unused images, build cache and
//! optionally unused volumes and networks, recording each run.

use std::time::{Duration, Instant};

use anyhow::Result;
use serde::Serialize;

use super::images::{self, ServerImage};
use super::{log_line, parse_human_size, shell_quote, HostTarget, MaintenanceSettings};
use crate::DbPool;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CleanupRun {
    pub id: String,
    pub server_id: String,
    pub status: String,
    pub trigger_type: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub freed_bytes: i64,
    pub log: String,
}

/// Options for one cleanup run.
#[derive(Debug, Clone, Default)]
pub struct CleanupOptions {
    pub remove_unused_volumes: bool,
    pub remove_unused_networks: bool,
    pub remove_retained_images: bool,
}

impl From<&MaintenanceSettings> for CleanupOptions {
    fn from(s: &MaintenanceSettings) -> Self {
        Self {
            remove_unused_volumes: s.remove_unused_volumes,
            remove_unused_networks: s.remove_unused_networks,
            remove_retained_images: s.remove_retained_images,
        }
    }
}

/// Sum every `Total reclaimed space: 1.2GB` line in prune output.
pub fn parse_reclaimed(output: &str) -> u64 {
    output
        .lines()
        .filter_map(|l| {
            l.trim()
                .strip_prefix("Total reclaimed space:")
                .and_then(|v| parse_human_size(v.trim()))
        })
        .sum()
}

/// Parse the use% column of `df -P` (last line), e.g. `/dev/sda1 ... 57% /`.
pub fn parse_df_percent(output: &str) -> Option<u8> {
    output
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())?
        .split_whitespace()
        .find_map(|c| c.strip_suffix('%').and_then(|n| n.parse::<u8>().ok()))
}

/// Disk usage percent of the filesystem holding container data.
pub async fn disk_usage_percent(target: &HostTarget) -> Result<u8> {
    let script = r#"R=$("$D" info --format '{{.DockerRootDir}}' 2>/dev/null)
[ -d "$R" ] || R=$("$D" info --format '{{.Store.GraphRoot}}' 2>/dev/null)
[ -d "$R" ] || R=/
df -P "$R" | tail -n 1"#;
    let out = target.exec(script, Duration::from_secs(30)).await?;
    parse_df_percent(&out.stdout)
        .ok_or_else(|| anyhow::anyhow!("could not read disk usage: {}", out.combined()))
}

/// Unused images to remove, skipping rollback images unless asked to remove them.
pub fn images_to_remove(images: &[ServerImage], remove_retained: bool) -> Vec<(String, u64)> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for img in images {
        // Dangling images are handled by `image prune`.
        if img.repository == "<none>" && img.tag == "<none>" {
            continue;
        }
        if !img.used_by.is_empty() {
            continue;
        }
        // Every tag of an image must be unretained for it to go.
        let retained = images.iter().any(|i| i.id == img.id && i.retained);
        if retained && !remove_retained {
            continue;
        }
        if seen.insert(img.id.clone()) {
            out.push((img.id.clone(), img.size_bytes));
        }
    }
    out
}

/// Script that removes unused networks except the built-in ones and Rivetr's.
const NETWORK_PRUNE_SCRIPT: &str = r#"for n in $("$D" network ls --format '{{.Name}}'); do
  case "$n" in bridge|host|none|ingress|docker_gwbridge|podman|rivetr|rivetr-*) continue ;; esac
  c=$("$D" network inspect -f '{{range .Containers}}x{{end}}' "$n" 2>/dev/null)
  if [ -z "$c" ]; then "$D" network rm "$n" >/dev/null 2>&1 && echo "Removed network $n"; fi
done
exit 0"#;

/// Is a cleanup already running for this server?
pub async fn is_running(db: &DbPool, server_id: &str) -> Result<bool> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM docker_cleanup_runs WHERE server_id = ? AND status = 'running'",
    )
    .bind(server_id)
    .fetch_one(db)
    .await?;
    Ok(n > 0)
}

pub async fn recent_runs(db: &DbPool, server_id: &str, limit: i64) -> Result<Vec<CleanupRun>> {
    Ok(sqlx::query_as::<_, CleanupRun>(
        "SELECT * FROM docker_cleanup_runs WHERE server_id = ? ORDER BY started_at DESC LIMIT ?",
    )
    .bind(server_id)
    .bind(limit)
    .fetch_all(db)
    .await?)
}

/// Record a run and execute it in the background. Returns the run id.
pub async fn start_run(
    db: DbPool,
    server_id: String,
    target: HostTarget,
    options: CleanupOptions,
    trigger: &str,
) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO docker_cleanup_runs (id, server_id, status, trigger_type, started_at) \
         VALUES (?, ?, 'running', ?, ?)",
    )
    .bind(&id)
    .bind(&server_id)
    .bind(trigger)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(&db)
    .await?;

    let run_id = id.clone();
    tokio::spawn(async move {
        let started = Instant::now();
        let (ok, freed, log) = execute(&db, &target, &options).await;
        let status = if ok { "success" } else { "failed" };
        if let Err(e) = sqlx::query(
            "UPDATE docker_cleanup_runs SET status = ?, finished_at = ?, duration_ms = ?, \
             freed_bytes = ?, log = ? WHERE id = ?",
        )
        .bind(status)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(started.elapsed().as_millis() as i64)
        .bind(freed as i64)
        .bind(&log)
        .bind(&run_id)
        .execute(&db)
        .await
        {
            tracing::error!(run_id = %run_id, "Failed to store cleanup result: {}", e);
        }
        tracing::info!(server_id = %server_id, freed, status, "Docker cleanup finished");
    });
    Ok(id)
}

async fn step(target: &HostTarget, log: &mut String, title: &str, script: &str) -> (bool, String) {
    log_line(log, format!("== {title}"));
    match target
        .exec(&format!("{script} 2>&1"), Duration::from_secs(900))
        .await
    {
        Ok(out) => {
            let text = out.combined();
            if !text.is_empty() {
                log_line(log, &text);
            }
            if !out.success() {
                log_line(log, format!("(exit code {:?})", out.code));
            }
            (out.success(), text)
        }
        Err(e) => {
            log_line(log, format!("Error: {e}"));
            (false, String::new())
        }
    }
}

async fn execute(db: &DbPool, target: &HostTarget, opts: &CleanupOptions) -> (bool, u64, String) {
    let mut log = String::new();
    let mut freed: u64 = 0;
    let mut ok = true;

    let (s, out) = step(
        target,
        &mut log,
        "Removing dangling images",
        r#""$D" image prune -f"#,
    )
    .await;
    ok &= s;
    freed += parse_reclaimed(&out);

    log_line(&mut log, "== Removing unused images");
    match images::list_images(db, target).await {
        Ok(inv) => {
            let doomed = images_to_remove(&inv.images, opts.remove_retained_images);
            if doomed.is_empty() {
                log_line(&mut log, "No unused images");
            }
            for (id, size) in doomed {
                let short: String = id.chars().take(12).collect();
                let script = format!("\"$D\" rmi -f {} 2>&1", shell_quote(&id));
                match target.exec(&script, Duration::from_secs(300)).await {
                    Ok(out) if out.success() => {
                        freed += size;
                        log_line(&mut log, format!("Removed {short}"));
                    }
                    Ok(out) => log_line(
                        &mut log,
                        format!("Skipped {short}: {}", out.combined().trim()),
                    ),
                    Err(e) => log_line(&mut log, format!("Skipped {short}: {e}")),
                }
            }
        }
        Err(e) => {
            ok = false;
            log_line(&mut log, format!("Error listing images: {e}"));
        }
    }

    let (_, out) = step(
        target,
        &mut log,
        "Removing build cache",
        r#""$D" builder prune -f || echo "Build cache prune not supported""#,
    )
    .await;
    freed += parse_reclaimed(&out);

    if opts.remove_unused_volumes {
        let (s, out) = step(
            target,
            &mut log,
            "Removing unused volumes",
            r#""$D" volume prune -af 2>/dev/null || "$D" volume prune -f"#,
        )
        .await;
        ok &= s;
        freed += parse_reclaimed(&out);
    }

    if opts.remove_unused_networks {
        let (s, _) = step(
            target,
            &mut log,
            "Removing unused networks",
            NETWORK_PRUNE_SCRIPT,
        )
        .await;
        ok &= s;
    }

    log_line(
        &mut log,
        format!("Done. Freed about {}", format_bytes(freed)),
    );
    (ok, freed, log)
}

fn format_bytes(b: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1000.0 && i < UNITS.len() - 1 {
        v /= 1000.0;
        i += 1;
    }
    if i == 0 {
        format!("{b} B")
    } else {
        format!("{v:.2} {}", UNITS[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(id: &str, repo: &str, tag: &str, used: bool, retained: bool) -> ServerImage {
        ServerImage {
            id: id.into(),
            short_id: id.chars().take(12).collect(),
            repository: repo.into(),
            tag: tag.into(),
            created_at: None,
            size_bytes: 100,
            used_by: if used { vec!["c".into()] } else { vec![] },
            retained,
        }
    }

    #[test]
    fn reclaimed_space_is_summed() {
        let out = "Deleted Images:\nuntagged: x\nTotal reclaimed space: 1.5GB\nTotal reclaimed space: 500MB\n";
        assert_eq!(parse_reclaimed(out), 2_000_000_000);
        assert_eq!(parse_reclaimed("nothing"), 0);
    }

    #[test]
    fn df_percent() {
        let out = "Filesystem     1024-blocks      Used Available Capacity Mounted on\n/dev/sda1        81000000  46000000  35000000      57% /\n";
        assert_eq!(parse_df_percent(out), Some(57));
        assert_eq!(parse_df_percent(""), None);
    }

    #[test]
    fn picks_unused_unretained_images() {
        let images = vec![
            img("a", "nginx", "latest", true, false),
            img("b", "node", "20", false, false),
            img("c", "rivetr-app", "v1", false, true),
            img("c", "rivetr-app", "latest", false, false),
            img("d", "<none>", "<none>", false, false),
        ];
        let keep = images_to_remove(&images, false);
        assert_eq!(keep, vec![("b".to_string(), 100)]);
        let all = images_to_remove(&images, true);
        assert_eq!(all, vec![("b".to_string(), 100), ("c".to_string(), 100)]);
    }

    #[test]
    fn bytes_format() {
        assert_eq!(format_bytes(999), "999 B");
        assert_eq!(format_bytes(1_500_000_000), "1.50 GB");
    }
}

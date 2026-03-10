//! Log drain manager that buffers and forwards logs to external services.

use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::LogDrain;
use crate::DbPool;

/// A single log entry to be sent
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub app_id: String,
    pub log_line: String,
    pub level: String,
    pub timestamp: String,
}

/// Per-drain buffer of log entries
struct DrainBuffer {
    entries: Vec<LogEntry>,
}

/// Manager for log drains that handles buffering and dispatch.
pub struct LogDrainManager {
    db: DbPool,
    http_client: reqwest::Client,
    /// Buffers keyed by drain ID
    buffers: Arc<Mutex<HashMap<String, DrainBuffer>>>,
}

impl LogDrainManager {
    /// Create a new log drain manager
    pub fn new(db: DbPool) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let manager = Self {
            db,
            http_client,
            buffers: Arc::new(Mutex::new(HashMap::new())),
        };

        // Start the flush timer
        manager.start_flush_timer();

        manager
    }

    /// Start a background task that flushes buffers every 5 seconds
    fn start_flush_timer(&self) {
        let db = self.db.clone();
        let http_client = self.http_client.clone();
        let buffers = self.buffers.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                Self::flush_all_buffers(&db, &http_client, &buffers).await;
            }
        });
    }

    /// Send a log entry to all enabled drains for the given app.
    /// Buffers the entry and flushes when the buffer reaches 100 entries.
    pub async fn send_log(&self, app_id: &str, log_line: &str, level: &str, timestamp: &str) {
        let entry = LogEntry {
            app_id: app_id.to_string(),
            log_line: log_line.to_string(),
            level: level.to_string(),
            timestamp: timestamp.to_string(),
        };

        // Get enabled drains for this app
        let drains = match self.get_enabled_drains(app_id).await {
            Ok(d) => d,
            Err(e) => {
                tracing::debug!(error = %e, app_id = %app_id, "Failed to fetch log drains");
                return;
            }
        };

        if drains.is_empty() {
            return;
        }

        let mut buffers = self.buffers.lock().await;

        for drain in &drains {
            let buffer = buffers
                .entry(drain.id.clone())
                .or_insert_with(|| DrainBuffer {
                    entries: Vec::new(),
                });

            buffer.entries.push(entry.clone());

            // Flush if buffer reaches 100 entries
            if buffer.entries.len() >= 100 {
                let entries = std::mem::take(&mut buffer.entries);
                let db = self.db.clone();
                let http_client = self.http_client.clone();
                let drain = drain.clone();
                tokio::spawn(async move {
                    Self::send_batch(&db, &http_client, &drain, &entries).await;
                });
            }
        }
    }

    /// Get all enabled log drains for an app
    async fn get_enabled_drains(&self, app_id: &str) -> Result<Vec<LogDrain>> {
        let drains = sqlx::query_as::<_, LogDrain>(
            "SELECT * FROM log_drains WHERE app_id = ? AND enabled = 1",
        )
        .bind(app_id)
        .fetch_all(&self.db)
        .await?;

        Ok(drains)
    }

    /// Flush all buffers for all drains
    async fn flush_all_buffers(
        db: &DbPool,
        http_client: &reqwest::Client,
        buffers: &Arc<Mutex<HashMap<String, DrainBuffer>>>,
    ) {
        let drain_entries: Vec<(String, Vec<LogEntry>)> = {
            let mut buffers = buffers.lock().await;
            buffers
                .iter_mut()
                .filter(|(_, buf)| !buf.entries.is_empty())
                .map(|(drain_id, buf)| {
                    let entries = std::mem::take(&mut buf.entries);
                    (drain_id.clone(), entries)
                })
                .collect()
        };

        for (drain_id, entries) in drain_entries {
            let drain = match sqlx::query_as::<_, LogDrain>(
                "SELECT * FROM log_drains WHERE id = ? AND enabled = 1",
            )
            .bind(&drain_id)
            .fetch_optional(db)
            .await
            {
                Ok(Some(d)) => d,
                Ok(None) => {
                    // Drain was deleted or disabled, clean up buffer
                    let mut buffers = buffers.lock().await;
                    buffers.remove(&drain_id);
                    continue;
                }
                Err(e) => {
                    tracing::warn!(error = %e, drain_id = %drain_id, "Failed to fetch drain for flush");
                    continue;
                }
            };

            let db = db.clone();
            let http_client = http_client.clone();
            tokio::spawn(async move {
                Self::send_batch(&db, &http_client, &drain, &entries).await;
            });
        }
    }

    /// Send a batch of log entries to a specific drain
    async fn send_batch(
        db: &DbPool,
        http_client: &reqwest::Client,
        drain: &LogDrain,
        entries: &[LogEntry],
    ) {
        if entries.is_empty() {
            return;
        }

        let result = match drain.provider.as_str() {
            "axiom" => Self::send_axiom(http_client, drain, entries).await,
            "newrelic" => Self::send_newrelic(http_client, drain, entries).await,
            "datadog" => Self::send_datadog(http_client, drain, entries).await,
            "logtail" => Self::send_logtail(http_client, drain, entries).await,
            "http" => Self::send_http(http_client, drain, entries).await,
            _ => {
                tracing::warn!(provider = %drain.provider, "Unknown log drain provider");
                return;
            }
        };

        match result {
            Ok(()) => {
                // Update last_sent_at and reset error count
                let now = chrono::Utc::now().to_rfc3339();
                let _ = sqlx::query(
                    "UPDATE log_drains SET last_sent_at = ?, error_count = 0, last_error = NULL, updated_at = ? WHERE id = ?",
                )
                .bind(&now)
                .bind(&now)
                .bind(&drain.id)
                .execute(db)
                .await;

                tracing::debug!(
                    drain_id = %drain.id,
                    provider = %drain.provider,
                    count = entries.len(),
                    "Log batch sent successfully"
                );
            }
            Err(e) => {
                let now = chrono::Utc::now().to_rfc3339();
                let error_msg = e.to_string();
                let _ = sqlx::query(
                    "UPDATE log_drains SET error_count = error_count + 1, last_error = ?, updated_at = ? WHERE id = ?",
                )
                .bind(&error_msg)
                .bind(&now)
                .bind(&drain.id)
                .execute(db)
                .await;

                tracing::warn!(
                    drain_id = %drain.id,
                    provider = %drain.provider,
                    error = %e,
                    "Failed to send log batch"
                );
            }
        }
    }

    /// Send logs to Axiom
    async fn send_axiom(
        http_client: &reqwest::Client,
        drain: &LogDrain,
        entries: &[LogEntry],
    ) -> Result<()> {
        let config = drain
            .get_axiom_config()
            .ok_or_else(|| anyhow::anyhow!("Invalid Axiom config"))?;

        let payload: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                json!({
                    "_time": e.timestamp,
                    "level": e.level,
                    "message": e.log_line,
                    "app_id": e.app_id,
                    "source": "rivetr"
                })
            })
            .collect();

        let url = format!("https://api.axiom.co/v1/datasets/{}/ingest", config.dataset);

        http_client
            .post(&url)
            .bearer_auth(&config.api_token)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Send logs to New Relic
    async fn send_newrelic(
        http_client: &reqwest::Client,
        drain: &LogDrain,
        entries: &[LogEntry],
    ) -> Result<()> {
        let config = drain
            .get_newrelic_config()
            .ok_or_else(|| anyhow::anyhow!("Invalid New Relic config"))?;

        let payload: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                json!({
                    "timestamp": e.timestamp,
                    "message": e.log_line,
                    "level": e.level,
                    "attributes": {
                        "app_id": e.app_id,
                        "source": "rivetr"
                    }
                })
            })
            .collect();

        let url = if config.region.to_lowercase() == "eu" {
            "https://log-api.eu.newrelic.com/log/v1"
        } else {
            "https://log-api.newrelic.com/log/v1"
        };

        http_client
            .post(url)
            .header("Api-Key", &config.api_key)
            .json(&json!([{ "logs": payload }]))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Send logs to Datadog
    async fn send_datadog(
        http_client: &reqwest::Client,
        drain: &LogDrain,
        entries: &[LogEntry],
    ) -> Result<()> {
        let config = drain
            .get_datadog_config()
            .ok_or_else(|| anyhow::anyhow!("Invalid Datadog config"))?;

        let payload: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                json!({
                    "ddsource": "rivetr",
                    "ddtags": format!("app_id:{}", e.app_id),
                    "message": e.log_line,
                    "status": e.level,
                    "timestamp": e.timestamp
                })
            })
            .collect();

        let url = format!("https://http-intake.logs.{}/api/v2/logs", config.site);

        http_client
            .post(&url)
            .header("DD-API-KEY", &config.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Send logs to Logtail (Better Stack)
    async fn send_logtail(
        http_client: &reqwest::Client,
        drain: &LogDrain,
        entries: &[LogEntry],
    ) -> Result<()> {
        let config = drain
            .get_logtail_config()
            .ok_or_else(|| anyhow::anyhow!("Invalid Logtail config"))?;

        let payload: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                json!({
                    "dt": e.timestamp,
                    "level": e.level,
                    "message": e.log_line,
                    "app_id": e.app_id,
                    "source": "rivetr"
                })
            })
            .collect();

        http_client
            .post("https://in.logtail.com")
            .bearer_auth(&config.source_token)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Send logs to a generic HTTP endpoint
    async fn send_http(
        http_client: &reqwest::Client,
        drain: &LogDrain,
        entries: &[LogEntry],
    ) -> Result<()> {
        let config = drain
            .get_http_config()
            .ok_or_else(|| anyhow::anyhow!("Invalid HTTP drain config"))?;

        let payload: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                json!({
                    "timestamp": e.timestamp,
                    "level": e.level,
                    "message": e.log_line,
                    "app_id": e.app_id,
                    "source": "rivetr"
                })
            })
            .collect();

        let mut request = http_client.post(&config.url).json(&payload);

        if let (Some(header_name), Some(header_value)) =
            (&config.auth_header_name, &config.auth_header_value)
        {
            request = request.header(header_name, header_value);
        }

        request.send().await?.error_for_status()?;

        Ok(())
    }

    /// Send a test log entry to a specific drain (for testing connectivity)
    pub async fn send_test(&self, drain: &LogDrain) -> Result<()> {
        let test_entry = LogEntry {
            app_id: "test".to_string(),
            log_line: "This is a test log entry from Rivetr.".to_string(),
            level: "info".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let entries = vec![test_entry];

        match drain.provider.as_str() {
            "axiom" => Self::send_axiom(&self.http_client, drain, &entries).await,
            "newrelic" => Self::send_newrelic(&self.http_client, drain, &entries).await,
            "datadog" => Self::send_datadog(&self.http_client, drain, &entries).await,
            "logtail" => Self::send_logtail(&self.http_client, drain, &entries).await,
            "http" => Self::send_http(&self.http_client, drain, &entries).await,
            _ => Err(anyhow::anyhow!("Unknown provider: {}", drain.provider)),
        }
    }
}

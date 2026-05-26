//! Live "start log" streaming for services and managed databases.
//!
//! Apps already get streaming deployment logs via the `deployment_logs` table
//! and the `/api/deployments/:id/logs/stream` WS. Services and managed
//! databases do not have a corresponding deployment record, so we emit
//! transient log events through an in-memory broadcast channel keyed by
//! resource (e.g. `service:<id>` or `database:<id>`) and expose a WebSocket
//! that subscribes to it. This keeps the change small (no migrations, no
//! schema changes) while still giving the dashboard the same live "image
//! pull → container start → ready" experience.
//!
//! The dashboard side panel reads from
//! `/api/services/:id/start-stream` and `/api/databases/:id/start-stream`.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

use super::ws::validate_ws_token_str;
use crate::AppState;

/// How many log events to buffer per stream. Once exceeded, slow subscribers
/// drop the oldest events (subscribers fetch the recent buffer on connect via
/// the `latest_buffer` snapshot, so a brief lag is fine).
const STREAM_CAPACITY: usize = 1024;

/// Maximum number of recent events kept in the snapshot buffer per stream.
const SNAPSHOT_CAPACITY: usize = 512;

/// Local query struct for WS auth — accepts the standard `?token=` query
/// string used elsewhere in the API.
#[derive(Deserialize)]
pub struct AuthQuery {
    pub token: Option<String>,
}

/// A single live log entry emitted during a service or database start.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartLogEntry {
    /// Sequence id, monotonic per stream. Used by the dashboard to dedupe and
    /// sort events (matches `deployment_logs.id` semantics).
    pub id: u64,
    /// Resource key e.g. `service:abc123` or `database:xyz`.
    pub resource: String,
    /// Log level: `info`, `warn`, `error`, `debug`.
    pub level: String,
    /// Coarse phase used by the dashboard to render a status badge:
    /// `pulling`, `starting`, `running`, `failed`, `info`.
    pub phase: String,
    /// Free-form log message.
    pub message: String,
    /// RFC3339 timestamp.
    pub timestamp: String,
}

impl StartLogEntry {
    fn new(id: u64, resource: &str, level: &str, phase: &str, message: impl Into<String>) -> Self {
        Self {
            id,
            resource: resource.to_string(),
            level: level.to_string(),
            phase: phase.to_string(),
            message: message.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Per-resource stream state: a broadcast sender, a snapshot of recent events
/// (so a late subscriber sees what already happened), and a sequence counter.
struct StreamState {
    tx: broadcast::Sender<StartLogEntry>,
    snapshot: parking_lot::Mutex<std::collections::VecDeque<StartLogEntry>>,
    next_id: parking_lot::Mutex<u64>,
}

impl StreamState {
    fn new() -> Self {
        let (tx, _rx) = broadcast::channel(STREAM_CAPACITY);
        Self {
            tx,
            snapshot: parking_lot::Mutex::new(std::collections::VecDeque::with_capacity(
                SNAPSHOT_CAPACITY,
            )),
            next_id: parking_lot::Mutex::new(0),
        }
    }
}

/// Registry of in-flight start log streams, keyed by `service:<id>` or
/// `database:<id>`.
pub struct StartLogRegistry {
    streams: DashMap<String, Arc<StreamState>>,
}

impl StartLogRegistry {
    pub fn new() -> Self {
        Self {
            streams: DashMap::new(),
        }
    }

    fn get_or_create(&self, key: &str) -> Arc<StreamState> {
        if let Some(s) = self.streams.get(key) {
            return s.clone();
        }
        let state = Arc::new(StreamState::new());
        self.streams.entry(key.to_string()).or_insert(state).clone()
    }

    /// Emit a single log entry to all subscribers of `resource_key`.
    pub fn emit(
        &self,
        resource_key: &str,
        level: &str,
        phase: &str,
        message: impl Into<String>,
    ) -> StartLogEntry {
        let state = self.get_or_create(resource_key);
        let id = {
            let mut guard = state.next_id.lock();
            *guard += 1;
            *guard
        };
        let entry = StartLogEntry::new(id, resource_key, level, phase, message);
        {
            let mut snap = state.snapshot.lock();
            if snap.len() >= SNAPSHOT_CAPACITY {
                snap.pop_front();
            }
            snap.push_back(entry.clone());
        }
        let _ = state.tx.send(entry.clone());
        entry
    }

    /// Convenience helpers used by start paths.
    pub fn info(&self, resource_key: &str, phase: &str, message: impl Into<String>) {
        self.emit(resource_key, "info", phase, message);
    }

    pub fn warn(&self, resource_key: &str, phase: &str, message: impl Into<String>) {
        self.emit(resource_key, "warn", phase, message);
    }

    pub fn error(&self, resource_key: &str, phase: &str, message: impl Into<String>) {
        self.emit(resource_key, "error", phase, message);
    }

    /// Mark a stream as ended — emits a sentinel `end` event.
    pub fn end(&self, resource_key: &str, phase: &str, message: impl Into<String>) {
        let state = self.get_or_create(resource_key);
        let id = {
            let mut guard = state.next_id.lock();
            *guard += 1;
            *guard
        };
        let entry = StartLogEntry::new(id, resource_key, "info", phase, message);
        {
            let mut snap = state.snapshot.lock();
            if snap.len() >= SNAPSHOT_CAPACITY {
                snap.pop_front();
            }
            snap.push_back(entry.clone());
        }
        let _ = state.tx.send(entry);
    }

    /// Drop a stream entirely once nobody cares (end of start cycle, or on
    /// service/db delete). Subsequent starts will create a fresh one.
    pub fn clear(&self, resource_key: &str) {
        self.streams.remove(resource_key);
    }

    /// Snapshot of recent events for `resource_key`. Returns empty Vec if
    /// no stream exists.
    pub fn snapshot(&self, resource_key: &str) -> Vec<StartLogEntry> {
        self.streams
            .get(resource_key)
            .map(|s| s.snapshot.lock().iter().cloned().collect())
            .unwrap_or_default()
    }

    fn subscribe(
        &self,
        resource_key: &str,
    ) -> (broadcast::Receiver<StartLogEntry>, Vec<StartLogEntry>) {
        let state = self.get_or_create(resource_key);
        let rx = state.tx.subscribe();
        let snap: Vec<StartLogEntry> = state.snapshot.lock().iter().cloned().collect();
        (rx, snap)
    }
}

impl Default for StartLogRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket endpoint: GET /api/services/:id/start-stream
pub async fn service_start_stream_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<AuthQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    if !validate_ws_token_str(&state, query.token.as_deref()).await {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let key = format!("service:{}", id);
    Ok(ws.on_upgrade(move |socket| handle_start_stream(socket, state, key)))
}

/// WebSocket endpoint: GET /api/databases/:id/start-stream
pub async fn database_start_stream_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<AuthQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    if !validate_ws_token_str(&state, query.token.as_deref()).await {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let key = format!("database:{}", id);
    Ok(ws.on_upgrade(move |socket| handle_start_stream(socket, state, key)))
}

async fn handle_start_stream(socket: WebSocket, state: Arc<AppState>, resource_key: String) {
    let (mut sender, mut receiver) = socket.split();

    let (mut rx, snapshot) = state.start_log_streams.subscribe(&resource_key);

    // Replay buffered history first so a late subscriber catches up.
    for entry in snapshot {
        if let Ok(text) = serde_json::to_string(&entry) {
            if sender.send(Message::Text(text)).await.is_err() {
                return;
            }
        }
    }

    loop {
        tokio::select! {
            recv = rx.recv() => {
                match recv {
                    Ok(entry) => {
                        if let Ok(text) = serde_json::to_string(&entry) {
                            if sender.send(Message::Text(text)).await.is_err() {
                                return;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        let snap = state.start_log_streams.snapshot(&resource_key);
                        let lag_msg = serde_json::json!({
                            "type": "lag",
                            "dropped": n,
                        });
                        let _ = sender.send(Message::Text(lag_msg.to_string())).await;
                        for entry in snap {
                            if let Ok(text) = serde_json::to_string(&entry) {
                                if sender.send(Message::Text(text)).await.is_err() {
                                    return;
                                }
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            return;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => return,
                    _ => {}
                }
            }
        }
    }
}

/// REST snapshot endpoint: GET /api/services/:id/start-events
/// Returns the buffered events so the panel can render even before the WS
/// connects (fallback / first paint).
pub async fn service_start_stream_snapshot(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> axum::Json<Vec<StartLogEntry>> {
    let key = format!("service:{}", id);
    axum::Json(state.start_log_streams.snapshot(&key))
}

/// REST snapshot endpoint: GET /api/databases/:id/start-events
pub async fn database_start_stream_snapshot(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> axum::Json<Vec<StartLogEntry>> {
    let key = format!("database:{}", id);
    axum::Json(state.start_log_streams.snapshot(&key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_increase_id_and_buffer() {
        let reg = StartLogRegistry::new();
        let key = "service:test";
        let a = reg.emit(key, "info", "pulling", "msg1");
        let b = reg.emit(key, "info", "starting", "msg2");
        assert_eq!(a.id, 1);
        assert_eq!(b.id, 2);
        let snap = reg.snapshot(key);
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].message, "msg1");
        assert_eq!(snap[1].phase, "starting");
    }

    #[test]
    fn clear_removes_state() {
        let reg = StartLogRegistry::new();
        let key = "database:gone";
        reg.emit(key, "info", "pulling", "x");
        reg.clear(key);
        assert!(reg.snapshot(key).is_empty());
    }

    #[tokio::test]
    async fn subscribe_replays_and_streams() {
        let reg = StartLogRegistry::new();
        let key = "service:live";
        reg.emit(key, "info", "pulling", "first");
        let (mut rx, snap) = reg.subscribe(key);
        assert_eq!(snap.len(), 1);
        reg.emit(key, "info", "starting", "second");
        let next = rx.recv().await.expect("recv");
        assert_eq!(next.message, "second");
    }
}

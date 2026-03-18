CREATE TABLE IF NOT EXISTS proxy_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts TEXT NOT NULL DEFAULT (datetime('now')),
    host TEXT NOT NULL,
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    status INTEGER NOT NULL,
    response_ms INTEGER NOT NULL,
    bytes_out INTEGER NOT NULL DEFAULT 0,
    client_ip TEXT,
    user_agent TEXT
);
CREATE INDEX IF NOT EXISTS idx_proxy_logs_ts ON proxy_logs(ts DESC);
CREATE INDEX IF NOT EXISTS idx_proxy_logs_host ON proxy_logs(host);

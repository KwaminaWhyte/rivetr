-- Cost snapshots for daily cost calculations
-- Stores daily cost calculations per app based on resource metrics

-- Cost snapshots table
CREATE TABLE IF NOT EXISTS cost_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    app_id TEXT NOT NULL,
    snapshot_date TEXT NOT NULL,
    avg_cpu_cores REAL NOT NULL DEFAULT 0,
    avg_memory_gb REAL NOT NULL DEFAULT 0,
    avg_disk_gb REAL NOT NULL DEFAULT 0,
    cpu_cost REAL NOT NULL DEFAULT 0,
    memory_cost REAL NOT NULL DEFAULT 0,
    disk_cost REAL NOT NULL DEFAULT 0,
    total_cost REAL NOT NULL DEFAULT 0,
    sample_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (app_id) REFERENCES apps(id) ON DELETE CASCADE,
    UNIQUE(app_id, snapshot_date)
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_cost_snapshots_app_id ON cost_snapshots(app_id);
CREATE INDEX IF NOT EXISTS idx_cost_snapshots_date ON cost_snapshots(snapshot_date);
CREATE INDEX IF NOT EXISTS idx_cost_snapshots_app_date ON cost_snapshots(app_id, snapshot_date);

-- Cost rate configurations for resource cost estimation
-- Allows platform operators to configure cost rates per resource type

-- Cost rates table for resource pricing
CREATE TABLE IF NOT EXISTS cost_rates (
    id TEXT PRIMARY KEY,
    resource_type TEXT NOT NULL UNIQUE CHECK(resource_type IN ('cpu', 'memory', 'disk')),
    rate_per_unit REAL NOT NULL CHECK(rate_per_unit >= 0),
    unit_description TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for efficient queries by resource type
CREATE INDEX IF NOT EXISTS idx_cost_rates_resource_type ON cost_rates(resource_type);

-- Seed default cost rates
-- CPU: $0.02 per core per month
-- Memory: $0.05 per GB per month
-- Disk: $0.10 per GB per month
INSERT OR IGNORE INTO cost_rates (id, resource_type, rate_per_unit, unit_description, is_default)
VALUES
    ('default-cpu', 'cpu', 0.02, 'USD per CPU core per month', 1),
    ('default-memory', 'memory', 0.05, 'USD per GB RAM per month', 1),
    ('default-disk', 'disk', 0.10, 'USD per GB disk per month', 1);

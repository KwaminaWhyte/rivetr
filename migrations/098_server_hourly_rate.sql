-- Add hourly_rate to servers for accurate server-based cost calculation.
-- Default is $0.036/hr which matches DigitalOcean Basic Regular s-2vcpu-4gb ($24/mo).
ALTER TABLE servers ADD COLUMN hourly_rate REAL NOT NULL DEFAULT 0.036;

-- Update cost rate defaults to DigitalOcean-aligned values.
-- Old defaults ($0.02/CPU, $0.05/GB) were wildly low and produced nearly-zero estimates.
-- New values match DO Basic Regular pricing:
--   2 vCPU × $10 + 4 GB × $1 = $24/mo (matches s-2vcpu-4gb at $24/mo)
UPDATE cost_rates SET
    rate_per_unit = 10.00,
    unit_description = 'USD per vCPU per month (DigitalOcean Basic Regular)'
WHERE id = 'default-cpu';

UPDATE cost_rates SET
    rate_per_unit = 1.00,
    unit_description = 'USD per GB RAM per month (DigitalOcean Basic Regular)'
WHERE id = 'default-memory';

-- Disk stays at $0.10/GB/month (DO Block Storage is $0.10/GB/month)

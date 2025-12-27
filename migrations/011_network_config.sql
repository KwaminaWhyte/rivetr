-- Network configuration for applications
-- Adds support for multiple port mappings, network aliases, and extra hosts

-- Add port_mappings column (JSON array of {host_port, container_port, protocol})
-- Example: [{"host_port": 8080, "container_port": 80, "protocol": "tcp"}]
ALTER TABLE apps ADD COLUMN port_mappings TEXT DEFAULT NULL;

-- Add network_aliases column (JSON array of alias strings)
-- Example: ["api", "backend", "myapp"]
ALTER TABLE apps ADD COLUMN network_aliases TEXT DEFAULT NULL;

-- Add extra_hosts column (JSON array of "hostname:ip" entries)
-- Example: ["host.docker.internal:host-gateway", "myhost:192.168.1.1"]
ALTER TABLE apps ADD COLUMN extra_hosts TEXT DEFAULT NULL;

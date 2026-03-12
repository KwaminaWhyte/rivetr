-- Add SSH password support to servers and build_servers tables
ALTER TABLE servers ADD COLUMN ssh_password TEXT;
ALTER TABLE build_servers ADD COLUMN ssh_password TEXT;

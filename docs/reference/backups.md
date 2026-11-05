# Backups and restore

What Rivetr backs up, where it stores it, how to restore it, and how you hear about it.
Three kinds of backup exist: managed databases, app volumes, and the Rivetr instance itself.
Each can be kept on the Rivetr host, copied to S3-compatible storage (AWS S3, MinIO,
Cloudflare R2, or any custom endpoint), and restored from either place.

## Summary

| | Manual | Scheduled | Local | S3 | Restore from local | Restore from S3 | Retention |
|---|---|---|---|---|---|---|---|
| Database | yes | hourly, daily, weekly | always | optional copy per schedule, or per backup | yes | yes | keep last N (local and S3) |
| Volume | yes | cron | yes | yes | yes | yes | keep last N |
| Instance | yes | cron | always | optional (schedule's S3 config, or the default config for manual backups) | upload the archive | yes | `retention_days` (local and S3) |

## S3 storage

Add a storage config under **Integrations > S3**. Credentials are encrypted with
`auth.encryption_key` when it is set. Object keys, relative to the config's path prefix:

| Kind | Key |
|---|---|
| Database | `backups/database/<database_id>/<name>_<YYYYMMDD_HHMMSS>.<sql\|archive\|rdb>` |
| Volume | `backups/volume/<volume_id>/<volume>-<YYYYMMDD-HHMMSS>.tar.gz` |
| Instance | `backups/instance/rivetr-backup-<YYYYMMDD-HHMMSS>.tar.gz` (older releases also wrote `instance-backups/<file>`) |

Uploads stream from disk. Each upload records its size and SHA-256 in `s3_backups`; a
restore streams the object to a temp file and refuses it when the size or checksum does
not match. **Scan S3** on a Backups page lists the bucket(s) for objects of that resource
that Rivetr has no record of (for example after a reinstall) and adds them, so they can be
restored too (without a stored checksum, only the size reported by S3 is checked).

## Databases

Database > Backups.

- **Create Backup** dumps the database now (PostgreSQL `pg_dump`, MySQL `mysqldump`,
  MariaDB `mariadb-dump`, MongoDB `mongodump --archive`, Redis/KeyDB RDB). Files live in
  `<data_dir>/<database_backup.backup_dir>/<database_id>/`.
- **Schedule**: hourly, daily or weekly at an hour (UTC), keep the last N, and a destination:
  local only, or local plus an S3 config. With S3, each scheduled dump is uploaded and the
  same retention is applied to that config's copies in the bucket.
- **Copy to S3** on a local row uploads an existing backup.
- **Restore** on any row (local or S3) starts a background job: the database must be
  running; a safety backup of the current data is taken first (it shows up as a manual
  backup); then the dump is loaded with the same per-engine code as rollback with data
  (PostgreSQL in one transaction after dropping the public schema; MySQL/MariaDB drop
  tables and views then replay; MongoDB validated with `--dryRun`, then dropped and
  restored; Redis/KeyDB stop, swap `dump.rdb`, start). Supported: PostgreSQL, MySQL,
  MariaDB, MongoDB, Redis, KeyDB. Restores are limited to 256 MiB dumps.
- The page shows the latest restore job and whether it succeeded.

## Volumes

App > Storage > Volume Backups.

- **Back up now** to local disk or an S3 config. Local archives live in
  `<data_dir>/backups/volumes/<volume_id>/`; S3 runs keep no local copy.
- **Snapshot mode**: `live` (default) tars the volume from a throwaway `alpine:3` helper
  container that mounts it read-only with no network, while the app keeps running (files
  being written during the copy can be caught mid-write). `stop` stops the app's containers
  for the duration of the copy and starts them again, which gives a consistent snapshot.
- **Schedule**: standard 5-field cron in UTC (`0 3 * * *`; weekday 0 or 7 is Sunday),
  keep the last N (1 to 365), destination, snapshot mode. Runs are background jobs with
  history, including failures.
- **Restore** (local or S3): the archive is checked first (readable gzip tar, no absolute
  or `..` paths). Then the app's containers are stopped, a safety backup of the current
  content is taken (kept as `pre_restore`, last 3 per volume), the volume is emptied and the
  archive extracted by a helper container, and the containers are started again whatever
  happened. The container monitor leaves those containers alone during the restore.
- Bind mounts (absolute host paths) and named Docker/Podman volumes both work. Compose
  service volumes are not covered yet.

## Instance

Settings > Backup.

- A manual backup creates `rivetr-backup-<timestamp>.tar.gz` (SQLite after a WAL
  checkpoint, `rivetr.toml`, ACME certificates) in `<data_dir>/backups/` and, when a default
  S3 config exists, copies it there.
- Backup schedules (`/api/backups/schedules`) run on a cron expression. Type `instance` can
  name an S3 config to copy to; `retention_days` deletes local instance backups and that
  config's S3 copies older than the given number of days, always keeping the newest one.
  Types `s3_database` and `s3_volume` back a database or volume (`target_id`) up to the
  schedule's S3 config and apply `retention_days` in the bucket.
- Restore from an uploaded archive (`POST /api/system/restore`) or from S3
  (`POST /api/s3/backups/:id/restore`). The S3 restore takes a local safety backup of the
  instance first. Restart Rivetr after an instance restore.

## Notifications

Subscribe a notification channel to `backup_succeeded`, `backup_failed` or `backup_missed`,
optionally limited to databases, volumes or the instance, or to one database or volume.
Every channel type renders them with the resource, trigger, size, duration, destination and
error. Manual and scheduled backups notify; safety and pre-deploy backups do not.

`backup_missed` fires once per missed run when an enabled schedule's run is more than 15
minutes (plus the scheduler interval) overdue and has not happened, for example when
`[database_backup] enabled = false`, the server was down at the time, or a scheduled
database backup was skipped because the database was not running. The check runs every 5
minutes.

## API

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/databases/:id/backups/catalog?scan=` | Local and S3 backups together (`scan=true` lists the buckets). |
| POST | `/api/databases/:id/backups/:backup_id/restore` | Body `{ "source": "local" \| "s3" }`. Returns the restore job (202). |
| POST | `/api/databases/:id/backups/:backup_id/upload-s3` | Body `{ "storage_config_id" }`. |
| GET | `/api/databases/:id/backups/restores` | Recent restore jobs. |
| GET | `/api/volumes/:id/backups?scan=` | Volume backup history, local and S3. |
| POST | `/api/volumes/:id/backups` | Body `{ "destination", "s3_config_id", "snapshot_mode" }`. Returns the pending run (202). |
| DELETE | `/api/volumes/:id/backups/:backup_id?source=` | Delete a local archive or an S3 object. |
| POST | `/api/volumes/:id/backups/:backup_id/restore` | Body `{ "source" }`. Returns the restore job (202). |
| GET | `/api/volumes/:id/backups/restores` | Recent restore jobs. |
| GET, PUT, DELETE | `/api/volumes/:id/backup-schedule` | Per-volume schedule. |
| POST | `/api/s3/backup` | `backup_type` instance, database or volume (`source_id`); runs in the background. |
| POST | `/api/s3/backups/:id/restore` | Database and volume: background job. Instance: restores in the request. |

## Storage tables

Migration 133: `volume_backup_schedules`, `volume_backups`, `backup_restores`,
`backup_missed_alerts`, `s3_backups.checksum_sha256`,
`database_backup_schedules.s3_config_id`, and `resource_type`/`resource_id` on
`notification_subscriptions`.

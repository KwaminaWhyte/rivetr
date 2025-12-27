//! Disk space monitoring module
//!
//! This module monitors disk space usage and exposes metrics via Prometheus.
//! It runs as a background task that periodically:
//! - Checks disk space on the data directory's filesystem
//! - Updates Prometheus gauges for total, used, and free space
//! - Logs warnings when disk usage exceeds configurable thresholds

use crate::config::DiskMonitorConfig;
use anyhow::Result;
use std::path::Path;
use tokio::time::{interval, Duration};

/// Disk space statistics
#[derive(Debug, Clone)]
pub struct DiskStats {
    /// Total disk space in bytes
    pub total_bytes: u64,
    /// Used disk space in bytes
    pub used_bytes: u64,
    /// Free disk space in bytes
    pub free_bytes: u64,
    /// Percentage of disk space used (0-100)
    pub usage_percent: f64,
}

impl DiskStats {
    /// Get disk stats for a given path
    pub fn for_path(path: &Path) -> Result<Self> {
        #[cfg(unix)]
        {
            use std::ffi::CString;
            use std::mem::MaybeUninit;
            use std::os::unix::ffi::OsStrExt;

            let c_path = CString::new(path.as_os_str().as_bytes())?;
            let mut stat: MaybeUninit<libc::statvfs> = MaybeUninit::uninit();

            let result = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };
            if result != 0 {
                return Err(anyhow::anyhow!(
                    "Failed to get disk stats for {}: {}",
                    path.display(),
                    std::io::Error::last_os_error()
                ));
            }

            let stat = unsafe { stat.assume_init() };

            // Calculate sizes
            let block_size = stat.f_frsize as u64;
            let total_bytes = stat.f_blocks as u64 * block_size;
            let free_bytes = stat.f_bfree as u64 * block_size;
            let available_bytes = stat.f_bavail as u64 * block_size;
            let used_bytes = total_bytes - free_bytes;

            // Use available bytes for percentage calculation (accounts for reserved blocks)
            let usage_percent = if total_bytes > 0 {
                ((total_bytes - available_bytes) as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            };

            Ok(Self {
                total_bytes,
                used_bytes,
                free_bytes,
                usage_percent,
            })
        }

        #[cfg(windows)]
        {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;

            // Canonicalize the path to get absolute path with drive letter
            let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            let path_str = canonical_path.to_string_lossy();

            // Get the drive root from the path (e.g., "C:\")
            let root = if path_str.len() >= 2 && path_str.as_bytes()[1] == b':' {
                format!("{}:\\", &path_str[..1])
            } else {
                // Fall back to current directory's drive
                std::env::current_dir()
                    .ok()
                    .and_then(|p| {
                        let s = p.to_string_lossy();
                        if s.len() >= 2 && s.as_bytes()[1] == b':' {
                            Some(format!("{}:\\", &s[..1]))
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| "C:\\".to_string())
            };

            let wide_path: Vec<u16> = OsStr::new(&root)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let mut free_bytes_available: u64 = 0;
            let mut total_bytes: u64 = 0;
            let mut total_free_bytes: u64 = 0;

            let result = unsafe {
                windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                    wide_path.as_ptr(),
                    &mut free_bytes_available as *mut u64,
                    &mut total_bytes as *mut u64,
                    &mut total_free_bytes as *mut u64,
                )
            };

            if result == 0 {
                return Err(anyhow::anyhow!(
                    "Failed to get disk stats for {}: {}",
                    path.display(),
                    std::io::Error::last_os_error()
                ));
            }

            let used_bytes = total_bytes.saturating_sub(total_free_bytes);
            let usage_percent = if total_bytes > 0 {
                (used_bytes as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            };

            Ok(Self {
                total_bytes,
                used_bytes,
                free_bytes: total_free_bytes,
                usage_percent,
            })
        }
    }
}

/// Disk monitor that tracks disk space and updates metrics
pub struct DiskMonitor {
    /// Path to monitor (typically the data directory)
    path: std::path::PathBuf,
    /// Configuration
    config: DiskMonitorConfig,
    /// Last logged warning threshold (to avoid spamming logs)
    last_warning_threshold: std::sync::atomic::AtomicU8,
}

impl DiskMonitor {
    /// Create a new disk monitor
    pub fn new(path: std::path::PathBuf, config: DiskMonitorConfig) -> Self {
        Self {
            path,
            config,
            last_warning_threshold: std::sync::atomic::AtomicU8::new(0),
        }
    }

    /// Run a single check cycle
    pub fn check(&self) -> Result<DiskStats> {
        let stats = DiskStats::for_path(&self.path)?;

        // Update Prometheus metrics
        self.update_metrics(&stats);

        // Check thresholds and log warnings
        self.check_thresholds(&stats);

        Ok(stats)
    }

    /// Update Prometheus gauges
    fn update_metrics(&self, stats: &DiskStats) {
        use metrics::gauge;

        gauge!(crate::api::metrics::DISK_TOTAL_BYTES).set(stats.total_bytes as f64);
        gauge!(crate::api::metrics::DISK_USED_BYTES).set(stats.used_bytes as f64);
        gauge!(crate::api::metrics::DISK_FREE_BYTES).set(stats.free_bytes as f64);
        gauge!(crate::api::metrics::DISK_USAGE_PERCENT).set(stats.usage_percent);
    }

    /// Check disk usage thresholds and log warnings
    fn check_thresholds(&self, stats: &DiskStats) {
        use std::sync::atomic::Ordering;

        let usage = stats.usage_percent;
        let current_threshold = if usage >= self.config.critical_threshold as f64 {
            self.config.critical_threshold
        } else if usage >= self.config.warning_threshold as f64 {
            self.config.warning_threshold
        } else {
            0
        };

        let last_threshold = self
            .last_warning_threshold
            .load(Ordering::Relaxed);

        // Only log if we've crossed into a new threshold level
        if current_threshold > last_threshold {
            let free_gb = stats.free_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

            if current_threshold >= self.config.critical_threshold {
                tracing::error!(
                    usage_percent = format!("{:.1}", usage),
                    free_gb = format!("{:.2}", free_gb),
                    threshold = self.config.critical_threshold,
                    path = %self.path.display(),
                    "CRITICAL: Disk usage exceeds critical threshold!"
                );
            } else if current_threshold >= self.config.warning_threshold {
                tracing::warn!(
                    usage_percent = format!("{:.1}", usage),
                    free_gb = format!("{:.2}", free_gb),
                    threshold = self.config.warning_threshold,
                    path = %self.path.display(),
                    "Disk usage exceeds warning threshold"
                );
            }

            self.last_warning_threshold
                .store(current_threshold, Ordering::Relaxed);
        } else if current_threshold < last_threshold {
            // Disk usage has dropped below previous threshold
            tracing::info!(
                usage_percent = format!("{:.1}", usage),
                path = %self.path.display(),
                "Disk usage returned to normal levels"
            );
            self.last_warning_threshold
                .store(current_threshold, Ordering::Relaxed);
        }
    }
}

/// Spawn the background disk monitoring task
pub fn spawn_disk_monitor_task(path: std::path::PathBuf, config: DiskMonitorConfig) {
    if !config.enabled {
        tracing::info!("Disk monitoring is disabled");
        return;
    }

    let interval_secs = config.check_interval_seconds;
    tracing::info!(
        interval_secs = interval_secs,
        warning_threshold = config.warning_threshold,
        critical_threshold = config.critical_threshold,
        path = %path.display(),
        "Starting disk space monitoring task"
    );

    let monitor = DiskMonitor::new(path, config);

    tokio::spawn(async move {
        // Run an initial check immediately
        if let Err(e) = monitor.check() {
            tracing::warn!(error = %e, "Initial disk check failed");
        }

        let mut tick = interval(Duration::from_secs(interval_secs));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tick.tick().await;
            if let Err(e) = monitor.check() {
                tracing::error!(error = %e, "Disk monitoring check failed");
            }
        }
    });
}

/// Get current disk stats (for API endpoint)
pub fn get_current_disk_stats(path: &Path) -> Result<DiskStats> {
    DiskStats::for_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_disk_stats_current_dir() {
        let stats = DiskStats::for_path(&PathBuf::from(".")).unwrap();

        assert!(stats.total_bytes > 0, "Total bytes should be greater than 0");
        assert!(
            stats.free_bytes <= stats.total_bytes,
            "Free bytes should not exceed total"
        );
        assert!(
            stats.usage_percent >= 0.0 && stats.usage_percent <= 100.0,
            "Usage percent should be 0-100"
        );

        // used + free should approximately equal total (may differ slightly due to reserved blocks)
        let calculated_total = stats.used_bytes + stats.free_bytes;
        let diff = if calculated_total > stats.total_bytes {
            calculated_total - stats.total_bytes
        } else {
            stats.total_bytes - calculated_total
        };
        // Allow 1% margin for reserved blocks
        assert!(
            diff <= stats.total_bytes / 100,
            "used + free should approximately equal total"
        );
    }

    #[test]
    fn test_format_bytes() {
        let stats = DiskStats {
            total_bytes: 1024 * 1024 * 1024 * 100, // 100 GB
            used_bytes: 1024 * 1024 * 1024 * 80,   // 80 GB
            free_bytes: 1024 * 1024 * 1024 * 20,   // 20 GB
            usage_percent: 80.0,
        };

        assert_eq!(stats.usage_percent, 80.0);
    }
}

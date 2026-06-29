use std::fs;
use std::path::{Path, PathBuf};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

const MAX_LOG_AGE_DAYS: u64 = 7;
const MAX_LOG_SIZE_BYTES: u64 = 100 * 1024 * 1024; // 100MB

/// Initialize logging with both console and file output.
/// Returns a guard that must be held for the lifetime of the application.
pub fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    let log_dir = log_directory();
    fs::create_dir_all(&log_dir).ok();

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("lightframe")
        .filename_suffix("log")
        .max_log_files(7) // tracing-appender built-in rotation
        .build(&log_dir)
        .expect("failed to create log appender");

    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with(fmt::layer().with_ansi(true)) // console with colors
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_writer(non_blocking)
                .with_target(true)
                .with_thread_ids(true),
        )
        .init();

    // Background cleanup on startup
    let cleanup_dir = log_dir.clone();
    std::thread::spawn(move || {
        cleanup_logs(&cleanup_dir);
    });

    guard
}

/// Get the log directory path.
/// On Windows: %APPDATA%/com.lightframe.app/logs
/// On macOS: ~/Library/Logs/com.lightframe.app
/// On Linux: ~/.local/share/com.lightframe.app/logs
pub fn log_directory() -> PathBuf {
    let base = dirs::data_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("com.lightframe.app").join("logs")
}

/// Clean up old log files:
/// 1. Delete files older than MAX_LOG_AGE_DAYS
/// 2. If total size > MAX_LOG_SIZE_BYTES, delete oldest files until under limit
pub fn cleanup_logs(log_dir: &Path) {
    cleanup_logs_with_limits(log_dir, MAX_LOG_AGE_DAYS, MAX_LOG_SIZE_BYTES);
}

fn cleanup_logs_with_limits(log_dir: &Path, max_age_days: u64, max_size_bytes: u64) {
    let Ok(entries) = fs::read_dir(log_dir) else {
        return;
    };

    let mut log_files: Vec<(PathBuf, std::time::SystemTime, u64)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("log"))
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            let modified = meta.modified().ok()?;
            Some((e.path(), modified, meta.len()))
        })
        .collect();

    // Sort by modification time (oldest first)
    log_files.sort_by_key(|(_, modified, _)| *modified);

    let now = std::time::SystemTime::now();
    let max_age = std::time::Duration::from_secs(max_age_days * 24 * 60 * 60);

    // Phase 1: Delete files older than max_age_days
    log_files.retain(|(path, modified, _)| {
        if let Ok(age) = now.duration_since(*modified)
            && age > max_age
        {
            tracing::debug!("cleaning up old log: {}", path.display());
            let _ = fs::remove_file(path);
            return false;
        }
        true
    });

    // Phase 2: If total size exceeds limit, delete oldest first
    let total_size: u64 = log_files.iter().map(|(_, _, size)| size).sum();
    if total_size > max_size_bytes {
        let mut current_size = total_size;
        for (path, _, size) in &log_files {
            if current_size <= max_size_bytes {
                break;
            }
            tracing::debug!("cleaning up oversized log: {}", path.display());
            if fs::remove_file(path).is_ok() {
                current_size -= size;
            }
        }
    }
}

/// Metadata about log files for future upload capability
#[derive(Debug, serde::Serialize)]
pub struct LogFileInfo {
    pub path: String,
    pub size_bytes: u64,
    pub modified: String,
}

/// List current log files with metadata (for future upload feature)
pub fn list_log_files() -> Vec<LogFileInfo> {
    list_log_files_in(&log_directory())
}

fn list_log_files_in(log_dir: &Path) -> Vec<LogFileInfo> {
    let Ok(entries) = fs::read_dir(log_dir) else {
        return Vec::new();
    };

    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("log"))
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            let modified = meta.modified().ok()?;
            let duration = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
            Some(LogFileInfo {
                path: e.path().to_string_lossy().to_string(),
                size_bytes: meta.len(),
                modified: format!("{}", duration.as_secs()),
            })
        })
        .collect()
}

/// Collect recent error logs for future upload (designed for extensibility)
#[allow(dead_code)]
pub fn collect_recent_errors(since_hours: u64) -> Vec<String> {
    collect_recent_errors_in(&log_directory(), since_hours)
}

#[cfg_attr(not(test), allow(dead_code))]
fn collect_recent_errors_in(log_dir: &Path, since_hours: u64) -> Vec<String> {
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(since_hours * 3600))
        .unwrap_or(std::time::UNIX_EPOCH);

    let Ok(entries) = fs::read_dir(log_dir) else {
        return Vec::new();
    };

    let mut errors = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("log") {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        if modified < cutoff {
            continue;
        }

        if let Ok(content) = fs::read_to_string(&path) {
            for line in content.lines() {
                if line.contains("ERROR") || line.contains("WARN") {
                    errors.push(line.to_string());
                }
            }
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{Duration, SystemTime};
    use tempfile::tempdir;

    fn set_file_modified(path: &Path, modified: SystemTime) {
        let file = fs::OpenOptions::new().write(true).open(path).unwrap();
        file.set_modified(modified).unwrap();
    }

    #[test]
    fn cleanup_removes_old_files() {
        let dir = tempdir().unwrap();
        let old_path = dir.path().join("old.log");
        fs::write(&old_path, "old log content").unwrap();

        let old_time = SystemTime::now() - Duration::from_secs(8 * 24 * 60 * 60);
        set_file_modified(&old_path, old_time);

        let recent_path = dir.path().join("recent.log");
        fs::write(&recent_path, "recent log content").unwrap();

        cleanup_logs_with_limits(dir.path(), 7, MAX_LOG_SIZE_BYTES);

        assert!(!old_path.exists());
        assert!(recent_path.exists());
    }

    #[test]
    fn cleanup_enforces_size_limit() {
        let dir = tempdir().unwrap();
        let max_size = 1024u64;

        for i in 0..3 {
            let path = dir.path().join(format!("file{i}.log"));
            let mut file = fs::File::create(&path).unwrap();
            file.write_all(&vec![b'x'; 512]).unwrap();
            let modified = SystemTime::now() + Duration::from_secs(i);
            set_file_modified(&path, modified);
        }

        cleanup_logs_with_limits(dir.path(), MAX_LOG_AGE_DAYS, max_size);

        let remaining: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("log"))
            .collect();

        let total_size: u64 = remaining
            .iter()
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum();

        assert!(total_size <= max_size);
        assert!(remaining.len() < 3);
    }

    #[test]
    fn list_log_files_returns_entries() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("test.log"), "content").unwrap();

        let files = list_log_files_in(dir.path());
        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("test.log"));
        assert_eq!(files[0].size_bytes, 7);
    }

    #[test]
    fn collect_recent_errors_finds_warn_and_error_lines() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("app.log"),
            "INFO ok\nWARN something bad\nERROR something worse\n",
        )
        .unwrap();

        let errors = collect_recent_errors_in(dir.path(), 24);
        assert_eq!(errors.len(), 2);
        assert!(errors[0].contains("WARN"));
        assert!(errors[1].contains("ERROR"));
    }
}

use tracing::info;

#[derive(Debug, Clone, Copy)]
pub struct MemorySnapshot {
    pub rss_kb: u64,
    pub vm_size_kb: u64,
}

pub fn current_memory() -> Option<MemorySnapshot> {
    #[cfg(target_os = "linux")]
    {
        read_proc_status()
    }
    #[cfg(not(target_os = "linux"))]
    {
        read_sysinfo()
    }
}

pub fn log_memory(label: &str) {
    if let Some(mem) = current_memory() {
        info!(
            label,
            rss_mb = mem.rss_kb as f64 / 1024.0,
            vm_size_mb = mem.vm_size_kb as f64 / 1024.0,
            "memory usage"
        );
    }
}

#[cfg(target_os = "linux")]
fn read_proc_status() -> Option<MemorySnapshot> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let mut rss_kb = 0u64;
    let mut vm_size_kb = 0u64;

    for line in status.lines() {
        if let Some(value) = line.strip_prefix("VmRSS:") {
            rss_kb = parse_kb(value)?;
        } else if let Some(value) = line.strip_prefix("VmSize:") {
            vm_size_kb = parse_kb(value)?;
        }
    }

    if rss_kb == 0 && vm_size_kb == 0 {
        None
    } else {
        Some(MemorySnapshot { rss_kb, vm_size_kb })
    }
}

#[cfg(not(target_os = "linux"))]
fn read_sysinfo() -> Option<MemorySnapshot> {
    use sysinfo::{Pid, ProcessesToUpdate, System};

    let mut system = System::new();
    let pid = Pid::from_u32(std::process::id());
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    let process = system.process(pid)?;
    Some(MemorySnapshot {
        rss_kb: process.memory() / 1024,
        vm_size_kb: process.virtual_memory() / 1024,
    })
}

#[cfg(any(target_os = "linux", test))]
fn parse_kb(raw: &str) -> Option<u64> {
    raw.split_whitespace().next().and_then(|s| s.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kb_extracts_first_integer() {
        assert_eq!(parse_kb(" 12345 kB"), Some(12345));
        assert_eq!(parse_kb("\t999999 kB"), Some(999999));
        assert_eq!(parse_kb("0 kB"), Some(0));
    }

    #[test]
    fn parse_kb_returns_none_for_invalid_input() {
        assert_eq!(parse_kb(""), None);
        assert_eq!(parse_kb("   "), None);
        assert_eq!(parse_kb("not-a-number kB"), None);
        assert_eq!(parse_kb("kB"), None);
    }

    #[test]
    fn parse_kb_handles_large_values() {
        assert_eq!(parse_kb("4294967295 kB"), Some(4_294_967_295));
    }

    #[test]
    fn memory_snapshot_fields_are_accessible() {
        let snap = MemorySnapshot {
            rss_kb: 1024,
            vm_size_kb: 2048,
        };
        assert_eq!(snap.rss_kb, 1024);
        assert_eq!(snap.vm_size_kb, 2048);
    }

    #[test]
    fn current_memory_returns_some_on_linux() {
        #[cfg(target_os = "linux")]
        {
            let mem = current_memory();
            assert!(mem.is_some(), "expected /proc/self/status memory on Linux");
            let snap = mem.unwrap();
            assert!(snap.rss_kb > 0 || snap.vm_size_kb > 0);
        }

        #[cfg(not(target_os = "linux"))]
        {
            let mem = current_memory();
            assert!(mem.is_some());
        }
    }

    #[test]
    fn log_memory_does_not_panic() {
        log_memory("unit_test");
        log_memory("");
        log_memory("scan_start");
    }
}

use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThumbBudget {
    pub micro_cap: usize,
    pub standard_cap: usize,
}

/// Compute thumbnail cache budget based on available system RAM (in MB).
pub fn compute_thumb_budget(available_mb: u64, cpu_cores: usize) -> ThumbBudget {
    let _cores = cpu_cores.max(1);
    if available_mb > 8192 {
        ThumbBudget {
            micro_cap: 2000,
            standard_cap: 500,
        }
    } else if available_mb >= 4096 {
        ThumbBudget {
            micro_cap: 1000,
            standard_cap: 250,
        }
    } else {
        ThumbBudget {
            micro_cap: 500,
            standard_cap: 100,
        }
    }
}

/// Returns true when memory usage data is unavailable (e.g. missing MemAvailable on Linux).
pub fn memory_data_unavailable(available_mb: u64, total_mb: u64) -> bool {
    available_mb == 0 && total_mb > 0
}

/// Determine if memory pressure exceeds the critical threshold (>80% used).
pub fn is_under_pressure(available_mb: u64, total_mb: u64) -> bool {
    if total_mb == 0 || memory_data_unavailable(available_mb, total_mb) {
        return false;
    }
    let used_ratio = 1.0 - (available_mb as f64 / total_mb as f64);
    used_ratio > 0.8
}

/// Tracks hysteresis: shrink at >80% used, grow back only when <70% used.
pub struct PressureTracker {
    under_pressure: AtomicBool,
}

impl PressureTracker {
    pub fn new() -> Self {
        Self {
            under_pressure: AtomicBool::new(false),
        }
    }

    pub fn check(&self, available_mb: u64, total_mb: u64) -> bool {
        if total_mb == 0 || memory_data_unavailable(available_mb, total_mb) {
            return false;
        }

        let used_ratio = 1.0 - (available_mb as f64 / total_mb as f64);
        let currently_under = self.under_pressure.load(Ordering::Relaxed);
        let next = if currently_under {
            used_ratio >= 0.7
        } else {
            used_ratio > 0.8
        };
        self.under_pressure.store(next, Ordering::Relaxed);
        next
    }
}

impl Default for PressureTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Minimum budget caps applied during memory pressure.
pub const PRESSURE_BUDGET: ThumbBudget = ThumbBudget {
    micro_cap: 500,
    standard_cap: 100,
};

/// Read total and available system RAM.
/// Returns (total_mb, available_mb).
pub fn get_system_memory() -> Option<(u64, u64)> {
    #[cfg(target_os = "linux")]
    {
        read_proc_meminfo()
    }
    #[cfg(not(target_os = "linux"))]
    {
        read_sysinfo_memory()
    }
}

#[cfg(target_os = "linux")]
fn read_proc_meminfo() -> Option<(u64, u64)> {
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut total_kb: u64 = 0;
    let mut available_kb: u64 = 0;
    let mut has_mem_available = false;

    for line in meminfo.lines() {
        if let Some(val) = line.strip_prefix("MemTotal:") {
            total_kb = parse_meminfo_kb(val)?;
        } else if let Some(val) = line.strip_prefix("MemAvailable:") {
            available_kb = parse_meminfo_kb(val)?;
            has_mem_available = true;
        }
    }

    if total_kb == 0 {
        return None;
    }

    if !has_mem_available {
        return None;
    }

    Some((total_kb / 1024, available_kb / 1024))
}

#[cfg(target_os = "linux")]
fn parse_meminfo_kb(raw: &str) -> Option<u64> {
    raw.split_whitespace().next().and_then(|s| s.parse().ok())
}

#[cfg(not(target_os = "linux"))]
fn read_sysinfo_memory() -> Option<(u64, u64)> {
    use sysinfo::System;

    let mut sys = System::new();
    sys.refresh_memory();

    let total = sys.total_memory(); // bytes
    let available = sys.available_memory(); // bytes

    if total == 0 {
        return None;
    }

    Some((total / (1024 * 1024), available / (1024 * 1024)))
}

/// Log current budget decision.
pub fn log_budget(budget: &ThumbBudget, available_mb: u64) {
    info!(
        available_mb,
        micro_cap = budget.micro_cap,
        standard_cap = budget.standard_cap,
        "thumb budget computed"
    );
}

/// Log memory pressure event.
pub fn log_pressure(available_mb: u64, total_mb: u64) {
    warn!(
        available_mb,
        total_mb,
        used_pct = format!(
            "{:.1}%",
            (1.0 - available_mb as f64 / total_mb as f64) * 100.0
        ),
        "memory pressure detected — shrinking cache"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_high_ram_uses_full_caps() {
        let budget = compute_thumb_budget(16384, 8);
        assert_eq!(budget.micro_cap, 2000);
        assert_eq!(budget.standard_cap, 500);
    }

    #[test]
    fn budget_above_8gb_threshold() {
        let budget = compute_thumb_budget(8193, 12);
        assert_eq!(budget.micro_cap, 2000);
        assert_eq!(budget.standard_cap, 500);
    }

    #[test]
    fn budget_exactly_8gb_uses_medium_caps() {
        let budget = compute_thumb_budget(8192, 8);
        assert_eq!(budget.micro_cap, 1000);
        assert_eq!(budget.standard_cap, 250);
    }

    #[test]
    fn budget_medium_ram_4gb() {
        let budget = compute_thumb_budget(4096, 8);
        assert_eq!(budget.micro_cap, 1000);
        assert_eq!(budget.standard_cap, 250);
    }

    #[test]
    fn budget_medium_ram_6gb() {
        let budget = compute_thumb_budget(6144, 4);
        assert_eq!(budget.micro_cap, 1000);
        assert_eq!(budget.standard_cap, 250);
    }

    #[test]
    fn budget_low_ram_below_4gb() {
        let budget = compute_thumb_budget(3072, 16);
        assert_eq!(budget.micro_cap, 500);
        assert_eq!(budget.standard_cap, 100);
    }

    #[test]
    fn budget_very_low_ram() {
        let budget = compute_thumb_budget(1024, 2);
        assert_eq!(budget.micro_cap, 500);
        assert_eq!(budget.standard_cap, 100);
    }

    #[test]
    fn budget_zero_ram_gives_minimum() {
        let budget = compute_thumb_budget(0, 4);
        assert_eq!(budget.micro_cap, 500);
        assert_eq!(budget.standard_cap, 100);
    }

    #[test]
    fn pressure_detected_when_over_80_percent() {
        assert!(is_under_pressure(1000, 16000)); // 93.75% used
        assert!(is_under_pressure(3000, 16000)); // 81.25% used
    }

    #[test]
    fn no_pressure_when_under_80_percent() {
        assert!(!is_under_pressure(4000, 16000)); // 75% used
        assert!(!is_under_pressure(8000, 16000)); // 50% used
        assert!(!is_under_pressure(16000, 16000)); // 0% used
    }

    #[test]
    fn pressure_boundary_exactly_80_percent() {
        // 80% used means available = 20% of total
        // 3200 / 16000 = 0.2, used = 0.8 — NOT over 0.8
        assert!(!is_under_pressure(3200, 16000));
        // 3199 / 16000 < 0.2, used > 0.8
        assert!(is_under_pressure(3199, 16000));
    }

    #[test]
    fn pressure_zero_total_is_no_pressure() {
        assert!(!is_under_pressure(0, 0));
        assert!(!is_under_pressure(100, 0));
    }

    #[test]
    fn memory_data_unavailable_when_available_zero_but_total_positive() {
        assert!(memory_data_unavailable(0, 16000));
        assert!(!memory_data_unavailable(0, 0));
        assert!(!memory_data_unavailable(1000, 16000));
    }

    #[test]
    fn unavailable_memory_is_not_pressure() {
        assert!(!is_under_pressure(0, 16000));
    }

    #[test]
    fn pressure_budget_is_minimum() {
        assert_eq!(PRESSURE_BUDGET.micro_cap, 500);
        assert_eq!(PRESSURE_BUDGET.standard_cap, 100);
    }

    #[test]
    fn pressure_tracker_hysteresis() {
        let tracker = PressureTracker::new();
        // 81.25% used → enter pressure
        assert!(tracker.check(3000, 16000));
        // 75% used → still under pressure (need <70%)
        assert!(tracker.check(4000, 16000));
        // 65% used → exit pressure
        assert!(!tracker.check(5600, 16000));
        // 75% used → not yet re-enter (need >80%)
        assert!(!tracker.check(4000, 16000));
        // 81.25% used → re-enter
        assert!(tracker.check(3000, 16000));
    }

    #[test]
    fn pressure_tracker_skips_unavailable_data() {
        let tracker = PressureTracker::new();
        assert!(!tracker.check(0, 16000));
        assert!(!tracker.check(4000, 16000));
    }

    #[test]
    fn get_system_memory_returns_something_on_linux() {
        #[cfg(target_os = "linux")]
        {
            let mem = get_system_memory();
            assert!(mem.is_some());
            let (total, available) = mem.unwrap();
            assert!(total > 0);
            assert!(available > 0);
            assert!(available <= total);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parse_meminfo_kb_extracts_value() {
        assert_eq!(parse_meminfo_kb("  16384000 kB"), Some(16384000));
        assert_eq!(parse_meminfo_kb("\t1234 kB"), Some(1234));
        assert_eq!(parse_meminfo_kb(""), None);
        assert_eq!(parse_meminfo_kb("abc"), None);
    }
}

//! Parser for /proc/meminfo, /proc/swaps, and cgroup limits.

use crate::cgroup::{detect_cgroup, CgroupInfo};
use serde::{Deserialize, Serialize};
use std::fs;

/// Parsed memory information (values in bytes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemInfo {
    pub total: u64,
    pub available: u64,
    pub used: u64,
    pub commit_as: u64,
    pub commit_limit: u64,
    pub cached: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub swap_desc: String,
    pub cgroup: Option<CgroupInfo>,
    pub valid: bool,
}

impl Default for MemInfo {
    fn default() -> Self {
        Self {
            total: 0,
            available: 0,
            used: 0,
            commit_as: 0,
            commit_limit: 0,
            cached: 0,
            swap_used: 0,
            swap_total: 0,
            swap_desc: "unknown".into(),
            cgroup: None,
            valid: false,
        }
    }
}

/// Parse /proc/meminfo content into a key→bytes map.
fn parse_meminfo_content(content: &str) -> std::collections::HashMap<String, u64> {
    let mut map = std::collections::HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((key, rest)) = line.split_once(':') {
            let key = key.trim().to_string();
            let rest = rest.trim();
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if let Some(val_str) = parts.first() {
                if let Ok(val) = val_str.parse::<u64>() {
                    // Values in /proc/meminfo are in kB
                    map.insert(key, val * 1024);
                }
            }
        }
    }
    map
}

/// Detect swap type from /proc/swaps content.
fn detect_swap_type(swaps_content: &str) -> (bool, bool) {
    let mut has_zram = false;
    let mut has_disk = false;
    for line in swaps_content.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if let Some(dev) = parts.first() {
            if dev.contains("zram") {
                has_zram = true;
            } else {
                has_disk = true;
            }
        }
    }
    (has_zram, has_disk)
}

/// Build swap description string.
fn swap_desc(has_zram: bool, has_disk: bool) -> String {
    match (has_zram, has_disk) {
        (true, true) => "zram + disk".into(),
        (true, false) => "zram".into(),
        (false, true) => "disk swap".into(),
        (false, false) => "none".into(),
    }
}

/// Collect memory info from custom paths (for testing).
pub fn collect_meminfo_from(
    meminfo_content: &str,
    swaps_content: Option<&str>,
    has_zram_block: bool,
) -> MemInfo {
    let info = parse_meminfo_content(meminfo_content);

    let total = *info.get("MemTotal").unwrap_or(&0);
    let available = *info
        .get("MemAvailable")
        .or_else(|| info.get("MemFree"))
        .unwrap_or(&0);
    let used = total.saturating_sub(available);
    let commit_as = *info.get("Committed_AS").unwrap_or(&0);
    let commit_limit = *info.get("CommitLimit").unwrap_or(&total);
    let cached = info.get("Cached").unwrap_or(&0)
        + info.get("Buffers").unwrap_or(&0)
        + info.get("SReclaimable").unwrap_or(&0);
    let swap_total = *info.get("SwapTotal").unwrap_or(&0);
    let swap_free = *info.get("SwapFree").unwrap_or(&0);
    let swap_used = swap_total.saturating_sub(swap_free);

    let (mut has_zram, has_disk) = if let Some(sc) = swaps_content {
        detect_swap_type(sc)
    } else {
        (false, false)
    };

    if !has_zram && has_zram_block {
        has_zram = true;
    }

    MemInfo {
        total,
        available,
        used,
        commit_as,
        commit_limit,
        cached,
        swap_used,
        swap_total,
        swap_desc: swap_desc(has_zram, has_disk),
        cgroup: None,
        valid: total > 0,
    }
}

/// Collect memory info from the live system.
pub fn collect_meminfo() -> MemInfo {
    let meminfo_content = match fs::read_to_string("/proc/meminfo") {
        Ok(c) => c,
        Err(_) => return MemInfo::default(),
    };

    let swaps_content = fs::read_to_string("/proc/swaps").ok();

    let has_zram_block = if let Ok(entries) = fs::read_dir("/sys/block") {
        entries.filter_map(|e| e.ok()).any(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("zram"))
        })
    } else {
        false
    };

    let mut info = collect_meminfo_from(&meminfo_content, swaps_content.as_deref(), has_zram_block);
    if info.valid {
        info.cgroup = detect_cgroup(info.total);
    }
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_meminfo() {
        let content = "MemTotal:       8192 kB\nMemAvailable:   4096 kB\nCached:         1024 kB\n";
        let info = collect_meminfo_from(content, None, false);
        assert!(info.valid);
        assert_eq!(info.total, 8192 * 1024);
        assert_eq!(info.available, 4096 * 1024);
        assert_eq!(info.used, 4096 * 1024);
        assert_eq!(info.cached, 1024 * 1024);
    }

    #[test]
    fn test_meminfo_missing_fields() {
        let content = "MemTotal:\nMemFree: 1000 kB\nInvalidLine\n";
        let info = collect_meminfo_from(content, None, false);
        assert_eq!(info.total, 0);
        assert_eq!(info.available, 1000 * 1024);
    }

    #[test]
    fn test_empty_meminfo() {
        let info = collect_meminfo_from("", None, false);
        assert!(!info.valid);
        assert_eq!(info.total, 0);
    }

    #[test]
    fn test_truncated_meminfo() {
        let content = "MemTotal: abc kB\nMemFree: 500 kB\n";
        let info = collect_meminfo_from(content, None, false);
        assert_eq!(info.total, 0);
        assert_eq!(info.available, 500 * 1024);
    }

    #[test]
    fn test_malformed_lines() {
        let content = "garbage without colon\n:no_key\nMemTotal: 16384 kB\n";
        let info = collect_meminfo_from(content, None, false);
        assert_eq!(info.total, 16384 * 1024);
    }

    #[test]
    fn test_swap_detection_zram() {
        let swaps = "Filename\tType\tSize\tUsed\tPriority\n/dev/zram0\tpartition\t4096\t100\t100\n";
        let info = collect_meminfo_from(
            "MemTotal: 8192 kB\nSwapTotal: 4096 kB\nSwapFree: 3996 kB\n",
            Some(swaps),
            false,
        );
        assert_eq!(info.swap_desc, "zram");
        assert_eq!(info.swap_used, 100 * 1024);
    }

    #[test]
    fn test_swap_detection_disk() {
        let swaps = "Filename\tType\tSize\tUsed\tPriority\n/dev/sda2\tpartition\t8192\t200\t-2\n";
        let info = collect_meminfo_from(
            "MemTotal: 8192 kB\nSwapTotal: 8192 kB\nSwapFree: 7992 kB\n",
            Some(swaps),
            false,
        );
        assert_eq!(info.swap_desc, "disk swap");
    }

    #[test]
    fn test_swap_detection_zram_plus_disk() {
        let swaps = "Filename\tType\tSize\tUsed\tPriority\n/dev/zram0\tpartition\t4096\t100\t100\n/dev/sda2\tpartition\t8192\t200\t-2\n";
        let info = collect_meminfo_from(
            "MemTotal: 8192 kB\nSwapTotal: 12288 kB\nSwapFree: 11988 kB\n",
            Some(swaps),
            false,
        );
        assert_eq!(info.swap_desc, "zram + disk");
    }

    #[test]
    fn test_swap_detection_none() {
        let info = collect_meminfo_from(
            "MemTotal: 8192 kB\n",
            Some("Filename\tType\tSize\tUsed\tPriority\n"),
            false,
        );
        assert_eq!(info.swap_desc, "none");
    }

    #[test]
    fn test_zram_fallback_via_sysblock() {
        let info = collect_meminfo_from(
            "MemTotal: 8192 kB\n",
            Some("Filename\tType\tSize\tUsed\tPriority\n"),
            true, // has_zram_block = true
        );
        assert_eq!(info.swap_desc, "zram");
    }

    #[test]
    fn test_full_meminfo_with_all_fields() {
        let content = "\
MemTotal:       16384 kB
MemAvailable:   8192 kB
MemFree:        4096 kB
Buffers:         512 kB
Cached:         2048 kB
SReclaimable:    256 kB
Committed_AS:   10000 kB
CommitLimit:    20000 kB
SwapTotal:      4096 kB
SwapFree:       3072 kB
";
        let info = collect_meminfo_from(content, None, false);
        assert!(info.valid);
        assert_eq!(info.total, 16384 * 1024);
        assert_eq!(info.available, 8192 * 1024);
        assert_eq!(info.used, (16384 - 8192) * 1024);
        assert_eq!(info.cached, (2048 + 512 + 256) * 1024);
        assert_eq!(info.commit_as, 10000 * 1024);
        assert_eq!(info.commit_limit, 20000 * 1024);
        assert_eq!(info.swap_total, 4096 * 1024);
        assert_eq!(info.swap_used, (4096 - 3072) * 1024);
    }
}

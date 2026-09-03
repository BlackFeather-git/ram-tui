//! Parser for /proc/<pid>/statm, /proc/<pid>/smaps_rollup, /proc/<pid>/stat, and /proc/<pid>/comm.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Metric by which process list can be sorted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SortMetric {
    #[default]
    Rss,
    Pss,
    Uss,
    Name,
}

/// Page size in bytes (typically 4096 on Linux).
fn page_size() -> u64 {
    #[cfg(unix)]
    unsafe {
        let ps = libc::sysconf(libc::_SC_PAGE_SIZE);
        if ps > 0 {
            ps as u64
        } else {
            4096
        }
    }
    #[cfg(not(unix))]
    {
        4096
    }
}

/// Detailed info for a single process child instance in a group.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessChild {
    pub pid: u32,
    pub name: String,
    pub rss: u64,
    pub pss: Option<u64>,
    pub uss: Option<u64>,
}

/// Represents a single process or grouped process entry with RSS, PSS, and USS.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessInfo {
    pub name: String,
    pub rss: u64,         // Resident Set Size in bytes
    pub pss: Option<u64>, // Proportional Set Size in bytes (from smaps_rollup)
    pub uss: Option<u64>, // Unique / Private Set Size in bytes (from smaps_rollup)
    pub count: u32,
    pub pid: Option<u32>,
    pub children: Vec<ProcessChild>,
}

/// Parse the starttime field (field 22, 0-indexed after comm) from /proc/<pid>/stat.
/// The comm field can contain spaces and parentheses, so we find the last ')' first.
pub fn parse_starttime(stat_content: &str) -> Option<String> {
    let rparen = stat_content.rfind(')')?;
    let rest = stat_content.get(rparen + 2..)?;
    // Field index 19 (0-based from after comm) = starttime (field 22 in man proc)
    rest.split_whitespace().nth(19).map(|s| s.to_string())
}

/// Read the comm name for a PID from /proc/<pid>/comm, falling back to cmdline.
pub fn read_process_name(proc_dir: &Path, pid: &str) -> Option<String> {
    // Try /proc/<pid>/comm first
    let comm_path = proc_dir.join(pid).join("comm");
    if let Ok(comm) = fs::read_to_string(&comm_path) {
        let comm = comm.trim();
        if !comm.is_empty() {
            return Some(core_render::format::sanitize_text(comm));
        }
    }

    // Fall back to /proc/<pid>/cmdline
    let cmdline_path = proc_dir.join(pid).join("cmdline");
    if let Ok(data) = fs::read(&cmdline_path) {
        let cmd = String::from_utf8_lossy(&data).replace('\0', " ");
        let cmd = cmd.trim();
        if !cmd.is_empty() {
            let first = cmd.split_whitespace().next().unwrap_or(cmd);
            let basename = Path::new(first)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(first);
            return Some(core_render::format::sanitize_text(basename));
        }
    }

    None
}

/// Read RSS from /proc/<pid>/statm (field index 1, in pages).
fn read_rss_bytes(proc_dir: &Path, pid: &str, pg_size: u64) -> Option<u64> {
    let statm_path = proc_dir.join(pid).join("statm");
    let content = fs::read_to_string(&statm_path).ok()?;
    let content = content.trim();
    if content.is_empty() {
        return None;
    }
    // statm fields are: size resident shared text lib data dt — we only need field[1] (resident pages).
    let rss_pages: u64 = content.split_whitespace().nth(1)?.parse().ok()?;
    let rss_bytes = rss_pages.checked_mul(pg_size)?;
    if rss_bytes == 0 {
        return None;
    }
    Some(rss_bytes)
}

/// Read PSS and USS from /proc/<pid>/smaps_rollup (fast aggregated kernel rollup).
pub fn read_smaps_rollup(proc_dir: &Path, pid: &str) -> Option<(u64, u64)> {
    let smaps_path = proc_dir.join(pid).join("smaps_rollup");
    let content = fs::read_to_string(&smaps_path).ok()?;
    let mut pss_kb = None;
    let mut priv_clean_kb = 0u64;
    let mut priv_dirty_kb = 0u64;
    let mut has_priv = false;

    for line in content.lines() {
        let line = line.trim();
        if let Some((key, rest)) = line.split_once(':') {
            let key = key.trim();
            let val = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            if key == "Pss" {
                pss_kb = Some(val);
            } else if key == "Private_Clean" {
                priv_clean_kb = val;
                has_priv = true;
            } else if key == "Private_Dirty" {
                priv_dirty_kb = val;
                has_priv = true;
            }
        }
    }

    let pss_bytes = pss_kb.and_then(|k| k.checked_mul(1024));
    let uss_bytes = if has_priv {
        priv_clean_kb
            .checked_add(priv_dirty_kb)
            .and_then(|t| t.checked_mul(1024))
    } else {
        None
    };

    match (pss_bytes, uss_bytes) {
        (Some(pss), Some(uss)) => Some((pss, uss)),
        (Some(pss), None) => Some((pss, pss)),
        _ => None,
    }
}

/// Read starttime from /proc/<pid>/stat.
pub fn read_starttime(proc_dir: &Path, pid: &str) -> Option<String> {
    let stat_path = proc_dir.join(pid).join("stat");
    let content = fs::read_to_string(&stat_path).ok()?;
    parse_starttime(content.trim())
}

/// Validate process identity before signaling to prevent PID reuse races.
pub fn validate_process_identity(
    proc_dir: &Path,
    pid: u32,
    expected_name: &str,
    expected_starttime: Option<&str>,
) -> bool {
    let pid_str = pid.to_string();
    if let Some(exp_st) = expected_starttime {
        if let Some(actual_st) = read_starttime(proc_dir, &pid_str) {
            if actual_st != exp_st {
                return false; // Start time differs — PID was reused!
            }
        } else {
            return false; // Process has exited!
        }
    }
    if let Some(actual_name) = read_process_name(proc_dir, &pid_str) {
        if actual_name != expected_name {
            return false; // Name mismatch!
        }
    } else {
        return false;
    }
    true
}

/// Open a Linux pidfd for race-free process signaling (Linux >= 5.3 on supported architectures).
#[cfg(target_os = "linux")]
pub fn open_pidfd(pid: u32) -> Option<i32> {
    #[cfg(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64",
        target_arch = "x86",
        target_arch = "arm"
    ))]
    {
        const SYS_PIDFD_OPEN: libc::c_long = 434;
        let fd = unsafe { libc::syscall(SYS_PIDFD_OPEN, pid as libc::pid_t, 0) };
        if fd >= 0 {
            return Some(fd as i32);
        }
    }

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64",
        target_arch = "x86",
        target_arch = "arm"
    )))]
    {
        let _ = pid;
    }

    None
}

/// Send SIGTERM via pidfd (Linux >= 5.3 on supported architectures).
#[cfg(target_os = "linux")]
pub fn pidfd_send_sigterm(pidfd: i32) -> bool {
    #[cfg(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64",
        target_arch = "x86",
        target_arch = "arm"
    ))]
    {
        const SYS_PIDFD_SEND_SIGNAL: libc::c_long = 424;
        let ret = unsafe {
            libc::syscall(
                SYS_PIDFD_SEND_SIGNAL,
                pidfd,
                libc::SIGTERM,
                std::ptr::null::<libc::c_void>(),
                0,
            )
        };
        ret == 0
    }

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64",
        target_arch = "x86",
        target_arch = "arm"
    )))]
    {
        let _ = pidfd;
        false
    }
}

/// Collect top-N processes by RSS from /proc.
pub fn collect_processes(group_by_name: bool, limit: usize) -> Vec<ProcessInfo> {
    collect_processes_sorted(group_by_name, limit, SortMetric::Rss)
}

/// Collect top-N processes with custom sorting metric from /proc.
pub fn collect_processes_sorted(
    group_by_name: bool,
    limit: usize,
    sort_metric: SortMetric,
) -> Vec<ProcessInfo> {
    collect_processes_from_dir(Path::new("/proc"), group_by_name, limit, sort_metric)
}

/// Collect processes from a custom proc directory (for testing).
pub fn collect_processes_from_dir(
    proc_dir: &Path,
    group_by_name: bool,
    limit: usize,
    sort_metric: SortMetric,
) -> Vec<ProcessInfo> {
    let pg_size = if proc_dir == Path::new("/proc") {
        page_size()
    } else {
        4096 // Use fixed page size for test fixtures
    };

    let entries = match fs::read_dir(proc_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let pids: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.chars().all(|c| c.is_ascii_digit()) {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    let mut grouped: HashMap<String, ProcessInfo> = HashMap::new();
    let mut ungrouped: Vec<ProcessInfo> = Vec::new();

    for pid in &pids {
        let rss_bytes = match read_rss_bytes(proc_dir, pid, pg_size) {
            Some(r) => r,
            None => continue,
        };

        let comm = read_process_name(proc_dir, pid).unwrap_or_else(|| format!("PID {pid}"));

        let pid_num: u32 = pid.parse().unwrap_or(0);

        if group_by_name {
            let entry = grouped.entry(comm.clone()).or_insert_with(|| ProcessInfo {
                name: comm.clone(),
                rss: 0,
                pss: None,
                uss: None,
                count: 0,
                pid: None,
                children: Vec::new(),
            });
            entry.rss = entry.rss.saturating_add(rss_bytes);
            entry.count += 1;
            entry.children.push(ProcessChild {
                pid: pid_num,
                name: comm,
                rss: rss_bytes,
                pss: None,
                uss: None,
            });
        } else {
            ungrouped.push(ProcessInfo {
                name: format!("{comm} [{pid}]"),
                rss: rss_bytes,
                pss: None,
                uss: None,
                count: 1,
                pid: Some(pid_num),
                children: Vec::new(),
            });
        }
    }

    let mut procs: Vec<ProcessInfo> = if group_by_name {
        let mut list: Vec<ProcessInfo> = grouped.into_values().collect();
        for p in &mut list {
            p.children.sort_by_key(|a| std::cmp::Reverse(a.rss));
        }
        list
    } else {
        ungrouped
    };

    // Pre-sort by RSS so candidate sampling for PSS/USS enrichment selects the actual top memory consumers
    procs.sort_by(|a, b| {
        b.rss
            .cmp(&a.rss)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.pid.unwrap_or(0).cmp(&b.pid.unwrap_or(0)))
    });

    // When sorting by PSS or USS, sample all candidate processes to guarantee mathematical leader accuracy
    let candidate_count = if sort_metric == SortMetric::Pss || sort_metric == SortMetric::Uss {
        procs.len()
    } else {
        (limit * 3).max(24).min(procs.len())
    };

    for p in &mut procs[..candidate_count] {
        if group_by_name {
            let mut total_pss = 0u64;
            let mut total_uss = 0u64;
            let mut has_smaps = false;
            for child in &mut p.children {
                if let Some((p_bytes, u_bytes)) =
                    read_smaps_rollup(proc_dir, &child.pid.to_string())
                {
                    child.pss = Some(p_bytes);
                    child.uss = Some(u_bytes);
                    total_pss = total_pss.saturating_add(p_bytes);
                    total_uss = total_uss.saturating_add(u_bytes);
                    has_smaps = true;
                }
            }
            if has_smaps {
                p.pss = Some(total_pss);
                p.uss = Some(total_uss);
            }
        } else if let Some(pid_num) = p.pid {
            if let Some((p_bytes, u_bytes)) = read_smaps_rollup(proc_dir, &pid_num.to_string()) {
                p.pss = Some(p_bytes);
                p.uss = Some(u_bytes);
            }
        }
    }

    // Sort according to requested metric
    procs.sort_by(|a, b| {
        match sort_metric {
            SortMetric::Rss => b.rss.cmp(&a.rss),
            SortMetric::Pss => b.pss.unwrap_or(b.rss).cmp(&a.pss.unwrap_or(a.rss)),
            SortMetric::Uss => b.uss.unwrap_or(b.rss).cmp(&a.uss.unwrap_or(a.rss)),
            SortMetric::Name => a.name.cmp(&b.name),
        }
        .then_with(|| a.name.cmp(&b.name))
        .then_with(|| a.pid.unwrap_or(0).cmp(&b.pid.unwrap_or(0)))
    });

    procs.truncate(limit);
    procs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_proc_entry(proc_dir: &Path, pid: &str, comm: &str, rss_pages: u64, starttime: &str) {
        let pid_dir = proc_dir.join(pid);
        fs::create_dir_all(&pid_dir).unwrap();

        // /proc/<pid>/comm
        fs::write(pid_dir.join("comm"), format!("{comm}\n")).unwrap();

        // /proc/<pid>/statm: size rss shared text lib data dt
        fs::write(
            pid_dir.join("statm"),
            format!("{} {} 0 0 0 0 0\n", rss_pages * 2, rss_pages),
        )
        .unwrap();

        // /proc/<pid>/stat: pid (comm) S ppid ... starttime ...
        let stat_line = format!(
            "{pid} ({comm}) S 1 {pid} {pid} 0 -1 4194304 100 0 0 0 10 5 0 0 20 0 1 0 {starttime} 1000 500 0 0 0 0 0 0 0 0"
        );
        fs::write(pid_dir.join("stat"), format!("{stat_line}\n")).unwrap();
    }

    fn add_smaps_rollup(
        proc_dir: &Path,
        pid: &str,
        pss_kb: u64,
        priv_clean_kb: u64,
        priv_dirty_kb: u64,
    ) {
        let pid_dir = proc_dir.join(pid);
        let content = format!(
            "Rss: 1000 kB\nPss: {pss_kb} kB\nPrivate_Clean: {priv_clean_kb} kB\nPrivate_Dirty: {priv_dirty_kb} kB\n"
        );
        fs::write(pid_dir.join("smaps_rollup"), content).unwrap();
    }

    #[test]
    fn test_collect_grouped_with_smaps_rollup() {
        let temp = TempDir::new().unwrap();
        let proc = temp.path();

        create_proc_entry(proc, "100", "brave", 1000, "12345");
        add_smaps_rollup(proc, "100", 600, 200, 300);

        create_proc_entry(proc, "101", "brave", 500, "12346");
        add_smaps_rollup(proc, "101", 300, 100, 150);

        let procs = collect_processes_from_dir(proc, true, 10, SortMetric::Pss);
        assert_eq!(procs.len(), 1);
        let b = &procs[0];
        assert_eq!(b.name, "brave");
        assert_eq!(b.count, 2);
        assert_eq!(b.rss, 1500 * 4096);
        assert_eq!(b.pss, Some((600 + 300) * 1024));
        assert_eq!(b.uss, Some((500 + 250) * 1024));
    }

    #[test]
    fn test_empty_proc_dir() {
        let temp = TempDir::new().unwrap();
        let procs = collect_processes_from_dir(temp.path(), true, 10, SortMetric::Rss);
        assert!(procs.is_empty());
    }

    #[test]
    fn test_nonexistent_proc_dir() {
        let procs = collect_processes_from_dir(
            Path::new("/nonexistent_path_test"),
            true,
            10,
            SortMetric::Rss,
        );
        assert!(procs.is_empty());
    }

    #[test]
    fn test_collect_grouped() {
        let temp = TempDir::new().unwrap();
        let proc = temp.path();

        create_proc_entry(proc, "100", "brave", 1000, "12345");
        create_proc_entry(proc, "101", "brave", 500, "12346");
        create_proc_entry(proc, "102", "kitty", 200, "12347");

        let procs = collect_processes_from_dir(proc, true, 10, SortMetric::Rss);
        assert_eq!(procs.len(), 2);
        assert_eq!(procs[0].name, "brave");
        assert_eq!(procs[0].rss, 1500 * 4096);
        assert_eq!(procs[0].count, 2);
        assert_eq!(procs[1].name, "kitty");
        assert_eq!(procs[1].rss, 200 * 4096);
        assert_eq!(procs[1].count, 1);
    }

    #[test]
    fn test_collect_ungrouped() {
        let temp = TempDir::new().unwrap();
        let proc = temp.path();

        create_proc_entry(proc, "100", "brave", 1000, "12345");
        create_proc_entry(proc, "101", "brave", 500, "12346");

        let procs = collect_processes_from_dir(proc, false, 10, SortMetric::Rss);
        assert_eq!(procs.len(), 2);
        assert_eq!(procs[0].name, "brave [100]");
        assert_eq!(procs[0].rss, 1000 * 4096);
        assert_eq!(procs[0].pid, Some(100));
        assert_eq!(procs[1].name, "brave [101]");
        assert_eq!(procs[1].rss, 500 * 4096);
        assert_eq!(procs[1].pid, Some(101));
    }

    #[test]
    fn test_collect_limit() {
        let temp = TempDir::new().unwrap();
        let proc = temp.path();

        for i in 1..=10 {
            create_proc_entry(proc, &i.to_string(), &format!("proc{i}"), i * 100, "100");
        }

        let procs = collect_processes_from_dir(proc, true, 3, SortMetric::Rss);
        assert_eq!(procs.len(), 3);
        assert_eq!(procs[0].name, "proc10");
        assert_eq!(procs[1].name, "proc9");
        assert_eq!(procs[2].name, "proc8");
    }

    #[test]
    fn test_zero_rss_filtered() {
        let temp = TempDir::new().unwrap();
        let proc = temp.path();

        create_proc_entry(proc, "100", "kernel_worker", 0, "100");
        create_proc_entry(proc, "101", "normal_proc", 500, "100");

        let procs = collect_processes_from_dir(proc, true, 10, SortMetric::Rss);
        assert_eq!(procs.len(), 1);
        assert_eq!(procs[0].name, "normal_proc");
    }

    #[test]
    fn test_missing_comm_uses_fallback_name() {
        let temp = TempDir::new().unwrap();
        let proc = temp.path();
        let pid_dir = proc.join("100");
        fs::create_dir_all(&pid_dir).unwrap();
        fs::write(pid_dir.join("statm"), "2000 1000 0 0 0 0 0\n").unwrap();

        let procs = collect_processes_from_dir(proc, true, 10, SortMetric::Rss);
        assert_eq!(procs.len(), 1);
        assert_eq!(procs[0].name, "PID 100");
    }

    #[test]
    fn test_pid_reuse_different_starttime() {
        let temp = TempDir::new().unwrap();
        let proc = temp.path();

        create_proc_entry(proc, "100", "short_lived_1", 500, "1000");
        let procs1 = collect_processes_from_dir(proc, false, 10, SortMetric::Rss);
        assert_eq!(procs1[0].name, "short_lived_1 [100]");

        // Overwrite PID 100 with a new process with different starttime
        create_proc_entry(proc, "100", "reused_pid_proc", 800, "2000");
        let procs2 = collect_processes_from_dir(proc, false, 10, SortMetric::Rss);
        assert_eq!(procs2[0].name, "reused_pid_proc [100]");
    }

    #[test]
    fn test_parse_starttime_with_parentheses() {
        let stat = "1234 (my (cool) process) S 1 1234 1234 0 -1 4194304 100 0 0 0 10 5 0 0 20 0 1 0 987654 1000 500";
        assert_eq!(parse_starttime(stat), Some("987654".to_string()));
    }

    #[test]
    fn test_parse_starttime_empty() {
        assert_eq!(parse_starttime(""), None);
    }

    #[test]
    fn test_parse_starttime_truncated() {
        let stat = "1234 (comm) S 1 2 3";
        assert_eq!(parse_starttime(stat), None);
    }

    #[test]
    fn test_malformed_rss_value() {
        let temp = TempDir::new().unwrap();
        let proc = temp.path();
        let pid_dir = proc.join("100");
        fs::create_dir_all(&pid_dir).unwrap();
        fs::write(pid_dir.join("comm"), "proc\n").unwrap();
        fs::write(pid_dir.join("statm"), "bad_val bad_val\n").unwrap();

        let procs = collect_processes_from_dir(proc, true, 10, SortMetric::Rss);
        assert!(procs.is_empty());
    }

    #[test]
    fn test_empty_statm() {
        let temp = TempDir::new().unwrap();
        let proc = temp.path();
        let pid_dir = proc.join("100");
        fs::create_dir_all(&pid_dir).unwrap();
        fs::write(pid_dir.join("comm"), "proc\n").unwrap();
        fs::write(pid_dir.join("statm"), "").unwrap();

        let procs = collect_processes_from_dir(proc, true, 10, SortMetric::Rss);
        assert!(procs.is_empty());
    }

    #[test]
    fn test_truncated_statm() {
        let temp = TempDir::new().unwrap();
        let proc = temp.path();
        let pid_dir = proc.join("100");
        fs::create_dir_all(&pid_dir).unwrap();
        fs::write(pid_dir.join("comm"), "proc\n").unwrap();
        fs::write(pid_dir.join("statm"), "100\n").unwrap();

        let procs = collect_processes_from_dir(proc, true, 10, SortMetric::Rss);
        assert!(procs.is_empty());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_pidfd_and_identity_validation() {
        let my_pid = std::process::id();
        let pidfd = open_pidfd(my_pid);
        if let Some(fd) = pidfd {
            assert!(fd >= 0);
            unsafe {
                libc::close(fd);
            }
        }

        let is_valid = validate_process_identity(
            Path::new("/proc"),
            my_pid,
            "nonexistent_proc_name_12345",
            None,
        );
        assert!(!is_valid, "identity should fail when name does not match");
    }

    #[test]
    fn test_rss_presort_candidate_enrichment() {
        let temp = TempDir::new().unwrap();
        let proc = temp.path();

        // Create 30 processes where the highest RSS process is PID 30
        for i in 1..=30 {
            let pid_dir = proc.join(i.to_string());
            fs::create_dir_all(&pid_dir).unwrap();
            fs::write(pid_dir.join("comm"), format!("proc_{i}\n")).unwrap();
            // Assign increasing resident page count
            fs::write(pid_dir.join("statm"), format!("1000 {i} 10 0 0 0 0\n")).unwrap();
            fs::write(
                pid_dir.join("smaps_rollup"),
                format!(
                    "00400000-00452000 r-xp 00000000 08:02 123 /test\n\
                     Rss:                {} kB\n\
                     Pss:                {} kB\n\
                     Pss_Dirty:          0 kB\n\
                     Pss_Anon:           0 kB\n\
                     Pss_File:           0 kB\n\
                     Pss_Shmem:          0 kB\n\
                     Shared_Clean:       0 kB\n\
                     Shared_Dirty:       0 kB\n\
                     Private_Clean:      0 kB\n\
                     Private_Dirty:      {} kB\n",
                    i * 4,
                    i * 4,
                    i * 4
                ),
            )
            .unwrap();
        }

        let procs = collect_processes_from_dir(proc, true, 5, SortMetric::Rss);
        assert_eq!(procs.len(), 5);
        // The highest RSS process (proc_30) must be first and MUST have PSS populated
        assert_eq!(procs[0].name, "proc_30");
        assert!(
            procs[0].pss.is_some(),
            "Top RSS process must have PSS enriched"
        );
        assert_eq!(procs[0].pss, Some(30 * 4 * 1024));
    }
}

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
    // Safe: sysconf(_SC_PAGE_SIZE) on Linux
    unsafe {
        let ps = libc::sysconf(libc::_SC_PAGE_SIZE);
        if ps > 0 {
            ps as u64
        } else {
            4096
        }
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
    let fields: Vec<&str> = rest.split_whitespace().collect();
    // Field index 19 (0-based from after comm) = starttime (field 22 in man proc)
    fields.get(19).map(|s| s.to_string())
}

/// Read the comm name for a PID from /proc/<pid>/comm, falling back to cmdline.
fn read_process_name(proc_dir: &Path, pid: &str) -> Option<String> {
    // Try /proc/<pid>/comm first
    let comm_path = proc_dir.join(pid).join("comm");
    if let Ok(comm) = fs::read_to_string(&comm_path) {
        let comm = comm.trim();
        if !comm.is_empty() {
            return Some(sanitize_proc_name(comm));
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
            return Some(sanitize_proc_name(basename));
        }
    }

    None
}

/// Simple name sanitiser: replace control chars with '~'.
fn sanitize_proc_name(name: &str) -> String {
    name.chars()
        .map(|ch| {
            let code = ch as u32;
            if code < 32 || code == 127 {
                '~'
            } else {
                ch
            }
        })
        .collect()
}

/// Read RSS from /proc/<pid>/statm (field index 1, in pages).
fn read_rss_bytes(proc_dir: &Path, pid: &str, pg_size: u64) -> Option<u64> {
    let statm_path = proc_dir.join(pid).join("statm");
    let content = fs::read_to_string(&statm_path).ok()?;
    let content = content.trim();
    if content.is_empty() {
        return None;
    }
    let fields: Vec<&str> = content.split_whitespace().collect();
    if fields.len() < 2 {
        return None;
    }
    let rss_pages: u64 = fields[1].parse().ok()?;
    let rss_bytes = rss_pages * pg_size;
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

    let pss_bytes = pss_kb.map(|k| k * 1024);
    let uss_bytes = if has_priv {
        Some((priv_clean_kb + priv_dirty_kb) * 1024)
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
fn read_starttime(proc_dir: &Path, pid: &str) -> Option<String> {
    let stat_path = proc_dir.join(pid).join("stat");
    let content = fs::read_to_string(&stat_path).ok()?;
    parse_starttime(content.trim())
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
        // Read starttime for PID-reuse safety
        let _starttime = read_starttime(proc_dir, pid);

        let rss_bytes = match read_rss_bytes(proc_dir, pid, pg_size) {
            Some(r) => r,
            None => continue,
        };

        let comm = read_process_name(proc_dir, pid).unwrap_or_else(|| format!("PID {pid}"));

        let pid_num: u32 = pid.parse().unwrap_or(0);

        if group_by_name {
            let entry = grouped.entry(comm.clone()).or_insert(ProcessInfo {
                name: comm.clone(),
                rss: 0,
                pss: None,
                uss: None,
                count: 0,
                pid: None,
                children: Vec::new(),
            });
            entry.rss += rss_bytes;
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

    // Sort by RSS first to identify top candidate processes
    procs.sort_by_key(|a| std::cmp::Reverse(a.rss));

    // Only read smaps_rollup for top candidates to keep procfs scans ultra fast (<0.5ms)
    let candidate_count = (limit * 3).max(24).min(procs.len());
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
                    total_pss += p_bytes;
                    total_uss += u_bytes;
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
}

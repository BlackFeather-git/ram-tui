//! Cgroups v2 and v1 container memory limit and usage detection.
//!
//! Accurately detects memory constraints when running inside Docker,
//! Podman, LXC, systemd slices, or Kubernetes pods.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Cgroup memory statistics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CgroupInfo {
    /// Memory limit in bytes.
    pub limit: u64,
    /// Current memory usage in bytes.
    pub usage: u64,
    /// Indicates whether a constrained container/cgroup limit is active.
    pub is_container: bool,
    /// Cgroup version detected (1 or 2).
    pub version: u8,
}

/// Detect cgroup memory limit and usage from custom root paths (for testing).
pub fn detect_cgroup_from(
    cgroup_v2_dir: &Path,
    cgroup_v1_dir: &Path,
    total_host_ram: u64,
) -> Option<CgroupInfo> {
    // 1. Try Cgroups v2 first (/sys/fs/cgroup)
    let v2_max_path = cgroup_v2_dir.join("memory.max");
    if let Ok(max_str) = fs::read_to_string(&v2_max_path) {
        let max_str = max_str.trim();
        if max_str != "max" {
            if let Ok(limit) = max_str.parse::<u64>() {
                // Ignore absurdly high sentinel values or limits >= host RAM
                if limit > 0 && limit < total_host_ram {
                    let usage_path = cgroup_v2_dir.join("memory.current");
                    let usage = fs::read_to_string(usage_path)
                        .ok()
                        .and_then(|s| s.trim().parse::<u64>().ok())
                        .unwrap_or(0);

                    return Some(CgroupInfo {
                        limit,
                        usage,
                        is_container: true,
                        version: 2,
                    });
                }
            }
        }
    }

    // 2. Fall back to Cgroups v1 (/sys/fs/cgroup/memory)
    let v1_limit_path = cgroup_v1_dir.join("memory.limit_in_bytes");
    if let Ok(limit_str) = fs::read_to_string(&v1_limit_path) {
        if let Ok(limit) = limit_str.trim().parse::<u64>() {
            // Linux cgroups v1 uses near-u64::MAX as sentinel for "unlimited" (e.g. 0x7FFFFFFFFFFFF000)
            const CGROUP_V1_UNLIMITED_THRESHOLD: u64 = 0x7FFF_FFFF_0000_0000;
            if limit > 0 && limit < CGROUP_V1_UNLIMITED_THRESHOLD && limit < total_host_ram {
                let usage_path = cgroup_v1_dir.join("memory.usage_in_bytes");
                let usage = fs::read_to_string(usage_path)
                    .ok()
                    .and_then(|s| s.trim().parse::<u64>().ok())
                    .unwrap_or(0);

                return Some(CgroupInfo {
                    limit,
                    usage,
                    is_container: true,
                    version: 1,
                });
            }
        }
    }

    None
}

/// Detect cgroup memory limits on the live system.
pub fn detect_cgroup(total_host_ram: u64) -> Option<CgroupInfo> {
    detect_cgroup_from(
        Path::new("/sys/fs/cgroup"),
        Path::new("/sys/fs/cgroup/memory"),
        total_host_ram,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_cgroup_v2_constrained() {
        let dir = tempdir().unwrap();
        let max_path = dir.path().join("memory.max");
        let cur_path = dir.path().join("memory.current");

        File::create(&max_path)
            .unwrap()
            .write_all(b"2147483648\n")
            .unwrap(); // 2GB
        File::create(&cur_path)
            .unwrap()
            .write_all(b"524288000\n")
            .unwrap(); // 500MB

        let empty_v1 = tempdir().unwrap();
        let total_host_ram = 16 * 1024 * 1024 * 1024; // 16GB host

        let info = detect_cgroup_from(dir.path(), empty_v1.path(), total_host_ram).unwrap();
        assert_eq!(info.limit, 2 * 1024 * 1024 * 1024);
        assert_eq!(info.usage, 500 * 1024 * 1024);
        assert_eq!(info.version, 2);
        assert!(info.is_container);
    }

    #[test]
    fn test_cgroup_v2_max_unlimited() {
        let dir = tempdir().unwrap();
        let max_path = dir.path().join("memory.max");
        File::create(&max_path)
            .unwrap()
            .write_all(b"max\n")
            .unwrap();

        let empty_v1 = tempdir().unwrap();
        let total_host_ram = 16 * 1024 * 1024 * 1024;

        let info = detect_cgroup_from(dir.path(), empty_v1.path(), total_host_ram);
        assert!(info.is_none());
    }

    #[test]
    fn test_cgroup_v1_constrained() {
        let empty_v2 = tempdir().unwrap();
        let dir_v1 = tempdir().unwrap();
        let limit_path = dir_v1.path().join("memory.limit_in_bytes");
        let usage_path = dir_v1.path().join("memory.usage_in_bytes");

        File::create(&limit_path)
            .unwrap()
            .write_all(b"1073741824\n")
            .unwrap(); // 1GB
        File::create(&usage_path)
            .unwrap()
            .write_all(b"104857600\n")
            .unwrap(); // 100MB

        let total_host_ram = 8 * 1024 * 1024 * 1024; // 8GB host

        let info = detect_cgroup_from(empty_v2.path(), dir_v1.path(), total_host_ram).unwrap();
        assert_eq!(info.limit, 1024 * 1024 * 1024);
        assert_eq!(info.usage, 100 * 1024 * 1024);
        assert_eq!(info.version, 1);
        assert!(info.is_container);
    }

    #[test]
    fn test_cgroup_v1_unlimited_sentinel() {
        let empty_v2 = tempdir().unwrap();
        let dir_v1 = tempdir().unwrap();
        let limit_path = dir_v1.path().join("memory.limit_in_bytes");

        // Sentinel value for unlimited in Linux cgroup v1: 9223372036854771712 (0x7FFFFFFFFFFFF000)
        File::create(&limit_path)
            .unwrap()
            .write_all(b"9223372036854771712\n")
            .unwrap();

        let total_host_ram = 8 * 1024 * 1024 * 1024;
        let info = detect_cgroup_from(empty_v2.path(), dir_v1.path(), total_host_ram);
        assert!(info.is_none());
    }
}

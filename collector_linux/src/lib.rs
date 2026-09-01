//! collector_linux — Single-pass `/proc` parsers for memory and process info.
//!
//! Reads `/proc/meminfo`, `/proc/swaps`, `/proc/<pid>/statm`, `/proc/<pid>/stat`,
//! `/proc/<pid>/comm` to collect system memory state and top-RSS processes.

pub mod cgroup;
pub mod meminfo;
pub mod processes;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

pub use cgroup::{detect_cgroup, detect_cgroup_from, CgroupInfo};
pub use meminfo::{collect_meminfo, collect_meminfo_from, MemInfo};
pub use processes::{
    collect_processes, collect_processes_from_dir, collect_processes_sorted, ProcessChild,
    ProcessInfo, SortMetric,
};

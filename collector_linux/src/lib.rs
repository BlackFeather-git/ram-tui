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

#[cfg(target_os = "linux")]
pub use meminfo::{collect_meminfo, collect_meminfo_from, MemInfo};
#[cfg(target_os = "linux")]
pub use processes::{
    collect_processes, collect_processes_from_dir, collect_processes_sorted, read_starttime,
    validate_process_identity, ProcessChild, ProcessInfo, SortMetric,
};

#[cfg(target_os = "macos")]
pub use macos::{collect_meminfo, collect_processes_sorted};
#[cfg(target_os = "macos")]
pub use meminfo::MemInfo;
#[cfg(target_os = "macos")]
pub use processes::{ProcessChild, ProcessInfo, SortMetric};

#[cfg(target_os = "windows")]
pub use meminfo::MemInfo;
#[cfg(target_os = "windows")]
pub use processes::{ProcessChild, ProcessInfo, SortMetric};
#[cfg(target_os = "windows")]
pub use windows::{collect_meminfo, collect_processes_sorted};

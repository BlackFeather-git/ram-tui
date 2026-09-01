//! collector — Single-pass cross-platform system memory and process telemetry engine.

pub mod cgroup;
pub mod meminfo;
pub mod processes;

#[cfg(target_os = "macos")]
#[allow(
    clippy::all,
    non_snake_case,
    non_camel_case_types,
    dead_code,
    unused_imports
)]
pub mod macos;

#[cfg(target_os = "windows")]
#[allow(
    clippy::all,
    non_snake_case,
    non_camel_case_types,
    dead_code,
    unused_imports
)]
pub mod windows;

pub use cgroup::{detect_cgroup, detect_cgroup_from, CgroupInfo};

#[cfg(target_os = "linux")]
pub use meminfo::{collect_meminfo, collect_meminfo_from, MemInfo};
#[cfg(target_os = "linux")]
pub use processes::{
    collect_processes, collect_processes_from_dir, collect_processes_sorted, open_pidfd,
    pidfd_send_sigterm, read_starttime, validate_process_identity, ProcessChild, ProcessInfo,
    SortMetric,
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

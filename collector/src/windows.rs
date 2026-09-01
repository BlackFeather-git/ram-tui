//! Native Windows telemetry backend using GlobalMemoryStatusEx and PSAPI.

use crate::cgroup::CgroupInfo;
use crate::meminfo::MemInfo;
use crate::processes::{ProcessChild, ProcessInfo, SortMetric};
use std::collections::HashMap;

#[cfg(target_os = "windows")]
#[repr(C)]
struct MEMORYSTATUSEX {
    dw_length: u32,
    dw_memory_load: u32,
    ull_total_phys: u64,
    ull_avail_phys: u64,
    ull_total_page_file: u64,
    ull_avail_page_file: u64,
    ull_total_virtual: u64,
    ull_avail_virtual: u64,
    ull_avail_extended_virtual: u64,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct PROCESS_MEMORY_COUNTERS_EX {
    cb: u32,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
    private_usage: usize,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct PERFORMANCE_INFORMATION {
    cb: u32,
    commit_total: usize,
    commit_limit: usize,
    commit_peak: usize,
    physical_total: usize,
    physical_available: usize,
    system_cache: usize,
    kernel_total: usize,
    kernel_paged: usize,
    kernel_nonpaged: usize,
    page_size: usize,
    handle_count: u32,
    process_count: u32,
    thread_count: u32,
}

#[cfg(target_os = "windows")]
extern "system" {
    fn GlobalMemoryStatusEx(lpBuffer: *mut MEMORYSTATUSEX) -> i32;
    fn K32GetPerformanceInfo(pPerformanceInformation: *mut PERFORMANCE_INFORMATION, cb: u32)
        -> i32;
    fn K32EnumProcesses(lpidProcess: *mut u32, cb: u32, lpcbNeeded: *mut u32) -> i32;
    fn OpenProcess(
        dwDesiredAccess: u32,
        bInheritHandle: i32,
        dwProcessId: u32,
    ) -> *mut libc::c_void;
    fn CloseHandle(hObject: *mut libc::c_void) -> i32;
    fn K32GetProcessMemoryInfo(
        hProcess: *mut libc::c_void,
        ppsmps: *mut PROCESS_MEMORY_COUNTERS_EX,
        cb: u32,
    ) -> i32;
    fn K32GetProcessImageFileNameA(
        hProcess: *mut libc::c_void,
        lpImageFileName: *mut u8,
        nSize: u32,
    ) -> u32;
}

#[cfg(target_os = "windows")]
pub fn collect_meminfo() -> MemInfo {
    let mut statex: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    statex.dw_length = std::mem::size_of::<MEMORYSTATUSEX>() as u32;

    let ret = unsafe { GlobalMemoryStatusEx(&mut statex) };
    if ret == 0 {
        return MemInfo::default();
    }

    let mut perf: PERFORMANCE_INFORMATION = unsafe { std::mem::zeroed() };
    perf.cb = std::mem::size_of::<PERFORMANCE_INFORMATION>() as u32;
    let has_perf = unsafe { K32GetPerformanceInfo(&mut perf, perf.cb) } != 0;

    let total = statex.ull_total_phys;
    let available = statex.ull_avail_phys;
    let used = total.saturating_sub(available);

    let (commit_as, commit_limit, cached) = if has_perf && perf.page_size > 0 {
        let ps = perf.page_size as u64;
        (
            (perf.commit_total as u64).saturating_mul(ps),
            (perf.commit_limit as u64).saturating_mul(ps),
            (perf.system_cache as u64).saturating_mul(ps),
        )
    } else {
        (0, 0, 0)
    };

    let swap_total = statex.ull_total_page_file.saturating_sub(total);
    let swap_used = 0;

    MemInfo {
        total,
        available,
        used,
        commit_as,
        commit_limit,
        cached,
        swap_used,
        swap_total,
        swap_desc: if swap_total > 0 {
            "pagefile".into()
        } else {
            "none".into()
        },
        cgroup: None,
        valid: total > 0,
    }
}

#[cfg(target_os = "windows")]
pub fn collect_processes_sorted(
    group_by_name: bool,
    limit: usize,
    sort_metric: SortMetric,
) -> Vec<ProcessInfo> {
    let mut pids = vec![0u32; 2048];
    let mut cb_needed: u32 = 0;
    let ret = unsafe {
        K32EnumProcesses(
            pids.as_mut_ptr(),
            (pids.len() * std::mem::size_of::<u32>()) as u32,
            &mut cb_needed,
        )
    };

    if ret == 0 || cb_needed == 0 {
        return Vec::new();
    }

    let num_pids = (cb_needed as usize) / std::mem::size_of::<u32>();
    pids.truncate(num_pids);

    let mut grouped: HashMap<String, ProcessInfo> = HashMap::new();
    let mut ungrouped: Vec<ProcessInfo> = Vec::new();

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const PROCESS_VM_READ: u32 = 0x0010;

    for pid in pids {
        if pid == 0 {
            continue;
        }

        let h_proc =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, 0, pid) };

        if h_proc.is_null() {
            continue;
        }

        let mut pmc: PROCESS_MEMORY_COUNTERS_EX = unsafe { std::mem::zeroed() };
        pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;

        let mem_ret = unsafe { K32GetProcessMemoryInfo(h_proc, &mut pmc, pmc.cb) };

        if mem_ret == 0 {
            unsafe {
                CloseHandle(h_proc);
            }
            continue;
        }

        let rss_bytes = pmc.working_set_size as u64;
        let uss_bytes = if pmc.private_usage > 0 {
            Some(pmc.private_usage as u64)
        } else {
            None
        };

        let mut name_buf = [0u8; 512];
        let name_ret = unsafe {
            K32GetProcessImageFileNameA(h_proc, name_buf.as_mut_ptr(), name_buf.len() as u32)
        };

        unsafe {
            CloseHandle(h_proc);
        }

        let raw_comm = if name_ret > 0 {
            let len = name_buf
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(name_ret as usize);
            let full_path = String::from_utf8_lossy(&name_buf[..len]);
            full_path
                .rsplit('\\')
                .next()
                .unwrap_or(&full_path)
                .to_string()
        } else {
            format!("PID {pid}")
        };
        let comm = core_render::format::sanitize_text(&raw_comm);

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
            if let Some(u) = uss_bytes {
                *entry.uss.get_or_insert(0) += u;
            }
            entry.count += 1;
            entry.children.push(ProcessChild {
                pid,
                name: comm,
                rss: rss_bytes,
                pss: None,
                uss: uss_bytes,
            });
        } else {
            ungrouped.push(ProcessInfo {
                name: format!("{comm} [{pid}]"),
                rss: rss_bytes,
                pss: None,
                uss: uss_bytes,
                count: 1,
                pid: Some(pid),
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

    procs.sort_by(|a, b| {
        match sort_metric {
            SortMetric::Rss | SortMetric::Pss => b.rss.cmp(&a.rss),
            SortMetric::Uss => b.uss.unwrap_or(b.rss).cmp(&a.uss.unwrap_or(a.rss)),
            SortMetric::Name => a.name.cmp(&b.name),
        }
        .then_with(|| a.name.cmp(&b.name))
        .then_with(|| a.pid.unwrap_or(0).cmp(&b.pid.unwrap_or(0)))
    });

    procs.truncate(limit);
    procs
}

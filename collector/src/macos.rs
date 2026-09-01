//! Native macOS Darwin telemetry backend using Mach kernel APIs and sysctl.

use crate::meminfo::MemInfo;
use crate::processes::{ProcessChild, ProcessInfo, SortMetric};
use std::collections::HashMap;

extern "C" {
    fn mach_host_self() -> libc::mach_port_t;
    fn proc_listpids(type_: u32, typeinfo: u32, buffer: *mut libc::c_void, buffersize: i32) -> i32;
    fn proc_pidinfo(
        pid: i32,
        flavor: i32,
        arg: u64,
        buffer: *mut libc::c_void,
        buffersize: i32,
    ) -> i32;
    fn proc_name(pid: i32, buffer: *mut libc::c_void, buffersize: u32) -> i32;
}

const PROC_ALL_PIDS: u32 = 1;
const PROC_PIDTASKINFO: i32 = 4;

#[repr(C)]
#[derive(Default)]
struct ProcTaskInfo {
    pub pti_virtual_size: u64,
    pub pti_resident_size: u64,
    pub pti_total_user: u64,
    pub pti_total_system: u64,
    pub pti_threads_user: u64,
    pub pti_threads_system: u64,
    pub pti_policy: i32,
    pub pti_faults: i32,
    pub pti_pageins: i32,
    pub pti_cow_faults: i32,
    pub pti_messages_sent: i32,
    pub pti_messages_received: i32,
    pub pti_syscalls_mach: i32,
    pub pti_syscalls_unix: i32,
    pub pti_csw: i32,
    pub pti_threadnum: i32,
    pub pti_numrunning: i32,
    pub pti_priority: i32,
}

#[cfg(target_os = "macos")]
pub fn collect_meminfo() -> MemInfo {
    let mut total: u64 = 0;
    let mut len = std::mem::size_of::<u64>();
    let mib = [libc::CTL_HW, libc::HW_MEMSIZE];
    unsafe {
        libc::sysctl(
            mib.as_ptr() as *mut _,
            2,
            &mut total as *mut _ as *mut libc::c_void,
            &mut len,
            std::ptr::null_mut(),
            0,
        );
    }

    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    let ps = if page_size > 0 {
        page_size as u64
    } else {
        4096
    };

    let mut free: u64 = 0;
    let mut active: u64 = 0;
    let mut inactive: u64 = 0;
    let mut speculative: u64 = 0;
    let mut wired: u64 = 0;
    let mut compressed: u64 = 0;

    unsafe {
        let host_port = mach_host_self();
        let mut vm_stat: libc::vm_statistics64 = std::mem::zeroed();
        let mut count = (std::mem::size_of::<libc::vm_statistics64>()
            / std::mem::size_of::<libc::integer_t>())
            as libc::mach_msg_type_number_t;
        let ret = libc::host_statistics64(
            host_port,
            libc::HOST_VM_INFO64,
            &mut vm_stat as *mut _ as *mut _,
            &mut count,
        );
        if ret == libc::KERN_SUCCESS {
            free = vm_stat.free_count as u64 * ps;
            active = vm_stat.active_count as u64 * ps;
            inactive = vm_stat.inactive_count as u64 * ps;
            speculative = vm_stat.speculative_count as u64 * ps;
            wired = vm_stat.wire_count as u64 * ps;
            compressed = vm_stat.compressor_page_count as u64 * ps;
        }
    }

    // Swap usage via sysctl vm.swapusage
    let mut swap_total: u64 = 0;
    let mut swap_used: u64 = 0;
    #[repr(C)]
    struct xsw_usage {
        xsu_total: u64,
        xsu_avail: u64,
        xsu_used: u64,
        xsu_pagesize: u32,
        xsu_encrypted: bool,
    }
    let mut xsu: xsw_usage = unsafe { std::mem::zeroed() };
    let mut xsu_len = std::mem::size_of::<xsw_usage>();
    let swap_name = std::ffi::CString::new("vm.swapusage").unwrap();
    unsafe {
        if libc::sysctlbyname(
            swap_name.as_ptr(),
            &mut xsu as *mut _ as *mut libc::c_void,
            &mut xsu_len,
            std::ptr::null_mut(),
            0,
        ) == 0
        {
            swap_total = xsu.xsu_total;
            swap_used = xsu.xsu_used;
        }
    }

    let available = free + inactive + speculative;
    let used = if total > available {
        total - available
    } else {
        active + wired + compressed
    };
    let cached = inactive + speculative;

    MemInfo {
        total,
        available,
        used,
        commit_as: 0,
        commit_limit: 0,
        cached,
        swap_used,
        swap_total,
        swap_desc: if swap_total > 0 {
            "mach swap".into()
        } else {
            "none".into()
        },
        cgroup: None,
        valid: total > 0,
    }
}

#[cfg(target_os = "macos")]
pub fn collect_processes_sorted(
    group_by_name: bool,
    limit: usize,
    sort_metric: SortMetric,
) -> Vec<ProcessInfo> {
    let mut pids = vec![0i32; 2048];
    let mut num_bytes = unsafe {
        proc_listpids(
            PROC_ALL_PIDS,
            0,
            pids.as_mut_ptr() as *mut libc::c_void,
            (pids.len() * std::mem::size_of::<i32>()) as i32,
        )
    };

    while num_bytes > 0 && (num_bytes as usize) >= pids.len() * std::mem::size_of::<i32>() {
        pids.resize(pids.len() * 2, 0);
        num_bytes = unsafe {
            proc_listpids(
                PROC_ALL_PIDS,
                0,
                pids.as_mut_ptr() as *mut libc::c_void,
                (pids.len() * std::mem::size_of::<i32>()) as i32,
            )
        };
    }

    if num_bytes <= 0 {
        return Vec::new();
    }

    let num_pids = (num_bytes as usize) / std::mem::size_of::<i32>();
    pids.truncate(num_pids);

    let mut grouped: HashMap<String, ProcessInfo> = HashMap::new();
    let mut ungrouped: Vec<ProcessInfo> = Vec::new();

    for pid in pids {
        if pid <= 0 {
            continue;
        }

        let mut task_info: ProcTaskInfo = unsafe { std::mem::zeroed() };
        let ret = unsafe {
            proc_pidinfo(
                pid,
                PROC_PIDTASKINFO,
                0,
                &mut task_info as *mut _ as *mut libc::c_void,
                std::mem::size_of::<ProcTaskInfo>() as i32,
            )
        };

        if ret <= 0 {
            continue;
        }

        let rss_bytes = task_info.pti_resident_size;
        if rss_bytes == 0 {
            continue;
        }

        let mut name_buf = [0u8; 256];
        let name_ret = unsafe {
            proc_name(
                pid,
                name_buf.as_mut_ptr() as *mut libc::c_void,
                name_buf.len() as u32,
            )
        };

        let raw_comm = if name_ret > 0 {
            let len = name_buf
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(name_buf.len());
            String::from_utf8_lossy(&name_buf[..len]).to_string()
        } else {
            format!("PID {pid}")
        };
        let comm = core_render::format::sanitize_text(&raw_comm);

        let pid_u32 = pid as u32;

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
                pid: pid_u32,
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
                pid: Some(pid_u32),
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
            SortMetric::Name => a.name.cmp(&b.name),
            _ => b.rss.cmp(&a.rss),
        }
        .then_with(|| a.name.cmp(&b.name))
        .then_with(|| a.pid.unwrap_or(0).cmp(&b.pid.unwrap_or(0)))
    });

    procs.truncate(limit);
    procs
}

use std::fs;
use std::path::Path;

/// /proc 基础操作与冻结原语（SIGSTOP / renice / cpuset）

#[derive(Clone)]
pub struct ProcEntry {
    pub pid: i32,
    pub name: String,
    pub oom_adj: i32,
    pub rss_kb: u64,
}

fn is_pid_dir(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

pub fn read_cmdline(pid: i32) -> Option<String> {
    let data = fs::read(format!("/proc/{}/cmdline", pid)).ok()?;
    let first = data.split(|&b| b == 0).next().unwrap_or(&[]);
    let name = String::from_utf8_lossy(first).to_string();
    if name.is_empty() { None } else { Some(name) }
}

pub fn read_oom_adj(pid: i32) -> Option<i32> {
    fs::read_to_string(format!("/proc/{}/oom_score_adj", pid))
        .ok()?.trim().parse().ok()
}

pub fn read_rss_kb(pid: i32) -> u64 {
    // statm 第二字段为 RSS 页数，页大小 4KB
    if let Ok(s) = fs::read_to_string(format!("/proc/{}/statm", pid)) {
        if let Some(rss) = s.split_whitespace().nth(1) {
            if let Ok(pages) = rss.parse::<u64>() {
                return pages * 4;
            }
        }
    }
    0
}

pub fn list_procs() -> Vec<ProcEntry> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir("/proc") {
        for e in entries.flatten() {
            let fname = e.file_name().to_string_lossy().into_owned();
            if !is_pid_dir(&fname) { continue; }
            let pid: i32 = match fname.parse() { Ok(p) => p, Err(_) => continue };
            let Some(name) = read_cmdline(pid) else { continue };
            let oom_adj = read_oom_adj(pid).unwrap_or(1000);
            let rss_kb = read_rss_kb(pid);
            out.push(ProcEntry { pid, name, oom_adj, rss_kb });
        }
    }
    out
}

pub fn sigstop(pid: i32) -> bool { unsafe { libc::kill(pid, libc::SIGSTOP) == 0 } }
pub fn sigcont(pid: i32) -> bool { unsafe { libc::kill(pid, libc::SIGCONT) == 0 } }

pub fn get_nice(pid: i32) -> i32 {
    unsafe { libc::getpriority(libc::PRIO_PROCESS, pid as u32) }
}

pub fn set_nice(pid: i32, nice: i32) -> bool {
    unsafe { libc::setpriority(libc::PRIO_PROCESS, pid as u32, nice) == 0 }
}

/// 尽可能把进程压入受限 cpuset（不同代 Android 路径不同，尽力而为）
const STOP_GROUPS: &[&str] = &[
    "/dev/cpuset/restricted/tasks",
    "/dev/cpuset/background/tasks",
    "/sys/fs/cgroup/cpuset/restricted/cgroup.procs",
    "/sys/fs/cgroup/cpuset/background/cgroup.procs",
];

const RESTORE_GROUPS: &[&str] = &[
    "/dev/cpuset/foreground/tasks",
    "/dev/cpuset/top-app/tasks",
    "/sys/fs/cgroup/cpuset/foreground/cgroup.procs",
    "/sys/fs/cgroup/cpuset/top-app/cgroup.procs",
    "/sys/fs/cgroup/cpuset/cgroup.procs",
];

fn write_pid_to_group(pid: i32, groups: &[&str]) -> bool {
    for g in groups {
        if Path::new(g).exists() {
            if fs::write(g, format!("{}\n", pid)).is_ok() {
                return true;
            }
        }
    }
    false
}

pub fn cpuset_restrict(pid: i32) -> bool { write_pid_to_group(pid, STOP_GROUPS) }
pub fn cpuset_restore(pid: i32) -> bool { write_pid_to_group(pid, RESTORE_GROUPS) }
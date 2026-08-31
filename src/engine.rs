use crate::config::Config;
use crate::logger::Logger;
use crate::sys;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize)]
pub struct ProcStatus {
    pub pid: i32,
    pub name: String,
    pub state: String, // running / frozen / preserved
    pub oom_adj: i32,
    pub rss_kb: u64,
    pub nice: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct AppStatus {
    pub package: String,
    pub label: String,
    pub enabled: bool,
    pub foreground: bool,
    pub grace: bool,
    pub power: u8,
    pub processes: Vec<ProcStatus>,
    pub frozen_kb: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct EngineStatus {
    pub version: &'static str,
    pub enabled: bool,
    pub interval_secs: u64,
    pub started_at: u64,
    pub uptime_secs: u64,
    pub last_cycle_ms: u64,
    pub cycles: u64,
    pub frozen_count: usize,
    pub frozen_kb: u64,
    pub apps: Vec<AppStatus>,
}

#[derive(Clone)]
struct FrozenMeta {
    name: String,
    old_nice: i32,
    app: String,
}

pub struct Engine {
    pub cfg_path: String,
    pub config: RwLock<Config>,
    pub log: Arc<Logger>,
    frozen: Mutex<HashMap<i32, FrozenMeta>>,
    visible_since: Mutex<HashMap<String, u64>>,
    status: Mutex<EngineStatus>,
    started_at: u64,
    started: Instant,
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl Engine {
    pub fn new(cfg_path: String, config: Config, log: Arc<Logger>) -> Arc<Engine> {
        let started_at = now_secs();
        let status = EngineStatus {
            version: "1.0.0",
            enabled: config.enabled,
            interval_secs: config.interval_secs,
            started_at,
            uptime_secs: 0,
            last_cycle_ms: 0,
            cycles: 0,
            frozen_count: 0,
            frozen_kb: 0,
            apps: Vec::new(),
        };
        Arc::new(Engine {
            cfg_path,
            config: RwLock::new(config),
            log,
            frozen: Mutex::new(HashMap::new()),
            visible_since: Mutex::new(HashMap::new()),
            status: Mutex::new(status),
            started_at,
            started: Instant::now(),
        })
    }

    pub fn run_loop(self: Arc<Self>) {
        loop {
            let t0 = Instant::now();
            self.cycle();
            let ms = t0.elapsed().as_millis() as u64;
            {
                let mut st = self.status.lock().unwrap();
                st.last_cycle_ms = ms;
                st.cycles += 1;
            }
            let secs = self.config.read().unwrap().interval_secs.max(1);
            std::thread::sleep(Duration::from_secs(secs));
        }
    }

    pub fn status_snapshot(&self) -> EngineStatus {
        self.status.lock().unwrap().clone()
    }

    // ---------- 冻结 / 解冻 ----------

    fn freeze_pid(&self, pid: i32, entry: &sys::ProcEntry, app: &str, power: u8) {
        let cfg = self.config.read().unwrap().clone();
        let old_nice = sys::get_nice(pid);
        let use_sig = cfg.use_sigstop && power >= 2;
        let use_cpu = cfg.use_cpuset && power >= 1;
        let use_nice = cfg.use_renice;
        let ok_sig = !use_sig || sys::sigstop(pid);
        let ok_cpu = !use_cpu || sys::cpuset_restrict(pid);
        let ok_nice = if use_nice { sys::set_nice(pid, 19) } else { true };
        if ok_sig || ok_cpu || ok_nice {
            self.frozen.lock().unwrap().insert(pid, FrozenMeta { name: entry.name.clone(), old_nice, app: app.to_string() });
            self.log.info(&format!(
                "FREEZE  pid={} name={} app={} power={} sigstop={} cpuset={} renice={}",
                pid, entry.name, app, power, ok_sig, ok_cpu, ok_nice
            ));
        } else {
            self.log.warn(&format!("FREEZE 失败 pid={} name={}", pid, entry.name));
        }
    }

    fn unfreeze_pid(&self, pid: i32) {
        if let Some(meta) = self.frozen.lock().unwrap().remove(&pid) {
            let ok_cont = sys::sigcont(pid);
            let ok_cpu = sys::cpuset_restore(pid);
            let ok_nice = sys::set_nice(pid, meta.old_nice);
            self.log.info(&format!(
                "UNFREEZE pid={} name={} app={} cont={} cpuset={} nice={}",
                pid, meta.name, meta.app, ok_cont, ok_cpu, ok_nice
            ));
        }
    }

    // ---------- 主循环 ----------

    fn cycle(&self) {
        let cfg = self.config.read().unwrap().clone();
        // 总开关关闭：解冻所有并停止
        if !cfg.enabled {
            if !self.frozen.lock().unwrap().is_empty() {
                let pids: Vec<i32> = self.frozen.lock().unwrap().keys().cloned().collect();
                for p in pids {
                    self.unfreeze_pid(p);
                }
            }
            return;
        }

        let procs = sys::list_procs();
        let mut seen: HashSet<i32> = HashSet::new();
        let mut apps_status: Vec<AppStatus> = Vec::new();
        let mut total_frozen = 0usize;
        let mut total_kb: u64 = 0;

        for app in &cfg.apps {
            if !app.enabled {
                continue;
            }
            let prefix = format!("{}:", app.package);
            let owned: Vec<sys::ProcEntry> = procs
                .iter()
                .filter(|p| p.name == app.package || p.name.starts_with(&prefix))
                .cloned()
                .collect();
            if owned.is_empty() {
                continue;
            }
            for p in &owned {
                seen.insert(p.pid);
            }

            let foreground = owned.iter().any(|p| p.oom_adj <= 200);
            // 宽限：应用可见时记录时间；切后台后 grace 秒内不冻结（避免切回时重新加载）
            {
                let mut vs = self.visible_since.lock().unwrap();
                if foreground {
                    vs.insert(app.package.clone(), now_secs());
                }
            }
            let in_grace = cfg.grace_secs > 0
                && self.visible_since.lock().unwrap().get(&app.package)
                    .map(|t| now_secs().saturating_sub(*t) < cfg.grace_secs)
                    .unwrap_or(false);
            let protected = foreground || in_grace;

            // 决策：收集需要冻结 / 解冻的 pid
            let mut to_freeze: Vec<sys::ProcEntry> = Vec::new();
            let mut to_unfreeze: Vec<i32> = Vec::new();
            {
                let frozen_map = self.frozen.lock().unwrap();
                for p in &owned {
                    let is_main = p.name == app.package;
                    let is_keep = !is_main && app.keep.iter().any(|k| p.name.ends_with(k));
                    let should_freeze = !protected && !is_keep && !(is_main && !app.freeze_main);
                    if should_freeze {
                        if !frozen_map.contains_key(&p.pid) {
                            to_freeze.push(p.clone());
                        }
                    } else if frozen_map.contains_key(&p.pid) {
                        to_unfreeze.push(p.pid);
                    }
                }
            }
            for p in &to_freeze {
                self.freeze_pid(p.pid, p, &app.package, app.power);
            }
            for pid in &to_unfreeze {
                self.unfreeze_pid(*pid);
            }

            // 组装该应用的状态视图
            let frozen_map = self.frozen.lock().unwrap();
            let mut procs_status: Vec<ProcStatus> = Vec::new();
            let mut app_kb: u64 = 0;
            let mut app_frozen = 0usize;
            for p in &owned {
                let is_main = p.name == app.package;
                let is_keep = !is_main && app.keep.iter().any(|k| p.name.ends_with(k));
                let is_frozen = frozen_map.contains_key(&p.pid);
                let is_preserved = !is_frozen && (is_keep || (is_main && !app.freeze_main));
                let state = if is_frozen { "frozen" } else if is_preserved { "preserved" } else { "running" };
                let nice = sys::get_nice(p.pid);
                procs_status.push(ProcStatus {
                    pid: p.pid,
                    name: p.name.clone(),
                    state: state.to_string(),
                    oom_adj: p.oom_adj,
                    rss_kb: p.rss_kb,
                    nice,
                });
                if is_frozen {
                    app_frozen += 1;
                    app_kb += p.rss_kb;
                }
            }
            total_frozen += app_frozen;
            total_kb += app_kb;
            drop(frozen_map);

            apps_status.push(AppStatus {
                package: app.package.clone(),
                label: app.label.clone(),
                enabled: app.enabled,
                foreground,
                grace: in_grace,
                power: app.power,
                processes: procs_status,
                frozen_kb: app_kb,
            });
        }

        // 清理已退出进程的冻结记录
        {
            let mut fm = self.frozen.lock().unwrap();
            let dead: Vec<i32> = fm.keys().filter(|k| !seen.contains(k)).cloned().collect();
            for pid in dead {
                if let Some(meta) = fm.remove(&pid) {
                    self.log.info(&format!("PRUNE  pid={} name={} 进程已退出", pid, meta.name));
                }
            }
        }

        let mut st = self.status.lock().unwrap();
        st.enabled = cfg.enabled;
        st.interval_secs = cfg.interval_secs;
        st.uptime_secs = self.started.elapsed().as_secs();
        st.frozen_count = total_frozen;
        st.frozen_kb = total_kb;
        st.apps = apps_status;
    }

    pub fn cycle_now(&self) {
        self.cycle();
    }

    pub fn publish_meta(&self) {
        let mut st = self.status.lock().unwrap();
        st.enabled = self.config.read().unwrap().enabled;
        st.interval_secs = self.config.read().unwrap().interval_secs;
    }

    pub fn unfreeze_all(&self) {
        let pids: Vec<i32> = self.frozen.lock().unwrap().keys().cloned().collect();
        for p in pids {
            self.unfreeze_pid(p);
        }
    }

    pub fn unfreeze_app(&self, pkg: &str) {
        let pids: Vec<i32> = self
            .frozen
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, m)| m.app == pkg)
            .map(|(k, _)| *k)
            .collect();
        for p in pids {
            self.unfreeze_pid(p);
        }
    }

    pub fn freeze_app(&self, pkg: &str) {
        let cfg = self.config.read().unwrap().clone();
        if !cfg.enabled {
            return;
        }
        let Some(app) = cfg.apps.iter().find(|a| a.package == pkg && a.enabled) else {
            return;
        };
        let prefix = format!("{}:", pkg);
        let procs = sys::list_procs();
        for p in procs
            .iter()
            .filter(|p| p.name == *pkg || p.name.starts_with(&prefix))
        {
            let is_main = p.name == *pkg;
            let is_keep = !is_main && app.keep.iter().any(|k| p.name.ends_with(k));
            let can = !is_keep && !(is_main && !app.freeze_main);
            if can && !self.frozen.lock().unwrap().contains_key(&p.pid) {
                self.freeze_pid(p.pid, p, pkg, app.power);
            }
        }
    }
}
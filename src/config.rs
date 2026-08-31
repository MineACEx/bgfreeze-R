use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 配置：总开关、循环间隔、冻结机制、目标应用（保留进程 / 是否冻结主进程）

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConf {
    pub package: String,
    pub label: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub keep: Vec<String>,
    #[serde(default = "default_true")]
    pub freeze_main: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
    #[serde(default = "default_true")]
    pub use_sigstop: bool,
    #[serde(default = "default_true")]
    pub use_renice: bool,
    #[serde(default = "default_true")]
    pub use_cpuset: bool,
    #[serde(default = "default_port")]
    pub port: u16,
    /// 解冻宽限期(秒)：应用切后台后此时间内不冻结，短时间切回秒开；0 = 禁用
    #[serde(default = "default_grace")]
    pub grace_secs: u64,
    #[serde(default)]
    pub apps: Vec<AppConf>,
}

fn default_true() -> bool { true }
fn default_interval() -> u64 { 10 }
fn default_port() -> u16 { 8765 }
fn default_grace() -> u64 { 120 }

impl Default for Config {
    fn default() -> Self {
        Config {
            enabled: true,
            interval_secs: 10,
            grace_secs: 120,
            use_sigstop: true,
            use_renice: true,
            use_cpuset: true,
            port: 8765,
            apps: vec![
                AppConf { package: "com.tencent.mm".into(), label: "微信".into(), enabled: true, keep: vec![":push".into()], freeze_main: true },
                AppConf { package: "com.tencent.mobileqq".into(), label: "QQ".into(), enabled: true, keep: vec![":MSF".into(), ":msf".into()], freeze_main: true },
                AppConf { package: "com.ss.android.ugc.aweme".into(), label: "抖音".into(), enabled: true, keep: vec![":pushservice".into(), ":push".into(), ":pump".into()], freeze_main: true },
            ],
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Config {
        match fs::read_to_string(path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
                eprintln!("config parse error: {}", e);
                Config::default()
            }),
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self, path: &str) -> bool {
        if let Ok(s) = serde_json::to_string_pretty(self) {
            if let Some(p) = Path::new(path).parent() {
                let _ = fs::create_dir_all(p);
            }
            match fs::write(path, s) {
                Ok(()) => true,
                Err(e) => { eprintln!("config save error: {}", e); false }
            }
        } else {
            false
        }
    }
}
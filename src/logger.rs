use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// 简易环形日志：追加文件 + 内存环形缓存（供 WebUI 实时查看）
pub struct Logger {
    file: Mutex<fs::File>,
    mem: Mutex<Vec<String>>,
    cap: usize,
}

impl Logger {
    pub fn new(path: &str) -> Logger {
        if let Some(p) = std::path::Path::new(path).parent() {
            let _ = fs::create_dir_all(p);
        }
        let file = OpenOptions::new().create(true).append(true).open(path)
            .expect("cannot open log file");
        Logger { file: Mutex::new(file), mem: Mutex::new(Vec::new()), cap: 600 }
    }

    pub fn log(&self, level: &str, msg: &str) {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs()).unwrap_or(0);
        let line = format!("{} [{}] {}", ts, level, msg);
        if let Ok(mut f) = self.file.lock() {
            let _ = writeln!(f, "{}", line);
        }
        if let Ok(mut m) = self.mem.lock() {
            m.push(line);
            if m.len() > self.cap {
                let over = m.len() - self.cap;
                m.drain(..over);
            }
        }
    }

    pub fn info(&self, msg: &str) { self.log("info", msg); }
    pub fn warn(&self, msg: &str) { self.log("warn", msg); }
    pub fn error(&self, msg: &str) { self.log("error", msg); }

    pub fn recent(&self, n: usize) -> Vec<String> {
        match self.mem.lock() {
            Ok(m) => {
                let from = if m.len() > n { m.len() - n } else { 0 };
                m[from..].to_vec()
            }
            Err(_) => Vec::new(),
        }
    }
}
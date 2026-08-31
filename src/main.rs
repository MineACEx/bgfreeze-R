mod config;
mod engine;
mod httpd;
mod logger;
mod sys;

use std::sync::Arc;

fn arg(args: &[String], key: &str) -> Option<String> {
    args.iter().position(|a| a == key).map(|i| args.get(i + 1).cloned()).flatten()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cfg_path = arg(&args, "--config").unwrap_or_else(|| "/data/adb/bgfreeze/config.json".into());
    let webroot = arg(&args, "--webroot").unwrap_or_else(|| "/data/adb/modules/bgfreeze/webroot".into());
    let port = arg(&args, "--port").and_then(|p| p.parse().ok()).unwrap_or(8765);

    let log = Arc::new(logger::Logger::new("/data/adb/bgfreeze/logs/bgfreeze.log"));
    log.info("bgfreeze daemon starting v1.0.0");

    let mut cfg = config::Config::load(&cfg_path);
    cfg.port = port;
    let saved = cfg.save(&cfg_path);
    log.info(&format!("config loaded from {}, saved: {}", cfg_path, saved));

    let engine = engine::Engine::new(cfg_path.clone(), cfg, log.clone());
    let engine_loop = engine.clone();
    std::thread::spawn(move || engine_loop.run_loop());

    httpd::serve(engine, log, webroot, port);
}
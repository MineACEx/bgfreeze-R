use crate::config::Config;
use crate::engine::Engine;
use crate::logger::Logger;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

pub fn serve(engine: Arc<Engine>, log: Arc<Logger>, webroot: String, port: u16) {
    let addr = format!("127.0.0.1:{}", port);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            log.error(&format!("web server bind {} failed: {}", addr, e));
            return;
        }
    };
    log.info(&format!("web UI 监听 http://{}/", addr));
    for conn in listener.incoming() {
        if let Ok(mut stream) = conn {
            let e = engine.clone();
            let l = log.clone();
            let w = webroot.clone();
            std::thread::spawn(move || {
                let _ = handle(&mut stream, &e, &l, &w);
            });
        }
    }
}

struct Req {
    method: String,
    path: String,
    query: String,
    body: Vec<u8>,
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn read_request(stream: &mut TcpStream) -> Option<Req> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let mut buf = Vec::with_capacity(8192);
    let mut chunk = [0u8; 8192];
    let mut header_end: Option<usize> = None;
    while buf.len() < 512 * 1024 {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if let Some(pos) = find_header_end(&buf) {
                    header_end = Some(pos);
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let end = header_end?;
    let header = String::from_utf8_lossy(&buf[..end]).to_string();
    let mut lines = header.lines();
    let req_line = lines.next()?.trim().to_string();
    let mut parts = req_line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?.to_string();
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target.clone(), String::new()),
    };
    let content_len: usize = lines
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            if k.trim().eq_ignore_ascii_case("content-length") {
                v.trim().parse().ok()
            } else {
                None
            }
        })
        .next()
        .unwrap_or(0);
    let mut body = buf[end + 4..].to_vec();
    while body.len() < content_len {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => body.extend_from_slice(&chunk[..n]),
            Err(_) => break,
        }
    }
    Some(Req { method, path, query, body })
}

fn write_resp(stream: &mut TcpStream, code: u16, ctype: &str, body: &[u8]) -> std::io::Result<()> {
    let reason = match code {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "OK",
    };
    stream.write_all(
        format!(
            "HTTP/1.1 {} {}\r\nAccess-Control-Allow-Origin: *\r\nContent-Type: {}; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            code, reason, ctype, body.len()
        )
        .as_bytes(),
    )?;
    stream.write_all(body)?;
    stream.flush()
}

fn handle(stream: &mut TcpStream, engine: &Arc<Engine>, log: &Arc<Logger>, webroot: &str) -> std::io::Result<()> {
    let Some(req) = read_request(stream) else {
        let _ = write_resp(stream, 400, "text/plain", b"bad request");
        return Ok(());
    };

    if req.method == "OPTIONS" {
        stream.write_all(
            b"HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET,POST,OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nAccess-Control-Max-Age: 86400\r\n\r\n",
        )?;
        return Ok(());
    }

    if req.path.starts_with("/api/") {
        handle_api(stream, &req, engine, log);
    } else {
        handle_static(stream, &req, webroot);
    }
    Ok(())
}

fn handle_api(stream: &mut TcpStream, req: &Req, engine: &Arc<Engine>, log: &Arc<Logger>) {
    match req.path.as_str() {
        "/api/status" => {
            let st = engine.status_snapshot();
            let body = serde_json::to_string(&st).unwrap_or_else(|_| "{}".into());
            let _ = write_resp(stream, 200, "application/json", body.as_bytes());
        }
        "/api/logs" => {
            let n: usize = req
                .query
                .split('&')
                .find_map(|kv| kv.strip_prefix("n="))
                .and_then(|v| v.parse().ok())
                .unwrap_or(200)
                .clamp(1, 1000);
            let logs = log.recent(n.max(200));
            let body = serde_json::to_string(&logs).unwrap_or_else(|_| "[]".into());
            let _ = write_resp(stream, 200, "application/json", body.as_bytes());
        }
        "/api/config" => match req.method.as_str() {
            "GET" => {
                let cfg = engine.config.read().unwrap().clone();
                let body = serde_json::to_string_pretty(&cfg).unwrap_or_else(|_| "{}".into());
                let _ = write_resp(stream, 200, "application/json", body.as_bytes());
            }
            "POST" => match serde_json::from_slice::<Config>(&req.body) {
                Ok(mut new_cfg) => {
                    // 端口由启动参数决定，不随配置改动
                    let port = engine.config.read().unwrap().port;
                    new_cfg.port = port;
                    let saved = new_cfg.save(&engine.cfg_path);

                    {
                        let mut c = engine.config.write().unwrap();
                        *c = new_cfg;
                    }

                        engine.publish_meta();
                    log.info("配置已通过 WebUI 更新");
                    let _ = write_resp(
                        stream,
                        200,
                        "application/json",
                        format!("{{\"ok\":true,\"saved\":{}}}", saved).as_bytes(),
                    );
                }
                Err(e) => {
                    let _ = write_resp(
                        stream,
                        400,
                        "application/json",
                        format!("{{\"ok\":false,\"error\":\"json: {}\"}}", e).as_bytes(),
                    );
                }
            },
            _ => {
                let _ = write_resp(stream, 405, "text/plain", b"method not allowed");
            }
        },
        "/api/action" => {
            let body_str = String::from_utf8_lossy(&req.body).to_string();
            if body_str.is_empty() {
                let _ = write_resp(stream, 400, "application/json", b"{\"ok\":false,\"error\":\"empty body\"}");
                return;
            }
            let v: serde_json::Value = serde_json::from_str(&body_str).unwrap_or(serde_json::Value::Null);
            let action = v.get("action").and_then(|a| a.as_str()).unwrap_or("");
            let pkg = v.get("package").and_then(|a| a.as_str()).unwrap_or("");
            match action {
                "unfreeze_all" => {
                    engine.unfreeze_all();
                    let _ = write_resp(stream, 200, "application/json", b"{\"ok\":true}");
                }
                "freeze_all" => {
                    engine.cycle_now();
                    let _ = write_resp(stream, 200, "application/json", b"{\"ok\":true}");
                }
                "unfreeze_app" => {
                    engine.unfreeze_app(pkg);
                    let _ = write_resp(stream, 200, "application/json", b"{\"ok\":true}");
                }
                "freeze_app" => {
                    engine.freeze_app(pkg);
                    let _ = write_resp(stream, 200, "application/json", b"{\"ok\":true}");
                }
                "update" => {
                    let url = v.get("url").and_then(|a| a.as_str()).unwrap_or("");
                    let file = v.get("file").and_then(|a| a.as_str()).unwrap_or("bgfreeze-R-update.zip");
                    if url.is_empty() {
                        let _ = write_resp(stream, 400, "application/json", b"{\"ok\":false,\"error\":\"no url\"}");
                        return;
                    }
                    let cmd = format!(
                        "f=/sdcard/Download/{}; (command -v curl >/dev/null 2>&1 && curl -ksSL -o \"$f\" '{}') || (command -v wget >/dev/null 2>&1 && wget -q -O \"$f\" '{}'); ls -l \"$f\"; /data/adb/ksud module install \"$f\" 2>&1 | tail -3",
                        file, url, url
                    );
                    let out = std::process::Command::new("sh").arg("-c").arg(&cmd).output();
                    match out {
                        Ok(o) => {
                            let msg = format!("out:{}\nerr:{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
                            let body = format!("{{\"ok\":true,\"out\":{}}}", serde_json::to_string(&msg).unwrap_or_else(|_| "\"\"".into()));
                            let _ = write_resp(stream, 200, "application/json", body.as_bytes());
                        }
                        Err(_) => {
                            let _ = write_resp(stream, 500, "application/json", b"{\"ok\":false,\"error\":\"spawn\"}");
                        }
                    }
                }
                "uninstall" => {
                    let st = std::process::Command::new("/data/adb/ksud")
                        .args(["module", "uninstall", "bgfreeze-R"]).status();
                    if st.is_ok() {
                        let _ = write_resp(stream, 200, "application/json", b"{\"ok\":true}");
                    } else {
                        let _ = write_resp(stream, 500, "application/json", b"{\"ok\":false,\"error\":\"run\"}");
                    }
                }
                "reboot" => {
                    let st = std::process::Command::new("reboot").status();
                    let _ = write_resp(stream, 200, "application/json", if st.is_ok() { b"{\"ok\":true}" } else { b"{\"ok\":false}" });
                }
                _ => {
                    let _ = write_resp(stream, 400, "application/json", b"{\"ok\":false,\"error\":\"unknown action\"}");
                }
            }
        }
        _ => {
            let _ = write_resp(stream, 404, "text/plain", b"not found");
        }
    }
}

fn handle_static(stream: &mut TcpStream, req: &Req, webroot: &str) {
    let rel = if req.path == "/" {
        "index.html"
    } else {
        req.path.trim_start_matches('/')
    };
    let mut full = std::path::PathBuf::from(webroot);
    for seg in rel.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." {
            continue;
        }
        full.push(seg);
    }
    if !full.starts_with(Path::new(webroot)) {
        let _ = write_resp(stream, 403, "text/plain", b"forbidden");
        return;
    }
    match std::fs::read(&full) {
        Ok(data) => {
            let ctype = mime_of(&full);
            let _ = write_resp(stream, 200, ctype, &data);
        }
        Err(_) => {
            let _ = write_resp(stream, 404, "text/plain", b"not found");
        }
    }
}

fn mime_of(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "json" => "application/json",
        "woff2" => "font/woff2",
        "jpg" | "jpeg" => "image/jpeg",
        _ => "application/octet-stream",
    }
}

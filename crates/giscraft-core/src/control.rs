//! JSON-lines control server for the running app (127.0.0.1 only).
//! Mirrors the storytold artcraft pattern (`designcraft --control PORT`):
//! one request object per line, one reply per line. Framing and validation
//! live here; command execution lives in the app (it owns the DB + view).
//! Same safety contract: only requests are read; anything that is not a
//! JSON object with a string `method` gets one error reply and the
//! connection is closed (so a browser `fetch` to the port cannot smuggle
//! commands in). 4 MiB line cap, 16 concurrent connections, no auth:
//! only enable it while you use it.

use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

pub const MAX_LINE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_CONNS: usize = 16;
const REPLY_TIMEOUT: Duration = Duration::from_secs(30);

/// A validated request waiting on the app. The app executes it on its own
/// (egui) thread and answers through `respond`.
pub struct ControlRequest {
    pub id: Value,
    pub method: String,
    pub params: Value,
    pub respond: Sender<Value>,
}

/// Validate one input line. Returns `(id, method, params)` or the error
/// string for the single error reply. Blank lines are skipped by the caller.
pub fn validate_line(line: &str) -> Result<(Value, String, Value), String> {
    if line.len() > MAX_LINE_BYTES {
        return Err("line exceeds 4 MiB: closing the connection".to_string());
    }
    let req: Value =
        serde_json::from_str(line).map_err(|_| "not JSON: closing the connection".to_string())?;
    if !req.is_object() {
        return Err("not an object: closing the connection".to_string());
    }
    let method = match req.get("method").and_then(|m| m.as_str()) {
        Some(m) => m.to_string(),
        None => return Err("missing method: closing the connection".to_string()),
    };
    if method.trim().is_empty() {
        return Err("empty method: closing the connection".to_string());
    }
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    Ok((id, method, params))
}

/// Blocking accept loop. Each connection gets a thread; past MAX_CONNS the
/// socket is answered with an error line and closed. Counts live handlers
/// so slots free up when threads exit.
pub fn serve(listener: TcpListener, app_tx: Sender<ControlRequest>) {
    let live = Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(_) => break,
        };
        if live.fetch_add(1, Ordering::SeqCst) >= MAX_CONNS {
            live.fetch_sub(1, Ordering::SeqCst);
            let _ = write_error(&stream, &Value::Null, "too many connections");
            continue;
        }
        let tx = app_tx.clone();
        let live = Arc::clone(&live);
        std::thread::spawn(move || {
            handle_conn(stream, tx);
            live.fetch_sub(1, Ordering::SeqCst);
        });
    }
}

fn write_error(stream: &TcpStream, id: &Value, msg: &str) -> std::io::Result<()> {
    let mut stream = stream.try_clone()?;
    let resp = serde_json::json!({"id": id, "ok": false, "error": msg});
    if let Ok(text) = serde_json::to_string(&resp) {
        let _ = writeln!(stream, "{text}");
        let _ = stream.flush();
    }
    Ok(())
}

fn handle_conn(stream: TcpStream, app_tx: Sender<ControlRequest>) {
    let reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });
    let mut writer = stream;
    let _ = writer.set_write_timeout(Some(Duration::from_secs(10)));
    for raw in reader.lines() {
        let line = match raw {
            Ok(line) => line,
            // Invalid UTF-8 (or I/O error): error + close, per contract.
            Err(_) => {
                let resp = serde_json::json!({"id": Value::Null, "ok": false, "error": "invalid UTF-8: closing the connection"});
                if let Ok(text) = serde_json::to_string(&resp) {
                    let _ = writeln!(writer, "{text}");
                    let _ = writer.flush();
                }
                break;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        if line.len() > MAX_LINE_BYTES {
            let resp = serde_json::json!({"id": Value::Null, "ok": false, "error": "line exceeds 4 MiB: closing the connection"});
            if let Ok(text) = serde_json::to_string(&resp) {
                let _ = writeln!(writer, "{text}");
                let _ = writer.flush();
            }
            break;
        }
        let (id, method, params) = match validate_line(&line) {
            Ok(v) => v,
            Err(e) => {
                let resp = serde_json::json!({"id": Value::Null, "ok": false, "error": e});
                if let Ok(text) = serde_json::to_string(&resp) {
                    let _ = writeln!(writer, "{text}");
                    let _ = writer.flush();
                }
                break;
            }
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let req = ControlRequest {
            id: id.clone(),
            method,
            params,
            respond: tx,
        };
        if app_tx.send(req).is_err() {
            let resp = serde_json::json!({"id": id, "ok": false, "error": "app gone"});
            if let Ok(text) = serde_json::to_string(&resp) {
                let _ = writeln!(writer, "{text}");
                let _ = writer.flush();
            }
            break;
        }
        match rx.recv_timeout(REPLY_TIMEOUT) {
            Ok(reply) => {
                let resp = serde_json::json!({"id": id, "ok": true, "result": reply});
                if let Ok(text) = serde_json::to_string(&resp) {
                    if writeln!(writer, "{text}").is_err() {
                        break;
                    }
                    let _ = writer.flush();
                }
            }
            Err(_) => {
                let resp = serde_json::json!({"id": id, "ok": false, "error": "app timeout"});
                if let Ok(text) = serde_json::to_string(&resp) {
                    let _ = writeln!(writer, "{text}");
                    let _ = writer.flush();
                }
            }
        }
    }
}

/// One-shot client: send a single request, read a single reply.
/// Used by `mcp --connect` and tests. Pure framing, no app involved.
pub fn roundtrip(port: u16, id: &Value, method: &str, params: &Value) -> Result<Value, String> {
    let addr = format!("127.0.0.1:{port}");
    let stream =
        TcpStream::connect(addr.as_str()).map_err(|e| format!("control unreachable: {e}"))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
    let req = serde_json::json!({"id": id, "method": method, "params": params});
    let text = serde_json::to_string(&req).map_err(|e| format!("encode: {e}"))?;
    let mut writer = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => return Err(format!("control unreachable: {e}")),
    };
    writeln!(writer, "{text}").map_err(|e| format!("control write: {e}"))?;
    let _ = writer.flush();
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("control read: {e}"))?;
    serde_json::from_str(line.trim()).map_err(|e| format!("control parse: {e}"))
}

/// Drain pending control requests without blocking (called every frame).
pub fn drain(rx: &Receiver<ControlRequest>, mut exec: impl FnMut(ControlRequest)) {
    while let Ok(req) = rx.try_recv() {
        exec(req);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn valid_line_parses() {
        let (id, method, params) = validate_line(
            r#"{"id":1,"method":"ui.inspect","params":{"a":1}}"#,
        )
        .unwrap_or((Value::Null, String::new(), Value::Null));
        assert_eq!(method, "ui.inspect");
        assert_eq!(id, serde_json::json!(1));
        assert!(params.get("a").is_some());
    }

    #[test]
    fn hostile_lines_rejected() {
        assert!(validate_line("[1,2]").is_err());
        assert!(validate_line("hello").is_err());
        assert!(validate_line(r#"{"id":1}"#).is_err());
        assert!(validate_line(r#"{"id":1,"method":""}"#).is_err());
        assert!(validate_line(r#"{"id":1,"method":42}"#).is_err());
    }

    #[test]
    fn live_tcp_roundtrip_with_stub_executor() {
        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(l) => l,
            // Ephemeral bind basically never fails; skip (pass) if sandboxed.
            Err(_) => return,
        };
        let port = match listener.local_addr() {
            Ok(a) => a.port(),
            Err(_) => return,
        };
        assert!(port != 0);
        let (tx, rx) = channel::<ControlRequest>();
        std::thread::spawn(move || {
            // Stub executor: echo params back as the result.
            for req in rx {
                let _ = req.respond.send(req.params.clone());
            }
        });
        std::thread::spawn(move || serve(listener, tx));
        // Give the accept loop a moment to start.
        std::thread::sleep(Duration::from_millis(200));
        let reply = roundtrip(
            port,
            &serde_json::json!(7),
            "ui.inspect",
            &serde_json::json!({"x": 1}),
        );
        assert!(reply.is_ok());
        if let Ok(reply) = reply {
            assert_eq!(reply.get("ok").and_then(|v| v.as_bool()), Some(true));
            assert_eq!(reply.get("id"), Some(&serde_json::json!(7)));
        }
        // Garbage closes the connection: reply is an error either way.
        let raw = std::net::TcpStream::connect(format!("127.0.0.1:{port}"));
        if let Ok(mut s) = raw {
            use std::io::Write as _;
            let _ = writeln!(s, "this is not json");
            let mut reader = BufReader::new(s);
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() {
                let v: Value = serde_json::from_str(line.trim()).unwrap_or(Value::Null);
                assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(false));
            }
        }
    }
}

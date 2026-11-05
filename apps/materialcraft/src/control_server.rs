//! Loopback JSON-lines control server (`--control <port>`): one request per line,
//! `{"id", "method", "params"}`, one reply per line. The first request must be
//! `{"method":"auth","params":{"token":"…"}}` with the token from `MATERIALCRAFT_CONTROL_TOKEN`
//! or, when that is unset, the token written to `<temp>/materialcraft-control-<port>.token`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use materialcraft_ui_egui::ControlRequest;
use serde_json::{Value, json};

/// Requests larger than this are rejected (screenshots go the other way).
const MAX_REQUEST: u64 = 8 << 20;

#[cfg(test)]
#[path = "control_server.test.rs"]
mod tests;

/// The token comes from the OS random generator and is written owner-only (shared with the CLI's
/// `serve`). Without a token the server does not start.
fn token(port: u16) -> Option<String> {
    match materialcraft_automation::security::server_token(port) {
        Ok((token, path)) => {
            if let Some(path) = path {
                eprintln!("materialcraft: control token written to {}", path.display());
            }
            Some(token)
        }
        Err(e) => {
            eprintln!("materialcraft: control server not started: {e}");
            None
        }
    }
}

/// Whether a first request authenticates: an `auth` call carrying exactly the server's token,
/// compared without an early exit.
fn auth_ok(token: &str, method: &str, params: &Value) -> bool {
    method == "auth" && params.get("token").and_then(Value::as_str).is_some_and(|t| materialcraft_automation::security::token_matches(token, t))
}

pub fn start(port: u16, ctx: egui::Context) -> Receiver<ControlRequest> {
    let (tx, rx) = channel::<ControlRequest>();
    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("materialcraft: control server failed to bind 127.0.0.1:{port}: {e}");
            return rx;
        }
    };
    let Some(token) = token(port) else { return rx };
    eprintln!("materialcraft: control server listening on 127.0.0.1:{port}");
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (tx, ctx, token) = (tx.clone(), ctx.clone(), token.clone());
            std::thread::spawn(move || serve(stream, &token, tx, ctx));
        }
    });
    rx
}

fn serve(stream: TcpStream, token: &str, tx: Sender<ControlRequest>, ctx: egui::Context) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(600)));
    let Ok(read) = stream.try_clone() else { return };
    let mut reader = BufReader::new(read.take(u64::MAX));
    let mut out = stream;
    let mut authed = false;
    loop {
        let mut line = String::new();
        let n = match reader.by_ref().take(MAX_REQUEST).read_line(&mut line) {
            Ok(n) => n,
            Err(_) => break,
        };
        if n == 0 {
            break;
        }
        if !line.ends_with('\n') && n as u64 >= MAX_REQUEST {
            let _ = writeln!(out, "{}", json!({"id": null, "ok": false, "error": "request too large"}));
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let _ = writeln!(out, "{}", json!({"id": null, "ok": false, "error": format!("bad JSON: {e}")}));
                continue;
            }
        };
        let id = msg.get("id").cloned().unwrap_or(Value::Null);
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("").to_string();
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        let mut reply = if !authed {
            let ok = auth_ok(token, &method, &params);
            authed = ok;
            if ok { json!({"ok": true}) } else { json!({"ok": false, "error": "authenticate first: {\"method\":\"auth\",\"params\":{\"token\":…}}"}) }
        } else {
            let (req, rrx) = ControlRequest::new(method, params);
            if tx.send(req).is_err() {
                break;
            }
            ctx.request_repaint();
            rrx.recv_timeout(Duration::from_secs(120)).unwrap_or_else(|_| json!({"ok": false, "error": "timeout"}))
        };
        if let Some(o) = reply.as_object_mut() {
            o.insert("id".into(), id);
        }
        // A failed authentication closes the connection, so guesses cost a reconnect each.
        if writeln!(out, "{reply}").is_err() || out.flush().is_err() || !authed {
            break;
        }
    }
}

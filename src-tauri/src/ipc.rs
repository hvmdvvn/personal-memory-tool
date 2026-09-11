//! Authenticated localhost IPC for the browser extension (issues #11–#12).

use crate::browser_capture::{save_browser_capture, BrowserCapturePayload};
use crate::db::Database;
use serde_json::json;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// Loopback port for extension ↔ desktop IPC.
pub const IPC_PORT: u16 = 17832;

/// Shared local token (dev). Must match `extension/` client.
pub const IPC_TOKEN: &str = "personal-memory-local-dev-token";

pub const TOKEN_HEADER: &str = "x-memory-token";

static SERVER_STARTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcAuth {
    Ok,
    Unauthorized,
}

/// Validate the shared token (handler unit-tested without binding a port).
pub fn authorize_token(provided: Option<&str>) -> IpcAuth {
    match provided {
        Some(t) if t == IPC_TOKEN => IpcAuth::Ok,
        _ => IpcAuth::Unauthorized,
    }
}

/// Build JSON body for a successful ping.
pub fn ping_ack_body() -> String {
    json!({ "ok": true, "pong": true, "service": "personal-memory" }).to_string()
}

fn json_response(status: u16, body: String) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    tiny_http::Response::from_string(body)
        .with_status_code(status)
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
        )
        .with_header(
            tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
        )
}

fn cors_preflight() -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    tiny_http::Response::from_string("")
        .with_status_code(204)
        .with_header(
            tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
        )
        .with_header(
            tiny_http::Header::from_bytes(
                &b"Access-Control-Allow-Headers"[..],
                &b"content-type, x-memory-token"[..],
            )
            .unwrap(),
        )
        .with_header(
            tiny_http::Header::from_bytes(
                &b"Access-Control-Allow-Methods"[..],
                &b"POST, OPTIONS"[..],
            )
            .unwrap(),
        )
}

fn handle_capture_post(app_data_dir: &PathBuf, body: &str) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let payload: BrowserCapturePayload = match serde_json::from_str(body) {
        Ok(p) => p,
        Err(e) => {
            return json_response(
                400,
                json!({ "ok": false, "error": format!("invalid_json: {e}") }).to_string(),
            );
        }
    };

    let db_path = app_data_dir.join("memory.sqlite");
    let db = match Database::open(&db_path) {
        Ok(db) => db,
        Err(e) => {
            return json_response(
                500,
                json!({ "ok": false, "error": format!("open_db: {e}") }).to_string(),
            );
        }
    };

    match save_browser_capture(&db, &payload) {
        Ok(id) => json_response(200, json!({ "ok": true, "id": id }).to_string()),
        Err(e) => json_response(500, json!({ "ok": false, "error": e.to_string() }).to_string()),
    }
}

/// Start the loopback IPC server once (background thread).
pub fn start_extension_ipc_server(app_data_dir: PathBuf) -> Result<(), String> {
    if SERVER_STARTED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    std::fs::create_dir_all(&app_data_dir).map_err(|e| format!("create app data dir: {e}"))?;

    let addr: SocketAddr = format!("127.0.0.1:{IPC_PORT}")
        .parse()
        .map_err(|e| format!("bad addr: {e}"))?;

    let server = tiny_http::Server::http(addr).map_err(|e| format!("bind {addr}: {e}"))?;
    eprintln!("[ipc] extension IPC listening on http://127.0.0.1:{IPC_PORT}");

    thread::spawn(move || {
        for mut request in server.incoming_requests() {
            let method = request.method().as_str().to_string();
            let url = request.url().to_string();
            let token = request
                .headers()
                .iter()
                .find(|h| h.field.equiv(TOKEN_HEADER))
                .map(|h| h.value.as_str().to_string());

            let mut body = String::new();
            let _ = request.as_reader().read_to_string(&mut body);

            let path = url.split('?').next().unwrap_or(url.as_str());

            let response = if method == "OPTIONS" {
                cors_preflight()
            } else if authorize_token(token.as_deref()) == IpcAuth::Unauthorized {
                json_response(
                    401,
                    json!({ "ok": false, "error": "unauthorized" }).to_string(),
                )
            } else if method == "POST" && path == "/ping" {
                json_response(200, ping_ack_body())
            } else if method == "POST" && path == "/capture" {
                handle_capture_post(&app_data_dir, &body)
            } else {
                json_response(404, json!({ "ok": false, "error": "not_found" }).to_string())
            };

            let _ = request.respond(response);
        }
    });

    thread::sleep(Duration::from_millis(20));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn authorize_accepts_matching_token() {
        assert_eq!(authorize_token(Some(IPC_TOKEN)), IpcAuth::Ok);
    }

    #[test]
    fn authorize_rejects_missing_or_wrong_token() {
        assert_eq!(authorize_token(None), IpcAuth::Unauthorized);
        assert_eq!(authorize_token(Some("nope")), IpcAuth::Unauthorized);
    }

    #[test]
    fn ping_ack_is_json_ok() {
        let body = ping_ack_body();
        assert!(body.contains("\"ok\":true") || body.contains("\"ok\": true"));
        assert!(body.contains("pong"));
    }

    #[test]
    fn localhost_ping_and_capture_round_trip() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pm_ipc_{nanos}"));
        start_extension_ipc_server(dir.clone()).expect("start");
        thread::sleep(Duration::from_millis(50));

        let mut stream =
            std::net::TcpStream::connect(("127.0.0.1", IPC_PORT)).expect("connect");
        let req = format!(
            "POST /ping HTTP/1.1\r\nHost: 127.0.0.1\r\n{TOKEN_HEADER}: {IPC_TOKEN}\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}",
        );
        stream.write_all(req.as_bytes()).unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).unwrap();
        assert!(resp.contains("200"));
        assert!(resp.contains("pong"));

        let mut stream =
            std::net::TcpStream::connect(("127.0.0.1", IPC_PORT)).expect("connect");
        let bad = format!(
            "POST /capture HTTP/1.1\r\nHost: 127.0.0.1\r\n{TOKEN_HEADER}: bad\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}",
        );
        stream.write_all(bad.as_bytes()).unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).unwrap();
        assert!(resp.contains("401"));

        let payload = json!({
            "url": "https://example.com/x",
            "title": "Example",
            "selection": "hello from browser"
        })
        .to_string();
        let mut stream =
            std::net::TcpStream::connect(("127.0.0.1", IPC_PORT)).expect("connect");
        let cap = format!(
            "POST /capture HTTP/1.1\r\nHost: 127.0.0.1\r\n{TOKEN_HEADER}: {IPC_TOKEN}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
            payload.len()
        );
        stream.write_all(cap.as_bytes()).unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).unwrap();
        assert!(resp.contains("200"), "{resp}");
        assert!(resp.contains("\"ok\":true") || resp.contains("\"ok\": true"));

        let db = Database::open(dir.join("memory.sqlite")).unwrap();
        let items = db.list_recent_captures(10).unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].snippet.contains("hello from browser"));
        assert_eq!(items[0].source_kind.as_deref(), Some("browser"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}

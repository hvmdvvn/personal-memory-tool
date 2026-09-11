//! Authenticated localhost IPC for the browser extension (issue #11).

use serde_json::json;
use std::io::{Read, Write};
use std::net::SocketAddr;
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

/// Start the loopback IPC server once (background thread).
pub fn start_extension_ipc_server() -> Result<(), String> {
    if SERVER_STARTED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

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

            // Drain body (ignore content for ping).
            let mut body = String::new();
            let _ = request.as_reader().read_to_string(&mut body);

            let response = if method == "OPTIONS" {
                tiny_http::Response::from_string("")
                    .with_status_code(204)
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Access-Control-Allow-Origin"[..],
                            &b"*"[..],
                        )
                        .unwrap(),
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
            } else if method == "POST" && (url == "/ping" || url.starts_with("/ping?")) {
                match authorize_token(token.as_deref()) {
                    IpcAuth::Ok => tiny_http::Response::from_string(ping_ack_body())
                        .with_header(
                            tiny_http::Header::from_bytes(
                                &b"Content-Type"[..],
                                &b"application/json"[..],
                            )
                            .unwrap(),
                        )
                        .with_header(
                            tiny_http::Header::from_bytes(
                                &b"Access-Control-Allow-Origin"[..],
                                &b"*"[..],
                            )
                            .unwrap(),
                        ),
                    IpcAuth::Unauthorized => tiny_http::Response::from_string(
                        json!({ "ok": false, "error": "unauthorized" }).to_string(),
                    )
                    .with_status_code(401)
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Content-Type"[..],
                            &b"application/json"[..],
                        )
                        .unwrap(),
                    ),
                }
            } else {
                tiny_http::Response::from_string(
                    json!({ "ok": false, "error": "not_found" }).to_string(),
                )
                .with_status_code(404)
                .with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                        .unwrap(),
                )
            };

            let _ = request.respond(response);
        }
    });

    // Brief pause so bind is visible in logs before continuing startup.
    thread::sleep(Duration::from_millis(20));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn localhost_ping_round_trip() {
        start_extension_ipc_server().expect("start");
        // Give the thread a moment.
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
            "POST /ping HTTP/1.1\r\nHost: 127.0.0.1\r\n{TOKEN_HEADER}: bad\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}",
        );
        stream.write_all(bad.as_bytes()).unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).unwrap();
        assert!(resp.contains("401"));
    }
}

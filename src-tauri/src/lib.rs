pub mod browser_capture;
pub mod capture;
pub mod classify;
pub mod db;
pub mod embeddings;
pub mod enrich;
pub mod ipc;
pub mod media;
pub mod ollama;
pub mod orchestrate;
pub mod search;
pub mod semantic;
pub mod shortcut;
pub mod window_context;

use db::Database;
use orchestrate::{CaptureNowOptions, DEFAULT_SCREENSHOT_ON_EMPTY_CLIPBOARD};
use std::path::PathBuf;
use tauri::Manager;

fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("create app data dir: {e}"))?;
    Ok(dir)
}

fn open_app_db(app: &tauri::AppHandle) -> Result<(PathBuf, Database), String> {
    let dir = app_data_dir(app)?;
    let db_path = dir.join("memory.sqlite");
    let db = Database::open(&db_path).map_err(|e| format!("open db: {e}"))?;
    Ok((dir, db))
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Read clipboard text and store as an immutable raw capture. Returns the new id.
#[tauri::command]
fn capture_clipboard(app: tauri::AppHandle) -> Result<String, String> {
    let (_dir, db) = open_app_db(&app)?;
    capture::capture_clipboard_to_db(&db).map_err(|e| e.to_string())
}

/// Grab a primary-monitor screenshot, store PNG under app data `media/`, insert raw capture.
#[tauri::command]
fn capture_screenshot(app: tauri::AppHandle) -> Result<String, String> {
    let (dir, db) = open_app_db(&app)?;
    media::capture_screenshot_to_db(&db, &dir).map_err(|e| e.to_string())
}

/// Global-shortcut capture: clipboard preferred; else fallback (± screenshot).
/// `screenshot_on_empty` defaults to true when omitted from JS as `null` is not used—
/// pass `true`/`false` explicitly; command default matches `DEFAULT_SCREENSHOT_ON_EMPTY_CLIPBOARD`.
#[tauri::command]
fn capture_now(app: tauri::AppHandle, screenshot_on_empty: Option<bool>) -> Result<String, String> {
    let (dir, db) = open_app_db(&app)?;
    let opts = CaptureNowOptions {
        screenshot_on_empty_clipboard: screenshot_on_empty
            .unwrap_or(DEFAULT_SCREENSHOT_ON_EMPTY_CLIPBOARD),
    };
    orchestrate::capture_now(&db, &dir, &opts).map_err(|e| e.to_string())
}

/// Recent captures for the Inbox UI (newest first).
#[tauri::command]
fn list_recent_captures(app: tauri::AppHandle, limit: Option<i64>) -> Result<Vec<db::CaptureSummary>, String> {
    let (_dir, db) = open_app_db(&app)?;
    db.list_recent_captures(limit.unwrap_or(50))
        .map_err(|e| e.to_string())
}

/// Check local Ollama HTTP API reachability and list model names.
#[tauri::command]
fn ollama_health() -> ollama::OllamaHealth {
    ollama::check_ollama_health(&ollama::ollama_base_url())
}

/// Generate and store an embedding for one capture's original text.
#[tauri::command]
fn embed_capture(app: tauri::AppHandle, capture_id: String) -> Result<String, String> {
    let (_dir, db) = open_app_db(&app)?;
    embeddings::embed_capture(
        &db,
        &capture_id,
        &ollama::ollama_base_url(),
        embeddings::DEFAULT_EMBED_MODEL,
    )
    .map_err(|e| e.to_string())
}

/// Semantic nearest-neighbor search over stored embeddings.
#[tauri::command]
fn search_semantic(
    app: tauri::AppHandle,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<semantic::SemanticHit>, String> {
    let (_dir, db) = open_app_db(&app)?;
    semantic::search_semantic(&db, &query, limit.unwrap_or(10), &ollama::ollama_base_url())
        .map_err(|e| e.to_string())
}

/// Classify one capture's type via local Ollama; writes AI metadata only.
#[tauri::command]
fn classify_capture(
    app: tauri::AppHandle,
    capture_id: String,
) -> Result<db::AiClassification, String> {
    let (_dir, db) = open_app_db(&app)?;
    classify::classify_capture(
        &db,
        &capture_id,
        &ollama::ollama_base_url(),
        classify::DEFAULT_CLASSIFY_MODEL,
    )
    .map_err(|e| e.to_string())
}

/// Enrich one capture with topics/keywords/entities/short description via Ollama.
#[tauri::command]
fn enrich_capture(
    app: tauri::AppHandle,
    capture_id: String,
) -> Result<db::AiEnrichment, String> {
    let (_dir, db) = open_app_db(&app)?;
    enrich::enrich_capture(
        &db,
        &capture_id,
        &ollama::ollama_base_url(),
        enrich::DEFAULT_ENRICH_MODEL,
    )
    .map_err(|e| e.to_string())
}

/// Unified exact (FTS) + semantic search with match reasons.
#[tauri::command]
fn search_unified(
    app: tauri::AppHandle,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<search::UnifiedHit>, String> {
    let (_dir, db) = open_app_db(&app)?;
    search::search_unified(
        &db,
        &query,
        limit.unwrap_or(20),
        &ollama::ollama_base_url(),
    )
}

fn run_capture_now_from_shortcut(app: &tauri::AppHandle) -> Result<String, String> {
    let (dir, db) = open_app_db(app)?;
    let opts = CaptureNowOptions::default();
    orchestrate::capture_now(&db, &dir, &opts).map_err(|e| e.to_string())
}

/// Trivial health helper used to prove the Rust test harness is wired.
pub fn health_check() -> &'static str {
    "ok"
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            match app.path().app_data_dir() {
                Ok(dir) => {
                    if let Err(e) = ipc::start_extension_ipc_server(dir) {
                        eprintln!("[ipc] failed to start extension IPC: {e}");
                    }
                }
                Err(e) => eprintln!("[ipc] app data dir unavailable: {e}"),
            }
            #[cfg(desktop)]
            {
                use tauri::Emitter;
                use tauri_plugin_global_shortcut::{
                    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
                };

                let default = shortcut::default_global_shortcut();
                // Ctrl+Shift+Space — keep in sync with `shortcut::DEFAULT_GLOBAL_SHORTCUT`.
                let capture_shortcut =
                    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space);
                let capture_shortcut_reg = capture_shortcut;

                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |app, sc, event| {
                            if event.state() == ShortcutState::Pressed
                                && sc.matches(Modifiers::CONTROL | Modifiers::SHIFT, Code::Space)
                            {
                                match run_capture_now_from_shortcut(app) {
                                    Ok(id) => {
                                        eprintln!("[shortcut] capture_now ok id={id}");
                                        let _ = app.emit(
                                            shortcut::CAPTURE_SHORTCUT_EVENT,
                                            serde_json::json!({
                                                "status": "ok",
                                                "id": id,
                                            }),
                                        );
                                    }
                                    Err(e) => {
                                        eprintln!("[shortcut] capture_now failed: {e}");
                                        let _ = app.emit(
                                            shortcut::CAPTURE_SHORTCUT_EVENT,
                                            serde_json::json!({
                                                "status": "error",
                                                "error": e,
                                            }),
                                        );
                                    }
                                }
                            }
                        })
                        .build(),
                )?;

                match app.global_shortcut().register(capture_shortcut_reg) {
                    Ok(()) => {
                        eprintln!("[shortcut] registered global hotkey: {default}");
                    }
                    Err(e) => {
                        eprintln!(
                            "[shortcut] failed to register {default}: {e}. Another app may own it."
                        );
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            capture_clipboard,
            capture_screenshot,
            capture_now,
            list_recent_captures,
            ollama_health,
            embed_capture,
            search_semantic,
            classify_capture,
            enrich_capture,
            search_unified
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_check_returns_ok() {
        assert_eq!(health_check(), "ok");
    }

    #[test]
    fn arithmetic_smoke() {
        assert_eq!(2 + 2, 4);
    }
}

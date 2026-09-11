pub mod capture;
pub mod db;
pub mod media;
pub mod shortcut;
pub mod window_context;

use db::Database;
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Read clipboard text and store as an immutable raw capture. Returns the new id.
#[tauri::command]
fn capture_clipboard(app: tauri::AppHandle) -> Result<String, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("create app data dir: {e}"))?;
    let db_path = dir.join("memory.sqlite");
    let db = Database::open(&db_path).map_err(|e| format!("open db: {e}"))?;
    capture::capture_clipboard_to_db(&db).map_err(|e| e.to_string())
}

/// Grab a primary-monitor screenshot, store PNG under app data `media/`, insert raw capture.
#[tauri::command]
fn capture_screenshot(app: tauri::AppHandle) -> Result<String, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("create app data dir: {e}"))?;
    let db_path = dir.join("memory.sqlite");
    let db = Database::open(&db_path).map_err(|e| format!("open db: {e}"))?;
    media::capture_screenshot_to_db(&db, &dir).map_err(|e| e.to_string())
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
                                eprintln!(
                                    "[shortcut] {} ({})",
                                    shortcut::SHORTCUT_ACK,
                                    default
                                );
                                let _ = app.emit(
                                    shortcut::CAPTURE_SHORTCUT_EVENT,
                                    shortcut::SHORTCUT_ACK,
                                );
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
        .invoke_handler(tauri::generate_handler![greet, capture_clipboard, capture_screenshot])
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

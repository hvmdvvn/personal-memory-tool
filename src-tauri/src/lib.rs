pub mod db;
pub mod shortcut;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
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
        .invoke_handler(tauri::generate_handler![greet])
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

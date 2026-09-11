//! Global capture shortcut defaults and stub acknowledgement (issue #5).

/// Default global hotkey (Tauri / plugin string form).
/// Change this constant (or later config from issue #28) to remake the shortcut.
pub const DEFAULT_GLOBAL_SHORTCUT: &str = "Ctrl+Shift+Space";

/// Event name emitted when the global shortcut is pressed.
pub const CAPTURE_SHORTCUT_EVENT: &str = "capture-shortcut";

/// Fixed acknowledgement payload for the stub handler (no capture yet).
pub const SHORTCUT_ACK: &str = "capture-shortcut-ack";

/// Returns the configured default shortcut string.
pub fn default_global_shortcut() -> &'static str {
    DEFAULT_GLOBAL_SHORTCUT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shortcut_is_documented_non_empty() {
        let s = default_global_shortcut();
        assert!(!s.trim().is_empty());
        assert_eq!(s, "Ctrl+Shift+Space");
    }

    #[test]
    fn stub_ack_is_stable() {
        assert_eq!(SHORTCUT_ACK, "capture-shortcut-ack");
        assert_eq!(CAPTURE_SHORTCUT_EVENT, "capture-shortcut");
    }
}

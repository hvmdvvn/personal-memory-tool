//! Foreground window context for captures (issue #7).

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowContext {
    pub app: Option<String>,
    pub title: Option<String>,
}

impl WindowContext {
    pub fn is_empty(&self) -> bool {
        self.app.is_none() && self.title.is_none()
    }
}

/// Resolve the foreground window's app name and title.
/// Never fails the caller: on error or unsupported platform, returns empty context.
pub fn foreground_window_context() -> WindowContext {
    #[cfg(windows)]
    {
        windows_foreground_context().unwrap_or_default()
    }
    #[cfg(not(windows))]
    {
        WindowContext::default()
    }
}

#[cfg(windows)]
fn windows_foreground_context() -> Option<WindowContext> {
    use std::path::Path;
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, HWND, MAX_PATH};
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    };

    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }

        let title = {
            let len = GetWindowTextLengthW(hwnd);
            if len <= 0 {
                None
            } else {
                let mut buf = vec![0u16; (len as usize) + 1];
                let written = GetWindowTextW(hwnd, &mut buf);
                if written <= 0 {
                    None
                } else {
                    Some(String::from_utf16_lossy(&buf[..written as usize]))
                }
            }
        };

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return Some(WindowContext { app: None, title });
        }

        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let app = (|| {
            let mut buf = vec![0u16; MAX_PATH as usize];
            let mut size = buf.len() as u32;
            QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                PWSTR(buf.as_mut_ptr()),
                &mut size,
            )
            .ok()?;
            let path = String::from_utf16_lossy(&buf[..size as usize]);
            let name = Path::new(&path)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned());
            name
        })();
        let _ = CloseHandle(process);

        Some(WindowContext { app, title })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_context_helper() {
        assert!(WindowContext::default().is_empty());
        assert!(!WindowContext {
            app: Some("Code".into()),
            title: None
        }
        .is_empty());
    }

    #[test]
    fn foreground_resolver_does_not_panic() {
        let _ = foreground_window_context();
    }
}

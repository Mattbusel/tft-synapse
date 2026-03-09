//! System tray icon and hide-to-tray support.
//!
//! Creates a simple tray icon so the user can hide/show the overlay
//! without it cluttering the taskbar.
//!
// TODO: implement with tray-icon crate once API is stable across platforms.
// The stub below ensures the workspace compiles; replace with a real
// TrayIconBuilder implementation when ready.

pub enum TrayMessage {
    Show,
    Quit,
}

pub struct AppTray;

impl AppTray {
    pub fn new() -> Result<Self, String> {
        Ok(Self)
    }

    pub fn poll(&self) -> Option<TrayMessage> {
        None
    }
}

impl Default for AppTray {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_tray_new_ok() {
        let tray = AppTray::new();
        assert!(tray.is_ok());
    }

    #[test]
    fn test_app_tray_poll_returns_none() {
        let tray = AppTray::new().expect("tray init failed in test");
        assert!(tray.poll().is_none());
    }

    #[test]
    fn test_tray_message_show_variant() {
        let msg = TrayMessage::Show;
        let is_show = matches!(msg, TrayMessage::Show);
        assert!(is_show);
    }

    #[test]
    fn test_tray_message_quit_variant() {
        let msg = TrayMessage::Quit;
        let is_quit = matches!(msg, TrayMessage::Quit);
        assert!(is_quit);
    }
}

//! UI state: what the application is currently showing.

use tft_types::GameState;
use tft_advisor::Recommendation;
use crate::overlay::OverlayConfig;

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Polling,
    Disconnected,
    Manual,
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionStatus::Connected => write!(f, "Connected"),
            ConnectionStatus::Polling => write!(f, "Polling..."),
            ConnectionStatus::Disconnected => write!(f, "Disconnected"),
            ConnectionStatus::Manual => write!(f, "Manual Mode"),
        }
    }
}

/// The complete UI state, updated each frame from background channels.
#[derive(Debug, Default)]
pub struct UiState {
    pub connection_status: Option<ConnectionStatus>,
    pub game_state: Option<GameState>,
    pub recommendation: Option<Recommendation>,
    pub games_trained: u32,
    pub last_error: Option<String>,
    pub last_info: Option<String>,
    pub overlay_mode: bool,
    pub overlay_config: OverlayConfig,
    /// Set to true when overlay settings changed and need to be applied.
    pub overlay_dirty: bool,
}

impl UiState {
    pub fn new() -> Self { Self::default() }

    pub fn set_connected(&mut self, status: ConnectionStatus) {
        self.connection_status = Some(status);
    }

    pub fn clear_error(&mut self) {
        self.last_error = None;
    }

    pub fn has_recommendation(&self) -> bool {
        self.recommendation.is_some()
    }

    /// Toggle click-through overlay mode and mark dirty.
    pub fn toggle_click_through(&mut self) {
        self.overlay_config.toggle_click_through();
        self.overlay_dirty = true;
    }

    /// Set overlay opacity and mark dirty.
    pub fn set_opacity(&mut self, v: f32) {
        self.overlay_config.set_opacity(v);
        self.overlay_dirty = true;
    }

    /// Return the last info message, if any.
    pub fn info_message(&self) -> Option<&str> {
        self.last_info.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_state_default_has_no_recommendation() {
        let state = UiState::new();
        assert!(!state.has_recommendation());
    }

    #[test]
    fn test_connection_status_display() {
        assert_eq!(format!("{}", ConnectionStatus::Connected), "Connected");
        assert_eq!(format!("{}", ConnectionStatus::Disconnected), "Disconnected");
        assert_eq!(format!("{}", ConnectionStatus::Manual), "Manual Mode");
    }

    #[test]
    fn test_ui_state_set_connected() {
        let mut state = UiState::new();
        state.set_connected(ConnectionStatus::Connected);
        assert_eq!(state.connection_status, Some(ConnectionStatus::Connected));
    }

    #[test]
    fn test_ui_state_clear_error() {
        let mut state = UiState::new();
        state.last_error = Some("test error".to_string());
        state.clear_error();
        assert!(state.last_error.is_none());
    }

    #[test]
    fn test_overlay_mode_default_false() {
        let state = UiState::new();
        assert!(!state.overlay_mode);
    }

    #[test]
    fn test_overlay_dirty_default_false() {
        let state = UiState::new();
        assert!(!state.overlay_dirty);
    }

    #[test]
    fn test_toggle_click_through_sets_dirty() {
        let mut state = UiState::new();
        state.toggle_click_through();
        assert!(state.overlay_dirty);
        assert!(state.overlay_config.click_through);
    }

    #[test]
    fn test_set_opacity_sets_dirty() {
        let mut state = UiState::new();
        state.set_opacity(0.5);
        assert!(state.overlay_dirty);
        assert!((state.overlay_config.opacity - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_last_info_default_none() {
        let state = UiState::new();
        assert!(state.last_info.is_none());
        assert!(state.info_message().is_none());
    }

    #[test]
    fn test_info_message_getter() {
        let mut state = UiState::new();
        state.last_info = Some("exported".to_string());
        assert_eq!(state.info_message(), Some("exported"));
    }
}

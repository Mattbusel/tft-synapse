//! UI state: what the application is currently showing.

use tft_types::GameState;
use tft_advisor::Recommendation;

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
    pub overlay_mode: bool,
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
}

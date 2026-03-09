use tft_types::{GameState, TftError};

#[derive(Debug, Clone, PartialEq)]
pub enum ReaderMode {
    LiveApi,
    Mock,
    Manual,
    ScreenCapture,
}

/// Trait for reading live game state.
pub trait GameStateReader {
    fn poll(&self) -> Result<Option<GameState>, TftError>;
    fn mode(&self) -> ReaderMode;
    fn is_connected(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reader_mode_equality() {
        assert_eq!(ReaderMode::LiveApi, ReaderMode::LiveApi);
        assert_ne!(ReaderMode::LiveApi, ReaderMode::Mock);
    }

    #[test]
    fn test_reader_mode_clone() {
        let m = ReaderMode::Mock;
        let m2 = m.clone();
        assert_eq!(m, m2);
    }
}

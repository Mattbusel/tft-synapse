//! # tft-capture
//!
//! Reads live game state from TFT.
//! Primary: Riot Games Live Client Data API (localhost:2999)
//! Fallback: ScreenCaptureReader (Win32 BitBlt), then MockReader for testing

pub mod live_api;
pub mod mock;
pub mod reader;
pub mod screen_capture;

pub use live_api::RiotLiveApiReader;
pub use mock::MockReader;
pub use reader::{GameStateReader, ReaderMode};
pub use screen_capture::ScreenCaptureReader;

/// Auto-detect the best available reader.
///
/// Priority order:
/// 1. Riot Live API — if a game is currently running
/// 2. Screen capture — on Windows, as a best-effort fallback
/// 3. Mock reader — always available, used for offline / test use
pub fn auto_detect_reader() -> Box<dyn GameStateReader + Send + Sync> {
    // 1. Try the Live API
    if let Ok(reader) = RiotLiveApiReader::new() {
        if reader.poll().ok().flatten().is_some() {
            return Box::new(reader);
        }
    }

    // 2. Screen capture fallback (Windows only)
    #[cfg(target_os = "windows")]
    {
        let sc = ScreenCaptureReader::new();
        if sc.is_enabled() {
            return Box::new(sc);
        }
    }

    // 3. Final fallback: mock
    Box::new(MockReader::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_detect_reader_returns_boxed_reader() {
        // Just verify it returns without panicking
        let reader = auto_detect_reader();
        // On CI/test without a live game, the result is the mock or screen reader
        let _ = reader.mode();
    }

    #[test]
    fn test_screen_capture_reader_exported() {
        let r = ScreenCaptureReader::new();
        let _ = r.is_enabled();
    }
}

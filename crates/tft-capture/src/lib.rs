//! # tft-capture
//!
//! Reads live game state from TFT.
//! Primary: Riot Games Live Client Data API (localhost:2999)
//! Fallback: MockReader for testing and offline use

pub mod reader;
pub mod live_api;
pub mod mock;

pub use reader::{GameStateReader, ReaderMode};
pub use mock::MockReader;
pub use live_api::RiotLiveApiReader;

/// Auto-detect the best available reader.
/// Tries Live API first; returns MockReader if not available.
pub fn auto_detect_reader() -> Box<dyn GameStateReader + Send + Sync> {
    // Try to detect if we should use live API
    // In production this would probe localhost:2999
    // For now return a mock that can be swapped at runtime
    Box::new(MockReader::new())
}

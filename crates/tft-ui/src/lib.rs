//! # tft-ui
//!
//! egui/eframe desktop GUI for TFT Synapse.
//! Provides both overlay mode and standalone window mode.

pub mod app;
pub mod overlay;
pub mod panels;
pub mod state;
pub mod theme;

pub use app::TftSynapseApp;
pub use state::UiState;

//! # tft-ui
//!
//! egui/eframe desktop GUI for TFT Synapse.
//! Provides both overlay mode and standalone window mode.

pub mod app;
pub mod panels;
pub mod theme;
pub mod state;

pub use app::TftSynapseApp;
pub use state::UiState;

//! # tft-game-state
//!
//! Feature extraction: converts GameState into a flat f32 vector for the ML model.

pub mod features;
pub mod encoder;
pub mod normalizer;

pub use features::{FeatureExtractor, FEATURE_DIM};

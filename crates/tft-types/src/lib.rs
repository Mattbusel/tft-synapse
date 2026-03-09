//! # tft-types
//!
//! Zero-dependency shared domain types for TFT Synapse.
//! Every other crate depends on this one.

pub mod action;
pub mod augment;
pub mod champion;
pub mod error;
pub mod game_state;
pub mod item;
pub mod reward;

pub use action::*;
pub use augment::*;
pub use champion::*;
pub use error::TftError;
pub use game_state::*;
pub use item::*;
pub use reward::*;

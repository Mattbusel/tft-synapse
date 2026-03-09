//! # tft-types
//!
//! Zero-dependency shared domain types for TFT Synapse.
//! Every other crate depends on this one.

pub mod augment;
pub mod champion;
pub mod error;
pub mod game_state;
pub mod action;
pub mod reward;
pub mod item;

pub use augment::*;
pub use champion::*;
pub use error::TftError;
pub use game_state::*;
pub use action::*;
pub use reward::*;
pub use item::*;

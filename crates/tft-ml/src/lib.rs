//! # tft-ml
//!
//! Contextual bandit with a shallow neural network scorer.
//! Starts as random policy, improves with each game via online gradient updates.

pub mod model;
pub mod bandit;
pub mod trainer;
pub mod policy;
pub mod persistence;

pub use policy::AugmentPolicy;
pub use trainer::ReplayBuffer;

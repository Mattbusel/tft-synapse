//! # tft-ml
//!
//! Contextual bandit with a shallow neural network scorer.
//! Starts as random policy, improves with each game via online gradient updates.

pub mod bandit;
pub mod model;
pub mod persistence;
pub mod policy;
pub mod trainer;

pub use policy::AugmentPolicy;
pub use trainer::ReplayBuffer;

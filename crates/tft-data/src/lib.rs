//! # tft-data
//!
//! Game data catalog with compile-time embedded YAML files.
//! Use `Catalog::global()` for a lazily-initialized static instance.

pub mod catalog;
pub mod loader;
pub mod embed;

pub use catalog::Catalog;

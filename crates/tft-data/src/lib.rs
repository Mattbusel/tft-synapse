//! # tft-data
//!
//! Game data catalog with compile-time embedded YAML files.
//! Use `Catalog::global()` for a lazily-initialized static instance.

pub mod catalog;
pub mod embed;
pub mod loader;

pub use catalog::Catalog;

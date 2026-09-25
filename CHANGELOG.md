# Changelog

## 0.6.0 (2026-09-25)

- Downloadable builds for Windows, macOS (Apple Silicon and Intel) and Linux on
  every release, with `SHA256SUMS.txt`, built by a new release workflow.
- Published to crates.io: `cargo install tft-synapse`. The library crates are
  published as `tft-synapse-*` (for example `tft-synapse-types`); their Rust
  names are unchanged.
- `--version` now reports the real version instead of 0.2.0.
- MIT licence metadata on every crate; a clippy fix for current Rust.

## 0.5.0 (2026-03-09)

- Full advisor suite. See the README for what shipped in 0.3.0 to 0.5.0.

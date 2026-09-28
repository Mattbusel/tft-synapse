# Changelog

## 0.6.1 (2026-09-28)

- README rewritten to say what works in a real game today (economy, stage and
  lobby advice from Riot's Live Client Data API) and what stays empty (board,
  shop, items, augment choices are not in that API). The old detail moved to
  `docs/REFERENCE.md`, corrected where it claimed more than the code does.
- `--help` no longer claims auto-play; it says how to start and what F9 does,
  with examples. `--overlay` and `--manual` were never used by the program:
  they are hidden and still accepted so old shortcuts keep working.
- Advice text uses colons instead of dashes.
- Releases also carry `tft-synapse-windows-x64.exe` under a fixed name for a
  direct download link; the stale `releases/tft-synapse.exe` left the tree.
- `examples/advise.rs` in `tft-synapse-advisor` prints the economy and stage
  advice for sample game states.
- Project site in `docs/`.

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

# Changelog

## 0.7.0 (2026-10-04)

- **Real game data.** The built-in catalog was a hand-made sample: 20 champions with
  trait mixes from different sets (Jinx as Gunner/Rebel, Lux as Arcanist/Demacia),
  labelled patch 14.23, and invented augment tiers and scores. It is now the live
  set from CommunityDragon (Set 17, patch 16.19): 63 champions with cost, traits and
  role, 42 traits with breakpoints, 49 items, 273 augments with rarity. A new example,
  `import_cdragon`, regenerates it for future sets.
- Positioning uses Riot's role for each champion; it used to guess from trait names
  that mostly do not exist in current sets.
- Augment tags come from each augment's description (no invented power ratings).
- The model's feature vector covers the whole catalog; it silently ignored augments
  past 64 and traits past 32.
- A saved model trained for a different catalog is moved aside and a fresh one started;
  before, every recommendation failed with "input dim mismatch".
- Breaking: `AugmentId` and `StateTransition::augment_chosen` are `u16` (273 augments
  do not fit a `u8`); `ChampionDef` has a `role` field.

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

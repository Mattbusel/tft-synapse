# tft-synapse reference

[README](../README.md) · [Reference](REFERENCE.md)

Everything that used to be in the README, checked against the code on 2026-09-28. Where the old text claimed more than the code does, this page says what the code really does.

## Game state detection

`tft_capture::auto_detect_reader()` picks a source **once, at startup**:

1. **Riot Live Client Data API** (`http://127.0.0.1:2999/liveclientdata/allgamedata`), the local server the game runs during a match. No API key. It is used only if it answers at startup, so start tft-synapse after your game has loaded.
2. **Screen capture fallback** (Windows only). If the API did not answer, the app reads HP, gold and round from screen pixels (Win32 BitBlt, brightness heuristics tuned for a 1080p layout) for the rest of the session. It never switches back to the API.
3. **Demo state** (macOS and Linux). If neither is available, a built-in example state is shown.

The status bar shows the stage, HP, gold and level it read, and "Connected" whenever a source returned a state. It does not say which source is active.

### What the Live Client Data API gives

| Field | Read | Source in `live_api.rs` |
|---|---|---|
| Gold | yes | `activePlayer.currentGold` |
| Level | yes | `activePlayer.level` |
| HP | yes, when reported | `activePlayer.championStats.currentHealth` or `health`, default 100 |
| Stage and round | estimated | from `gameData.gameTime` and a fixed round schedule |
| Lobby | names, HP, level | `allPlayers` |
| Active traits, augments held | only if the API ever sends them | `activePlayer.traits`, `activePlayer.augments` |
| Board, bench, shop, items, augment choices | no | always empty |

## The panels

| Panel | Needs | In a real game today |
|---|---|---|
| Economy (save, level up, roll down, keep streak) | gold, HP, level, round | works (streak is always 0, the API has no streak) |
| Stage awareness (target level, next augment, carousel, PvE) | round, level | works, from the time-based stage estimate |
| Lobby (each player's HP and threat level; contested traits) | lobby | HP and threat work; contested traits need opponents' traits, which are empty |
| Augment ranking | augment choices | empty (no source provides them) |
| Shop, reroll, board, items, carry targets, positioning, pool tracker | board, shop, items | empty |
| Game review | augment decisions | empty (it lists augment choices made through the app) |
| Stats and CSV export | recorded placements | the Export button writes `~/.tft-synapse/history.csv` and `stats.csv`, but the app does not record placements yet, so they have no games |

The rule-based advisors behind the empty panels are implemented and unit-tested in `crates/tft-advisor`; they only lack input.

## The learning part (augments)

Implemented in `crates/tft-ml`, pure Rust, no ML framework:

- A small neural network (512 input features, two hidden layers of 64 and 32, one output per augment in the catalog) scores augments from the game state.
- Thompson sampling (Beta distributions per augment) adds exploration.
- `AugmentPolicy::record_game_outcome` turns a placement into a reward (1st = 1.0, 8th = 0.0, linear), stores the decisions in a 1000-entry replay buffer and runs a mini-batch update (batch 32) once the buffer has 32 entries.
- Weights are saved as JSON to `~/.tft-synapse/model.json` (or `--model-path`).

**Not connected yet:** the app never calls `Advisor::finish_game`, so no placement is recorded, the model is not updated and `model.json` is not written by the app. With no augment choices coming from the game, the ranking is not shown either.

## Command line

```
tft-synapse [OPTIONS]

  --model-path <PATH>   Model weights (default ~/.tft-synapse/model.json)
  --log-level <LEVEL>   trace, debug, info, warn, error (default info)
  --width <PX>          Window width (default 500)
  --height <PX>         Window height (default 600)
  -h, --help            Help
  -V, --version         Version
```

`--overlay` and `--manual` were documented before 0.6.1 but were never used by the program. They are still accepted, so old shortcuts keep working, and do nothing.

## Window

- Always on top, resizable, 500 x 600 by default.
- **F9** (while the window has focus) toggles click-through. When click-through is on, Alt+Tab back to the window and press F9 again to turn it off.
- **Overlay Settings** (bottom of the window): opacity slider (10 to 100 percent) and the click-through checkbox. They apply to the window that has focus when you change them.
- **Update notice**: at startup the app asks the GitHub Releases API for the latest version and shows a Download link if the tag differs from the running version.

## Game data

The catalog is compiled into the binary from `crates/tft-data/data/*.yaml`: 20 champions, 17 traits, 20 augments and 20 items, labelled patch 14.23 (items 14.1). It is a small sample, not a full current set.

To use your own data without rebuilding, put a `catalog.json` in `~/.tft-synapse/`. It is read once at startup (restart to pick up changes). The schema is `CatalogJson` in `crates/tft-data/src/catalog.rs`.

## Build from source

Needs current stable Rust; on Windows the MSVC toolchain. Linux needs the usual egui/eframe system libraries.

```bash
git clone https://gitlab.com/mattbusel/tft-synapse
cd tft-synapse
cargo build --release -p tft-synapse
# Windows: target/release/tft-synapse.exe
```

The Windows build links the C runtime statically (`.cargo/config.toml`), so the exe runs without the Visual C++ redistributable.

## Workspace

```
crates/
  tft-types       shared domain types, TftError, GameState   (crates.io: tft-synapse-types)
  tft-data        YAML catalog embedded at compile time       (tft-synapse-data)
  tft-game-state  512-dim feature vector per game state       (tft-synapse-game-state)
  tft-ml          neural net + Thompson sampling, online learning (tft-synapse-ml)
  tft-capture     Live API reader, screen capture, demo reader (tft-synapse-capture)
  tft-advisor     decision engine, session, reasoning text    (tft-synapse-advisor)
  tft-ui          egui window and panels                      (tft-synapse-ui)
  tft-synapse     the binary
```

## Engineering

- `unwrap`, `expect`, `panic!` and `todo!` are denied by workspace Clippy lints (tests excepted).
- One typed error enum, `TftError`.
- 496 `#[test]` functions across the workspace. CI (Ubuntu) runs `cargo fmt --check`, `cargo clippy --all-features -D warnings` and `cargo test --all-features` on every push.

## History

- **0.6.1**: README and `--help` describe what works today; `--overlay`/`--manual` hidden; stable `tft-synapse-windows-x64.exe` download; Pages site.
- **0.6.0**: release builds for Windows, macOS and Linux; crates.io publish.
- **0.5.0**: pool tracker, positioning advisor, stage awareness, post-game review, update notifier.
- **0.4.0**: economy advisor, carry identification, item advisor, opponent tracker, `catalog.json` override.
- **0.3.0**: screen capture fallback, shop and reroll advice, board analysis, F9 click-through, CSV export.

## Ideas (not built)

System tray (a stub exists in `crates/tft-ui/src/tray.rs`), community tier-list weighting, multi-game augment trends, Discord webhook, pool hit odds.

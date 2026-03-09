# tft-synapse

**AI-powered Teamfight Tactics advisor. Real-time recommendations across every decision that learn from every game you play.**

[![Rust](https://img.shields.io/badge/built%20with-Rust-000000?style=flat&logo=rust)](https://www.rust-lang.org/)
[![Release](https://img.shields.io/github/v/release/Mattbusel/tft-synapse?style=flat)](https://github.com/Mattbusel/tft-synapse/releases/latest)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat)](LICENSE)

---

## Download

[**tft-synapse.exe**](https://github.com/Mattbusel/tft-synapse/releases/latest) - Windows x64, 8.1MB, no installer required.

Run it. Start a TFT game. That is it.

---

## What it does

tft-synapse runs alongside TFT as a transparent overlay and gives you real-time recommendations across all major decisions.

**Augment selection**
```
BEST Last Stand: strong comeback option at 28hp
2nd Blue Battery: solid - synergizes with your 2 Arcanists
3rd Scoped Weapons: situational (score: 38%)
```

**Shop advisor** - shows which units to buy and whether to reroll based on your current gold, upgrade potential, and active traits.

**Board advisor** - scores your composition coherence and recommends swaps to strengthen your trait synergies.

**Economy advisor** - tells you whether to save, level up, roll down, or maintain your streak based on your current HP, gold, and streak state. Tracks interest thresholds so you always know how far you are from the next 10g bracket.

**Carry identification** - finds the top 3 carry targets to build toward 3-star, scored by copies already held across board, bench, and shop, weighted by unit cost and star level.

**Item advisor** - matches each item you hold to the best champion on your board based on trait alignment. AP items to Arcanists, AD/crit to Gunners, tank items to frontline.

**Opponent tracker** - reads the lobby from the Live API, flags contested traits, and suggests a pivot if 3+ players are running the same comp as you.

**Patch hot-reload** - drop a `~/.tft-synapse/catalog.json` file to override the embedded catalog without reinstalling. Update champions and augments when patches drop.

**Stats panel** - tracks placement history, top-four rate, first-place rate, and total games the model has trained on. Export to CSV at any time.

**Champion pool tracker** - shows how many copies of each unit remain in the shared pool. Flags exhausted and critical units so you know when a comp is contested before committing to it.

**Positioning advisor** - classifies every unit on your board as frontline, carry, secondary carry, or support, then assigns recommended hex positions. Warns you if your board has no frontline or is carry-starved.

**Stage awareness** - tracks the current stage and round, recommends your target level, and shows the next 3 key events (augments, carousels, PvE rounds) with how many rounds away they are.

**Post-game review** - after each game, shows a breakdown of every augment decision you made: what you chose, what the model scored it, and what the alternatives were.

**Auto-update notifier** - checks GitHub Releases on startup and shows a download link if a newer version is available.

**Stats panel** - tracks placement history, top-four rate, first-place rate, and total games the model has trained on. Export to CSV at any time.

**F9 toggle** - switches the overlay between interactive mode and click-through mode so it never blocks gameplay.

---

## How the AI works

tft-synapse ships with zero training data. It starts as a near-random policy and gets smarter every game you play.

**Architecture: contextual bandit + shallow neural network**

- A 3-layer neural net maps your current board state (champions, traits, gold, HP, level, augments held) to a score for each augment in the catalog
- Thompson Sampling drives exploration early on, gradually deferring to the learned net as more games accumulate
- After each game your final placement is converted to a reward signal (1st = 1.0, 8th = 0.0) and a mini-batch gradient update runs against a circular replay buffer
- Weights are saved to `~/.tft-synapse/model.json` after every game and loaded automatically on the next launch

The model improves continuously. After 20-30 games it starts reflecting real patterns. After 100+ games it is personalized to your playstyle and the current meta.

---

## Game state detection

tft-synapse uses a three-tier detection chain:

1. **Riot Games Live Client Data API** - a local HTTP server TFT runs on `localhost:2999`. No API key required. This is the primary source and gives full game state.
2. **Screen capture fallback** - if the Live API is unavailable, Win32 BitBlt captures HP and gold directly from the screen.
3. **Mock mode** - used when no game is running, so the UI stays responsive for testing.

The status bar shows which source is active.

---

## Getting started

**Requirements:** Windows 10/11 x64. DirectX 11 (built into Windows, no download needed).

**Step 1:** Download [tft-synapse.exe](https://github.com/Mattbusel/tft-synapse/releases/latest)

**Step 2:** Run it

**Step 3:** Start a TFT game. The status bar shows "Connected" once the Live API is detected.

**Step 4:** When augment selection appears, ranked recommendations show automatically.

**Step 5:** After each game, your placement is recorded and the model updates.

The window is always-on-top by default. Press **F9** to toggle click-through mode when you need to interact with the game underneath. You can resize it freely.

---

## CLI options

```
tft-synapse.exe [OPTIONS]

Options:
 --overlay Transparent always-on-top overlay mode
 --manual Manual input mode (no Live API)
 --model-path <PATH> Path to model weights (default: ~/.tft-synapse/model.json)
 --log-level <LEVEL> trace / debug / info / warn / error (default: info)
 --width <PX> Window width in pixels (default: 500)
 --height <PX> Window height in pixels (default: 600)
 --help Print help
```

---

## Build from source

Requires Rust 1.75+ and the MSVC toolchain on Windows.

```bash
git clone https://github.com/Mattbusel/tft-synapse
cd tft-synapse
cargo build --release
# binary at target/release/tft-synapse.exe
```

The binary embeds all game data (augments, champions, traits, items) at compile time. No external data files needed. To override the catalog without rebuilding, place a `catalog.json` in `~/.tft-synapse/` and it will be loaded at startup instead.

---

## Workspace structure

```
crates/
 tft-types - shared domain types, error enum, GameState
 tft-data - YAML catalog embedded at compile time via include_str!
 tft-game-state - feature extraction (512-dim f32 vector per game state)
 tft-ml - neural net + Thompson Sampling bandit, online learning
 tft-capture - Riot Live API reader and mock reader for testing
 tft-advisor - decision engine, session tracking, reasoning text
 tft-ui - egui desktop GUI (score bars, stats panel, status bar)
 tft-synapse - binary entrypoint
```

Zero external ML dependencies. The neural network is implemented in pure Rust.

---

## Engineering

- Zero panics in production code paths (`unwrap`, `expect`, `panic!` denied by clippy lint)
- Typed error enum (`TftError`) covering every failure surface
- 488 unit tests across all crates, all passing
- Game data baked into the binary at compile time - single file distribution
- Model weights serialized as JSON to `~/.tft-synapse/model.json`
- Patch hot-reload: drop `~/.tft-synapse/catalog.json` to override embedded catalog

---

## What was shipped in v0.5.0

- Champion pool tracker - real-time pool depletion for all 58 champions
- Positioning advisor - hex position recommendations with frontline/carry/support roles
- Stage awareness panel - level targets, upcoming events (augments/carousels/PvE), one-line priority action
- Post-game review - per-decision breakdown after each game
- Auto-update notifier - startup check against GitHub Releases API

## What was shipped in v0.4.0

- Economy advisor with streak detection and gold interest tracking
- Carry identification - top 3 units to build toward 3-star
- Item advisor - matches held items to best champions by trait
- Opponent tracker - flags contested comps and pivot suggestions
- Patch hot-reload - override embedded catalog via `~/.tft-synapse/catalog.json`

## What was shipped in v0.3.0

- Screen capture fallback when Live API is unavailable
- Shop buy and reroll recommendations
- Board composition analysis and trait coherence scoring
- Overlay click-through toggle (F9)
- CSV export for placement history and aggregate stats

## Roadmap

- **Full system tray** - minimize to tray with Show/Quit menu (stub in place)
- **Augment tier list sync** - pull community tier list data to weight recommendations by current meta
- **Multi-game trend analysis** - track which augments are winning for your playstyle over time
- **Discord webhook** - post post-game stats to a Discord channel automatically
- **3-cost/4-cost pool probability** - estimate odds of hitting a unit given known pool depletion

---

## License

MIT

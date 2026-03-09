# tft-synapse

**AI-powered Teamfight Tactics advisor. Real-time augment recommendations that learn from every game you play.**

[![Rust](https://img.shields.io/badge/built%20with-Rust-000000?style=flat&logo=rust)](https://www.rust-lang.org/)
[![Release](https://img.shields.io/github/v/release/Mattbusel/tft-synapse?style=flat)](https://github.com/Mattbusel/tft-synapse/releases/latest)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat)](LICENSE)

---

## Download

[**tft-synapse.exe**](https://github.com/Mattbusel/tft-synapse/releases/latest) - Windows x64, 5.5MB, no installer required.

Run it. Start a TFT game. That is it.

---

## What it does

When you reach an augment selection, tft-synapse ranks your three choices and explains why.

```
BEST   Last Stand: strong comeback option at 28hp
2nd    Blue Battery: solid - synergizes with your 2 Arcanists
3rd    Scoped Weapons: situational (score: 38%)
```

The top panel shows score bars for each augment. The stats panel tracks your placement history and how many games the model has trained on. The window stays on top of TFT so you never need to alt-tab.

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

tft-synapse connects to the **Riot Games Live Client Data API** - a local HTTP server that TFT runs on `localhost:2999` during any active game. No API key required. No screen capture. No OCR.

If the API is not available (no game running, custom game, etc.) the advisor shows a disconnected status and waits.

---

## Getting started

**Requirements:** Windows 10/11 x64. DirectX 11 (built into Windows, no download needed).

**Step 1:** Download [tft-synapse.exe](https://github.com/Mattbusel/tft-synapse/releases/latest)

**Step 2:** Run it

**Step 3:** Start a TFT game. The status bar will show "Connected" once the Live API is detected.

**Step 4:** When augment selection appears, ranked recommendations show automatically.

**Step 5:** After each game, your placement is recorded and the model updates.

The window is always-on-top by default. You can resize it freely.

---

## CLI options

```
tft-synapse.exe [OPTIONS]

Options:
  --overlay           Transparent always-on-top overlay mode
  --manual            Manual input mode (no Live API)
  --model-path <PATH> Path to model weights (default: ~/.tft-synapse/model.json)
  --log-level <LEVEL> trace / debug / info / warn / error (default: info)
  --width <PX>        Window width in pixels (default: 500)
  --height <PX>       Window height in pixels (default: 600)
  --help              Print help
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

The binary embeds all game data (augments, champions, traits) at compile time. No external data files needed.

---

## Workspace structure

```
crates/
  tft-types        - shared domain types, error enum, GameState
  tft-data         - YAML catalog embedded at compile time via include_str!
  tft-game-state   - feature extraction (512-dim f32 vector per game state)
  tft-ml           - neural net + Thompson Sampling bandit, online learning
  tft-capture      - Riot Live API reader and mock reader for testing
  tft-advisor      - decision engine, session tracking, reasoning text
  tft-ui           - egui desktop GUI (score bars, stats panel, status bar)
  tft-synapse      - binary entrypoint
```

Zero external ML dependencies. The neural network is implemented in pure Rust.

---

## Engineering

- Zero panics in production code paths (`unwrap`, `expect`, `panic!` denied by clippy lint)
- Typed error enum (`TftError`) covering every failure surface
- 174 unit tests across all crates, all passing
- Game data baked into the binary at compile time - single file distribution
- Model weights serialized as JSON to `~/.tft-synapse/model.json`

---

## Roadmap

- Screen capture fallback (for games where the Live API is unavailable)
- Champion buy and reroll recommendations
- Board composition suggestions based on current traits
- Overlay transparency and click-through toggle
- Export placement history and model stats to CSV

---

## License

MIT

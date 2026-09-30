# tft-synapse

**A free Teamfight Tactics (TFT) helper for Windows: a small always-on-top window that reads your gold, level and HP from the running game and tells you when to save, level up or roll down, what level to aim for, and when the next augment, carousel and PvE round come.**

For TFT players who want a second opinion on economy and leveling while they play. No login and no API key: it only reads the data the game itself serves on your own PC.

[![Release](https://img.shields.io/github/v/release/Mattbusel/tft-synapse?style=flat)](https://gitlab.com/mattbusel/tft-synapse/-/releases)

[![crates.io](https://img.shields.io/crates/v/tft-synapse.svg)](https://crates.io/crates/tft-synapse)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

> **Early version, read this first.** Riot's local game API does not send your board, shop, items or augment choices. So in a real game the economy, stage and lobby advice work, and the augment, shop, board, item, carry and positioning panels stay empty. The learning model for augments is in the code but the app does not record placements yet. Details in [docs/REFERENCE.md](docs/REFERENCE.md).

## Download

### [Download for Windows (.exe)](https://gitlab.com/mattbusel/tft-synapse/-/releases/permalink/latest/downloads/tft-synapse-windows-x64.exe)

One file, no installer. Windows may say "unknown publisher" because it is unsigned: click **More info**, then **Run anyway**.

| Also | |
|---|---|
| macOS (Apple Silicon or Intel), Linux | archives on the [Releases](https://gitlab.com/mattbusel/tft-synapse/-/releases) page, with `SHA256SUMS.txt` |
| With Rust installed | `cargo install tft-synapse` |

## How it works

<img alt="Diagram: every half second tft-synapse asks the running TFT game for its state through Riot's local Live Client Data API at 127.0.0.1:2999. It reads gold, level, HP, the stage estimated from game time, and the lobby's names, HP and levels. Board, bench, shop, items and augment choices are not in the API and stay empty. The window shows economy advice, the target level with the next augment, carousel and PvE rounds, and each lobby player's HP. If no game is running at startup, Windows falls back to reading HP, gold and round from screen pixels, and macOS or Linux show a demo state." src="docs/img/how-it-works.svg" width="100%">

## Examples

The advice it gives, from the real advisor code (`cargo run -p tft-synapse-advisor --example advise`), for three game states that carry only what the game API provides:

```text
Stage 2-1  gold 18  HP 92  level 4
  Economy: SAVE GOLD. Save 2 more gold to reach the 20 interest threshold
  Target level 5 (behind). Augment this round: pick carefully
  In 0 round(s): Stage 2 carousel
  In 0 round(s): First augment choice
  In 0 round(s): Target level 4 by stage 2-1

Stage 3-2  gold 52  HP 64  level 6
  Economy: LEVEL UP. Gold at interest cap (52); leveling up from 6 improves shop odds
  Target level 6. Augment this round: pick carefully
  In 0 round(s): Second augment choice
  In 3 round(s): PvE: Raptors
  In 9 round(s): Stage 4 carousel

Stage 4-5  gold 34  HP 22  level 7
  Economy: ROLL DOWN. Low HP (22): roll to find upgrades urgently
  Target level 8 (behind). Roll down: you're behind on levels
  In 0 round(s): PvE: Dragon / Baron
  In 0 round(s): Target level 8 by stage 4-5
  In 6 round(s): Stage 5 carousel
```

The window shows the same lines in its Economy and Stage Awareness panels, plus the lobby's HP.

## Use it in 3 steps

1. **Start a TFT game** and wait until you are in the match.
2. **Run `tft-synapse.exe`.** The status bar shows the stage, HP, gold and level it read.
3. **Keep it beside or over the game.** Press **F9** in the window to let clicks pass through to the game; Alt+Tab back and press F9 again to turn that off. Opacity is under Overlay Settings.

Start it after the game has loaded: if no game answers at startup, it uses a rough screen-pixel reader for the whole session instead. `tft-synapse --help` lists the options (window size, log level, model path).

## Documentation

| Read this | For |
|---|---|
| [docs/REFERENCE.md](docs/REFERENCE.md) | Detection chain, what each panel needs, the learning model, command line, game data and `catalog.json`, building from source, workspace layout, history |
| [CHANGELOG.md](CHANGELOG.md) | Release notes |

MIT licensed, see [LICENSE](LICENSE). Not affiliated with or endorsed by Riot Games.

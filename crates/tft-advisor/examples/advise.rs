//! Print the economy and stage advice the app shows, for a few game states.
//!
//! The states carry only what Riot's Live Client Data API gives tft-synapse
//! during a real game (gold, HP, level, the stage estimated from game time);
//! board, shop and augment choices stay empty, as they do in a live game.
//! No game, window or network needed.
//!
//! ```bash
//! cargo run -p tft-synapse-advisor --example advise
//! ```

use tft_advisor::{EconomyAction, EconomyAdvisor, RoundTimer};
use tft_types::{GameState, RoundInfo, ShopSlot};

fn live_api_state(stage: u8, round: u8, gold: u8, hp: u8, level: u8) -> GameState {
    GameState {
        round: RoundInfo { stage, round },
        board: vec![],
        bench: vec![None; 9],
        shop: (0..5)
            .map(|_| ShopSlot {
                champion_id: None,
                cost: 0,
                locked: false,
                sold: false,
            })
            .collect(),
        gold,
        hp,
        level,
        xp: 0,
        streak: 0,
        current_augments: vec![],
        augment_choices: None,
        active_traits: vec![],
        opponents: vec![],
    }
}

fn main() {
    let economy = EconomyAdvisor::new();
    let timer = RoundTimer::new();

    for (stage, round, gold, hp, level) in [(2, 1, 18, 92, 4), (3, 2, 52, 64, 6), (4, 5, 34, 22, 7)]
    {
        let state = live_api_state(stage, round, gold, hp, level);
        println!("Stage {stage}-{round}  gold {gold}  HP {hp}  level {level}");
        match economy.advise(&state) {
            Ok(advice) => {
                let action = match advice.recommended_action {
                    EconomyAction::Save => "SAVE GOLD",
                    EconomyAction::LevelUp => "LEVEL UP",
                    EconomyAction::Roll => "ROLL DOWN",
                    EconomyAction::MaintainStreak => "MAINTAIN STREAK",
                };
                println!("  Economy: {action}. {}", advice.reason);
            }
            Err(e) => println!("  Economy: error: {e}"),
        }
        let stage_info = timer.analyze(&state);
        println!(
            "  Target level {}{}. {}",
            stage_info.recommended_level,
            if stage_info.is_level_behind {
                " (behind)"
            } else {
                ""
            },
            stage_info.current_priority
        );
        for event in &stage_info.upcoming_events {
            println!("  In {} round(s): {}", event.rounds_away, event.description);
        }
        println!();
    }
}

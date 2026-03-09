//! # Stage: ShopAdvisor
//!
//! ## Responsibility
//! Recommend which units to buy from the shop and whether to reroll, based on
//! the current game state and the champion catalog.
//!
//! ## Guarantees
//! - Deterministic: same state + catalog always produces the same output
//! - Non-panicking: all operations are fallible via `Result` or return value types
//! - Bounded: O(shop_size * board_size) per recommendation call
//!
//! ## NOT Responsible For
//! - Positioning / item recommendations (see board_advisor)
//! - Long-term economy projections beyond current gold

use tft_types::{GameState, TftError};
use tft_data::Catalog;

/// A recommendation for a single shop slot.
#[derive(Debug, Clone)]
pub struct ShopRecommendation {
    /// 0-based index into `GameState::shop`.
    pub slot: usize,
    /// Display name of the champion in this slot.
    pub champion_name: String,
    /// Whether the advisor recommends buying this unit.
    pub should_buy: bool,
    /// Human-readable explanation.
    pub reason: String,
    /// Relative priority from 0.0 (skip) to 1.0 (must buy).
    pub priority: f32,
}

/// A recommendation on whether to reroll this turn.
#[derive(Debug, Clone)]
pub struct RerollRecommendation {
    /// Whether the advisor recommends rerolling.
    pub should_reroll: bool,
    /// Human-readable explanation.
    pub reason: String,
    /// The gold floor above which rerolling is considered safe.
    pub gold_threshold: u8,
}

/// Advisor for shop buy / reroll decisions.
pub struct ShopAdvisor;

impl ShopAdvisor {
    /// Construct a new `ShopAdvisor`.
    pub fn new() -> Self {
        Self
    }

    /// Recommend which shop units to buy.
    ///
    /// # Buy logic
    /// - Never buy if gold < 4 (save for level/reroll economy).
    /// - Buy a unit if it is already on the board (upgrade path to 2★ / 3★).
    /// - Buy a unit if it shares a trait with ≥2 board units and would push
    ///   that trait to a breakpoint.
    ///
    /// # Arguments
    /// * `state`   — current observable game state
    /// * `catalog` — full champion / trait catalog
    ///
    /// # Returns
    /// A `Vec` with one `ShopRecommendation` per non-empty, non-sold shop slot.
    pub fn advise_buys(
        &self,
        state: &GameState,
        catalog: &Catalog,
    ) -> Result<Vec<ShopRecommendation>, TftError> {
        let mut recommendations = Vec::new();

        for (slot_idx, slot) in state.shop.iter().enumerate() {
            if slot.sold || slot.champion_id.is_none() {
                continue;
            }

            let champ_id = match slot.champion_id {
                Some(id) => id,
                None => continue,
            };

            let champ_def = catalog
                .champion_by_id(champ_id)
                .ok_or_else(|| TftError::ChampionNotFound(format!("{:?}", champ_id)))?;

            // Rule 1: never buy when gold < 4
            if state.gold < 4 {
                recommendations.push(ShopRecommendation {
                    slot: slot_idx,
                    champion_name: champ_def.name.clone(),
                    should_buy: false,
                    reason: "gold below safe threshold (need ≥4)".to_string(),
                    priority: 0.0,
                });
                continue;
            }

            // Rule 2: upgrade path — unit already on board or bench
            let on_board = state.board.iter().any(|b| b.champion_id == champ_id);
            let on_bench = state
                .bench
                .iter()
                .any(|b| b.as_ref().is_some_and(|s| s.champion_id == champ_id));

            if on_board || on_bench {
                recommendations.push(ShopRecommendation {
                    slot: slot_idx,
                    champion_name: champ_def.name.clone(),
                    should_buy: true,
                    reason: "upgrade path: unit already on board/bench".to_string(),
                    priority: 0.9,
                });
                continue;
            }

            // Rule 3: trait synergy — contributes to a breakpoint
            let synergy_score = self.trait_synergy_score(state, &champ_def.traits, catalog);
            if synergy_score > 0.5 {
                recommendations.push(ShopRecommendation {
                    slot: slot_idx,
                    champion_name: champ_def.name.clone(),
                    should_buy: true,
                    reason: format!(
                        "shares traits with board units and advances a breakpoint (score {:.2})",
                        synergy_score
                    ),
                    priority: synergy_score,
                });
                continue;
            }

            // Default: do not buy
            recommendations.push(ShopRecommendation {
                slot: slot_idx,
                champion_name: champ_def.name.clone(),
                should_buy: false,
                reason: "no compelling upgrade or synergy reason".to_string(),
                priority: synergy_score * 0.5,
            });
        }

        Ok(recommendations)
    }

    /// Recommend whether to reroll.
    ///
    /// # Reroll logic
    /// - Reroll if gold ≥ 50 (interest is capped; free to spend).
    /// - Reroll if hp < 30 and the current shop offers no buyable upgrades.
    /// - Never reroll below the per-level gold threshold.
    ///
    /// # Arguments
    /// * `state` — current observable game state
    pub fn advise_reroll(&self, state: &GameState) -> RerollRecommendation {
        let threshold = gold_threshold_for_level(state.level);

        // Never reroll below threshold
        if state.gold < threshold {
            return RerollRecommendation {
                should_reroll: false,
                reason: format!(
                    "gold ({}) below level-{} threshold ({})",
                    state.gold, state.level, threshold
                ),
                gold_threshold: threshold,
            };
        }

        // Always reroll at 50+ (interest is capped)
        if state.gold >= 50 {
            return RerollRecommendation {
                should_reroll: true,
                reason: "gold at cap (≥50) — safe to spend freely".to_string(),
                gold_threshold: threshold,
            };
        }

        // Desperation reroll: low hp and no shop upgrades
        let has_upgrade = state.shop.iter().any(|slot| {
            if slot.sold || slot.champion_id.is_none() {
                return false;
            }
            let cid = match slot.champion_id {
                Some(id) => id,
                None => return false,
            };
            state.board.iter().any(|b| b.champion_id == cid)
                || state
                    .bench
                    .iter()
                    .any(|b| b.as_ref().is_some_and(|s| s.champion_id == cid))
        });

        if state.hp < 30 && !has_upgrade {
            return RerollRecommendation {
                should_reroll: true,
                reason: "low HP (<30) with no upgrade available in current shop".to_string(),
                gold_threshold: threshold,
            };
        }

        RerollRecommendation {
            should_reroll: false,
            reason: "no strong reason to reroll; save gold for interest".to_string(),
            gold_threshold: threshold,
        }
    }

    /// Compute a 0.0–1.0 synergy score for adding a champion with the given
    /// traits to the current board.
    ///
    /// The score is high when the champion's trait is already represented by
    /// ≥2 board units and adding it would reach a trait breakpoint.
    fn trait_synergy_score(
        &self,
        state: &GameState,
        champ_traits: &[String],
        catalog: &Catalog,
    ) -> f32 {
        let mut best: f32 = 0.0;

        for trait_name in champ_traits {
            // Count how many board units already share this trait
            let board_count = state.board.iter().filter(|slot| {
                catalog
                    .champion_by_id(slot.champion_id)
                    .is_some_and(|def| def.traits.contains(trait_name))
            }).count() as u8;

            if board_count < 2 {
                // Trait not yet represented enough to be compelling
                continue;
            }

            // Look up the next breakpoint for this trait
            if let Some(trait_idx) = catalog.trait_by_name.get(trait_name.as_str()) {
                if let Some(raw_trait) = catalog.traits.get(*trait_idx) {
                    let new_count = board_count + 1; // after buying
                    let next_bp = raw_trait
                        .breakpoints
                        .iter()
                        .copied()
                        .find(|&bp| bp >= new_count);

                    let score = if next_bp == Some(new_count) {
                        // Exactly hits the breakpoint — high value
                        0.85
                    } else if board_count >= 2 {
                        // Contributes to trait even if breakpoint not yet hit
                        0.55
                    } else {
                        0.0
                    };

                    if score > best {
                        best = score;
                    }
                }
            }
        }

        best
    }
}

impl Default for ShopAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

/// Minimum gold to hold before rerolling at each level.
fn gold_threshold_for_level(level: u8) -> u8 {
    match level {
        1..=4 => 20,
        5 => 24,
        6 => 30,
        7 => 36,
        8 => 44,
        _ => 50,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tft_types::{ChampionId, ChampionSlot, RoundInfo, ShopSlot, StarLevel};

    fn catalog() -> &'static Catalog {
        Catalog::global().expect("catalog init failed in test")
    }

    fn base_state() -> GameState {
        GameState {
            round: RoundInfo { stage: 2, round: 1 },
            board: vec![],
            bench: vec![None; 9],
            shop: vec![],
            gold: 30,
            hp: 80,
            level: 5,
            xp: 0,
            streak: 0,
            current_augments: vec![],
            augment_choices: None,
            active_traits: vec![],
        }
    }

    fn slot_with(id: u8, cost: u8) -> ShopSlot {
        ShopSlot {
            champion_id: Some(ChampionId(id)),
            cost,
            locked: false,
            sold: false,
        }
    }

    fn board_slot(id: u8) -> ChampionSlot {
        ChampionSlot {
            champion_id: ChampionId(id),
            star_level: StarLevel::One,
            items: vec![],
        }
    }

    // ── advise_buys: gold threshold ───────────────────────────────────────────

    #[test]
    fn test_advise_buys_low_gold_never_buys() {
        let advisor = ShopAdvisor::new();
        let cat = catalog();
        let mut state = base_state();
        state.gold = 2;
        state.shop = vec![slot_with(0, 1)];
        let recs = advisor.advise_buys(&state, cat).expect("advise_buys failed in test");
        assert!(!recs.is_empty());
        for r in &recs {
            assert!(!r.should_buy, "should not buy when gold < 4, got {:?}", r.reason);
        }
    }

    #[test]
    fn test_advise_buys_gold_exactly_four_allows_buy() {
        let advisor = ShopAdvisor::new();
        let cat = catalog();
        let mut state = base_state();
        state.gold = 4;
        // Put champion 0 in shop and on board → upgrade path
        state.shop = vec![slot_with(0, 1)];
        state.board = vec![board_slot(0)];
        let recs = advisor.advise_buys(&state, cat).expect("advise_buys failed in test");
        let r = &recs[0];
        assert!(r.should_buy, "should buy upgrade when gold == 4");
    }

    // ── advise_buys: upgrade path ─────────────────────────────────────────────

    #[test]
    fn test_advise_buys_upgrade_path_on_board() {
        let advisor = ShopAdvisor::new();
        let cat = catalog();
        let mut state = base_state();
        state.gold = 20;
        state.shop = vec![slot_with(0, 1)];
        state.board = vec![board_slot(0)];
        let recs = advisor.advise_buys(&state, cat).expect("advise_buys failed in test");
        assert_eq!(recs.len(), 1);
        assert!(recs[0].should_buy);
        assert!(recs[0].priority >= 0.8);
    }

    #[test]
    fn test_advise_buys_upgrade_path_on_bench() {
        let advisor = ShopAdvisor::new();
        let cat = catalog();
        let mut state = base_state();
        state.gold = 20;
        state.shop = vec![slot_with(0, 1)];
        state.bench = vec![Some(board_slot(0)), None, None, None, None, None, None, None, None];
        let recs = advisor.advise_buys(&state, cat).expect("advise_buys failed in test");
        assert!(recs[0].should_buy, "should buy upgrade from bench");
    }

    // ── advise_buys: empty / sold slots ──────────────────────────────────────

    #[test]
    fn test_advise_buys_sold_slot_is_skipped() {
        let advisor = ShopAdvisor::new();
        let cat = catalog();
        let mut state = base_state();
        state.gold = 30;
        state.shop = vec![ShopSlot {
            champion_id: Some(ChampionId(0)),
            cost: 1,
            locked: false,
            sold: true,
        }];
        let recs = advisor.advise_buys(&state, cat).expect("advise_buys failed in test");
        assert!(recs.is_empty(), "sold slots should be skipped");
    }

    #[test]
    fn test_advise_buys_empty_slot_is_skipped() {
        let advisor = ShopAdvisor::new();
        let cat = catalog();
        let mut state = base_state();
        state.gold = 30;
        state.shop = vec![ShopSlot {
            champion_id: None,
            cost: 0,
            locked: false,
            sold: false,
        }];
        let recs = advisor.advise_buys(&state, cat).expect("advise_buys failed in test");
        assert!(recs.is_empty(), "empty slots should be skipped");
    }

    // ── advise_buys: recommendation fields ───────────────────────────────────

    #[test]
    fn test_advise_buys_recommendation_has_name() {
        let advisor = ShopAdvisor::new();
        let cat = catalog();
        let mut state = base_state();
        state.gold = 30;
        state.shop = vec![slot_with(0, 1)];
        let recs = advisor.advise_buys(&state, cat).expect("advise_buys failed in test");
        if let Some(r) = recs.first() {
            assert!(!r.champion_name.is_empty());
        }
    }

    #[test]
    fn test_advise_buys_priority_in_range() {
        let advisor = ShopAdvisor::new();
        let cat = catalog();
        let mut state = base_state();
        state.gold = 30;
        state.shop = vec![slot_with(0, 1)];
        let recs = advisor.advise_buys(&state, cat).expect("advise_buys failed in test");
        for r in &recs {
            assert!((0.0..=1.0).contains(&r.priority),
                "priority out of range: {}", r.priority);
        }
    }

    // ── advise_reroll ─────────────────────────────────────────────────────────

    #[test]
    fn test_advise_reroll_gold_at_cap_should_reroll() {
        let advisor = ShopAdvisor::new();
        let mut state = base_state();
        state.gold = 50;
        let rec = advisor.advise_reroll(&state);
        assert!(rec.should_reroll);
    }

    #[test]
    fn test_advise_reroll_below_threshold_no_reroll() {
        let advisor = ShopAdvisor::new();
        let mut state = base_state();
        state.level = 5;
        state.gold = 10; // below level-5 threshold of 24
        let rec = advisor.advise_reroll(&state);
        assert!(!rec.should_reroll);
    }

    #[test]
    fn test_advise_reroll_low_hp_no_upgrades_should_reroll() {
        let advisor = ShopAdvisor::new();
        let mut state = base_state();
        state.hp = 20;
        state.gold = 30;
        state.level = 5;
        // shop has a unit NOT on board/bench
        state.shop = vec![slot_with(5, 2)];
        let rec = advisor.advise_reroll(&state);
        assert!(rec.should_reroll, "should reroll when hp < 30 and no upgrades");
    }

    #[test]
    fn test_advise_reroll_low_hp_with_upgrades_no_reroll() {
        let advisor = ShopAdvisor::new();
        let mut state = base_state();
        state.hp = 20;
        state.gold = 30;
        state.level = 5;
        // champion 0 in shop AND on board → upgrade available
        state.shop = vec![slot_with(0, 1)];
        state.board = vec![board_slot(0)];
        let rec = advisor.advise_reroll(&state);
        assert!(!rec.should_reroll, "should not reroll when upgrades are available");
    }

    #[test]
    fn test_advise_reroll_gold_threshold_correct_for_level() {
        let advisor = ShopAdvisor::new();
        let mut state = base_state();
        state.level = 8;
        state.gold = 50;
        let rec = advisor.advise_reroll(&state);
        assert_eq!(rec.gold_threshold, 44);
    }

    #[test]
    fn test_advise_reroll_reason_not_empty() {
        let advisor = ShopAdvisor::new();
        let state = base_state();
        let rec = advisor.advise_reroll(&state);
        assert!(!rec.reason.is_empty());
    }

    // ── gold_threshold_for_level ──────────────────────────────────────────────

    #[test]
    fn test_gold_threshold_levels() {
        assert_eq!(gold_threshold_for_level(4), 20);
        assert_eq!(gold_threshold_for_level(5), 24);
        assert_eq!(gold_threshold_for_level(6), 30);
        assert_eq!(gold_threshold_for_level(7), 36);
        assert_eq!(gold_threshold_for_level(8), 44);
        assert_eq!(gold_threshold_for_level(9), 50);
    }
}

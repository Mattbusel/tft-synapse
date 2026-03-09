//! # Stage: CarryAdvisor
//!
//! ## Responsibility
//! Identify the top carry candidates on the board and bench, ranked by how
//! achievable a 3-star upgrade is given current copies and unit cost.
//!
//! ## Guarantees
//! - Deterministic: same state + catalog always produces the same ranked list
//! - Non-panicking: all operations return `Result`
//! - Bounded: O(N) where N = distinct champion ids on board + bench + shop
//!
//! ## NOT Responsible For
//! - Economy decisions (see economy_advisor)
//! - Shop buy/reroll decisions (see shop_advisor)
//! - Item recommendations

use std::collections::HashMap;
use tft_data::Catalog;
use tft_types::{ChampionId, GameState, StarLevel, TftError};

/// A single carry candidate with scoring information.
#[derive(Debug, Clone)]
pub struct CarryCandidate {
    /// Champion identifier.
    pub champion_id: ChampionId,
    /// Display name from catalog.
    pub champion_name: String,
    /// Total copies currently held (board + bench + shop).
    pub copies_held: u8,
    /// Copies still needed to reach 3-star (0 if already there).
    pub copies_needed: u8,
    /// Composite carry score; higher is better.
    pub score: f32,
    /// Human-readable justification.
    pub reason: String,
}

/// Advisor for identifying the top carry units to build towards.
pub struct CarryAdvisor;

impl CarryAdvisor {
    /// Construct a new `CarryAdvisor`.
    pub fn new() -> Self {
        Self
    }

    /// Return up to 3 carry candidates sorted by score descending.
    ///
    /// # Scoring formula
    /// `score = (copies_held / 9.0) * cost_weight * star_bonus`
    ///
    /// - `cost_weight`: 1-cost = 0.3, 2-cost = 0.5, 3-cost = 0.8, 4-cost = 1.0, 5-cost = 1.2
    /// - `star_bonus`: 1.5 if any held copy is already 2-star, otherwise 1.0
    ///
    /// # Arguments
    /// * `state`   — current observable game state
    /// * `catalog` — champion / trait catalog
    ///
    /// # Returns
    /// `Ok(Vec<CarryCandidate>)` with up to 3 entries, sorted descending.
    ///
    /// # Panics
    /// This function never panics.
    pub fn identify_carries(
        &self,
        state: &GameState,
        catalog: &Catalog,
    ) -> Result<Vec<CarryCandidate>, TftError> {
        // Step 1: accumulate copies and max star level per champion id
        let mut copies: HashMap<ChampionId, u8> = HashMap::new();
        let mut max_star: HashMap<ChampionId, StarLevel> = HashMap::new();

        // Board
        for slot in &state.board {
            *copies.entry(slot.champion_id).or_insert(0) += 1;
            let entry = max_star.entry(slot.champion_id).or_insert(StarLevel::One);
            if slot.star_level as u8 > *entry as u8 {
                *entry = slot.star_level;
            }
        }

        // Bench
        for slot in state.bench.iter().flatten() {
            *copies.entry(slot.champion_id).or_insert(0) += 1;
            let entry = max_star.entry(slot.champion_id).or_insert(StarLevel::One);
            if slot.star_level as u8 > *entry as u8 {
                *entry = slot.star_level;
            }
        }

        // Shop (non-sold slots only)
        for slot in &state.shop {
            if slot.sold {
                continue;
            }
            if let Some(cid) = slot.champion_id {
                *copies.entry(cid).or_insert(0) += 1;
                // Shop units are always 1-star — don't update max_star
            }
        }

        // Step 2: score each candidate
        let mut candidates: Vec<CarryCandidate> = copies
            .into_iter()
            .map(|(id, held)| -> Result<CarryCandidate, TftError> {
                let def = catalog
                    .champion_by_id(id)
                    .ok_or_else(|| TftError::ChampionNotFound(format!("{:?}", id)))?;

                let cost_weight = cost_weight(def.cost.as_u8());
                let star = max_star.get(&id).copied().unwrap_or(StarLevel::One);
                let star_bonus: f32 = if star == StarLevel::Two || star == StarLevel::Three {
                    1.5
                } else {
                    1.0
                };

                let score = (held as f32 / 9.0) * cost_weight * star_bonus;
                let copies_needed = 9u8.saturating_sub(held);

                let reason = build_reason(held, copies_needed, star, def.cost.as_u8());

                Ok(CarryCandidate {
                    champion_id: id,
                    champion_name: def.name.clone(),
                    copies_held: held,
                    copies_needed,
                    score,
                    reason,
                })
            })
            .collect::<Result<Vec<_>, TftError>>()?;

        // Step 3: sort descending by score, take top 3
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.truncate(3);

        Ok(candidates)
    }
}

impl Default for CarryAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

/// Cost-tier weight for the carry scoring formula.
fn cost_weight(cost: u8) -> f32 {
    match cost {
        1 => 0.3,
        2 => 0.5,
        3 => 0.8,
        4 => 1.0,
        _ => 1.2, // 5-cost
    }
}

/// Build a human-readable reason string.
fn build_reason(held: u8, needed: u8, star: StarLevel, cost: u8) -> String {
    let star_note = match star {
        StarLevel::Three => ", already 3-star".to_string(),
        StarLevel::Two => ", already 2-star".to_string(),
        StarLevel::One => String::new(),
    };
    format!(
        "{}/9 copies, cost-{} unit{}; {} more to 3-star",
        held, cost, star_note, needed
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tft_types::{AugmentId, ChampionSlot, GameState, RoundInfo, ShopSlot};

    fn catalog() -> &'static Catalog {
        Catalog::global().expect("catalog init failed in test")
    }

    fn advisor() -> CarryAdvisor {
        CarryAdvisor::new()
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
            augment_choices: Some([AugmentId(0), AugmentId(1), AugmentId(2)]),
            active_traits: vec![("Arcanist".to_string(), 2)],
            opponents: vec![],
        }
    }

    fn board_slot(id: u8, star: StarLevel) -> ChampionSlot {
        ChampionSlot {
            champion_id: ChampionId(id),
            star_level: star,
            items: vec![],
        }
    }

    fn shop_slot(id: u8, cost: u8) -> ShopSlot {
        ShopSlot {
            champion_id: Some(ChampionId(id)),
            cost,
            locked: false,
            sold: false,
        }
    }

    // ── empty state ───────────────────────────────────────────────────────────

    #[test]
    fn test_identify_carries_empty_state_returns_empty() {
        let state = base_state();
        let result = advisor()
            .identify_carries(&state, catalog())
            .expect("failed in test");
        assert!(result.is_empty());
    }

    // ── copy counting ─────────────────────────────────────────────────────────

    #[test]
    fn test_copies_counted_from_board() {
        let mut state = base_state();
        state.board = vec![board_slot(0, StarLevel::One), board_slot(0, StarLevel::One)];
        let result = advisor()
            .identify_carries(&state, catalog())
            .expect("failed in test");
        assert_eq!(result[0].copies_held, 2);
    }

    #[test]
    fn test_copies_counted_from_bench() {
        let mut state = base_state();
        state.bench = vec![
            Some(board_slot(0, StarLevel::One)),
            Some(board_slot(0, StarLevel::One)),
            None,
        ];
        let result = advisor()
            .identify_carries(&state, catalog())
            .expect("failed in test");
        assert_eq!(result[0].copies_held, 2);
    }

    #[test]
    fn test_copies_counted_from_shop() {
        let mut state = base_state();
        state.shop = vec![shop_slot(0, 1), shop_slot(0, 1)];
        let result = advisor()
            .identify_carries(&state, catalog())
            .expect("failed in test");
        assert_eq!(result[0].copies_held, 2);
    }

    #[test]
    fn test_sold_shop_slots_excluded() {
        let mut state = base_state();
        state.shop = vec![ShopSlot {
            champion_id: Some(ChampionId(0)),
            cost: 1,
            locked: false,
            sold: true,
        }];
        let result = advisor()
            .identify_carries(&state, catalog())
            .expect("failed in test");
        assert!(result.is_empty(), "sold slots should be excluded");
    }

    // ── copies_needed ─────────────────────────────────────────────────────────

    #[test]
    fn test_copies_needed_correct() {
        let mut state = base_state();
        state.board = vec![board_slot(0, StarLevel::One); 3];
        let result = advisor()
            .identify_carries(&state, catalog())
            .expect("failed in test");
        assert_eq!(result[0].copies_needed, 6); // 9 - 3
    }

    #[test]
    fn test_copies_needed_zero_when_full() {
        let mut state = base_state();
        state.board = vec![board_slot(0, StarLevel::One); 9];
        let result = advisor()
            .identify_carries(&state, catalog())
            .expect("failed in test");
        assert_eq!(result[0].copies_needed, 0);
    }

    // ── star bonus ────────────────────────────────────────────────────────────

    #[test]
    fn test_two_star_raises_score_vs_one_star() {
        let mut state_one = base_state();
        state_one.board = vec![board_slot(0, StarLevel::One)];

        let mut state_two = base_state();
        state_two.board = vec![board_slot(0, StarLevel::Two)];

        let res_one = advisor()
            .identify_carries(&state_one, catalog())
            .expect("failed in test");
        let res_two = advisor()
            .identify_carries(&state_two, catalog())
            .expect("failed in test");

        assert!(
            res_two[0].score > res_one[0].score,
            "2-star should score higher"
        );
    }

    // ── sorting & top-3 cap ───────────────────────────────────────────────────

    #[test]
    fn test_results_sorted_descending_by_score() {
        let mut state = base_state();
        // Champion 0: 3 copies (1-star); Champion 1: 1 copy (1-star)
        state.board = vec![
            board_slot(0, StarLevel::One),
            board_slot(0, StarLevel::One),
            board_slot(0, StarLevel::One),
            board_slot(1, StarLevel::One),
        ];
        let result = advisor()
            .identify_carries(&state, catalog())
            .expect("failed in test");
        for w in result.windows(2) {
            assert!(w[0].score >= w[1].score, "results not sorted descending");
        }
    }

    #[test]
    fn test_at_most_three_candidates_returned() {
        let mut state = base_state();
        // Five distinct champions on board
        state.board = (0u8..5).map(|i| board_slot(i, StarLevel::One)).collect();
        let result = advisor()
            .identify_carries(&state, catalog())
            .expect("failed in test");
        assert!(result.len() <= 3);
    }

    // ── cost_weight helper ────────────────────────────────────────────────────

    #[test]
    fn test_cost_weight_values() {
        assert!((cost_weight(1) - 0.3).abs() < f32::EPSILON);
        assert!((cost_weight(2) - 0.5).abs() < f32::EPSILON);
        assert!((cost_weight(3) - 0.8).abs() < f32::EPSILON);
        assert!((cost_weight(4) - 1.0).abs() < f32::EPSILON);
        assert!((cost_weight(5) - 1.2).abs() < f32::EPSILON);
    }

    // ── reason is never empty ─────────────────────────────────────────────────

    #[test]
    fn test_reason_never_empty() {
        let mut state = base_state();
        state.board = vec![board_slot(0, StarLevel::One)];
        let result = advisor()
            .identify_carries(&state, catalog())
            .expect("failed in test");
        for c in &result {
            assert!(
                !c.reason.is_empty(),
                "reason should not be empty for {:?}",
                c.champion_id
            );
        }
    }

    // ── champion_name populated ───────────────────────────────────────────────

    #[test]
    fn test_champion_name_populated() {
        let mut state = base_state();
        state.board = vec![board_slot(0, StarLevel::One)];
        let result = advisor()
            .identify_carries(&state, catalog())
            .expect("failed in test");
        assert!(!result[0].champion_name.is_empty());
    }

    // ── Default impl ──────────────────────────────────────────────────────────

    #[test]
    fn test_carry_advisor_default() {
        let adv = CarryAdvisor::default();
        let state = base_state();
        assert!(adv.identify_carries(&state, catalog()).is_ok());
    }
}

//! # Stage: BoardAdvisor
//!
//! ## Responsibility
//! Analyse the current board composition, calculate trait statuses, identify
//! the strongest active synergies, and score overall board coherence.
//!
//! ## Guarantees
//! - Non-panicking: all operations return `Result` or well-bounded values
//! - Deterministic: same state + catalog always produces the same output
//! - Bounded: coherence score ∈ [0.0, 1.0]
//!
//! ## NOT Responsible For
//! - Item placement recommendations
//! - Shop buy / reroll decisions (see shop_advisor)

use tft_types::{GameState, TftError};
use tft_data::Catalog;

/// Snapshot of one trait's activation status on the current board.
#[derive(Debug, Clone)]
pub struct TraitStatus {
    /// Trait name as it appears in the catalog.
    pub trait_name: String,
    /// Number of board units with this trait.
    pub current_count: u8,
    /// The next breakpoint above `current_count`, if any.
    pub next_breakpoint: Option<u8>,
    /// How many more units are needed to reach `next_breakpoint`.
    pub units_needed: u8,
    /// Broad category: "AP", "AD", "Tank", or "Utility".
    pub trait_type: String,
}

/// Full board analysis result.
#[derive(Debug, Clone)]
pub struct BoardRecommendation {
    /// Status of every trait that has ≥1 unit on the board.
    pub trait_statuses: Vec<TraitStatus>,
    /// Name of the trait with the highest active breakpoint count, if any.
    pub strongest_synergy: Option<String>,
    /// Champion names from the catalog that would complete a breakpoint.
    pub suggested_additions: Vec<String>,
    /// Names of board units that share the fewest traits with teammates.
    pub suggested_removals: Vec<String>,
    /// 0.0–1.0 coherence score (how close the board is to trait breakpoints).
    pub overall_strength: f32,
}

/// Advisor for board composition analysis.
pub struct BoardAdvisor;

impl BoardAdvisor {
    /// Construct a new `BoardAdvisor`.
    pub fn new() -> Self {
        Self
    }

    /// Analyse the current board composition.
    ///
    /// # Arguments
    /// * `state`   — current observable game state
    /// * `catalog` — full champion / trait catalog
    ///
    /// # Returns
    /// A `BoardRecommendation` summarising the board's trait state and
    /// actionable suggestions.
    pub fn analyze_board(
        &self,
        state: &GameState,
        catalog: &Catalog,
    ) -> Result<BoardRecommendation, TftError> {
        // Build trait-count map from board units
        let mut trait_counts: std::collections::HashMap<String, u8> =
            std::collections::HashMap::new();

        for slot in &state.board {
            let def = catalog
                .champion_by_id(slot.champion_id)
                .ok_or_else(|| TftError::ChampionNotFound(format!("{:?}", slot.champion_id)))?;
            for t in &def.traits {
                *trait_counts.entry(t.clone()).or_insert(0) += 1;
            }
        }

        // Build TraitStatus for each represented trait
        let mut trait_statuses: Vec<TraitStatus> = trait_counts
            .iter()
            .filter_map(|(name, &count)| {
                let idx = *catalog.trait_by_name.get(name.as_str())?;
                let raw = catalog.traits.get(idx)?;
                let next_bp = raw
                    .breakpoints
                    .iter()
                    .copied()
                    .find(|&bp| bp > count);
                let units_needed = next_bp.map_or(0, |bp| bp.saturating_sub(count));
                Some(TraitStatus {
                    trait_name: name.clone(),
                    current_count: count,
                    next_breakpoint: next_bp,
                    units_needed,
                    trait_type: raw.r#type.clone(),
                })
            })
            .collect();

        // Sort by current_count descending for stable output
        trait_statuses.sort_by(|a, b| b.current_count.cmp(&a.current_count));

        // Strongest synergy: trait at highest current breakpoint
        let strongest_synergy = trait_statuses.first().map(|ts| ts.trait_name.clone());

        // Suggested additions: champions not on board that would complete a breakpoint
        let board_ids: std::collections::HashSet<_> =
            state.board.iter().map(|s| s.champion_id).collect();

        let mut suggested_additions: Vec<String> = catalog
            .champions
            .iter()
            .filter(|def| !board_ids.contains(&def.id))
            .filter(|def| {
                def.traits.iter().any(|t| {
                    trait_statuses.iter().any(|ts| {
                        ts.trait_name == *t
                            && ts.units_needed == 1
                    })
                })
            })
            .map(|def| def.name.clone())
            .collect();

        suggested_additions.sort();
        suggested_additions.dedup();

        // Suggested removals: board units that contribute to the fewest active traits
        let mut unit_contribution: Vec<(String, usize)> = state
            .board
            .iter()
            .filter_map(|slot| {
                let def = catalog.champion_by_id(slot.champion_id)?;
                let contrib = def
                    .traits
                    .iter()
                    .filter(|t| trait_counts.get(*t).copied().unwrap_or(0) > 0)
                    .count();
                Some((def.name.clone(), contrib))
            })
            .collect();

        unit_contribution.sort_by_key(|(_, c)| *c);

        let suggested_removals: Vec<String> = unit_contribution
            .iter()
            .take(2)
            .map(|(name, _)| name.clone())
            .collect();

        let overall_strength =
            Self::board_coherence_score(&trait_counts.iter().map(|(k, &v)| (k.clone(), v)).collect::<Vec<_>>(), catalog);

        Ok(BoardRecommendation {
            trait_statuses,
            strongest_synergy,
            suggested_additions,
            suggested_removals,
            overall_strength,
        })
    }

    /// Score how coherent the board is: the fraction of represented traits
    /// that are at or beyond their first breakpoint.
    ///
    /// Returns a value in [0.0, 1.0].  An empty board returns 0.0.
    pub(crate) fn board_coherence_score(
        traits: &[(String, u8)],
        catalog: &Catalog,
    ) -> f32 {
        if traits.is_empty() {
            return 0.0;
        }

        let mut total_score = 0.0f32;
        let mut count = 0u32;

        for (name, current) in traits {
            let current = *current;
            let idx = match catalog.trait_by_name.get(name.as_str()) {
                Some(&i) => i,
                None => continue,
            };
            let raw = match catalog.traits.get(idx) {
                Some(t) => t,
                None => continue,
            };

            count += 1;

            // Find the highest breakpoint that has been reached
            let reached: u8 = raw
                .breakpoints
                .iter()
                .copied()
                .filter(|&bp| bp <= current)
                .max()
                .unwrap_or(0);

            // Find the maximum breakpoint for normalisation
            let max_bp: u8 = raw.breakpoints.iter().copied().max().unwrap_or(1);

            total_score += reached as f32 / max_bp as f32;
        }

        if count == 0 {
            return 0.0;
        }

        (total_score / count as f32).min(1.0)
    }
}

impl Default for BoardAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tft_types::{AugmentId, ChampionId, ChampionSlot, RoundInfo, ShopSlot, StarLevel};

    fn catalog() -> &'static Catalog {
        Catalog::global().expect("catalog init failed in test")
    }

    fn base_state() -> GameState {
        GameState {
            round: RoundInfo { stage: 2, round: 1 },
            board: vec![],
            bench: vec![None; 9],
            shop: vec![
                ShopSlot { champion_id: None, cost: 0, locked: false, sold: false },
            ],
            gold: 30,
            hp: 80,
            level: 5,
            xp: 0,
            streak: 0,
            current_augments: vec![AugmentId(0)],
            augment_choices: None,
            active_traits: vec![],
            opponents: vec![],
        }
    }

    fn board_slot(id: u8) -> ChampionSlot {
        ChampionSlot {
            champion_id: ChampionId(id),
            star_level: StarLevel::One,
            items: vec![],
        }
    }

    // ── analyze_board: empty board ───────────────────────────────────────────

    #[test]
    fn test_analyze_board_empty_board_returns_ok() {
        let advisor = BoardAdvisor::new();
        let state = base_state();
        let result = advisor.analyze_board(&state, catalog());
        assert!(result.is_ok());
    }

    #[test]
    fn test_analyze_board_empty_board_no_traits() {
        let advisor = BoardAdvisor::new();
        let state = base_state();
        let rec = advisor.analyze_board(&state, catalog()).expect("failed in test");
        assert!(rec.trait_statuses.is_empty());
        assert!(rec.strongest_synergy.is_none());
    }

    #[test]
    fn test_analyze_board_empty_board_coherence_zero() {
        let advisor = BoardAdvisor::new();
        let state = base_state();
        let rec = advisor.analyze_board(&state, catalog()).expect("failed in test");
        assert_eq!(rec.overall_strength, 0.0);
    }

    // ── analyze_board: populated board ───────────────────────────────────────

    #[test]
    fn test_analyze_board_trait_statuses_populated() {
        let advisor = BoardAdvisor::new();
        let cat = catalog();
        let mut state = base_state();
        // Use the first two champions from the catalog
        if cat.champion_count() >= 2 {
            state.board = vec![board_slot(0), board_slot(1)];
            let rec = advisor.analyze_board(&state, cat).expect("failed in test");
            // Should have at least one trait status
            assert!(!rec.trait_statuses.is_empty());
        }
    }

    #[test]
    fn test_analyze_board_strongest_synergy_is_some_when_board_nonempty() {
        let advisor = BoardAdvisor::new();
        let cat = catalog();
        let mut state = base_state();
        if cat.champion_count() >= 1 {
            state.board = vec![board_slot(0)];
            let rec = advisor.analyze_board(&state, cat).expect("failed in test");
            if !rec.trait_statuses.is_empty() {
                assert!(rec.strongest_synergy.is_some());
            }
        }
    }

    // ── analyze_board: trait status fields ───────────────────────────────────

    #[test]
    fn test_trait_status_fields_valid() {
        let advisor = BoardAdvisor::new();
        let cat = catalog();
        let mut state = base_state();
        // Put two units sharing a trait (Arcanist — ids 0 and 1 in embedded catalog)
        state.board = vec![board_slot(0), board_slot(1)];
        let rec = advisor.analyze_board(&state, cat).expect("failed in test");
        for ts in &rec.trait_statuses {
            assert!(!ts.trait_name.is_empty());
            assert!(ts.current_count >= 1);
            assert!(!ts.trait_type.is_empty());
        }
    }

    #[test]
    fn test_trait_status_units_needed_consistent() {
        let advisor = BoardAdvisor::new();
        let cat = catalog();
        let mut state = base_state();
        state.board = vec![board_slot(0), board_slot(1)];
        let rec = advisor.analyze_board(&state, cat).expect("failed in test");
        for ts in &rec.trait_statuses {
            if let Some(bp) = ts.next_breakpoint {
                assert_eq!(ts.units_needed, bp - ts.current_count,
                    "units_needed inconsistent for trait {}", ts.trait_name);
            } else {
                assert_eq!(ts.units_needed, 0);
            }
        }
    }

    // ── coherence score ───────────────────────────────────────────────────────

    #[test]
    fn test_coherence_score_empty_returns_zero() {
        let score = BoardAdvisor::board_coherence_score(&[], catalog());
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_coherence_score_bounds() {
        let cat = catalog();
        // Use all known trait names with count=1
        let traits: Vec<(String, u8)> = cat
            .traits
            .iter()
            .map(|t| (t.name.clone(), 1u8))
            .collect();
        let score = BoardAdvisor::board_coherence_score(&traits, cat);
        assert!((0.0..=1.0).contains(&score),
            "coherence score out of bounds: {}", score);
    }

    #[test]
    fn test_coherence_score_at_breakpoint_is_nonzero() {
        let cat = catalog();
        // Arcanist breakpoint 2 — set count == 2
        if cat.trait_by_name.contains_key("Arcanist") {
            let traits = vec![("Arcanist".to_string(), 2u8)];
            let score = BoardAdvisor::board_coherence_score(&traits, cat);
            assert!(score > 0.0, "score should be > 0 when at breakpoint");
        }
    }

    #[test]
    fn test_coherence_score_unknown_trait_ignored() {
        let cat = catalog();
        let traits = vec![("NonExistentTrait".to_string(), 5u8)];
        let score = BoardAdvisor::board_coherence_score(&traits, cat);
        assert_eq!(score, 0.0, "unknown trait should not contribute to score");
    }

    // ── suggestions ───────────────────────────────────────────────────────────

    #[test]
    fn test_suggested_removals_at_most_two() {
        let advisor = BoardAdvisor::new();
        let cat = catalog();
        let mut state = base_state();
        state.board = (0..4).map(|i| board_slot(i as u8)).collect();
        let rec = advisor.analyze_board(&state, cat).expect("failed in test");
        assert!(rec.suggested_removals.len() <= 2);
    }

    #[test]
    fn test_overall_strength_in_range() {
        let advisor = BoardAdvisor::new();
        let cat = catalog();
        let mut state = base_state();
        state.board = vec![board_slot(0), board_slot(1)];
        let rec = advisor.analyze_board(&state, cat).expect("failed in test");
        assert!((0.0..=1.0).contains(&rec.overall_strength),
            "overall_strength out of range: {}", rec.overall_strength);
    }
}

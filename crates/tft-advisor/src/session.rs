//! GameSession: tracks all decisions made during a single game.

use tft_data::Catalog;
use tft_types::{AugmentId, GameState};
use tracing::info;

/// Score threshold above which a pick is considered a "top pick" in post-game review.
const TOP_PICK_THRESHOLD: f64 = 0.7;

/// A post-game review entry for one augment decision.
#[derive(Debug, Clone)]
pub struct ReviewEntry {
    /// Stage the decision was made at.
    pub stage: u8,
    /// Round the decision was made at.
    pub round: u8,
    /// Name of the augment that was chosen.
    pub chosen_name: String,
    /// Score assigned to the chosen augment.
    pub chosen_score: f32,
    /// Names of the other augments offered (with placeholder score 0.0).
    pub alternatives: Vec<(String, f32)>,
    /// True if the chosen score was >= TOP_PICK_THRESHOLD (heuristic for "good pick").
    pub was_top_pick: bool,
}

/// A single augment decision made during the session.
#[derive(Debug, Clone)]
pub struct AugmentDecision {
    pub round_stage: u8,
    pub round_number: u8,
    pub offered: Vec<AugmentId>,
    pub chosen: AugmentId,
    pub score: f32,
}

/// Tracks all decisions and state transitions during a game.
pub struct GameSession {
    decisions: Vec<AugmentDecision>,
    game_id: u64,
}

impl GameSession {
    pub fn new(game_id: u64) -> Self {
        Self {
            decisions: Vec::new(),
            game_id,
        }
    }

    pub fn record_decision(
        &mut self,
        state: &GameState,
        offered: Vec<AugmentId>,
        chosen: AugmentId,
        score: f32,
    ) {
        let decision = AugmentDecision {
            round_stage: state.round.stage,
            round_number: state.round.round,
            offered,
            chosen,
            score,
        };
        info!(
            "Decision recorded: chose {:?} with score {:.3} at {}-{}",
            chosen, score, state.round.stage, state.round.round
        );
        self.decisions.push(decision);
    }

    pub fn decisions(&self) -> &[AugmentDecision] {
        &self.decisions
    }
    pub fn decision_count(&self) -> usize {
        self.decisions.len()
    }
    pub fn game_id(&self) -> u64 {
        self.game_id
    }

    /// Report chosen augment indices for ML training.
    pub fn chosen_augment_indices(&self) -> Vec<u8> {
        self.decisions.iter().map(|d| d.chosen.0).collect()
    }

    /// Returns a human-readable summary of all decisions made this game.
    /// Used for the post-game review panel.
    ///
    /// # Arguments
    /// * `catalog` — the game data catalog used for augment name lookup
    ///
    /// # Returns
    /// A `Vec<ReviewEntry>`, one per augment decision recorded this session.
    /// Empty if no decisions were recorded yet.
    ///
    /// # Panics
    /// This function never panics.
    pub fn review_summary(&self, catalog: &Catalog) -> Vec<ReviewEntry> {
        self.decisions
            .iter()
            .map(|d| {
                let chosen_name = catalog
                    .augment_by_id(d.chosen)
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| format!("Augment#{}", d.chosen.0));

                let alternatives: Vec<(String, f32)> = d
                    .offered
                    .iter()
                    .filter(|&&id| id != d.chosen)
                    .map(|&id| {
                        let name = catalog
                            .augment_by_id(id)
                            .map(|a| a.name.clone())
                            .unwrap_or_else(|| format!("Augment#{}", id.0));
                        (name, 0.0f32)
                    })
                    .collect();

                let was_top_pick = f64::from(d.score) >= TOP_PICK_THRESHOLD;

                ReviewEntry {
                    stage: d.round_stage,
                    round: d.round_number,
                    chosen_name,
                    chosen_score: d.score,
                    alternatives,
                    was_top_pick,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_data::Catalog;
    use tft_types::{AugmentId, GameState, RoundInfo};

    fn make_state(stage: u8, round: u8) -> GameState {
        GameState {
            round: RoundInfo { stage, round },
            ..Default::default()
        }
    }

    #[test]
    fn test_new_session_empty() {
        let session = GameSession::new(1);
        assert_eq!(session.decision_count(), 0);
        assert_eq!(session.game_id(), 1);
    }

    #[test]
    fn test_record_decision_increments_count() {
        let mut session = GameSession::new(1);
        let state = make_state(2, 1);
        session.record_decision(&state, vec![AugmentId(0)], AugmentId(0), 0.8);
        assert_eq!(session.decision_count(), 1);
    }

    #[test]
    fn test_record_multiple_decisions() {
        let mut session = GameSession::new(1);
        for i in 0u8..3 {
            let state = make_state(2 + i as u8, 1);
            session.record_decision(&state, vec![AugmentId(i)], AugmentId(i), 0.7);
        }
        assert_eq!(session.decision_count(), 3);
    }

    #[test]
    fn test_chosen_augment_indices() {
        let mut session = GameSession::new(42);
        let state = make_state(3, 2);
        session.record_decision(&state, vec![AugmentId(5), AugmentId(6)], AugmentId(5), 0.9);
        let indices = session.chosen_augment_indices();
        assert_eq!(indices, vec![5u8]);
    }

    #[test]
    fn test_decisions_returns_all() {
        let mut session = GameSession::new(1);
        let state = make_state(2, 1);
        session.record_decision(&state, vec![], AugmentId(0), 0.5);
        session.record_decision(&state, vec![], AugmentId(1), 0.6);
        assert_eq!(session.decisions().len(), 2);
    }

    // ── review_summary ──────────────────────────────────────────────────────

    fn global_catalog() -> &'static Catalog {
        Catalog::global().expect("catalog init failed in test")
    }

    #[test]
    fn test_review_summary_empty_session_returns_empty() {
        let session = GameSession::new(1);
        let entries = session.review_summary(global_catalog());
        assert!(entries.is_empty());
    }

    #[test]
    fn test_review_summary_one_decision_returns_one_entry() {
        let mut session = GameSession::new(1);
        let state = make_state(2, 1);
        session.record_decision(
            &state,
            vec![AugmentId(0), AugmentId(1), AugmentId(2)],
            AugmentId(0),
            0.8,
        );
        let entries = session.review_summary(global_catalog());
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_review_summary_entry_stage_round_correct() {
        let mut session = GameSession::new(1);
        let state = make_state(3, 2);
        session.record_decision(&state, vec![AugmentId(0)], AugmentId(0), 0.9);
        let entries = session.review_summary(global_catalog());
        assert_eq!(entries[0].stage, 3);
        assert_eq!(entries[0].round, 2);
    }

    #[test]
    fn test_review_summary_chosen_score_preserved() {
        let mut session = GameSession::new(1);
        let state = make_state(2, 3);
        session.record_decision(&state, vec![AugmentId(0)], AugmentId(0), 0.75);
        let entries = session.review_summary(global_catalog());
        assert!((entries[0].chosen_score - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_review_summary_was_top_pick_above_threshold() {
        let mut session = GameSession::new(1);
        let state = make_state(2, 1);
        // Use 0.71 rather than exactly 0.7: f32 → f64 promotion makes 0.7f32
        // slightly less than 0.7f64, which would cause a false failure.
        session.record_decision(&state, vec![AugmentId(0)], AugmentId(0), 0.71);
        let entries = session.review_summary(global_catalog());
        assert!(entries[0].was_top_pick);
    }

    #[test]
    fn test_review_summary_was_top_pick_below_threshold() {
        let mut session = GameSession::new(1);
        let state = make_state(2, 1);
        session.record_decision(&state, vec![AugmentId(0)], AugmentId(0), 0.6);
        let entries = session.review_summary(global_catalog());
        assert!(!entries[0].was_top_pick);
    }

    #[test]
    fn test_review_summary_alternatives_excludes_chosen() {
        let mut session = GameSession::new(1);
        let state = make_state(2, 1);
        // Offer 0, 1, 2; choose 0 — alternatives should be 1 and 2 only
        session.record_decision(
            &state,
            vec![AugmentId(0), AugmentId(1), AugmentId(2)],
            AugmentId(0),
            0.9,
        );
        let entries = session.review_summary(global_catalog());
        // Alternatives should not contain the chosen id
        let catalog = global_catalog();
        let chosen_name = catalog
            .augment_by_id(AugmentId(0))
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "Augment#0".to_string());
        assert!(!entries[0]
            .alternatives
            .iter()
            .any(|(n, _)| n == &chosen_name));
        assert_eq!(entries[0].alternatives.len(), 2);
    }

    #[test]
    fn test_review_summary_multiple_decisions_preserves_order() {
        let mut session = GameSession::new(1);
        for i in 0..3u8 {
            let state = make_state(2 + i, 1);
            session.record_decision(
                &state,
                vec![AugmentId(i)],
                AugmentId(i),
                0.5 + i as f32 * 0.1,
            );
        }
        let entries = session.review_summary(global_catalog());
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].stage, 2);
        assert_eq!(entries[1].stage, 3);
        assert_eq!(entries[2].stage, 4);
    }
}

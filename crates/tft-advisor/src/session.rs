//! GameSession: tracks all decisions made during a single game.

use tft_types::{AugmentId, GameState};
use tracing::info;

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
        Self { decisions: Vec::new(), game_id }
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
        info!("Decision recorded: chose {:?} with score {:.3} at {}-{}", chosen, score, state.round.stage, state.round.round);
        self.decisions.push(decision);
    }

    pub fn decisions(&self) -> &[AugmentDecision] { &self.decisions }
    pub fn decision_count(&self) -> usize { self.decisions.len() }
    pub fn game_id(&self) -> u64 { self.game_id }

    /// Report chosen augment indices for ML training.
    pub fn chosen_augment_indices(&self) -> Vec<u8> {
        self.decisions.iter().map(|d| d.chosen.0).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_types::{AugmentId, GameState, RoundInfo};

    fn make_state(stage: u8, round: u8) -> GameState {
        GameState { round: RoundInfo { stage, round }, ..Default::default() }
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
}

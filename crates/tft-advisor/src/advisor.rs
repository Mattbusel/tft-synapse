//! Advisor: main decision engine tying together ML and reasoning.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tft_types::{AugmentId, GameState, Placement, TftError};
use tft_data::Catalog;
use tft_ml::AugmentPolicy;
use crate::reasoning::explain_augment;
use crate::round_timer::{RoundTimer, StageAwareness};
use crate::session::{GameSession, ReviewEntry};
use crate::metrics::AdvisorMetrics;
use crate::shop_advisor::{ShopAdvisor, ShopRecommendation, RerollRecommendation};
use crate::board_advisor::{BoardAdvisor, BoardRecommendation};
use crate::economy_advisor::{EconomyAdvisor, EconomyAdvice};
use crate::carry_advisor::{CarryAdvisor, CarryCandidate};
use crate::item_advisor::{ItemAdvisor, ItemRecommendation};
use crate::opponent_tracker::{LobbyAnalysis, OpponentTracker};
use crate::pool_tracker::{PoolEntry, PoolTracker};
use crate::positioning_advisor::{BoardLayout, PositioningAdvisor};
use tracing::info;

/// A single augment recommendation with score and reasoning.
#[derive(Debug, Clone)]
pub struct RecommendedAugment {
    pub id: AugmentId,
    pub score: f32,
    pub reasoning: String,
}

/// The full recommendation for an augment choice situation.
#[derive(Debug, Clone)]
pub struct Recommendation {
    pub ranked: Vec<RecommendedAugment>,
    pub top_pick: AugmentId,
}

/// A complete recommendation covering augment, shop, reroll, board, economy, and carry decisions.
#[derive(Debug)]
pub struct FullRecommendation {
    /// Augment pick recommendation (present only during augment phase).
    pub augment: Option<Recommendation>,
    /// Per-slot shop buy recommendations.
    pub shop: Vec<ShopRecommendation>,
    /// Reroll decision.
    pub reroll: RerollRecommendation,
    /// Board composition analysis.
    pub board: BoardRecommendation,
    /// Economy (gold / leveling) advice.
    pub economy: EconomyAdvice,
    /// Top carry candidates sorted by score descending.
    pub carries: Vec<CarryCandidate>,
    /// Item placement recommendations.
    pub items: Vec<ItemRecommendation>,
    /// Opponent lobby analysis.
    pub lobby: LobbyAnalysis,
    /// Stage/round awareness and upcoming key events.
    pub stage_awareness: StageAwareness,
    /// Post-game review: all augment decisions this session (empty during active game).
    pub review: Vec<ReviewEntry>,
    /// Champion pool state — remaining copies per unit.
    pub pool: Vec<PoolEntry>,
    /// Board positioning recommendations.
    pub positions: BoardLayout,
}

/// The main advisor: reads state, calls policy, returns recommendations.
pub struct Advisor {
    policy: AugmentPolicy,
    catalog: &'static Catalog,
    session: GameSession,
    pub metrics: AdvisorMetrics,
    shop_advisor: ShopAdvisor,
    board_advisor: BoardAdvisor,
    economy_advisor: EconomyAdvisor,
    carry_advisor: CarryAdvisor,
    item_advisor: ItemAdvisor,
    opponent_tracker: OpponentTracker,
    round_timer: RoundTimer,
    pool_tracker: PoolTracker,
    positioning_advisor: PositioningAdvisor,
}

impl Advisor {
    pub fn new(model_path: PathBuf) -> Result<Self, TftError> {
        let catalog = Catalog::global()?;
        let policy = AugmentPolicy::load_or_init(catalog, model_path)?;
        let game_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Ok(Self {
            policy,
            catalog,
            session: GameSession::new(game_id),
            metrics: AdvisorMetrics::new(),
            shop_advisor: ShopAdvisor::new(),
            board_advisor: BoardAdvisor::new(),
            economy_advisor: EconomyAdvisor::new(),
            carry_advisor: CarryAdvisor::new(),
            item_advisor: ItemAdvisor::new(),
            opponent_tracker: OpponentTracker::new(),
            round_timer: RoundTimer::new(),
            pool_tracker: PoolTracker::new(),
            positioning_advisor: PositioningAdvisor::new(),
        })
    }

    /// Produce a ranked recommendation for the current augment choices.
    pub fn advise(&mut self, state: &GameState) -> Result<Option<Recommendation>, TftError> {
        let choices = match &state.augment_choices {
            Some(c) => c.to_vec(),
            None => return Ok(None),
        };

        let ranked_scores = self.policy.rank_augments(state, &choices)?;

        let ranked: Vec<RecommendedAugment> = ranked_scores.iter().map(|&(id, score)| {
            let reasoning = explain_augment(id, score, state, self.catalog);
            RecommendedAugment { id, score, reasoning }
        }).collect();

        let top_pick = ranked.first()
            .map(|r| r.id)
            .ok_or_else(|| TftError::InvalidState("no ranked augments".to_string()))?;

        // Record the decision in the session
        if let Some(top) = ranked.first() {
            self.session.record_decision(state, choices, top.id, top.score);
        }

        info!("Advise: top pick {:?} score={:.3}", top_pick, ranked[0].score);
        Ok(Some(Recommendation { ranked, top_pick }))
    }

    /// Produce a full recommendation covering augment, shop, reroll, and board.
    ///
    /// This is the primary entry point for the complete decision pipeline.
    /// The existing `advise()` method is retained for backwards compatibility.
    pub fn advise_full(&mut self, state: &GameState) -> Result<FullRecommendation, TftError> {
        let augment = self.advise(state)?;
        let shop = self.shop_advisor.advise_buys(state, self.catalog)?;
        let reroll = self.shop_advisor.advise_reroll(state);
        let board = self.board_advisor.analyze_board(state, self.catalog)?;
        let economy = self.economy_advisor.advise(state)?;
        let carries = self.carry_advisor.identify_carries(state, self.catalog)?;
        let items = self.item_advisor.advise_items(state, self.catalog)?;
        let lobby = self.opponent_tracker.analyze_lobby(state, self.catalog)?;
        let stage_awareness = self.round_timer.analyze(state);
        let review = self.session.review_summary(self.catalog);
        let pool = self.pool_tracker.track(state, self.catalog)?;
        let positions = self.positioning_advisor.advise_positions(state, self.catalog)?;
        Ok(FullRecommendation { augment, shop, reroll, board, economy, carries, items, lobby, stage_awareness, review, pool, positions })
    }

    /// Call after a game ends with the final placement.
    /// This triggers ML training and saves the model.
    pub fn finish_game(&mut self, placement: Placement) -> Result<(), TftError> {
        self.policy.record_game_outcome(placement)?;
        self.policy.save()?;
        self.metrics.record_placement(placement);
        info!("Game finished: placement={}, total_games={}", placement.0, self.metrics.games_played);
        Ok(())
    }

    pub fn games_trained(&self) -> u32 { self.policy.games_trained() }
    pub fn session(&self) -> &GameSession { &self.session }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_types::{ChampionId, ChampionSlot, GameState, RoundInfo, StarLevel};
    use std::env::temp_dir;

    fn make_advisor() -> Advisor {
        let path = temp_dir().join("tft_advisor_test_model.json");
        Advisor::new(path).expect("advisor init failed in test")
    }

    fn make_state_with_choices() -> GameState {
        GameState {
            round: RoundInfo { stage: 2, round: 1 },
            board: vec![ChampionSlot { champion_id: ChampionId(0), star_level: StarLevel::One, items: vec![] }],
            bench: vec![],
            shop: vec![],
            gold: 30,
            hp: 80,
            level: 4,
            xp: 0,
            streak: 0,
            current_augments: vec![],
            augment_choices: Some([AugmentId(0), AugmentId(1), AugmentId(2)]),
            active_traits: vec![],
            opponents: vec![],
        }
    }

    #[test]
    fn test_advisor_new_succeeds() {
        let _ = make_advisor();
    }

    #[test]
    fn test_advise_returns_recommendation_when_augment_phase() {
        let mut advisor = make_advisor();
        let state = make_state_with_choices();
        let result = advisor.advise(&state).expect("advise failed in test");
        assert!(result.is_some());
    }

    #[test]
    fn test_advise_returns_none_when_not_augment_phase() {
        let mut advisor = make_advisor();
        let mut state = make_state_with_choices();
        state.augment_choices = None;
        let result = advisor.advise(&state).expect("advise failed in test");
        assert!(result.is_none());
    }

    #[test]
    fn test_recommendation_has_three_options() {
        let mut advisor = make_advisor();
        let state = make_state_with_choices();
        let rec = advisor.advise(&state).expect("advise failed in test").expect("no recommendation");
        assert_eq!(rec.ranked.len(), 3);
    }

    #[test]
    fn test_recommendation_top_pick_matches_first_ranked() {
        let mut advisor = make_advisor();
        let state = make_state_with_choices();
        let rec = advisor.advise(&state).expect("advise failed in test").expect("no recommendation");
        assert_eq!(rec.top_pick, rec.ranked[0].id);
    }

    #[test]
    fn test_recommendation_reasoning_not_empty() {
        let mut advisor = make_advisor();
        let state = make_state_with_choices();
        let rec = advisor.advise(&state).expect("advise failed in test").expect("no recommendation");
        for r in &rec.ranked {
            assert!(!r.reasoning.is_empty(), "reasoning should not be empty for {:?}", r.id);
        }
    }

    #[test]
    fn test_finish_game_increments_games() {
        let mut advisor = make_advisor();
        let state = make_state_with_choices();
        advisor.advise(&state).expect("advise failed in test");
        advisor.finish_game(Placement(3)).expect("finish game failed in test");
        assert_eq!(advisor.metrics.games_played, 1);
    }

    #[test]
    fn test_session_records_decision() {
        let mut advisor = make_advisor();
        let state = make_state_with_choices();
        advisor.advise(&state).expect("advise failed in test");
        assert_eq!(advisor.session().decision_count(), 1);
    }

    // ── advise_full ───────────────────────────────────────────────────────────

    #[test]
    fn test_advise_full_returns_ok() {
        let mut advisor = make_advisor();
        let state = make_state_with_choices();
        let result = advisor.advise_full(&state);
        assert!(result.is_ok(), "advise_full failed: {:?}", result.err());
    }

    #[test]
    fn test_advise_full_augment_present_in_augment_phase() {
        let mut advisor = make_advisor();
        let state = make_state_with_choices();
        let full = advisor.advise_full(&state).expect("advise_full failed in test");
        assert!(full.augment.is_some(), "augment should be present during augment phase");
    }

    #[test]
    fn test_advise_full_augment_absent_outside_augment_phase() {
        let mut advisor = make_advisor();
        let mut state = make_state_with_choices();
        state.augment_choices = None;
        let full = advisor.advise_full(&state).expect("advise_full failed in test");
        assert!(full.augment.is_none());
    }

    #[test]
    fn test_advise_full_board_recommendation_present() {
        let mut advisor = make_advisor();
        let state = make_state_with_choices();
        let full = advisor.advise_full(&state).expect("advise_full failed in test");
        // board recommendation struct always present (even for empty board)
        let _ = &full.board;
    }

    #[test]
    fn test_advise_full_reroll_recommendation_present() {
        let mut advisor = make_advisor();
        let state = make_state_with_choices();
        let full = advisor.advise_full(&state).expect("advise_full failed in test");
        assert!(!full.reroll.reason.is_empty());
    }
}

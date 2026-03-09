//! # Stage: EconomyAdvisor
//!
//! ## Responsibility
//! Recommend the optimal economic action each round: save gold for interest
//! thresholds, level up, roll down for upgrades, or maintain a streak bonus.
//!
//! ## Guarantees
//! - Deterministic: same state always produces the same advice
//! - Non-panicking: all operations return Result
//! - Bounded: O(1) per call — no heap allocation on the hot path
//!
//! ## NOT Responsible For
//! - Which units to buy (see shop_advisor)
//! - Board positioning (see board_advisor)
//! - Long-horizon simulations (single-round advice only)

use tft_types::{GameState, TftError};

/// The gold-interest breakpoints in TFT (10 / 20 / 30 / 40 / 50).
const INTEREST_THRESHOLDS: [u8; 5] = [10, 20, 30, 40, 50];

/// Recommended economic action for the current round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EconomyAction {
    /// Hold gold — approach the next interest threshold.
    Save,
    /// Spend XP to gain a level.
    LevelUp,
    /// Spend gold to reroll the shop for upgrades.
    Roll,
    /// Keep the current streak alive (win or loss streak bonus).
    MaintainStreak,
}

/// Whether the active streak is a win streak or a loss streak.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreakType {
    Win,
    Loss,
}

/// Full economy recommendation for the current round.
#[derive(Debug, Clone)]
pub struct EconomyAdvice {
    /// The single most important action to take this round.
    pub recommended_action: EconomyAction,
    /// Human-readable explanation of why.
    pub reason: String,
    /// Present when the player has an active streak worth preserving.
    pub streak_type: Option<StreakType>,
    /// How much more gold is needed to reach the next interest threshold.
    pub gold_to_interest: u8,
    /// The nearest interest threshold above current gold.
    pub next_interest_threshold: u8,
}

/// Advisor for gold economy decisions.
pub struct EconomyAdvisor;

impl EconomyAdvisor {
    /// Construct a new `EconomyAdvisor`.
    pub fn new() -> Self {
        Self
    }

    /// Produce an economy recommendation for the current game state.
    ///
    /// # Priority order
    /// 1. Streak ≥ 3 (win) or ≤ -3 (loss) → `MaintainStreak`
    /// 2. Safe to level (gold ≥ 50, level < 8, stage ≥ 3) → `LevelUp`
    /// 3. Danger zone or gold-cap roll → `Roll`
    /// 4. Default → `Save`
    ///
    /// # Arguments
    /// * `state` — current observable game state
    ///
    /// # Returns
    /// `Ok(EconomyAdvice)` — always succeeds for valid game states.
    ///
    /// # Panics
    /// This function never panics.
    pub fn advise(&self, state: &GameState) -> Result<EconomyAdvice, TftError> {
        let (next_threshold, gold_to_interest) = next_interest_info(state.gold);

        let streak_type = classify_streak(state.streak);

        // Priority 1: active streak worth maintaining
        if state.streak >= 3 || state.streak <= -3 {
            let stype = streak_type.clone();
            let label = match &stype {
                Some(StreakType::Win) => "win",
                Some(StreakType::Loss) => "loss",
                None => "streak",
            };
            return Ok(EconomyAdvice {
                recommended_action: EconomyAction::MaintainStreak,
                reason: format!(
                    "Active {} streak ({}): maintain it for bonus gold",
                    label, state.streak
                ),
                streak_type: stype,
                gold_to_interest,
                next_interest_threshold: next_threshold,
            });
        }

        // Priority 2: level up — safe when gold ≥ 50, level < 8, late game
        if state.gold >= 50 && state.level < 8 && state.round.stage >= 3 {
            return Ok(EconomyAdvice {
                recommended_action: EconomyAction::LevelUp,
                reason: format!(
                    "Gold at interest cap ({}); leveling up from {} improves shop odds",
                    state.gold, state.level
                ),
                streak_type,
                gold_to_interest,
                next_interest_threshold: next_threshold,
            });
        }

        // Priority 3: roll — danger zone or gold is capped at 50+ and level 8+
        if state.hp <= 30 || (state.gold > 50 && state.level >= 8) {
            let reason = if state.hp <= 30 {
                format!("Low HP ({}): roll to find upgrades urgently", state.hp)
            } else {
                format!(
                    "Gold ({}) above cap with max-level board; spend freely on rolls",
                    state.gold
                )
            };
            return Ok(EconomyAdvice {
                recommended_action: EconomyAction::Roll,
                reason,
                streak_type,
                gold_to_interest,
                next_interest_threshold: next_threshold,
            });
        }

        // Default: save toward next interest threshold
        Ok(EconomyAdvice {
            recommended_action: EconomyAction::Save,
            reason: format!(
                "Save {} more gold to reach the {} interest threshold",
                gold_to_interest, next_threshold
            ),
            streak_type,
            gold_to_interest,
            next_interest_threshold: next_threshold,
        })
    }
}

impl Default for EconomyAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

/// Return (next_threshold, gold_to_interest) for the current gold amount.
fn next_interest_info(gold: u8) -> (u8, u8) {
    for &t in &INTEREST_THRESHOLDS {
        if gold < t {
            return (t, t - gold);
        }
    }
    // Already at or above 50 — show 50 as the cap
    (50, 0)
}

/// Classify the streak into a StreakType (or None if below threshold).
fn classify_streak(streak: i8) -> Option<StreakType> {
    if streak >= 3 {
        Some(StreakType::Win)
    } else if streak <= -3 {
        Some(StreakType::Loss)
    } else {
        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tft_types::{AugmentId, ChampionId, ChampionSlot, GameState, RoundInfo, StarLevel};

    fn advisor() -> EconomyAdvisor {
        EconomyAdvisor::new()
    }

    fn base_state() -> GameState {
        GameState {
            round: RoundInfo { stage: 2, round: 1 },
            board: vec![ChampionSlot {
                champion_id: ChampionId(0),
                star_level: StarLevel::One,
                items: vec![],
            }],
            bench: vec![],
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

    // ── next_interest_info ────────────────────────────────────────────────────

    #[test]
    fn test_next_interest_info_below_first_threshold() {
        let (t, gap) = next_interest_info(5);
        assert_eq!(t, 10);
        assert_eq!(gap, 5);
    }

    #[test]
    fn test_next_interest_info_exactly_at_threshold() {
        let (t, gap) = next_interest_info(10);
        assert_eq!(t, 20);
        assert_eq!(gap, 10);
    }

    #[test]
    fn test_next_interest_info_at_cap() {
        let (t, gap) = next_interest_info(50);
        assert_eq!(t, 50);
        assert_eq!(gap, 0);
    }

    #[test]
    fn test_next_interest_info_above_cap() {
        let (t, gap) = next_interest_info(60);
        assert_eq!(t, 50);
        assert_eq!(gap, 0);
    }

    // ── classify_streak ───────────────────────────────────────────────────────

    #[test]
    fn test_classify_streak_no_streak() {
        assert_eq!(classify_streak(0), None);
        assert_eq!(classify_streak(2), None);
        assert_eq!(classify_streak(-2), None);
    }

    #[test]
    fn test_classify_streak_win() {
        assert_eq!(classify_streak(3), Some(StreakType::Win));
        assert_eq!(classify_streak(5), Some(StreakType::Win));
    }

    #[test]
    fn test_classify_streak_loss() {
        assert_eq!(classify_streak(-3), Some(StreakType::Loss));
        assert_eq!(classify_streak(-5), Some(StreakType::Loss));
    }

    // ── advise: MaintainStreak ────────────────────────────────────────────────

    #[test]
    fn test_advise_win_streak_recommends_maintain() {
        let adv = advisor();
        let mut state = base_state();
        state.streak = 3;
        let advice = adv.advise(&state).expect("advise failed in test");
        assert_eq!(advice.recommended_action, EconomyAction::MaintainStreak);
        assert_eq!(advice.streak_type, Some(StreakType::Win));
    }

    #[test]
    fn test_advise_loss_streak_recommends_maintain() {
        let adv = advisor();
        let mut state = base_state();
        state.streak = -3;
        let advice = adv.advise(&state).expect("advise failed in test");
        assert_eq!(advice.recommended_action, EconomyAction::MaintainStreak);
        assert_eq!(advice.streak_type, Some(StreakType::Loss));
    }

    #[test]
    fn test_advise_streak_2_does_not_trigger_maintain() {
        let adv = advisor();
        let mut state = base_state();
        state.streak = 2;
        let advice = adv.advise(&state).expect("advise failed in test");
        assert_ne!(advice.recommended_action, EconomyAction::MaintainStreak);
    }

    // ── advise: LevelUp ───────────────────────────────────────────────────────

    #[test]
    fn test_advise_level_up_when_conditions_met() {
        let adv = advisor();
        let mut state = base_state();
        state.gold = 50;
        state.level = 7;
        state.round = RoundInfo { stage: 3, round: 1 };
        let advice = adv.advise(&state).expect("advise failed in test");
        assert_eq!(advice.recommended_action, EconomyAction::LevelUp);
    }

    #[test]
    fn test_advise_no_level_up_when_already_level_8() {
        let adv = advisor();
        let mut state = base_state();
        state.gold = 50;
        state.level = 8;
        state.round = RoundInfo { stage: 3, round: 1 };
        let advice = adv.advise(&state).expect("advise failed in test");
        assert_ne!(advice.recommended_action, EconomyAction::LevelUp);
    }

    #[test]
    fn test_advise_no_level_up_early_game() {
        let adv = advisor();
        let mut state = base_state();
        state.gold = 50;
        state.level = 7;
        state.round = RoundInfo { stage: 2, round: 1 };
        let advice = adv.advise(&state).expect("advise failed in test");
        // stage < 3, so LevelUp should NOT fire
        assert_ne!(advice.recommended_action, EconomyAction::LevelUp);
    }

    // ── advise: Roll ──────────────────────────────────────────────────────────

    #[test]
    fn test_advise_roll_when_low_hp() {
        let adv = advisor();
        let mut state = base_state();
        state.hp = 30;
        state.streak = 0; // no streak
        let advice = adv.advise(&state).expect("advise failed in test");
        assert_eq!(advice.recommended_action, EconomyAction::Roll);
    }

    #[test]
    fn test_advise_roll_when_gold_above_cap_and_max_level() {
        let adv = advisor();
        let mut state = base_state();
        state.gold = 55;
        state.level = 8;
        state.hp = 80;
        state.streak = 0;
        let advice = adv.advise(&state).expect("advise failed in test");
        assert_eq!(advice.recommended_action, EconomyAction::Roll);
    }

    #[test]
    fn test_advise_no_roll_when_gold_exactly_50_and_level_8() {
        // gold == 50 does NOT satisfy `gold > 50`, and hp is safe
        let adv = advisor();
        let mut state = base_state();
        state.gold = 50;
        state.level = 8;
        state.hp = 80;
        state.streak = 0;
        state.round = RoundInfo { stage: 2, round: 1 }; // stage < 3, no LevelUp either
        let advice = adv.advise(&state).expect("advise failed in test");
        // Should fall through to Save (gold==50 doesn't trigger LevelUp because stage<3)
        assert_eq!(advice.recommended_action, EconomyAction::Save);
    }

    // ── advise: Save ──────────────────────────────────────────────────────────

    #[test]
    fn test_advise_save_as_default() {
        let adv = advisor();
        let state = base_state();
        let advice = adv.advise(&state).expect("advise failed in test");
        assert_eq!(advice.recommended_action, EconomyAction::Save);
    }

    #[test]
    fn test_advise_save_gold_to_interest_correct() {
        let adv = advisor();
        let mut state = base_state();
        state.gold = 27;
        let advice = adv.advise(&state).expect("advise failed in test");
        assert_eq!(advice.next_interest_threshold, 30);
        assert_eq!(advice.gold_to_interest, 3);
    }

    // ── advise: reason is never empty ────────────────────────────────────────

    #[test]
    fn test_advise_reason_never_empty() {
        let adv = advisor();
        for gold in [0u8, 9, 10, 29, 30, 49, 50, 55] {
            let mut state = base_state();
            state.gold = gold;
            let advice = adv.advise(&state).expect("advise failed in test");
            assert!(!advice.reason.is_empty(), "reason empty for gold={gold}");
        }
    }

    // ── Default impl ──────────────────────────────────────────────────────────

    #[test]
    fn test_economy_advisor_default() {
        let adv = EconomyAdvisor::default();
        let state = base_state();
        assert!(adv.advise(&state).is_ok());
    }
}

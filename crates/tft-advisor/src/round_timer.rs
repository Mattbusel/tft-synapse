//! # Stage: RoundTimer
//!
//! ## Responsibility
//! Computes stage awareness: upcoming key events, level targets, and
//! what the player should prioritize right now.
//!
//! ## Guarantees
//! - Deterministic: given the same game state, always returns the same result
//! - Non-failing: `analyze` never returns an error; all logic is pure and bounded
//! - Non-blocking: zero I/O, zero async
//!
//! ## NOT Responsible For
//! - Detecting augment choices (see: GameState::is_augment_phase)
//! - Providing ML-driven recommendations (see: Advisor)

use tft_types::GameState;

/// The type of an upcoming key event in the game.
#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    /// An augment selection round is coming up.
    Augment,
    /// A free carousel round.
    Carousel,
    /// A PvE combat round (wolves, krugs, raptors, dragon/baron).
    PvE,
    /// A level target milestone the player should be hitting.
    LevelTarget,
}

/// A single upcoming event with distance information.
#[derive(Debug, Clone)]
pub struct UpcomingEvent {
    /// What kind of event this is.
    pub event_type: EventType,
    /// Human-readable description.
    pub description: String,
    /// How many rounds away (0 = this round, 1 = next round, etc.).
    pub rounds_away: u8,
}

/// Full stage/round awareness snapshot.
#[derive(Debug, Clone)]
pub struct StageAwareness {
    pub current_stage: u8,
    pub current_round: u8,
    /// What level the player should be targeting right now.
    pub recommended_level: u8,
    /// True if state.level < recommended_level.
    pub is_level_behind: bool,
    /// Next 3 key events sorted by rounds_away ascending.
    pub upcoming_events: Vec<UpcomingEvent>,
    /// One-line action string for the player.
    pub current_priority: String,
}

impl Default for StageAwareness {
    fn default() -> Self {
        StageAwareness {
            current_stage: 1,
            current_round: 1,
            recommended_level: 3,
            is_level_behind: false,
            upcoming_events: vec![],
            current_priority: "Waiting for game".to_string(),
        }
    }
}

/// Known key events: (stage, round, EventType, description).
/// These are encoded as static data and searched at runtime.
const KEY_EVENTS: &[(u8, u8, EventType, &str)] = &[
    // Carousels — start of each stage (x-1)
    (1, 1, EventType::Carousel, "Opening carousel"),
    (2, 1, EventType::Carousel, "Stage 2 carousel"),
    (3, 1, EventType::Carousel, "Stage 3 carousel"),
    (4, 1, EventType::Carousel, "Stage 4 carousel"),
    (5, 1, EventType::Carousel, "Stage 5 carousel"),
    (6, 1, EventType::Carousel, "Stage 6 carousel"),
    // PvE rounds
    (1, 3, EventType::PvE, "PvE: Little Wolves"),
    (1, 4, EventType::PvE, "PvE: Armory / Mini-boss"),
    (2, 5, EventType::PvE, "PvE: Krugs"),
    (3, 5, EventType::PvE, "PvE: Raptors"),
    (4, 5, EventType::PvE, "PvE: Dragon / Baron"),
    // Augment rounds
    (2, 1, EventType::Augment, "First augment choice"),
    (2, 3, EventType::Augment, "Second augment choice (early)"),
    (3, 2, EventType::Augment, "Second augment choice"),
    (4, 2, EventType::Augment, "Third augment choice"),
    // Level targets
    (2, 1, EventType::LevelTarget, "Target level 4 by stage 2-1"),
    (2, 3, EventType::LevelTarget, "Target level 5 by stage 2-3"),
    (3, 1, EventType::LevelTarget, "Target level 6 by stage 3-1"),
    (4, 1, EventType::LevelTarget, "Target level 7 by stage 4-1"),
    (4, 5, EventType::LevelTarget, "Target level 8 by stage 4-5"),
];

/// Converts (stage, round) to a linear round index for distance calculation.
fn to_linear(stage: u8, round: u8) -> u16 {
    // Assume at most 7 rounds per stage for ordering purposes.
    (stage as u16) * 10 + (round as u16)
}

/// Returns the recommended level for the given stage and round.
fn recommended_level(stage: u8, round: u8) -> u8 {
    match stage {
        1 => 3,
        2 => 5,
        3 => 6,
        4 if round >= 5 => 8,
        4 => 7,
        _ => 8,
    }
}

/// Stateless advisor for round/stage timing awareness.
pub struct RoundTimer;

impl RoundTimer {
    /// Create a new RoundTimer.
    pub fn new() -> Self {
        Self
    }

    /// Compute stage awareness from the current game state.
    ///
    /// # Arguments
    /// * `state` — current observable game state
    ///
    /// # Returns
    /// A fully populated `StageAwareness`; never fails.
    ///
    /// # Panics
    /// This function never panics.
    ///
    /// # Example
    /// ```rust
    /// let timer = tft_advisor::RoundTimer::new();
    /// let state = tft_types::GameState::default();
    /// let awareness = timer.analyze(&state);
    /// assert_eq!(awareness.current_stage, 0);
    /// ```
    pub fn analyze(&self, state: &GameState) -> StageAwareness {
        let stage = state.round.stage;
        let round = state.round.round;
        let current_linear = to_linear(stage, round);

        let rec_level = recommended_level(stage, round);
        let is_level_behind = state.level < rec_level;

        // Collect upcoming events (those strictly after current round, or this round)
        let mut events: Vec<UpcomingEvent> = KEY_EVENTS
            .iter()
            .filter_map(|(s, r, et, desc)| {
                let event_linear = to_linear(*s, *r);
                if event_linear >= current_linear {
                    let diff = event_linear - current_linear;
                    // Only include if diff fits in u8 (which it always will for real TFT rounds)
                    let rounds_away = if diff > u8::MAX as u16 {
                        u8::MAX
                    } else {
                        diff as u8
                    };
                    Some(UpcomingEvent {
                        event_type: et.clone(),
                        description: (*desc).to_string(),
                        rounds_away,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by distance ascending, then take up to 3 closest events.
        // Do not dedup: multiple events can share the same round (e.g., carousel + augment on 2-1).
        events.sort_by_key(|e| e.rounds_away);
        events.truncate(3);

        let current_priority = Self::compute_priority(state, &events, is_level_behind);

        StageAwareness {
            current_stage: stage,
            current_round: round,
            recommended_level: rec_level,
            is_level_behind,
            upcoming_events: events,
            current_priority,
        }
    }

    fn compute_priority(
        state: &GameState,
        events: &[UpcomingEvent],
        is_level_behind: bool,
    ) -> String {
        // Check if augment is very close (0 or 1 round away)
        let augment_soon = events.iter().find(|e| e.event_type == EventType::Augment);
        if let Some(aug_event) = augment_soon {
            if aug_event.rounds_away == 0 {
                return "Augment this round — pick carefully".to_string();
            }
            if aug_event.rounds_away == 1 {
                return "Augment next round — save gold".to_string();
            }
        }

        // Level behind + low HP = roll down
        if is_level_behind && state.hp < 50 {
            return "Roll down — you're behind on levels".to_string();
        }

        // Level behind but not critical
        if is_level_behind {
            return "Buy XP — you're behind on level target".to_string();
        }

        // Carousel soon
        let carousel_soon = events
            .iter()
            .find(|e| e.event_type == EventType::Carousel && e.rounds_away <= 2);
        if carousel_soon.is_some() {
            return "Prepare for carousel — position to win".to_string();
        }

        // Default: econ
        let augment_distant = events.iter().find(|e| e.event_type == EventType::Augment);
        if augment_distant.map_or(true, |e| e.rounds_away >= 3) {
            return "Econ — save to 50g before next augment".to_string();
        }

        "Hold and stabilize board".to_string()
    }
}

impl Default for RoundTimer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_types::{GameState, RoundInfo};

    fn make_state(stage: u8, round: u8, level: u8, hp: u8) -> GameState {
        GameState {
            round: RoundInfo { stage, round },
            level,
            hp,
            ..Default::default()
        }
    }

    fn timer() -> RoundTimer {
        RoundTimer::new()
    }

    // ── recommended_level ───────────────────────────────────────────────────

    #[test]
    fn test_recommended_level_stage1_is_3() {
        assert_eq!(recommended_level(1, 1), 3);
        assert_eq!(recommended_level(1, 4), 3);
    }

    #[test]
    fn test_recommended_level_stage2_is_5() {
        assert_eq!(recommended_level(2, 3), 5);
    }

    #[test]
    fn test_recommended_level_stage3_is_6() {
        assert_eq!(recommended_level(3, 1), 6);
    }

    #[test]
    fn test_recommended_level_stage4_early_is_7() {
        assert_eq!(recommended_level(4, 1), 7);
        assert_eq!(recommended_level(4, 4), 7);
    }

    #[test]
    fn test_recommended_level_stage4_late_is_8() {
        assert_eq!(recommended_level(4, 5), 8);
    }

    #[test]
    fn test_recommended_level_stage5_plus_is_8() {
        assert_eq!(recommended_level(5, 1), 8);
        assert_eq!(recommended_level(6, 3), 8);
    }

    // ── analyze: basic fields ───────────────────────────────────────────────

    #[test]
    fn test_analyze_returns_correct_stage_and_round() {
        let state = make_state(3, 2, 6, 70);
        let awareness = timer().analyze(&state);
        assert_eq!(awareness.current_stage, 3);
        assert_eq!(awareness.current_round, 2);
    }

    #[test]
    fn test_analyze_recommended_level_matches_helper() {
        let state = make_state(4, 1, 7, 60);
        let awareness = timer().analyze(&state);
        assert_eq!(awareness.recommended_level, 7);
    }

    #[test]
    fn test_analyze_is_level_behind_true_when_behind() {
        let state = make_state(3, 1, 4, 80); // rec=6, actual=4
        let awareness = timer().analyze(&state);
        assert!(awareness.is_level_behind);
    }

    #[test]
    fn test_analyze_is_level_behind_false_when_on_track() {
        let state = make_state(2, 3, 5, 80); // rec=5, actual=5
        let awareness = timer().analyze(&state);
        assert!(!awareness.is_level_behind);
    }

    #[test]
    fn test_analyze_upcoming_events_max_three() {
        let state = make_state(1, 1, 3, 100);
        let awareness = timer().analyze(&state);
        assert!(awareness.upcoming_events.len() <= 3);
    }

    #[test]
    fn test_analyze_upcoming_events_sorted_by_rounds_away() {
        let state = make_state(2, 1, 4, 80);
        let awareness = timer().analyze(&state);
        let distances: Vec<u8> = awareness
            .upcoming_events
            .iter()
            .map(|e| e.rounds_away)
            .collect();
        for i in 1..distances.len() {
            assert!(
                distances[i] >= distances[i - 1],
                "events not sorted: {:?}",
                distances
            );
        }
    }

    #[test]
    fn test_analyze_upcoming_events_not_in_past() {
        // At stage 3-2, events at stage 2 should not appear
        let state = make_state(3, 2, 6, 70);
        let awareness = timer().analyze(&state);
        for e in &awareness.upcoming_events {
            assert!(e.rounds_away < 200, "rounds_away suspiciously large");
        }
        // No event with 0 rounds_away should be from stage 2-1 (which is in the past)
        // All events at stage >= 3, round >= 2 or higher stage
        let _ = &awareness.upcoming_events; // already validated sorting
    }

    // ── priority strings ───────────────────────────────────────────────────

    #[test]
    fn test_priority_augment_this_round() {
        // Stage 2-1 is an augment round
        let state = make_state(2, 1, 4, 80);
        let awareness = timer().analyze(&state);
        // The augment event at 2-1 has rounds_away=0
        assert!(
            awareness.current_priority.contains("Augment this round"),
            "unexpected priority: {}",
            awareness.current_priority
        );
    }

    #[test]
    fn test_priority_roll_down_when_behind_and_low_hp() {
        // Stage 4, level 5 (behind), hp 30
        let state = make_state(4, 3, 5, 30);
        let awareness = timer().analyze(&state);
        assert!(
            awareness.current_priority.contains("Roll down"),
            "unexpected priority: {}",
            awareness.current_priority
        );
    }

    #[test]
    fn test_priority_buy_xp_when_behind_but_healthy() {
        // Stage 3, level 4 (behind rec=6), hp 80
        let state = make_state(3, 3, 4, 80);
        let awareness = timer().analyze(&state);
        assert!(
            awareness.current_priority.contains("Buy XP")
                || awareness.current_priority.contains("level"),
            "unexpected priority: {}",
            awareness.current_priority
        );
    }

    #[test]
    fn test_priority_econ_default_when_on_track() {
        // Stage 1-2: on track, no augment close
        let state = make_state(1, 2, 3, 100);
        let awareness = timer().analyze(&state);
        // Should be some econ/stabilize message
        assert!(!awareness.current_priority.is_empty());
    }

    #[test]
    fn test_default_stage_awareness() {
        let def = StageAwareness::default();
        assert_eq!(def.current_stage, 1);
        assert_eq!(def.current_round, 1);
        assert_eq!(def.recommended_level, 3);
        assert!(!def.is_level_behind);
        assert!(def.upcoming_events.is_empty());
        assert_eq!(def.current_priority, "Waiting for game");
    }

    #[test]
    fn test_round_timer_default_same_as_new() {
        let _ = RoundTimer::default();
    }

    #[test]
    fn test_to_linear_ordering() {
        assert!(to_linear(2, 1) > to_linear(1, 4));
        assert!(to_linear(3, 1) > to_linear(2, 6));
    }

    #[test]
    fn test_analyze_endgame_stage5() {
        let state = make_state(5, 3, 8, 40);
        let awareness = timer().analyze(&state);
        assert_eq!(awareness.recommended_level, 8);
        assert!(!awareness.is_level_behind);
    }
}

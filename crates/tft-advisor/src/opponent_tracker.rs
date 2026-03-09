//! # Stage: OpponentTracker
//!
//! ## Responsibility
//! Analyze visible opponent boards to identify contested comps and flag threat levels.
//!
//! ## Guarantees
//! - Deterministic: same state + catalog produces same output
//! - Non-panicking: all operations via Result
//! - Thread-safe: OpponentTracker holds no mutable state

use tft_data::Catalog;
use tft_types::{GameState, TftError};

/// Threat level from a specific opponent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreatLevel {
    /// Opponent HP >= 70: still strong, likely to fight multiple more rounds.
    High,
    /// Opponent 30 < HP < 70: moderate threat.
    Medium,
    /// Opponent HP <= 30: likely to be eliminated soon.
    Low,
}

/// Analysis of a single opponent.
#[derive(Debug, Clone)]
pub struct OpponentAnalysis {
    pub player_name: String,
    pub hp: u8,
    pub threat_level: ThreatLevel,
    pub contested_traits: Vec<String>,
    pub summary: String,
}

/// Summary of the lobby state.
#[derive(Debug, Clone, Default)]
pub struct LobbyAnalysis {
    pub opponents: Vec<OpponentAnalysis>,
    /// Trait names contested by 2 or more opponents.
    pub contested_comps: Vec<String>,
    /// Suggest a pivot if your most important trait is heavily contested.
    pub recommended_pivot: Option<String>,
}

/// Analyzes the current lobby state to surface threat levels and contested comps.
pub struct OpponentTracker;

impl OpponentTracker {
    pub fn new() -> Self {
        Self
    }

    /// Classify an HP value into a threat level.
    ///
    /// - `hp >= 70` → `High`
    /// - `30 < hp < 70` → `Medium`
    /// - `hp <= 30` → `Low`
    fn classify_threat(hp: u8) -> ThreatLevel {
        if hp >= 70 {
            ThreatLevel::High
        } else if hp > 30 {
            ThreatLevel::Medium
        } else {
            ThreatLevel::Low
        }
    }

    /// Identify which of `your_traits` also appear in `opponent_traits`.
    fn find_contested(your_traits: &[String], opponent_traits: &[String]) -> Vec<String> {
        your_traits
            .iter()
            .filter(|t| opponent_traits.contains(t))
            .cloned()
            .collect()
    }

    /// Build a human-readable summary for a single opponent.
    fn build_summary(name: &str, threat: &ThreatLevel, contested: &[String]) -> String {
        let threat_str = match threat {
            ThreatLevel::High => "high threat",
            ThreatLevel::Medium => "medium threat",
            ThreatLevel::Low => "low threat",
        };
        if contested.is_empty() {
            format!("{} is {} (no contested traits)", name, threat_str)
        } else {
            format!(
                "{} is {} — contested: {}",
                name,
                threat_str,
                contested.join(", ")
            )
        }
    }

    /// Analyze the current lobby given the game state.
    ///
    /// # Arguments
    /// * `state` — current observable game state (includes opponent snapshots)
    /// * `_catalog` — game data catalog (reserved for future use)
    ///
    /// # Returns
    /// - `Ok(LobbyAnalysis)` — full lobby breakdown
    /// - `Err(TftError)` — on unexpected failure (currently infallible, future-proof)
    ///
    /// # Panics
    /// This function never panics.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use tft_advisor::OpponentTracker;
    /// # let state = tft_types::GameState::default();
    /// # let catalog = tft_data::Catalog::global().unwrap();
    /// let analysis = OpponentTracker::new().analyze_lobby(&state, catalog)?;
    /// # Ok::<(), tft_types::TftError>(())
    /// ```
    pub fn analyze_lobby(
        &self,
        state: &GameState,
        _catalog: &Catalog,
    ) -> Result<LobbyAnalysis, TftError> {
        let your_traits: Vec<String> = state
            .active_traits
            .iter()
            .map(|(name, _)| name.clone())
            .collect();

        // Build per-opponent analysis
        let mut opponent_analyses: Vec<OpponentAnalysis> = Vec::new();
        for opp in &state.opponents {
            let threat = Self::classify_threat(opp.hp);
            let contested = Self::find_contested(&your_traits, &opp.active_traits);
            let summary = Self::build_summary(&opp.player_name, &threat, &contested);
            opponent_analyses.push(OpponentAnalysis {
                player_name: opp.player_name.clone(),
                hp: opp.hp,
                threat_level: threat,
                contested_traits: contested,
                summary,
            });
        }

        // Count how many opponents contest each trait
        let mut trait_contest_count: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for analysis in &opponent_analyses {
            for trait_name in &analysis.contested_traits {
                *trait_contest_count.entry(trait_name.clone()).or_insert(0) += 1;
            }
        }

        let mut contested_comps: Vec<String> = trait_contest_count
            .iter()
            .filter(|(_, &count)| count >= 2)
            .map(|(name, _)| name.clone())
            .collect();
        contested_comps.sort();

        // Recommend pivot if your most prominent trait is contested by 3+ players
        let recommended_pivot = your_traits
            .iter()
            .find(|trait_name| trait_contest_count.get(*trait_name).copied().unwrap_or(0) >= 3)
            .map(|trait_name| format!("Consider pivoting away from {}", trait_name));

        Ok(LobbyAnalysis {
            opponents: opponent_analyses,
            contested_comps,
            recommended_pivot,
        })
    }
}

impl Default for OpponentTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_types::{
        AugmentId, ChampionId, ChampionSlot, GameState, OpponentSnapshot, RoundInfo, StarLevel,
    };

    fn make_catalog() -> &'static tft_data::Catalog {
        tft_data::Catalog::global().expect("catalog failed in test")
    }

    fn make_state(active_traits: Vec<(String, u8)>, opponents: Vec<OpponentSnapshot>) -> GameState {
        GameState {
            round: RoundInfo { stage: 3, round: 1 },
            board: vec![ChampionSlot {
                champion_id: ChampionId(0),
                star_level: StarLevel::One,
                items: vec![],
            }],
            bench: vec![],
            shop: vec![],
            gold: 30,
            hp: 80,
            level: 6,
            xp: 0,
            streak: 0,
            current_augments: vec![AugmentId(0)],
            augment_choices: None,
            active_traits,
            opponents,
        }
    }

    fn make_opponent(name: &str, hp: u8, traits: Vec<&str>) -> OpponentSnapshot {
        OpponentSnapshot {
            player_name: name.to_string(),
            hp,
            level: 6,
            board_champions: vec![],
            active_traits: traits.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    // ── Basic construction and empty state ───────────────────────────────────

    #[test]
    fn test_opponent_tracker_new_succeeds() {
        let _ = OpponentTracker::new();
    }

    #[test]
    fn test_opponent_tracker_default_equals_new() {
        let _ = OpponentTracker::default();
    }

    #[test]
    fn test_analyze_lobby_empty_opponents_returns_ok() {
        let state = make_state(vec![], vec![]);
        let result = OpponentTracker::new().analyze_lobby(&state, make_catalog());
        assert!(result.is_ok());
    }

    #[test]
    fn test_analyze_lobby_empty_opponents_gives_empty_analysis() {
        let state = make_state(vec![], vec![]);
        let analysis = OpponentTracker::new()
            .analyze_lobby(&state, make_catalog())
            .expect("analyze failed in test");
        assert!(analysis.opponents.is_empty());
        assert!(analysis.contested_comps.is_empty());
        assert!(analysis.recommended_pivot.is_none());
    }

    // ── Threat level classification ──────────────────────────────────────────

    #[test]
    fn test_threat_level_high_above_70() {
        let state = make_state(vec![], vec![make_opponent("A", 80, vec![])]);
        let analysis = OpponentTracker::new()
            .analyze_lobby(&state, make_catalog())
            .expect("analyze failed in test");
        assert_eq!(analysis.opponents[0].threat_level, ThreatLevel::High);
    }

    #[test]
    fn test_threat_level_high_at_71() {
        let state = make_state(vec![], vec![make_opponent("A", 71, vec![])]);
        let analysis = OpponentTracker::new()
            .analyze_lobby(&state, make_catalog())
            .expect("analyze failed in test");
        assert_eq!(analysis.opponents[0].threat_level, ThreatLevel::High);
    }

    #[test]
    fn test_threat_level_high_at_70() {
        // hp=70 exactly is High (boundary is inclusive: hp >= 70 → High)
        let state = make_state(vec![], vec![make_opponent("A", 70, vec![])]);
        let analysis = OpponentTracker::new()
            .analyze_lobby(&state, make_catalog())
            .expect("analyze failed in test");
        assert_eq!(analysis.opponents[0].threat_level, ThreatLevel::High);
    }

    #[test]
    fn test_threat_level_medium_at_50() {
        let state = make_state(vec![], vec![make_opponent("A", 50, vec![])]);
        let analysis = OpponentTracker::new()
            .analyze_lobby(&state, make_catalog())
            .expect("analyze failed in test");
        assert_eq!(analysis.opponents[0].threat_level, ThreatLevel::Medium);
    }

    #[test]
    fn test_threat_level_medium_at_31() {
        let state = make_state(vec![], vec![make_opponent("A", 31, vec![])]);
        let analysis = OpponentTracker::new()
            .analyze_lobby(&state, make_catalog())
            .expect("analyze failed in test");
        assert_eq!(analysis.opponents[0].threat_level, ThreatLevel::Medium);
    }

    #[test]
    fn test_threat_level_low_at_30() {
        let state = make_state(vec![], vec![make_opponent("A", 30, vec![])]);
        let analysis = OpponentTracker::new()
            .analyze_lobby(&state, make_catalog())
            .expect("analyze failed in test");
        assert_eq!(analysis.opponents[0].threat_level, ThreatLevel::Low);
    }

    #[test]
    fn test_threat_level_low_at_0() {
        let state = make_state(vec![], vec![make_opponent("A", 0, vec![])]);
        let analysis = OpponentTracker::new()
            .analyze_lobby(&state, make_catalog())
            .expect("analyze failed in test");
        assert_eq!(analysis.opponents[0].threat_level, ThreatLevel::Low);
    }

    // ── Contested trait detection ────────────────────────────────────────────

    #[test]
    fn test_contested_trait_detected_when_opponent_shares_trait() {
        let state = make_state(
            vec![("Gunner".to_string(), 2)],
            vec![make_opponent("Bob", 60, vec!["Gunner"])],
        );
        let analysis = OpponentTracker::new()
            .analyze_lobby(&state, make_catalog())
            .expect("analyze failed in test");
        assert!(analysis.opponents[0]
            .contested_traits
            .contains(&"Gunner".to_string()));
    }

    #[test]
    fn test_no_contested_trait_when_opponent_different_comp() {
        let state = make_state(
            vec![("Gunner".to_string(), 2)],
            vec![make_opponent("Carol", 60, vec!["Arcanist"])],
        );
        let analysis = OpponentTracker::new()
            .analyze_lobby(&state, make_catalog())
            .expect("analyze failed in test");
        assert!(analysis.opponents[0].contested_traits.is_empty());
    }

    #[test]
    fn test_multiple_contested_traits() {
        let state = make_state(
            vec![("Gunner".to_string(), 2), ("Arcanist".to_string(), 2)],
            vec![make_opponent(
                "Dan",
                55,
                vec!["Gunner", "Arcanist", "Invoker"],
            )],
        );
        let analysis = OpponentTracker::new()
            .analyze_lobby(&state, make_catalog())
            .expect("analyze failed in test");
        let contested = &analysis.opponents[0].contested_traits;
        assert!(contested.contains(&"Gunner".to_string()));
        assert!(contested.contains(&"Arcanist".to_string()));
        assert!(!contested.contains(&"Invoker".to_string()));
    }

    // ── contested_comps (2+ opponents) ───────────────────────────────────────

    #[test]
    fn test_contested_comps_flagged_when_two_opponents_share_trait() {
        let state = make_state(
            vec![("Gunner".to_string(), 2)],
            vec![
                make_opponent("P1", 60, vec!["Gunner"]),
                make_opponent("P2", 50, vec!["Gunner"]),
            ],
        );
        let analysis = OpponentTracker::new()
            .analyze_lobby(&state, make_catalog())
            .expect("analyze failed in test");
        assert!(analysis.contested_comps.contains(&"Gunner".to_string()));
    }

    #[test]
    fn test_contested_comps_not_flagged_for_single_opponent() {
        let state = make_state(
            vec![("Gunner".to_string(), 2)],
            vec![make_opponent("P1", 60, vec!["Gunner"])],
        );
        let analysis = OpponentTracker::new()
            .analyze_lobby(&state, make_catalog())
            .expect("analyze failed in test");
        assert!(!analysis.contested_comps.contains(&"Gunner".to_string()));
    }

    // ── Pivot recommendation ─────────────────────────────────────────────────

    #[test]
    fn test_pivot_recommended_when_trait_contested_by_3_or_more() {
        let state = make_state(
            vec![("Arcanist".to_string(), 4)],
            vec![
                make_opponent("P1", 60, vec!["Arcanist"]),
                make_opponent("P2", 55, vec!["Arcanist"]),
                make_opponent("P3", 70, vec!["Arcanist"]),
            ],
        );
        let analysis = OpponentTracker::new()
            .analyze_lobby(&state, make_catalog())
            .expect("analyze failed in test");
        assert!(analysis.recommended_pivot.is_some());
        let pivot = analysis.recommended_pivot.as_deref().unwrap_or("");
        assert!(pivot.contains("Arcanist"));
    }

    #[test]
    fn test_no_pivot_when_only_two_opponents_contest_trait() {
        let state = make_state(
            vec![("Arcanist".to_string(), 4)],
            vec![
                make_opponent("P1", 60, vec!["Arcanist"]),
                make_opponent("P2", 55, vec!["Arcanist"]),
            ],
        );
        let analysis = OpponentTracker::new()
            .analyze_lobby(&state, make_catalog())
            .expect("analyze failed in test");
        assert!(analysis.recommended_pivot.is_none());
    }

    // ── Summary field ────────────────────────────────────────────────────────

    #[test]
    fn test_opponent_summary_not_empty() {
        let state = make_state(
            vec![("Gunner".to_string(), 2)],
            vec![make_opponent("Eve", 75, vec!["Gunner"])],
        );
        let analysis = OpponentTracker::new()
            .analyze_lobby(&state, make_catalog())
            .expect("analyze failed in test");
        assert!(!analysis.opponents[0].summary.is_empty());
    }

    #[test]
    fn test_opponent_summary_includes_player_name() {
        let state = make_state(vec![], vec![make_opponent("UniquePlayerXYZ", 50, vec![])]);
        let analysis = OpponentTracker::new()
            .analyze_lobby(&state, make_catalog())
            .expect("analyze failed in test");
        assert!(analysis.opponents[0].summary.contains("UniquePlayerXYZ"));
    }

    // ── Lobby analysis default ───────────────────────────────────────────────

    #[test]
    fn test_lobby_analysis_default_is_empty() {
        let d = LobbyAnalysis::default();
        assert!(d.opponents.is_empty());
        assert!(d.contested_comps.is_empty());
        assert!(d.recommended_pivot.is_none());
    }
}

//! Builds human-readable explanations for augment recommendations.

use tft_data::Catalog;
use tft_types::{AugmentId, GameState};

/// Generate a reasoning string for why an augment was recommended.
pub fn explain_augment(id: AugmentId, score: f32, state: &GameState, catalog: &Catalog) -> String {
    let name = catalog
        .augment_by_id(id)
        .map(|a| a.name.as_str())
        .unwrap_or("Unknown");

    let tags = catalog
        .augment_by_id(id)
        .map(|a| a.tags.clone())
        .unwrap_or_default();

    let mut reasons = Vec::new();

    // Check tag-based reasons
    if tags.iter().any(|t| t == "econ" || t == "early") && state.round.stage <= 2 {
        reasons.push("strong in early stages".to_string());
    }
    if tags.iter().any(|t| t == "scaling" || t == "late") && state.round.stage >= 4 {
        reasons.push("scales well into late game".to_string());
    }
    if tags.iter().any(|t| t == "AP") {
        let arcanist_count = state
            .active_traits
            .iter()
            .find(|(trait_name, _)| trait_name == "Arcanist")
            .map(|(_, c)| *c)
            .unwrap_or(0);
        if arcanist_count >= 2 {
            reasons.push(format!("synergizes with your {} Arcanists", arcanist_count));
        }
    }
    if tags.iter().any(|t| t == "comeback") && state.hp < 40 {
        reasons.push(format!("good comeback option at {}hp", state.hp));
    }
    if tags.iter().any(|t| t == "items") {
        reasons.push("provides item flexibility".to_string());
    }

    let score_label = if score > 0.7 {
        "strong"
    } else if score > 0.5 {
        "solid"
    } else {
        "situational"
    };

    if reasons.is_empty() {
        format!("{}: {} pick (score: {:.2})", name, score_label, score)
    } else {
        format!("{}: {} — {}", name, score_label, reasons.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_types::GameState;

    fn catalog() -> Catalog {
        Catalog::from_embedded().expect("catalog init failed in test")
    }

    #[test]
    fn test_explain_augment_returns_string() {
        let cat = catalog();
        let state = GameState::default();
        let result = explain_augment(AugmentId(0), 0.8, &state, &cat);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_explain_augment_includes_name() {
        let cat = catalog();
        let state = GameState::default();
        let result = explain_augment(AugmentId(0), 0.8, &state, &cat);
        // AugmentId(0) = "Blue Battery"
        assert!(
            result.contains("Blue Battery"),
            "expected name in: {}",
            result
        );
    }

    #[test]
    fn test_explain_augment_unknown_id_graceful() {
        let cat = catalog();
        let state = GameState::default();
        let result = explain_augment(AugmentId(200), 0.5, &state, &cat);
        assert!(result.contains("Unknown"));
    }

    #[test]
    fn test_explain_augment_low_hp_comeback() {
        let cat = catalog();
        let mut state = GameState::default();
        state.hp = 20;
        // Last Stand has comeback tag
        let last_stand_id = cat
            .augment_id_by_name("Last Stand")
            .expect("Last Stand not found in test");
        let result = explain_augment(last_stand_id, 0.9, &state, &cat);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_explain_score_label_strong() {
        let cat = catalog();
        let state = GameState::default();
        let result = explain_augment(AugmentId(0), 0.85, &state, &cat);
        assert!(
            result.contains("strong"),
            "expected 'strong' in: {}",
            result
        );
    }

    #[test]
    fn test_explain_score_label_situational() {
        let cat = catalog();
        let state = GameState::default();
        let result = explain_augment(AugmentId(0), 0.3, &state, &cat);
        assert!(
            result.contains("situational"),
            "expected 'situational' in: {}",
            result
        );
    }
}

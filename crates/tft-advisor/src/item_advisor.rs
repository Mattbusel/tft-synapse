//! # Stage: ItemAdvisor
//!
//! ## Responsibility
//! Recommend which items to equip on which champions based on champion traits,
//! role, and item category alignment.
//!
//! ## Guarantees
//! - Deterministic: same state + catalog produces same output
//! - Non-panicking: all operations via Result
//! - O(board_size * items_held) per recommendation call

use tft_data::Catalog;
use tft_types::{ChampionId, GameState, ItemCategory, ItemId, TftError};

/// A single item placement recommendation.
#[derive(Debug, Clone)]
pub struct ItemRecommendation {
    pub item_id: ItemId,
    pub item_name: String,
    pub target_champion_id: Option<ChampionId>,
    pub target_champion_name: Option<String>,
    pub reason: String,
    pub confidence: f32,
}

/// Recommends item assignments for champions on board and bench.
pub struct ItemAdvisor;

impl ItemAdvisor {
    pub fn new() -> Self {
        ItemAdvisor
    }

    /// Produce item placement recommendations for the current game state.
    pub fn advise_items(
        &self,
        state: &GameState,
        catalog: &Catalog,
    ) -> Result<Vec<ItemRecommendation>, TftError> {
        // Collect all champion slots from board and bench.
        let mut all_slots: Vec<(ChampionId, bool)> =
            state.board.iter().map(|s| (s.champion_id, true)).collect();
        let bench_slots: Vec<(ChampionId, bool)> = state
            .bench
            .iter()
            .filter_map(|opt| opt.as_ref().map(|s| (s.champion_id, false)))
            .collect();
        all_slots.extend(bench_slots);

        // Collect held items from bench champions (treat as unequipped / carried).
        let unequipped: Vec<ItemId> = state
            .bench
            .iter()
            .filter_map(|opt| opt.as_ref())
            .flat_map(|slot| slot.items.iter().copied())
            .collect();

        let mut recommendations = Vec::new();

        for item_id in &unequipped {
            let item_def = catalog
                .item_by_id(*item_id)
                .ok_or_else(|| TftError::Catalog(format!("Item {:?} not found", item_id)))?;

            let preferred_traits = preferred_traits_for_category(&item_def.category);

            // Find the best board champion for this item.
            let mut best_champ: Option<ChampionId> = None;
            let mut best_score: f32 = -1.0;

            for &(champ_id, on_board) in &all_slots {
                if !on_board {
                    continue;
                }
                let champ_def = match catalog.champion_by_id(champ_id) {
                    Some(d) => d,
                    None => continue,
                };

                let score =
                    score_champion_for_item(champ_def, &preferred_traits, &item_def.category);
                if score > best_score {
                    best_score = score;
                    best_champ = Some(champ_id);
                }
            }

            let (target_name, reason) = if let Some(cid) = best_champ {
                let name = catalog
                    .champion_by_id(cid)
                    .map(|d| d.name.clone())
                    .unwrap_or_else(|| format!("Champion {:?}", cid));
                let r = build_reason(&item_def.name, &name, &item_def.category, &preferred_traits);
                (Some(name), r)
            } else {
                (
                    None,
                    format!("No board champion found for {}", item_def.name),
                )
            };

            let confidence = if best_score > 0.0 {
                (best_score / 3.0_f32).min(1.0)
            } else {
                0.1
            };

            recommendations.push(ItemRecommendation {
                item_id: *item_id,
                item_name: item_def.name.clone(),
                target_champion_id: best_champ,
                target_champion_name: target_name,
                reason,
                confidence,
            });
        }

        Ok(recommendations)
    }
}

impl Default for ItemAdvisor {
    fn default() -> Self {
        ItemAdvisor::new()
    }
}

/// Returns the preferred trait keywords for each item category.
fn preferred_traits_for_category(category: &ItemCategory) -> Vec<&'static str> {
    match category {
        ItemCategory::AbilityPower | ItemCategory::Mana => {
            vec!["Arcanist", "Mage", "Scholar"]
        }
        ItemCategory::AttackDamage | ItemCategory::CriticalStrike | ItemCategory::AttackSpeed => {
            vec!["Gunner", "Marksman", "Duelist"]
        }
        ItemCategory::Tank => {
            vec!["Bruiser", "Colossus", "Juggernaut", "Guardian"]
        }
        ItemCategory::Healing | ItemCategory::Utility => {
            vec![] // handled by fallback (highest cost)
        }
    }
}

/// Score a champion for receiving a particular item.
fn score_champion_for_item(
    champ: &tft_types::ChampionDef,
    preferred_traits: &[&str],
    category: &ItemCategory,
) -> f32 {
    if preferred_traits.is_empty() {
        // Utility/Healing: prefer highest-cost champion.
        return champ.cost.as_u8() as f32;
    }

    let trait_matches = champ
        .traits
        .iter()
        .filter(|t| preferred_traits.contains(&t.as_str()))
        .count() as f32;

    // Tank items also favor lower-cost (frontline) champions.
    if matches!(category, ItemCategory::Tank) {
        let cost_bonus = (6 - champ.cost.as_u8()) as f32 * 0.3;
        trait_matches + cost_bonus
    } else {
        trait_matches
    }
}

fn build_reason(
    item_name: &str,
    champ_name: &str,
    category: &ItemCategory,
    preferred_traits: &[&str],
) -> String {
    if preferred_traits.is_empty() {
        format!(
            "{} is a flexible item — placing on {} (highest cost carry)",
            item_name, champ_name
        )
    } else {
        format!(
            "{} suits {} — aligns with {:?} preferred traits: {}",
            item_name,
            champ_name,
            category,
            preferred_traits.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_data::Catalog;
    use tft_types::{ChampionId, ChampionSlot, RoundInfo, StarLevel};

    fn make_catalog() -> Catalog {
        Catalog::from_embedded().expect("catalog init failed in test")
    }

    fn empty_state() -> GameState {
        GameState {
            round: RoundInfo { stage: 2, round: 1 },
            board: vec![],
            bench: vec![],
            shop: vec![],
            gold: 20,
            hp: 80,
            level: 5,
            xp: 0,
            streak: 0,
            current_augments: vec![],
            augment_choices: None,
            active_traits: vec![],
            opponents: vec![],
        }
    }

    #[test]
    fn test_item_advisor_new() {
        let _ = ItemAdvisor::new();
    }

    #[test]
    fn test_item_advisor_default() {
        let _ = ItemAdvisor::default();
    }

    #[test]
    fn test_advise_items_empty_state_returns_empty() {
        let advisor = ItemAdvisor::new();
        let catalog = make_catalog();
        let state = empty_state();
        let result = advisor.advise_items(&state, &catalog);
        assert!(result.is_ok());
        assert!(result.expect("failed in test").is_empty());
    }

    #[test]
    fn test_preferred_traits_ap_includes_arcanist() {
        let traits = preferred_traits_for_category(&ItemCategory::AbilityPower);
        assert!(traits.contains(&"Arcanist"));
        assert!(traits.contains(&"Mage"));
    }

    #[test]
    fn test_preferred_traits_mana_includes_mage() {
        let traits = preferred_traits_for_category(&ItemCategory::Mana);
        assert!(traits.contains(&"Mage"));
        assert!(traits.contains(&"Scholar"));
    }

    #[test]
    fn test_preferred_traits_ad_includes_gunner() {
        let traits = preferred_traits_for_category(&ItemCategory::AttackDamage);
        assert!(traits.contains(&"Gunner"));
    }

    #[test]
    fn test_preferred_traits_crit_includes_marksman() {
        let traits = preferred_traits_for_category(&ItemCategory::CriticalStrike);
        assert!(traits.contains(&"Marksman"));
    }

    #[test]
    fn test_preferred_traits_as_includes_duelist() {
        let traits = preferred_traits_for_category(&ItemCategory::AttackSpeed);
        assert!(traits.contains(&"Duelist"));
    }

    #[test]
    fn test_preferred_traits_tank_includes_bruiser() {
        let traits = preferred_traits_for_category(&ItemCategory::Tank);
        assert!(traits.contains(&"Bruiser"));
        assert!(traits.contains(&"Guardian"));
    }

    #[test]
    fn test_preferred_traits_utility_is_empty() {
        let traits = preferred_traits_for_category(&ItemCategory::Utility);
        assert!(traits.is_empty());
    }

    #[test]
    fn test_preferred_traits_healing_is_empty() {
        let traits = preferred_traits_for_category(&ItemCategory::Healing);
        assert!(traits.is_empty());
    }

    #[test]
    fn test_advise_items_bench_item_produces_recommendation() {
        let advisor = ItemAdvisor::new();
        let catalog = make_catalog();
        // Find a valid item id and champion id from the catalog.
        let item_id = ItemId(0); // B.F. Sword
        let champ_id = ChampionId(0);

        let mut state = empty_state();
        // Board champion to receive the item.
        state.board.push(ChampionSlot {
            champion_id: champ_id,
            star_level: StarLevel::One,
            items: vec![],
        });
        // Bench champion carrying the item.
        state.bench.push(Some(ChampionSlot {
            champion_id: ChampionId(1),
            star_level: StarLevel::One,
            items: vec![item_id],
        }));

        let result = advisor.advise_items(&state, &catalog);
        assert!(result.is_ok(), "advise_items failed: {:?}", result.err());
        let recs = result.expect("failed in test");
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].item_id, item_id);
    }

    #[test]
    fn test_advise_items_recommendation_has_name() {
        let advisor = ItemAdvisor::new();
        let catalog = make_catalog();
        let item_id = ItemId(0);
        let mut state = empty_state();
        state.board.push(ChampionSlot {
            champion_id: ChampionId(0),
            star_level: StarLevel::One,
            items: vec![],
        });
        state.bench.push(Some(ChampionSlot {
            champion_id: ChampionId(1),
            star_level: StarLevel::One,
            items: vec![item_id],
        }));
        let recs = advisor
            .advise_items(&state, &catalog)
            .expect("failed in test");
        assert!(!recs[0].item_name.is_empty());
    }

    #[test]
    fn test_advise_items_confidence_range() {
        let advisor = ItemAdvisor::new();
        let catalog = make_catalog();
        let item_id = ItemId(0);
        let mut state = empty_state();
        state.board.push(ChampionSlot {
            champion_id: ChampionId(0),
            star_level: StarLevel::One,
            items: vec![],
        });
        state.bench.push(Some(ChampionSlot {
            champion_id: ChampionId(1),
            star_level: StarLevel::One,
            items: vec![item_id],
        }));
        let recs = advisor
            .advise_items(&state, &catalog)
            .expect("failed in test");
        for rec in &recs {
            assert!(
                rec.confidence >= 0.0 && rec.confidence <= 1.0,
                "confidence {} out of range",
                rec.confidence
            );
        }
    }

    #[test]
    fn test_advise_items_reason_not_empty() {
        let advisor = ItemAdvisor::new();
        let catalog = make_catalog();
        let item_id = ItemId(0);
        let mut state = empty_state();
        state.board.push(ChampionSlot {
            champion_id: ChampionId(0),
            star_level: StarLevel::One,
            items: vec![],
        });
        state.bench.push(Some(ChampionSlot {
            champion_id: ChampionId(1),
            star_level: StarLevel::One,
            items: vec![item_id],
        }));
        let recs = advisor
            .advise_items(&state, &catalog)
            .expect("failed in test");
        for rec in &recs {
            assert!(!rec.reason.is_empty());
        }
    }

    #[test]
    fn test_advise_items_no_board_champion_target_is_none() {
        let advisor = ItemAdvisor::new();
        let catalog = make_catalog();
        let item_id = ItemId(0);
        let mut state = empty_state();
        // No board champions, only bench.
        state.bench.push(Some(ChampionSlot {
            champion_id: ChampionId(1),
            star_level: StarLevel::One,
            items: vec![item_id],
        }));
        let recs = advisor
            .advise_items(&state, &catalog)
            .expect("failed in test");
        assert_eq!(recs.len(), 1);
        assert!(recs[0].target_champion_id.is_none());
    }

    #[test]
    fn test_score_champion_utility_prefers_high_cost() {
        use tft_types::{ChampionDef, Cost};
        let champ_low = ChampionDef {
            id: ChampionId(0),
            name: "Low".to_string(),
            cost: Cost::One,
            traits: vec![],
        };
        let champ_high = ChampionDef {
            id: ChampionId(1),
            name: "High".to_string(),
            cost: Cost::Five,
            traits: vec![],
        };
        let s_low = score_champion_for_item(&champ_low, &[], &ItemCategory::Utility);
        let s_high = score_champion_for_item(&champ_high, &[], &ItemCategory::Utility);
        assert!(s_high > s_low);
    }
}

//! Feature extraction: converts GameState to a fixed-size f32 feature vector.
//!
//! FEATURE_DIM layout:
//!   [0..N_CHAMP]       board champion multi-hot
//!   [N..2N]            bench champion multi-hot
//!   [2N..7N]           shop slots (5 x one-hot per slot)
//!   [7N..7N+N_AUG]     current augments multi-hot
//!   [..]               active traits (normalized counts)
//!   [..]               6 scalar features: gold, hp, level, round, streak, xp

use crate::encoder::{encode_augments, encode_traits, multi_hot, one_hot};
use crate::normalizer::*;
use std::collections::HashMap;
use tft_data::Catalog;
use tft_types::{GameState, TftError};

/// Total feature vector dimension (upper bound for allocation hints).
pub const FEATURE_DIM: usize = 512;

/// Extracts features from a GameState into a fixed-size f32 vector.
pub struct FeatureExtractor {
    pub n_champions: usize,
    pub n_augments: usize,
    pub n_traits: usize,
    pub trait_index: HashMap<String, usize>,
}

impl FeatureExtractor {
    /// Create a new extractor from a catalog.
    pub fn from_catalog(catalog: &Catalog) -> Self {
        let trait_index: HashMap<String, usize> = catalog
            .traits
            .iter()
            .enumerate()
            .map(|(i, t)| (t.name.clone(), i))
            .collect();
        Self {
            n_champions: catalog.champion_count().min(64),
            n_augments: catalog.augment_count().min(64),
            n_traits: catalog.traits.len().min(32),
            trait_index,
        }
    }

    /// Returns the actual feature dimension for this extractor.
    pub fn dim(&self) -> usize {
        // board + bench + shop(5 slots) + augments + traits + 6 scalars
        self.n_champions      // board
        + self.n_champions    // bench
        + 5 * self.n_champions // shop (5 slots, each one-hot over champions)
        + self.n_augments     // current augments
        + self.n_traits       // active traits
        + 6 // gold, hp, level, round, streak, xp
    }

    /// Extract a feature vector from a GameState.
    /// Returns Err if the state is clearly malformed.
    pub fn extract(&self, state: &GameState) -> Result<Vec<f32>, TftError> {
        let mut features = Vec::with_capacity(self.dim());

        // Board: multi-hot over champion ids
        let board_ids: Vec<usize> = state
            .board
            .iter()
            .map(|s| s.champion_id.0 as usize)
            .collect();
        multi_hot(&mut features, &board_ids, self.n_champions);

        // Bench: multi-hot over champion ids (None slots contribute nothing)
        let bench_ids: Vec<usize> = state
            .bench
            .iter()
            .filter_map(|s| s.as_ref().map(|c| c.champion_id.0 as usize))
            .collect();
        multi_hot(&mut features, &bench_ids, self.n_champions);

        // Shop: 5 slots, each one-hot over champion ids
        for i in 0..5 {
            let idx = state
                .shop
                .get(i)
                .and_then(|s| s.champion_id)
                .map(|c| c.0 as usize)
                .unwrap_or(self.n_champions); // out-of-bounds = no champion (all zeros)
            one_hot(&mut features, idx, self.n_champions);
        }

        // Current augments: multi-hot
        encode_augments(&mut features, &state.current_augments, self.n_augments);

        // Active traits: normalized count vector
        encode_traits(
            &mut features,
            &state.active_traits,
            &self.trait_index,
            self.n_traits,
        );

        // Scalars
        features.push(normalize_gold(state.gold));
        features.push(normalize_hp(state.hp));
        features.push(normalize_level(state.level));
        features.push(normalize_round(state.round.stage, state.round.round));
        features.push(normalize_streak(state.streak));
        features.push(normalize_xp(state.xp));

        Ok(features)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_types::{
        AugmentId, ChampionId, ChampionSlot, GameState, RoundInfo, ShopSlot, StarLevel,
    };

    fn make_extractor() -> FeatureExtractor {
        let catalog = Catalog::from_embedded().expect("catalog init failed in test");
        FeatureExtractor::from_catalog(&catalog)
    }

    fn make_state() -> GameState {
        GameState {
            round: RoundInfo { stage: 3, round: 2 },
            board: vec![
                ChampionSlot {
                    champion_id: ChampionId(0),
                    star_level: StarLevel::Two,
                    items: vec![],
                },
                ChampionSlot {
                    champion_id: ChampionId(1),
                    star_level: StarLevel::One,
                    items: vec![],
                },
            ],
            bench: vec![
                None,
                None,
                Some(ChampionSlot {
                    champion_id: ChampionId(2),
                    star_level: StarLevel::One,
                    items: vec![],
                }),
            ],
            shop: vec![
                ShopSlot {
                    champion_id: Some(ChampionId(3)),
                    cost: 2,
                    locked: false,
                    sold: false,
                },
                ShopSlot {
                    champion_id: None,
                    cost: 0,
                    locked: false,
                    sold: false,
                },
                ShopSlot {
                    champion_id: None,
                    cost: 0,
                    locked: false,
                    sold: false,
                },
                ShopSlot {
                    champion_id: None,
                    cost: 0,
                    locked: false,
                    sold: false,
                },
                ShopSlot {
                    champion_id: None,
                    cost: 0,
                    locked: false,
                    sold: false,
                },
            ],
            gold: 45,
            hp: 72,
            level: 6,
            xp: 30,
            streak: 2,
            current_augments: vec![AugmentId(0)],
            augment_choices: None,
            active_traits: vec![("Arcanist".to_string(), 2)],
            opponents: vec![],
        }
    }

    #[test]
    fn test_extract_returns_correct_dimension() {
        let extractor = make_extractor();
        let state = make_state();
        let features = extractor
            .extract(&state)
            .expect("extraction failed in test");
        assert_eq!(
            features.len(),
            extractor.dim(),
            "feature vector length mismatch"
        );
    }

    #[test]
    fn test_extract_all_values_in_range() {
        let extractor = make_extractor();
        let state = make_state();
        let features = extractor
            .extract(&state)
            .expect("extraction failed in test");
        for (i, &v) in features.iter().enumerate() {
            assert!(v >= 0.0 && v <= 1.0, "feature[{}] = {} out of [0,1]", i, v);
        }
    }

    #[test]
    fn test_extract_is_deterministic() {
        let extractor = make_extractor();
        let state = make_state();
        let f1 = extractor
            .extract(&state)
            .expect("extraction failed in test");
        let f2 = extractor
            .extract(&state)
            .expect("extraction failed in test");
        assert_eq!(f1, f2, "feature extraction must be deterministic");
    }

    #[test]
    fn test_extract_empty_board_all_zeros_for_board_segment() {
        let extractor = make_extractor();
        let mut state = make_state();
        state.board.clear();
        state.bench = vec![None; 9];
        state.shop = (0..5)
            .map(|_| ShopSlot {
                champion_id: None,
                cost: 0,
                locked: false,
                sold: false,
            })
            .collect();
        state.current_augments.clear();
        state.active_traits.clear();
        let features = extractor
            .extract(&state)
            .expect("extraction failed in test");
        let board_segment = &features[..extractor.n_champions];
        assert!(
            board_segment.iter().all(|&v| v == 0.0),
            "empty board should produce all zeros"
        );
    }

    #[test]
    fn test_extract_known_champion_sets_board_bit() {
        let extractor = make_extractor();
        let state = make_state();
        let features = extractor
            .extract(&state)
            .expect("extraction failed in test");
        // Champion 0 is on board, so features[0] should be 1.0
        assert_eq!(features[0], 1.0, "champion 0 should be on board");
        // Champion 1 is on board
        assert_eq!(features[1], 1.0, "champion 1 should be on board");
    }

    #[test]
    fn test_extract_known_augment_sets_augment_bit() {
        let extractor = make_extractor();
        let state = make_state();
        let features = extractor
            .extract(&state)
            .expect("extraction failed in test");
        // Augment 0 is in current_augments
        // Augment offset = n_champions*7 (board + bench + 5 shop slots)
        let aug_offset = extractor.n_champions * 7;
        assert_eq!(features[aug_offset], 1.0, "augment 0 should be set");
    }

    #[test]
    fn test_extract_gold_scalar_correct() {
        let extractor = make_extractor();
        let mut state = make_state();
        state.gold = 50;
        let features = extractor
            .extract(&state)
            .expect("extraction failed in test");
        let scalar_offset = extractor.dim() - 6;
        let expected = 50.0 / 100.0;
        assert!(
            (features[scalar_offset] - expected).abs() < 1e-5,
            "gold feature mismatch: got {}, expected {}",
            features[scalar_offset],
            expected
        );
    }

    #[test]
    fn test_dim_matches_actual_output_length() {
        let extractor = make_extractor();
        let state = make_state();
        let features = extractor
            .extract(&state)
            .expect("extraction failed in test");
        assert_eq!(features.len(), extractor.dim());
    }
}

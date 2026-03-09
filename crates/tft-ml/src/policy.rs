//! Public API: AugmentPolicy ties together the net, bandit, trainer, and persistence.

use std::path::PathBuf;
use tft_types::{AugmentId, GameState, Placement, StateTransition, TftError};
use tft_data::Catalog;
use tft_game_state::FeatureExtractor;
use crate::model::{ShallowNet, softmax};
use crate::bandit::ThompsonSampling;
use crate::trainer::{ReplayBuffer, mini_batch_update};
use crate::persistence::{save_model, load_model};
use tracing::{info, debug};

const HIDDEN1: usize = 64;
const HIDDEN2: usize = 32;
const BATCH_SIZE: usize = 32;

pub struct AugmentPolicy {
    net: ShallowNet,
    bandit: ThompsonSampling,
    replay: ReplayBuffer,
    extractor: FeatureExtractor,
    n_augments: usize,
    games_trained: u32,
    model_path: PathBuf,
    pending_transitions: Vec<(Vec<f32>, u8)>,
}

impl AugmentPolicy {
    /// Create a new policy with random initialization.
    pub fn new(catalog: &Catalog, model_path: PathBuf) -> Result<Self, TftError> {
        let extractor = FeatureExtractor::from_catalog(catalog);
        let n_augments = catalog.augment_count();
        let input_dim = extractor.dim();
        let net = ShallowNet::new(input_dim, HIDDEN1, HIDDEN2, n_augments);
        let bandit = ThompsonSampling::new(n_augments);
        info!("AugmentPolicy initialized: input_dim={}, n_augments={}", input_dim, n_augments);
        Ok(Self {
            net,
            bandit,
            replay: ReplayBuffer::with_default_capacity(),
            extractor,
            n_augments,
            games_trained: 0,
            model_path,
            pending_transitions: Vec::new(),
        })
    }

    /// Load from saved model file, or initialize fresh if not found.
    pub fn load_or_init(catalog: &Catalog, model_path: PathBuf) -> Result<Self, TftError> {
        let mut policy = Self::new(catalog, model_path.clone())?;
        if model_path.exists() {
            match load_model(&model_path) {
                Ok((net, games)) => {
                    policy.net = net;
                    policy.games_trained = games;
                    info!("Loaded model from {:?} ({} games trained)", model_path, games);
                }
                Err(e) => {
                    info!("Could not load model ({}), starting fresh", e);
                }
            }
        }
        Ok(policy)
    }

    /// Rank augment choices from best to worst. Returns (id, combined_score).
    pub fn rank_augments(
        &mut self,
        state: &GameState,
        choices: &[AugmentId],
    ) -> Result<Vec<(AugmentId, f32)>, TftError> {
        let features = self.extractor.extract(state)?;
        let mut logits = self.net.forward(&features)?;
        softmax(&mut logits);

        let seed = state.round.stage as u64 * 1000 + state.round.round as u64;
        let mut scored: Vec<(AugmentId, f32)> = choices.iter().map(|&id| {
            let idx = id.0 as usize;
            let nn_score = logits.get(idx).copied().unwrap_or(0.0);
            let bandit_score = self.bandit.sample_score(idx, seed + idx as u64).unwrap_or(0.5);
            let combined = self.bandit.combined_score(nn_score, bandit_score);
            (id, combined)
        }).collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Record for training
        if let Some(&(top_id, _)) = scored.first() {
            self.pending_transitions.push((features, top_id.0));
        }

        debug!("Ranked {} augments", scored.len());
        Ok(scored)
    }

    /// Record game outcome and trigger online update.
    pub fn record_game_outcome(
        &mut self,
        placement: Placement,
    ) -> Result<(), TftError> {
        let reward = placement.to_reward();

        for (features, chosen) in self.pending_transitions.drain(..) {
            let transition = StateTransition {
                features,
                augment_chosen: chosen,
                placement: Some(placement),
            };
            self.replay.push(transition);
            self.bandit.update(chosen as usize, reward)?;
        }

        if self.replay.len() >= BATCH_SIZE {
            let seed = self.games_trained as u64;
            let loss = mini_batch_update(&mut self.net, &self.replay, BATCH_SIZE, seed)?;
            debug!("Mini-batch update: avg_reward={:.3}", loss);
        }

        self.games_trained += 1;
        Ok(())
    }

    /// Save model weights to disk.
    pub fn save(&self) -> Result<(), TftError> {
        save_model(&self.net, self.games_trained, &self.model_path)?;
        info!("Model saved to {:?} ({} games)", self.model_path, self.games_trained);
        Ok(())
    }

    pub fn games_trained(&self) -> u32 { self.games_trained }
    pub fn n_augments(&self) -> usize { self.n_augments }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_types::{AugmentId, ChampionSlot, ChampionId, GameState, RoundInfo, StarLevel};
    use std::env::temp_dir;

    fn make_policy() -> AugmentPolicy {
        let catalog = Catalog::from_embedded().expect("catalog init failed in test");
        let path = temp_dir().join("tft_policy_test.json");
        AugmentPolicy::new(&catalog, path).expect("policy init failed in test")
    }

    fn make_state() -> GameState {
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
            augment_choices: None,
            active_traits: vec![],
        }
    }

    #[test]
    fn test_policy_new_succeeds() {
        let _ = make_policy();
    }

    #[test]
    fn test_rank_augments_returns_all_choices() {
        let mut policy = make_policy();
        let state = make_state();
        let choices = vec![AugmentId(0), AugmentId(1), AugmentId(2)];
        let ranked = policy.rank_augments(&state, &choices).expect("rank failed in test");
        assert_eq!(ranked.len(), 3);
    }

    #[test]
    fn test_rank_augments_scores_in_range() {
        let mut policy = make_policy();
        let state = make_state();
        let choices = vec![AugmentId(0), AugmentId(1), AugmentId(2)];
        let ranked = policy.rank_augments(&state, &choices).expect("rank failed in test");
        for (_, score) in &ranked {
            assert!(*score >= 0.0 && *score <= 1.0, "score {} out of range", score);
        }
    }

    #[test]
    fn test_rank_augments_sorted_descending() {
        let mut policy = make_policy();
        let state = make_state();
        let choices = vec![AugmentId(0), AugmentId(1), AugmentId(2)];
        let ranked = policy.rank_augments(&state, &choices).expect("rank failed in test");
        for w in ranked.windows(2) {
            assert!(w[0].1 >= w[1].1, "rankings not sorted descending");
        }
    }

    #[test]
    fn test_record_game_outcome_increments_games() {
        let mut policy = make_policy();
        let state = make_state();
        let choices = vec![AugmentId(0), AugmentId(1), AugmentId(2)];
        policy.rank_augments(&state, &choices).expect("rank failed in test");
        policy.record_game_outcome(Placement(3)).expect("record failed in test");
        assert_eq!(policy.games_trained(), 1);
    }

    #[test]
    fn test_save_and_load_or_init() {
        let catalog = Catalog::from_embedded().expect("catalog init failed in test");
        let path = temp_dir().join("tft_policy_save_test.json");
        let mut policy = AugmentPolicy::new(&catalog, path.clone()).expect("init failed in test");
        let state = make_state();
        let choices = vec![AugmentId(0)];
        policy.rank_augments(&state, &choices).expect("rank failed in test");
        policy.record_game_outcome(Placement(1)).expect("record failed in test");
        policy.save().expect("save failed in test");

        let loaded = AugmentPolicy::load_or_init(&catalog, path.clone()).expect("load failed in test");
        assert_eq!(loaded.games_trained(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_rank_empty_choices_returns_empty() {
        let mut policy = make_policy();
        let state = make_state();
        let ranked = policy.rank_augments(&state, &[]).expect("rank failed in test");
        assert_eq!(ranked.len(), 0);
    }
}

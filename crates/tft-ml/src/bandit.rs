//! Thompson Sampling bandit for augment selection.
//! Maintains Beta(alpha, beta) distributions per augment.

use tft_types::TftError;

/// Per-augment Beta distribution parameters.
#[derive(Debug, Clone)]
pub struct BetaParams {
    pub alpha: f32, // successes + 1
    pub beta: f32,  // failures + 1
}

impl Default for BetaParams {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            beta: 1.0,
        }
    }
}

impl BetaParams {
    /// Approximate Thompson sample using normal approximation to Beta.
    /// For Beta(a, b): mean = a/(a+b), var = ab/((a+b)^2*(a+b+1))
    pub fn sample(&self, seed: u64) -> f32 {
        let a = self.alpha;
        let b = self.beta;
        let mean = a / (a + b);
        let var = (a * b) / ((a + b).powi(2) * (a + b + 1.0));
        let std = var.sqrt();
        // Box-Muller using deterministic seed
        let u1 = ((seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407)) as f64
            / u64::MAX as f64) as f32;
        let u2 = ((seed
            .wrapping_mul(2862933555777941757)
            .wrapping_add(3037000499)) as f64
            / u64::MAX as f64) as f32;
        let n = (-2.0 * (u1 + f32::EPSILON).ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos();
        (mean + std * n).clamp(0.0, 1.0)
    }

    /// Update after observing a reward.
    pub fn update(&mut self, reward: f32) {
        self.alpha += reward;
        self.beta += 1.0 - reward;
    }
}

/// Thompson Sampling over augments.
pub struct ThompsonSampling {
    params: Vec<BetaParams>,
    games_seen: u32,
}

impl ThompsonSampling {
    pub fn new(n_augments: usize) -> Self {
        Self {
            params: vec![BetaParams::default(); n_augments],
            games_seen: 0,
        }
    }

    /// Lambda that anneals from 0.1 (pure bandit) toward 0.9 (neural net dominant) as games_seen grows.
    pub fn neural_net_lambda(&self) -> f32 {
        let base = 0.1f32;
        let target = 0.9f32;
        let rate = 0.01f32;
        (target - (target - base) * (-rate * self.games_seen as f32).exp()).clamp(base, target)
    }

    /// Get a Thompson sample score for a given augment index.
    pub fn sample_score(&self, augment_idx: usize, seed: u64) -> Result<f32, TftError> {
        self.params
            .get(augment_idx)
            .map(|p| p.sample(seed))
            .ok_or_else(|| TftError::AugmentNotFound(format!("index {}", augment_idx)))
    }

    /// Combine neural net logit and bandit sample into a final score.
    pub fn combined_score(&self, nn_score: f32, bandit_score: f32) -> f32 {
        let lambda = self.neural_net_lambda();
        lambda * nn_score + (1.0 - lambda) * bandit_score
    }

    /// Update bandit params for a chosen augment after observing reward.
    pub fn update(&mut self, augment_idx: usize, reward: f32) -> Result<(), TftError> {
        self.params
            .get_mut(augment_idx)
            .ok_or_else(|| TftError::AugmentNotFound(format!("index {}", augment_idx)))?
            .update(reward);
        self.games_seen += 1;
        Ok(())
    }

    pub fn games_seen(&self) -> u32 {
        self.games_seen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beta_default_params() {
        let p = BetaParams::default();
        assert_eq!(p.alpha, 1.0);
        assert_eq!(p.beta, 1.0);
    }

    #[test]
    fn test_beta_sample_in_range() {
        let p = BetaParams {
            alpha: 5.0,
            beta: 2.0,
        };
        for seed in 0u64..100 {
            let s = p.sample(seed);
            assert!(
                s >= 0.0 && s <= 1.0,
                "sample {} out of range for seed {}",
                s,
                seed
            );
        }
    }

    #[test]
    fn test_beta_update_increases_alpha_on_high_reward() {
        let mut p = BetaParams::default();
        let alpha_before = p.alpha;
        p.update(0.9);
        assert!(p.alpha > alpha_before);
    }

    #[test]
    fn test_beta_update_increases_beta_on_low_reward() {
        let mut p = BetaParams::default();
        let beta_before = p.beta;
        p.update(0.1);
        assert!(p.beta > beta_before);
    }

    #[test]
    fn test_thompson_sampling_new_correct_size() {
        let ts = ThompsonSampling::new(20);
        assert_eq!(ts.params.len(), 20);
        assert_eq!(ts.games_seen(), 0);
    }

    #[test]
    fn test_sample_score_valid_index() {
        let ts = ThompsonSampling::new(10);
        let result = ts.sample_score(5, 42);
        assert!(result.is_ok());
        let s = result.expect("sample failed in test");
        assert!(s >= 0.0 && s <= 1.0);
    }

    #[test]
    fn test_sample_score_invalid_index_errors() {
        let ts = ThompsonSampling::new(5);
        assert!(ts.sample_score(10, 0).is_err());
    }

    #[test]
    fn test_lambda_starts_low() {
        let ts = ThompsonSampling::new(5);
        let lambda = ts.neural_net_lambda();
        assert!(lambda < 0.2, "initial lambda should be low, got {}", lambda);
    }

    #[test]
    fn test_lambda_increases_with_games() {
        let mut ts = ThompsonSampling::new(5);
        let lambda_0 = ts.neural_net_lambda();
        for _ in 0..200 {
            ts.update(0, 0.7).expect("update failed in test");
        }
        let lambda_200 = ts.neural_net_lambda();
        assert!(
            lambda_200 > lambda_0,
            "lambda should increase with more games"
        );
    }

    #[test]
    fn test_combined_score_in_range() {
        let ts = ThompsonSampling::new(5);
        let score = ts.combined_score(0.8, 0.5);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_update_invalid_index_errors() {
        let mut ts = ThompsonSampling::new(5);
        assert!(ts.update(99, 0.5).is_err());
    }
}

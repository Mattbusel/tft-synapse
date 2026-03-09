use serde::{Deserialize, Serialize};

/// Final game placement (1 = first, 8 = last).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement(pub u8);

impl Placement {
    /// Convert placement to a reward signal in [0.0, 1.0].
    /// 1st = 1.0, 8th = 0.0.
    pub fn to_reward(self) -> f32 {
        let p = self.0.clamp(1, 8);
        (8 - p) as f32 / 7.0
    }

    pub fn is_valid(self) -> bool {
        self.0 >= 1 && self.0 <= 8
    }

    pub fn is_top_four(self) -> bool {
        self.0 <= 4
    }
}

/// A game state transition used for ML training.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub features: Vec<f32>,
    pub augment_chosen: u8,
    pub placement: Option<Placement>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placement_first_is_max_reward() {
        let p = Placement(1);
        assert!((p.to_reward() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_placement_last_is_zero_reward() {
        let p = Placement(8);
        assert!(p.to_reward().abs() < f32::EPSILON);
    }

    #[test]
    fn test_placement_reward_monotone_decreasing() {
        let rewards: Vec<f32> = (1u8..=8).map(|i| Placement(i).to_reward()).collect();
        for w in rewards.windows(2) {
            assert!(
                w[0] >= w[1],
                "rewards should be non-increasing: {} < {}",
                w[0],
                w[1]
            );
        }
    }

    #[test]
    fn test_placement_reward_bounds() {
        for i in 1u8..=8 {
            let r = Placement(i).to_reward();
            assert!(
                r >= 0.0 && r <= 1.0,
                "reward {} out of bounds for placement {}",
                r,
                i
            );
        }
    }

    #[test]
    fn test_placement_validity() {
        assert!(Placement(1).is_valid());
        assert!(Placement(8).is_valid());
        assert!(!Placement(0).is_valid());
        assert!(!Placement(9).is_valid());
    }

    #[test]
    fn test_placement_top_four() {
        for i in 1u8..=4 {
            assert!(Placement(i).is_top_four());
        }
        for i in 5u8..=8 {
            assert!(!Placement(i).is_top_four());
        }
    }

    #[test]
    fn test_state_transition_serde() {
        let t = StateTransition {
            features: vec![1.0, 2.0, 3.0],
            augment_chosen: 5,
            placement: Some(Placement(3)),
        };
        let json = serde_json::to_string(&t).expect("serialize failed in test");
        let back: StateTransition =
            serde_json::from_str(&json).expect("deserialize failed in test");
        assert_eq!(t.augment_chosen, back.augment_chosen);
        assert_eq!(t.features, back.features);
    }
}

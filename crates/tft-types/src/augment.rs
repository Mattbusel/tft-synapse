use serde::{Deserialize, Serialize};

/// Unique identifier for an augment, indexed into the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AugmentId(pub u8);

/// Augment power tier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AugmentTier {
    S,
    SPlus,
    SMinus,
    A,
    APlus,
    AMinus,
    B,
    BPlus,
    BMinus,
    C,
}

impl AugmentTier {
    /// Numeric weight for tier comparisons.
    pub fn weight(&self) -> f32 {
        match self {
            AugmentTier::SPlus => 1.0,
            AugmentTier::S => 0.95,
            AugmentTier::SMinus => 0.90,
            AugmentTier::APlus => 0.85,
            AugmentTier::A => 0.80,
            AugmentTier::AMinus => 0.75,
            AugmentTier::BPlus => 0.70,
            AugmentTier::B => 0.65,
            AugmentTier::BMinus => 0.60,
            AugmentTier::C => 0.50,
        }
    }
}

/// Full definition of an augment from the catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentDef {
    pub id: AugmentId,
    pub name: String,
    pub tier: Option<AugmentTier>,
    pub base_score: f32,
    pub tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_augment_id_equality() {
        let a = AugmentId(1);
        let b = AugmentId(1);
        let c = AugmentId(2);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_augment_tier_weight_ordering() {
        assert!(AugmentTier::S.weight() > AugmentTier::A.weight());
        assert!(AugmentTier::A.weight() > AugmentTier::B.weight());
        assert!(AugmentTier::B.weight() > AugmentTier::C.weight());
    }

    #[test]
    fn test_augment_tier_weight_bounds() {
        for tier in [
            AugmentTier::SPlus,
            AugmentTier::S,
            AugmentTier::SMinus,
            AugmentTier::APlus,
            AugmentTier::A,
            AugmentTier::AMinus,
            AugmentTier::BPlus,
            AugmentTier::B,
            AugmentTier::BMinus,
            AugmentTier::C,
        ] {
            let w = tier.weight();
            assert!(
                w >= 0.0 && w <= 1.0,
                "weight {} out of bounds for {:?}",
                w,
                tier
            );
        }
    }

    #[test]
    fn test_augment_def_clone() {
        let def = AugmentDef {
            id: AugmentId(5),
            name: "Blue Battery".to_string(),
            tier: Some(AugmentTier::S),
            base_score: 88.0,
            tags: vec!["AP".to_string(), "mana".to_string()],
        };
        let cloned = def.clone();
        assert_eq!(def.name, cloned.name);
        assert_eq!(def.base_score, cloned.base_score);
    }

    #[test]
    fn test_augment_id_serde() {
        let id = AugmentId(42);
        let json = serde_json::to_string(&id).expect("serialize failed in test");
        let back: AugmentId = serde_json::from_str(&json).expect("deserialize failed in test");
        assert_eq!(id, back);
    }
}

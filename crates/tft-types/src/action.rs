use crate::{AugmentId, ChampionId};
use serde::{Deserialize, Serialize};

/// All possible player actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    BuyAugment(AugmentId),
    BuyChampion {
        shop_slot: usize,
        champion_id: ChampionId,
    },
    SellChampion {
        bench_slot: usize,
    },
    BuyXp,
    Reroll,
    LevelUp,
    LockShop,
    Wait,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_buy_augment_equality() {
        let a = Action::BuyAugment(AugmentId(1));
        let b = Action::BuyAugment(AugmentId(1));
        let c = Action::BuyAugment(AugmentId(2));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_action_serde_roundtrip() {
        let action = Action::BuyChampion {
            shop_slot: 2,
            champion_id: ChampionId(5),
        };
        let json = serde_json::to_string(&action).expect("serialize failed in test");
        let back: Action = serde_json::from_str(&json).expect("deserialize failed in test");
        assert_eq!(action, back);
    }

    #[test]
    fn test_all_action_variants_clone() {
        let actions = vec![
            Action::BuyAugment(AugmentId(0)),
            Action::BuyChampion {
                shop_slot: 0,
                champion_id: ChampionId(0),
            },
            Action::SellChampion { bench_slot: 1 },
            Action::BuyXp,
            Action::Reroll,
            Action::LevelUp,
            Action::LockShop,
            Action::Wait,
        ];
        for a in &actions {
            let _ = a.clone();
        }
    }
}

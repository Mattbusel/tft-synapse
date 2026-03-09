use crate::item::ItemId;
use serde::{Deserialize, Serialize};

/// Unique identifier for a champion, indexed into the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChampionId(pub u8);

/// Gold cost of a champion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cost {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
}

impl Cost {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Cost::One),
            2 => Some(Cost::Two),
            3 => Some(Cost::Three),
            4 => Some(Cost::Four),
            5 => Some(Cost::Five),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Champion star level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StarLevel {
    One = 1,
    Two = 2,
    Three = 3,
}

/// A champion slot on the board or bench.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChampionSlot {
    pub champion_id: ChampionId,
    pub star_level: StarLevel,
    pub items: Vec<ItemId>, // item ids
}

/// Full definition from catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChampionDef {
    pub id: ChampionId,
    pub name: String,
    pub cost: Cost,
    pub traits: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_from_u8_valid_range() {
        for i in 1u8..=5 {
            assert!(Cost::from_u8(i).is_some(), "Cost {} should be valid", i);
        }
    }

    #[test]
    fn test_cost_from_u8_invalid() {
        assert!(Cost::from_u8(0).is_none());
        assert!(Cost::from_u8(6).is_none());
    }

    #[test]
    fn test_cost_roundtrip() {
        for cost in [Cost::One, Cost::Two, Cost::Three, Cost::Four, Cost::Five] {
            let n = cost.as_u8();
            assert_eq!(Cost::from_u8(n), Some(cost));
        }
    }

    #[test]
    fn test_champion_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ChampionId(1));
        set.insert(ChampionId(2));
        set.insert(ChampionId(1));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_champion_def_serde() {
        let def = ChampionDef {
            id: ChampionId(10),
            name: "Jinx".to_string(),
            cost: Cost::Three,
            traits: vec!["Gunner".to_string()],
        };
        let json = serde_json::to_string(&def).expect("serialize failed in test");
        let back: ChampionDef = serde_json::from_str(&json).expect("deserialize failed in test");
        assert_eq!(def.name, back.name);
        assert_eq!(def.cost, back.cost);
    }

    #[test]
    fn test_champion_slot_clone() {
        let slot = ChampionSlot {
            champion_id: ChampionId(3),
            star_level: StarLevel::Two,
            items: vec![ItemId(1), ItemId(2)],
        };
        let c = slot.clone();
        assert_eq!(slot.champion_id, c.champion_id);
        assert_eq!(slot.items, c.items);
    }
}

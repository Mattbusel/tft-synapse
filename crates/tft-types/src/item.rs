use serde::{Deserialize, Serialize};

/// Unique identifier for an item, indexed into the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub u8);

/// Item category for recommendation logic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemCategory {
    AttackDamage,
    AbilityPower,
    Tank,
    Mana,
    CriticalStrike,
    AttackSpeed,
    Healing,
    Utility,
}

/// Full item definition from the catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    pub id: ItemId,
    pub name: String,
    pub category: ItemCategory,
    pub is_component: bool, // true = basic component, false = combined item
    pub tags: Vec<String>,  // e.g. ["AD", "tank", "fighter"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_id_equality() {
        let a = ItemId(5);
        let b = ItemId(5);
        let c = ItemId(6);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_item_id_copy() {
        let a = ItemId(3);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn test_item_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ItemId(1));
        set.insert(ItemId(2));
        set.insert(ItemId(1));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_item_def_serde_roundtrip() {
        let def = ItemDef {
            id: ItemId(7),
            name: "Infinity Edge".to_string(),
            category: ItemCategory::CriticalStrike,
            is_component: false,
            tags: vec!["crit".to_string(), "AD".to_string()],
        };
        let json = serde_json::to_string(&def).expect("serialize failed in test");
        let back: ItemDef = serde_json::from_str(&json).expect("deserialize failed in test");
        assert_eq!(def.id, back.id);
        assert_eq!(def.name, back.name);
        assert_eq!(def.category, back.category);
        assert_eq!(def.is_component, back.is_component);
        assert_eq!(def.tags, back.tags);
    }

    #[test]
    fn test_item_def_clone() {
        let def = ItemDef {
            id: ItemId(2),
            name: "B.F. Sword".to_string(),
            category: ItemCategory::AttackDamage,
            is_component: true,
            tags: vec!["AD".to_string()],
        };
        let cloned = def.clone();
        assert_eq!(def.id, cloned.id);
        assert_eq!(def.name, cloned.name);
    }

    #[test]
    fn test_item_category_eq() {
        assert_eq!(ItemCategory::AbilityPower, ItemCategory::AbilityPower);
        assert_ne!(ItemCategory::Tank, ItemCategory::Mana);
    }

    #[test]
    fn test_item_def_component_flag() {
        let component = ItemDef {
            id: ItemId(0),
            name: "Recurve Bow".to_string(),
            category: ItemCategory::AttackSpeed,
            is_component: true,
            tags: vec![],
        };
        let combined = ItemDef {
            id: ItemId(1),
            name: "Guinsoo's Rageblade".to_string(),
            category: ItemCategory::AttackSpeed,
            is_component: false,
            tags: vec![],
        };
        assert!(component.is_component);
        assert!(!combined.is_component);
    }
}

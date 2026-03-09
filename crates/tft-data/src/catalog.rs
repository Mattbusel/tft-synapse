use std::collections::HashMap;
use std::sync::OnceLock;
use tft_types::{AugmentDef, AugmentId, ChampionDef, ChampionId, TftError};
use crate::loader::{parse_augments, parse_champions, parse_traits, RawTrait};
use crate::embed::{AUGMENTS_YAML, CHAMPIONS_YAML, TRAITS_YAML};

/// The complete game data catalog, initialized once at startup.
pub struct Catalog {
    pub augments: Vec<AugmentDef>,
    pub augment_by_name: HashMap<String, usize>,
    pub champions: Vec<ChampionDef>,
    pub champion_by_name: HashMap<String, usize>,
    pub traits: Vec<RawTrait>,
    pub trait_by_name: HashMap<String, usize>,
}

static GLOBAL_CATALOG: OnceLock<Result<Catalog, String>> = OnceLock::new();

impl Catalog {
    /// Build a catalog from the embedded YAML data.
    pub fn from_embedded() -> Result<Self, TftError> {
        let augments = parse_augments(AUGMENTS_YAML)?;
        let champions = parse_champions(CHAMPIONS_YAML)?;
        let traits = parse_traits(TRAITS_YAML)?;

        let augment_by_name = augments.iter().enumerate()
            .map(|(i, a)| (a.name.clone(), i)).collect();
        let champion_by_name = champions.iter().enumerate()
            .map(|(i, c)| (c.name.clone(), i)).collect();
        let trait_by_name = traits.iter().enumerate()
            .map(|(i, t)| (t.name.clone(), i)).collect();

        Ok(Catalog { augments, augment_by_name, champions, champion_by_name, traits, trait_by_name })
    }

    /// Returns a reference to the global singleton, initializing it on first call.
    pub fn global() -> Result<&'static Catalog, TftError> {
        let cell = GLOBAL_CATALOG.get_or_init(|| {
            Catalog::from_embedded().map_err(|e| e.to_string())
        });
        cell.as_ref().map_err(|e| TftError::Catalog(e.clone()))
    }

    pub fn augment_count(&self) -> usize { self.augments.len() }
    pub fn champion_count(&self) -> usize { self.champions.len() }

    pub fn augment_by_id(&self, id: AugmentId) -> Option<&AugmentDef> {
        self.augments.get(id.0 as usize)
    }

    pub fn champion_by_id(&self, id: ChampionId) -> Option<&ChampionDef> {
        self.champions.get(id.0 as usize)
    }

    pub fn augment_id_by_name(&self, name: &str) -> Option<AugmentId> {
        self.augment_by_name.get(name).map(|&i| AugmentId(i as u8))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_from_embedded_succeeds() {
        let result = Catalog::from_embedded();
        assert!(result.is_ok(), "catalog init failed: {:?}", result.err());
    }

    #[test]
    fn test_catalog_augment_count_nonzero() {
        let catalog = Catalog::from_embedded().expect("catalog init failed in test");
        assert!(catalog.augment_count() > 0);
    }

    #[test]
    fn test_catalog_champion_count_nonzero() {
        let catalog = Catalog::from_embedded().expect("catalog init failed in test");
        assert!(catalog.champion_count() > 0);
    }

    #[test]
    fn test_catalog_augment_by_id_roundtrip() {
        let catalog = Catalog::from_embedded().expect("catalog init failed in test");
        for i in 0..catalog.augment_count() {
            let id = AugmentId(i as u8);
            let def = catalog.augment_by_id(id);
            assert!(def.is_some(), "augment {} not found by id", i);
        }
    }

    #[test]
    fn test_catalog_augment_by_name_lookup() {
        let catalog = Catalog::from_embedded().expect("catalog init failed in test");
        let id = catalog.augment_id_by_name("Blue Battery");
        assert!(id.is_some(), "Blue Battery not found in catalog");
    }

    #[test]
    fn test_catalog_augment_name_id_consistent() {
        let catalog = Catalog::from_embedded().expect("catalog init failed in test");
        for aug in &catalog.augments {
            let found_id = catalog.augment_id_by_name(&aug.name);
            assert_eq!(found_id, Some(aug.id), "id mismatch for {}", aug.name);
        }
    }

    #[test]
    fn test_catalog_champion_by_id_roundtrip() {
        let catalog = Catalog::from_embedded().expect("catalog init failed in test");
        for i in 0..catalog.champion_count() {
            let id = ChampionId(i as u8);
            let def = catalog.champion_by_id(id);
            assert!(def.is_some(), "champion {} not found by id", i);
        }
    }

    #[test]
    fn test_catalog_global_is_ok() {
        let result = Catalog::global();
        assert!(result.is_ok());
    }

    #[test]
    fn test_catalog_global_returns_same_instance() {
        let a = Catalog::global().expect("global catalog failed in test");
        let b = Catalog::global().expect("global catalog failed in test");
        assert_eq!(a.augment_count(), b.augment_count());
        assert!(std::ptr::eq(a, b), "global() should return the same pointer");
    }

    #[test]
    fn test_catalog_traits_nonempty() {
        let catalog = Catalog::from_embedded().expect("catalog init failed in test");
        assert!(!catalog.traits.is_empty());
    }

    #[test]
    fn test_catalog_trait_by_name_lookup() {
        let catalog = Catalog::from_embedded().expect("catalog init failed in test");
        assert!(catalog.trait_by_name.contains_key("Arcanist"));
        assert!(catalog.trait_by_name.contains_key("Gunner"));
    }
}

use crate::embed::{AUGMENTS_YAML, CHAMPIONS_YAML, ITEMS_YAML, TRAITS_YAML};
use crate::loader::{parse_augments, parse_champions, parse_items, parse_traits, RawTrait};
use std::collections::HashMap;
use std::sync::OnceLock;
use tft_types::{AugmentDef, AugmentId, ChampionDef, ChampionId, ItemDef, ItemId, TftError};

/// JSON schema for the hot-reload catalog file at `~/.tft-synapse/catalog.json`.
#[derive(serde::Deserialize)]
pub struct CatalogJson {
    pub augments: Vec<AugmentDef>,
    pub champions: Vec<ChampionDef>,
}

/// The complete game data catalog, initialized once at startup.
pub struct Catalog {
    pub augments: Vec<AugmentDef>,
    pub augment_by_name: HashMap<String, usize>,
    pub champions: Vec<ChampionDef>,
    pub champion_by_name: HashMap<String, usize>,
    pub traits: Vec<RawTrait>,
    pub trait_by_name: HashMap<String, usize>,
    pub items: Vec<ItemDef>,
    pub item_by_name: HashMap<String, usize>,
}

static GLOBAL_CATALOG: OnceLock<Result<Catalog, String>> = OnceLock::new();

impl Catalog {
    /// Build a catalog from the embedded YAML data.
    pub fn from_embedded() -> Result<Self, TftError> {
        let augments = parse_augments(AUGMENTS_YAML)?;
        let champions = parse_champions(CHAMPIONS_YAML)?;
        let traits = parse_traits(TRAITS_YAML)?;
        let items = parse_items(ITEMS_YAML)?;

        let augment_by_name = augments
            .iter()
            .enumerate()
            .map(|(i, a)| (a.name.clone(), i))
            .collect();
        let champion_by_name = champions
            .iter()
            .enumerate()
            .map(|(i, c)| (c.name.clone(), i))
            .collect();
        let trait_by_name = traits
            .iter()
            .enumerate()
            .map(|(i, t)| (t.name.clone(), i))
            .collect();
        let item_by_name = items
            .iter()
            .enumerate()
            .map(|(i, item)| (item.name.clone(), i))
            .collect();

        Ok(Catalog {
            augments,
            augment_by_name,
            champions,
            champion_by_name,
            traits,
            trait_by_name,
            items,
            item_by_name,
        })
    }

    /// Load catalog from a JSON file at `path`.
    ///
    /// Augments and champions come from the JSON file.
    /// Traits and items are loaded from embedded YAML.
    ///
    /// # Arguments
    /// * `path` — Path to a JSON file matching the `CatalogJson` schema
    ///
    /// # Returns
    /// - `Ok(Catalog)` — populated catalog
    /// - `Err(TftError::Catalog(...))` — if the file cannot be read or parsed
    ///
    /// # Panics
    /// This function never panics.
    ///
    /// # Example
    /// ```rust,no_run
    /// let catalog = tft_data::Catalog::from_json_file(std::path::Path::new("/tmp/catalog.json"))?;
    /// # Ok::<(), tft_types::TftError>(())
    /// ```
    pub fn from_json_file(path: &std::path::Path) -> Result<Self, TftError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| TftError::Catalog(format!("Failed to read catalog.json: {}", e)))?;
        let json: CatalogJson = serde_json::from_str(&content)
            .map_err(|e| TftError::Catalog(format!("Failed to parse catalog.json: {}", e)))?;

        let augment_by_name = json
            .augments
            .iter()
            .enumerate()
            .map(|(i, a)| (a.name.clone(), i))
            .collect();
        let champion_by_name = json
            .champions
            .iter()
            .enumerate()
            .map(|(i, c)| (c.name.clone(), i))
            .collect();

        let traits = parse_traits(TRAITS_YAML)?;
        let trait_by_name = traits
            .iter()
            .enumerate()
            .map(|(i, t)| (t.name.clone(), i))
            .collect();

        let items = parse_items(ITEMS_YAML)?;
        let item_by_name = items
            .iter()
            .enumerate()
            .map(|(i, item)| (item.name.clone(), i))
            .collect();

        Ok(Catalog {
            augments: json.augments,
            augment_by_name,
            champions: json.champions,
            champion_by_name,
            traits,
            trait_by_name,
            items,
            item_by_name,
        })
    }

    /// Returns a reference to the global singleton, initializing it on first call.
    ///
    /// Tries `~/.tft-synapse/catalog.json` first; falls back to embedded YAML
    /// if the file does not exist or fails to parse.
    pub fn global() -> Result<&'static Catalog, TftError> {
        let cell = GLOBAL_CATALOG.get_or_init(|| {
            let json_path =
                dirs_next::home_dir().map(|h| h.join(".tft-synapse").join("catalog.json"));
            if let Some(ref path) = json_path {
                if path.exists() {
                    if let Ok(c) = Catalog::from_json_file(path) {
                        return Ok(c);
                    }
                }
            }
            Catalog::from_embedded().map_err(|e| e.to_string())
        });
        cell.as_ref().map_err(|e| TftError::Catalog(e.clone()))
    }

    pub fn augment_count(&self) -> usize {
        self.augments.len()
    }
    pub fn champion_count(&self) -> usize {
        self.champions.len()
    }

    pub fn augment_by_id(&self, id: AugmentId) -> Option<&AugmentDef> {
        self.augments.get(id.0 as usize)
    }

    pub fn champion_by_id(&self, id: ChampionId) -> Option<&ChampionDef> {
        self.champions.get(id.0 as usize)
    }

    pub fn augment_id_by_name(&self, name: &str) -> Option<AugmentId> {
        self.augment_by_name.get(name).map(|&i| AugmentId(i as u8))
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn item_by_id(&self, id: ItemId) -> Option<&ItemDef> {
        self.items.get(id.0 as usize)
    }

    pub fn item_id_by_name(&self, name: &str) -> Option<ItemId> {
        self.item_by_name.get(name).map(|&i| ItemId(i as u8))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as IoWrite;

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
        assert!(
            std::ptr::eq(a, b),
            "global() should return the same pointer"
        );
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

    #[test]
    fn test_catalog_item_count_nonzero() {
        let catalog = Catalog::from_embedded().expect("catalog init failed in test");
        assert!(catalog.item_count() > 0);
    }

    #[test]
    fn test_catalog_item_by_id_roundtrip() {
        let catalog = Catalog::from_embedded().expect("catalog init failed in test");
        for i in 0..catalog.item_count() {
            let id = ItemId(i as u8);
            let def = catalog.item_by_id(id);
            assert!(def.is_some(), "item {} not found by id", i);
        }
    }

    #[test]
    fn test_catalog_item_by_name_lookup() {
        let catalog = Catalog::from_embedded().expect("catalog init failed in test");
        let id = catalog.item_id_by_name("Infinity Edge");
        assert!(id.is_some(), "Infinity Edge not found in catalog");
    }

    #[test]
    fn test_catalog_item_name_id_consistent() {
        let catalog = Catalog::from_embedded().expect("catalog init failed in test");
        for item in &catalog.items {
            let found_id = catalog.item_id_by_name(&item.name);
            assert_eq!(found_id, Some(item.id), "id mismatch for {}", item.name);
        }
    }

    #[test]
    fn test_catalog_item_by_id_out_of_range_returns_none() {
        let catalog = Catalog::from_embedded().expect("catalog init failed in test");
        let id = ItemId(255);
        assert!(catalog.item_by_id(id).is_none());
    }

    // ── from_json_file tests ──────────────────────────────────────────────────

    fn make_catalog_json_str(augment_name: &str, champion_name: &str) -> String {
        serde_json::json!({
            "augments": [{
                "id": 0,
                "name": augment_name,
                "tier": null,
                "base_score": 0.5,
                "tags": ["econ"]
            }],
            "champions": [{
                "id": 0,
                "name": champion_name,
                "cost": "One",
                "traits": ["Gunner"]
            }]
        })
        .to_string()
    }

    #[test]
    fn test_from_json_file_valid_file_returns_catalog() {
        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile failed in test");
        write!(
            tmp,
            "{}",
            make_catalog_json_str("Test Augment", "TestChamp")
        )
        .expect("write failed in test");
        let result = Catalog::from_json_file(tmp.path());
        assert!(result.is_ok(), "from_json_file failed: {:?}", result.err());
    }

    #[test]
    fn test_from_json_file_augments_loaded_correctly() {
        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile failed in test");
        write!(
            tmp,
            "{}",
            make_catalog_json_str("Custom Augment", "SomeChamp")
        )
        .expect("write failed in test");
        let catalog = Catalog::from_json_file(tmp.path()).expect("from_json_file failed in test");
        assert_eq!(catalog.augment_count(), 1);
        assert_eq!(catalog.augments[0].name, "Custom Augment");
    }

    #[test]
    fn test_from_json_file_champions_loaded_correctly() {
        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile failed in test");
        write!(tmp, "{}", make_catalog_json_str("SomeAug", "MyChampion"))
            .expect("write failed in test");
        let catalog = Catalog::from_json_file(tmp.path()).expect("from_json_file failed in test");
        assert_eq!(catalog.champion_count(), 1);
        assert_eq!(catalog.champions[0].name, "MyChampion");
    }

    #[test]
    fn test_from_json_file_augment_by_name_lookup_works() {
        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile failed in test");
        write!(tmp, "{}", make_catalog_json_str("LookupAug", "Champ"))
            .expect("write failed in test");
        let catalog = Catalog::from_json_file(tmp.path()).expect("from_json_file failed in test");
        let id = catalog.augment_id_by_name("LookupAug");
        assert!(id.is_some());
    }

    #[test]
    fn test_from_json_file_traits_from_embedded() {
        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile failed in test");
        write!(tmp, "{}", make_catalog_json_str("A", "B")).expect("write failed in test");
        let catalog = Catalog::from_json_file(tmp.path()).expect("from_json_file failed in test");
        // Traits come from embedded — should be non-empty
        assert!(!catalog.traits.is_empty());
    }

    #[test]
    fn test_from_json_file_items_from_embedded() {
        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile failed in test");
        write!(tmp, "{}", make_catalog_json_str("A", "B")).expect("write failed in test");
        let catalog = Catalog::from_json_file(tmp.path()).expect("from_json_file failed in test");
        // Items come from embedded — should be non-empty
        assert!(catalog.item_count() > 0);
    }

    #[test]
    fn test_from_json_file_nonexistent_path_returns_err() {
        let result =
            Catalog::from_json_file(std::path::Path::new("/nonexistent/path/catalog.json"));
        assert!(result.is_err());
        if let Err(TftError::Catalog(msg)) = result {
            assert!(msg.contains("Failed to read"));
        }
    }

    #[test]
    fn test_from_json_file_invalid_json_returns_err() {
        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile failed in test");
        write!(tmp, "{{not valid json}}").expect("write failed in test");
        let result = Catalog::from_json_file(tmp.path());
        assert!(result.is_err());
        if let Err(TftError::Catalog(msg)) = result {
            assert!(msg.contains("Failed to parse"));
        }
    }

    #[test]
    fn test_from_json_file_multiple_augments() {
        let json = serde_json::json!({
            "augments": [
                { "id": 0, "name": "AugA", "tier": null, "base_score": 0.5, "tags": [] },
                { "id": 1, "name": "AugB", "tier": null, "base_score": 0.7, "tags": [] },
                { "id": 2, "name": "AugC", "tier": null, "base_score": 0.3, "tags": [] },
            ],
            "champions": []
        })
        .to_string();
        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile failed in test");
        write!(tmp, "{}", json).expect("write failed in test");
        let catalog = Catalog::from_json_file(tmp.path()).expect("from_json_file failed in test");
        assert_eq!(catalog.augment_count(), 3);
        assert!(catalog.augment_id_by_name("AugB").is_some());
    }
}

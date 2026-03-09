use serde::Deserialize;
use tft_types::{AugmentDef, AugmentId, AugmentTier, ChampionDef, ChampionId, Cost, TftError};

#[derive(Deserialize)]
struct RawAugmentsFile {
    #[allow(dead_code)]
    meta_version: String,
    augments: Vec<RawAugment>,
}

#[derive(Deserialize)]
struct RawAugment {
    name: String,
    tier: Option<String>,
    base_score: f32,
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct RawChampionsFile {
    #[allow(dead_code)]
    meta_version: String,
    champions: Vec<RawChampion>,
}

#[derive(Deserialize)]
struct RawChampion {
    name: String,
    cost: u8,
    traits: Vec<String>,
}

#[derive(Deserialize)]
pub struct RawTraitsFile {
    #[allow(dead_code)]
    pub meta_version: String,
    pub traits: Vec<RawTrait>,
}

#[derive(Deserialize, Clone)]
pub struct RawTrait {
    pub name: String,
    pub r#type: String,
    pub breakpoints: Vec<u8>,
}

fn parse_tier(s: &str) -> Option<AugmentTier> {
    match s {
        "S+" => Some(AugmentTier::SPlus),
        "S"  => Some(AugmentTier::S),
        "S-" => Some(AugmentTier::SMinus),
        "A+" => Some(AugmentTier::APlus),
        "A"  => Some(AugmentTier::A),
        "A-" => Some(AugmentTier::AMinus),
        "B+" => Some(AugmentTier::BPlus),
        "B"  => Some(AugmentTier::B),
        "B-" => Some(AugmentTier::BMinus),
        "C"  => Some(AugmentTier::C),
        _    => None,
    }
}

pub fn parse_augments(yaml: &str) -> Result<Vec<AugmentDef>, TftError> {
    let raw: RawAugmentsFile = serde_yaml::from_str(yaml)
        .map_err(|e| TftError::Catalog(format!("Failed to parse augments.yaml: {}", e)))?;
    let defs = raw.augments.into_iter().enumerate().map(|(i, a)| {
        AugmentDef {
            id: AugmentId(i as u8),
            name: a.name,
            tier: a.tier.as_deref().and_then(parse_tier),
            base_score: a.base_score,
            tags: a.tags,
        }
    }).collect();
    Ok(defs)
}

pub fn parse_champions(yaml: &str) -> Result<Vec<ChampionDef>, TftError> {
    let raw: RawChampionsFile = serde_yaml::from_str(yaml)
        .map_err(|e| TftError::Catalog(format!("Failed to parse champions.yaml: {}", e)))?;
    let defs = raw.champions.into_iter().enumerate().map(|(i, c)| {
        ChampionDef {
            id: ChampionId(i as u8),
            name: c.name,
            cost: Cost::from_u8(c.cost).unwrap_or(Cost::One),
            traits: c.traits,
        }
    }).collect();
    Ok(defs)
}

pub fn parse_traits(yaml: &str) -> Result<Vec<RawTrait>, TftError> {
    let raw: RawTraitsFile = serde_yaml::from_str(yaml)
        .map_err(|e| TftError::Catalog(format!("Failed to parse traits.yaml: {}", e)))?;
    Ok(raw.traits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::*;

    #[test]
    fn test_parse_augments_from_embedded_yaml() {
        let result = parse_augments(AUGMENTS_YAML);
        assert!(result.is_ok(), "parse_augments failed: {:?}", result.err());
        let defs = result.expect("parse failed in test");
        assert!(!defs.is_empty());
    }

    #[test]
    fn test_augment_ids_are_sequential() {
        let defs = parse_augments(AUGMENTS_YAML).expect("parse failed in test");
        for (i, def) in defs.iter().enumerate() {
            assert_eq!(def.id.0 as usize, i, "augment id mismatch at index {}", i);
        }
    }

    #[test]
    fn test_parse_champions_from_embedded_yaml() {
        let result = parse_champions(CHAMPIONS_YAML);
        assert!(result.is_ok(), "parse_champions failed: {:?}", result.err());
        let defs = result.expect("parse failed in test");
        assert!(!defs.is_empty());
    }

    #[test]
    fn test_all_champion_costs_valid() {
        let defs = parse_champions(CHAMPIONS_YAML).expect("parse failed in test");
        for def in &defs {
            assert!(def.cost.as_u8() >= 1 && def.cost.as_u8() <= 5,
                "invalid cost {} for {}", def.cost.as_u8(), def.name);
        }
    }

    #[test]
    fn test_parse_traits_from_embedded_yaml() {
        let result = parse_traits(TRAITS_YAML);
        assert!(result.is_ok(), "parse_traits failed: {:?}", result.err());
        let traits = result.expect("parse failed in test");
        assert!(!traits.is_empty());
    }

    #[test]
    fn test_all_traits_have_breakpoints() {
        let traits = parse_traits(TRAITS_YAML).expect("parse failed in test");
        for t in &traits {
            assert!(!t.breakpoints.is_empty(), "trait {} has no breakpoints", t.name);
        }
    }

    #[test]
    fn test_parse_tier_known_values() {
        assert_eq!(parse_tier("S"), Some(AugmentTier::S));
        assert_eq!(parse_tier("A+"), Some(AugmentTier::APlus));
        assert_eq!(parse_tier("B-"), Some(AugmentTier::BMinus));
        assert_eq!(parse_tier("unknown"), None);
    }

    #[test]
    fn test_augments_have_names() {
        let defs = parse_augments(AUGMENTS_YAML).expect("parse failed in test");
        for def in &defs {
            assert!(!def.name.is_empty(), "augment {:?} has empty name", def.id);
        }
    }
}

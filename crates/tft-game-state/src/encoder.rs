//! One-hot and multi-hot encoding for categorical game state fields.

use tft_types::AugmentId;

/// Write a one-hot vector of length `size` into `out[offset..offset+size]`.
/// Sets `out[offset + idx] = 1.0`. If idx >= size, nothing is set.
pub fn one_hot(out: &mut Vec<f32>, idx: usize, size: usize) {
    let start = out.len();
    out.resize(start + size, 0.0);
    if idx < size {
        out[start + idx] = 1.0;
    }
}

/// Write a multi-hot vector: for each id in `ids`, set the corresponding bit.
pub fn multi_hot(out: &mut Vec<f32>, ids: &[usize], size: usize) {
    let start = out.len();
    out.resize(start + size, 0.0);
    for &id in ids {
        if id < size {
            out[start + id] = 1.0;
        }
    }
}

/// Encode currently-held augments as a multi-hot vector of length `catalog_size`.
pub fn encode_augments(out: &mut Vec<f32>, augments: &[AugmentId], catalog_size: usize) {
    let ids: Vec<usize> = augments.iter().map(|a| a.0 as usize).collect();
    multi_hot(out, &ids, catalog_size);
}

/// Encode trait activations as a multi-hot vector of length `n_traits`.
/// active_traits is a slice of (trait_name, count) pairs.
/// trait_index maps trait_name -> index in output vector.
pub fn encode_traits(
    out: &mut Vec<f32>,
    active_traits: &[(String, u8)],
    trait_index: &std::collections::HashMap<String, usize>,
    n_traits: usize,
) {
    let start = out.len();
    out.resize(start + n_traits, 0.0);
    for (name, count) in active_traits {
        if let Some(&idx) = trait_index.get(name.as_str()) {
            if idx < n_traits {
                // Normalize count by a max of 9
                out[start + idx] = (*count as f32 / 9.0).clamp(0.0, 1.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_one_hot_sets_correct_index() {
        let mut v = Vec::new();
        one_hot(&mut v, 2, 5);
        assert_eq!(v, vec![0.0, 0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_one_hot_out_of_bounds_is_all_zeros() {
        let mut v = Vec::new();
        one_hot(&mut v, 10, 5);
        assert_eq!(v, vec![0.0, 0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_one_hot_appends_to_existing() {
        let mut v = vec![9.0];
        one_hot(&mut v, 0, 3);
        assert_eq!(v.len(), 4);
        assert_eq!(v[1], 1.0);
    }

    #[test]
    fn test_multi_hot_multiple_ids() {
        let mut v = Vec::new();
        multi_hot(&mut v, &[0, 2, 4], 5);
        assert_eq!(v, vec![1.0, 0.0, 1.0, 0.0, 1.0]);
    }

    #[test]
    fn test_multi_hot_empty_ids() {
        let mut v = Vec::new();
        multi_hot(&mut v, &[], 4);
        assert_eq!(v, vec![0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_encode_augments_two_augments() {
        let mut v = Vec::new();
        encode_augments(&mut v, &[AugmentId(0), AugmentId(3)], 5);
        assert_eq!(v[0], 1.0);
        assert_eq!(v[1], 0.0);
        assert_eq!(v[3], 1.0);
    }

    #[test]
    fn test_encode_traits_active_trait() {
        let mut trait_index = HashMap::new();
        trait_index.insert("Arcanist".to_string(), 0usize);
        trait_index.insert("Gunner".to_string(), 1usize);
        let active = vec![("Arcanist".to_string(), 4u8)];
        let mut v = Vec::new();
        encode_traits(&mut v, &active, &trait_index, 2);
        assert!(v[0] > 0.0, "Arcanist should be active");
        assert_eq!(v[1], 0.0, "Gunner should not be active");
    }

    #[test]
    fn test_encode_traits_normalizes_count() {
        let mut trait_index = HashMap::new();
        trait_index.insert("Arcanist".to_string(), 0usize);
        let active = vec![("Arcanist".to_string(), 9u8)];
        let mut v = Vec::new();
        encode_traits(&mut v, &active, &trait_index, 1);
        assert!((v[0] - 1.0).abs() < f32::EPSILON, "9/9 should normalize to 1.0");
    }

    #[test]
    fn test_encode_traits_unknown_trait_ignored() {
        let trait_index = HashMap::new(); // empty
        let active = vec![("Unknown".to_string(), 2u8)];
        let mut v = Vec::new();
        encode_traits(&mut v, &active, &trait_index, 3);
        assert_eq!(v, vec![0.0, 0.0, 0.0]);
    }
}

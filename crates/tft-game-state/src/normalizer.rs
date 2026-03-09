//! Normalizes raw game state scalars to [0.0, 1.0] for ML input.

/// Normalize gold amount. Max reasonable gold is ~100.
pub fn normalize_gold(gold: u8) -> f32 {
    (gold as f32 / 100.0).clamp(0.0, 1.0)
}

/// Normalize HP. Range 0-100.
pub fn normalize_hp(hp: u8) -> f32 {
    (hp as f32 / 100.0).clamp(0.0, 1.0)
}

/// Normalize player level. TFT max level is 10 (or 11 in some sets).
pub fn normalize_level(level: u8) -> f32 {
    (level as f32 / 10.0).clamp(0.0, 1.0)
}

/// Normalize round number (stage * 10 + round, max ~60).
pub fn normalize_round(stage: u8, round: u8) -> f32 {
    let combined = stage as f32 * 10.0 + round as f32;
    (combined / 60.0).clamp(0.0, 1.0)
}

/// Normalize streak. Range -10 to +10.
pub fn normalize_streak(streak: i8) -> f32 {
    ((streak as f32 + 10.0) / 20.0).clamp(0.0, 1.0)
}

/// Normalize XP. Max XP needed per level is about 100.
pub fn normalize_xp(xp: u8) -> f32 {
    (xp as f32 / 100.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_gold_zero() {
        assert!((normalize_gold(0) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_normalize_gold_max() {
        assert!((normalize_gold(100) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_normalize_gold_overflow_clamps() {
        assert!((normalize_gold(255) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_normalize_hp_range() {
        for v in 0u8..=100 {
            let n = normalize_hp(v);
            assert!(n >= 0.0 && n <= 1.0, "hp {} normalized to {}", v, n);
        }
    }

    #[test]
    fn test_normalize_level_max() {
        assert!((normalize_level(10) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_normalize_round_stage3_round2() {
        let n = normalize_round(3, 2);
        assert!(n >= 0.0 && n <= 1.0);
        assert!(n > 0.0);
    }

    #[test]
    fn test_normalize_streak_zero_is_midpoint() {
        let n = normalize_streak(0);
        assert!((n - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_normalize_streak_positive() {
        let pos = normalize_streak(5);
        let zero = normalize_streak(0);
        assert!(pos > zero);
    }

    #[test]
    fn test_normalize_streak_range() {
        for v in -10i8..=10 {
            let n = normalize_streak(v);
            assert!(n >= 0.0 && n <= 1.0, "streak {} normalized to {}", v, n);
        }
    }

    #[test]
    fn test_normalize_xp_range() {
        for v in 0u8..=100 {
            let n = normalize_xp(v);
            assert!(n >= 0.0 && n <= 1.0);
        }
    }
}

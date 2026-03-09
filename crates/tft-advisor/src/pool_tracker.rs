//! # Stage: PoolTracker
//!
//! ## Responsibility
//! Track the remaining copies of each champion in the shared pool,
//! using all visible information (player board/bench/shop + opponent boards).
//!
//! ## Guarantees
//! - Deterministic: same state + catalog produces same output
//! - Non-panicking: all fallible paths return Result
//! - O(champions * (board_size + opponents)) per call
//!
//! ## NOT Responsible For
//! - Cross-node deduplication
//! - Predicting opponents' bench contents
//! - Persistence of pool state across rounds

use tft_data::Catalog;
use tft_types::{ChampionId, GameState, TftError};

/// Availability status of a champion in the shared pool.
#[derive(Debug, Clone, PartialEq)]
pub enum PoolStatus {
    /// More than 6 copies remaining.
    Available,
    /// 3–6 copies remaining.
    Low,
    /// 1–2 copies remaining.
    Critical,
    /// 0 copies remaining.
    Exhausted,
}

/// Pool information for a single champion.
#[derive(Debug, Clone)]
pub struct PoolEntry {
    /// Unique champion identifier.
    pub champion_id: ChampionId,
    /// Display name of the champion.
    pub champion_name: String,
    /// Gold cost of this champion.
    pub cost: u8,
    /// Total copies available in the shared pool for this cost tier.
    pub pool_size: u8,
    /// Copies currently visible (player board/bench/shop + all opponent boards).
    pub visible_copies: u8,
    /// `pool_size - visible_copies` (saturating).
    pub remaining: u8,
    /// Categorical availability status derived from `remaining`.
    pub status: PoolStatus,
}

/// Computes remaining pool counts for every champion in the catalog.
pub struct PoolTracker;

impl PoolTracker {
    /// Create a new `PoolTracker`.
    pub fn new() -> Self {
        Self
    }

    /// Compute pool state for every champion in the catalog.
    ///
    /// # Arguments
    /// * `state`   — Current observable game state.
    /// * `catalog` — Champion data catalog.
    ///
    /// # Returns
    /// A `Vec<PoolEntry>` sorted by `remaining` ascending (most contested first).
    ///
    /// # Errors
    /// Returns `TftError::InvalidState` if visible copies overflow a `u8`.
    ///
    /// # Panics
    /// This function never panics.
    pub fn track(&self, state: &GameState, catalog: &Catalog) -> Result<Vec<PoolEntry>, TftError> {
        let mut entries: Vec<PoolEntry> = catalog
            .champions
            .iter()
            .map(|def| {
                let id = def.id;
                let cost = def.cost.as_u8();
                let pool_size = pool_size_for_cost(cost);

                let board_count = state.board.iter().filter(|s| s.champion_id == id).count() as u8;

                let bench_count = state
                    .bench
                    .iter()
                    .filter_map(|s| s.as_ref())
                    .filter(|s| s.champion_id == id)
                    .count() as u8;

                let shop_count = state
                    .shop
                    .iter()
                    .filter(|s| s.champion_id == Some(id) && !s.sold)
                    .count() as u8;

                let opponent_count: u8 = state
                    .opponents
                    .iter()
                    .flat_map(|opp| opp.board_champions.iter())
                    .filter(|&&oid| oid == id)
                    .count() as u8;

                let visible_copies = board_count
                    .saturating_add(bench_count)
                    .saturating_add(shop_count)
                    .saturating_add(opponent_count);

                let remaining = pool_size.saturating_sub(visible_copies);
                let status = pool_status(remaining);

                PoolEntry {
                    champion_id: id,
                    champion_name: def.name.clone(),
                    cost,
                    pool_size,
                    visible_copies,
                    remaining,
                    status,
                }
            })
            .collect();

        entries.sort_by_key(|e| e.remaining);
        Ok(entries)
    }
}

impl Default for PoolTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the fixed pool size for a given cost tier.
///
/// Unknown costs return 0.
pub fn pool_size_for_cost(cost: u8) -> u8 {
    match cost {
        1 => 29,
        2 => 22,
        3 => 18,
        4 => 12,
        5 => 10,
        _ => 0,
    }
}

/// Derive a `PoolStatus` from a remaining count.
pub fn pool_status(remaining: u8) -> PoolStatus {
    match remaining {
        0 => PoolStatus::Exhausted,
        1..=2 => PoolStatus::Critical,
        3..=6 => PoolStatus::Low,
        _ => PoolStatus::Available,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tft_data::Catalog;
    use tft_types::{ChampionSlot, GameState, OpponentSnapshot, ShopSlot, StarLevel};

    fn catalog() -> &'static Catalog {
        Catalog::global().expect("catalog init failed in test")
    }

    fn empty_state() -> GameState {
        GameState::default()
    }

    /// Look up a champion id by name from the global catalog.
    fn champ_id(name: &str) -> ChampionId {
        let cat = catalog();
        let idx = cat
            .champion_by_name
            .get(name)
            .copied()
            .expect("champion not found in test catalog");
        ChampionId(idx as u8)
    }

    fn board_slot(id: ChampionId) -> ChampionSlot {
        ChampionSlot {
            champion_id: id,
            star_level: StarLevel::One,
            items: vec![],
        }
    }

    fn shop_slot_for(id: ChampionId, sold: bool) -> ShopSlot {
        ShopSlot {
            champion_id: Some(id),
            cost: 1,
            locked: false,
            sold,
        }
    }

    fn empty_shop_slot() -> ShopSlot {
        ShopSlot {
            champion_id: None,
            cost: 1,
            locked: false,
            sold: false,
        }
    }

    // ── pool_size_for_cost ────────────────────────────────────────────────────

    #[test]
    fn test_pool_size_cost_1_is_29() {
        assert_eq!(pool_size_for_cost(1), 29);
    }

    #[test]
    fn test_pool_size_cost_2_is_22() {
        assert_eq!(pool_size_for_cost(2), 22);
    }

    #[test]
    fn test_pool_size_cost_3_is_18() {
        assert_eq!(pool_size_for_cost(3), 18);
    }

    #[test]
    fn test_pool_size_cost_4_is_12() {
        assert_eq!(pool_size_for_cost(4), 12);
    }

    #[test]
    fn test_pool_size_cost_5_is_10() {
        assert_eq!(pool_size_for_cost(5), 10);
    }

    #[test]
    fn test_pool_size_unknown_cost_zero() {
        assert_eq!(pool_size_for_cost(0), 0);
        assert_eq!(pool_size_for_cost(6), 0);
    }

    // ── pool_status ───────────────────────────────────────────────────────────

    #[test]
    fn test_pool_status_exhausted_at_zero() {
        assert_eq!(pool_status(0), PoolStatus::Exhausted);
    }

    #[test]
    fn test_pool_status_critical_at_1_and_2() {
        assert_eq!(pool_status(1), PoolStatus::Critical);
        assert_eq!(pool_status(2), PoolStatus::Critical);
    }

    #[test]
    fn test_pool_status_low_at_3_through_6() {
        for v in 3u8..=6 {
            assert_eq!(pool_status(v), PoolStatus::Low, "remaining={}", v);
        }
    }

    #[test]
    fn test_pool_status_available_at_7_plus() {
        assert_eq!(pool_status(7), PoolStatus::Available);
        assert_eq!(pool_status(29), PoolStatus::Available);
    }

    // ── PoolTracker::track ────────────────────────────────────────────────────

    #[test]
    fn test_track_empty_state_all_remaining_equals_pool_size() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let entries = tracker.track(&empty_state(), cat).expect("track failed");
        // Every entry should have visible_copies == 0 and remaining == pool_size
        for e in &entries {
            assert_eq!(
                e.visible_copies, 0,
                "{} should have 0 visible",
                e.champion_name
            );
            assert_eq!(
                e.remaining, e.pool_size,
                "{} remaining should equal pool_size",
                e.champion_name
            );
        }
    }

    #[test]
    fn test_track_returns_entry_per_catalog_champion() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let entries = tracker.track(&empty_state(), cat).expect("track failed");
        assert_eq!(entries.len(), cat.champion_count());
    }

    #[test]
    fn test_track_board_copies_counted() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let jinx = champ_id("Jinx");
        let mut state = empty_state();
        state.board.push(board_slot(jinx));
        state.board.push(board_slot(jinx));
        let entries = tracker.track(&state, cat).expect("track failed");
        let e = entries
            .iter()
            .find(|e| e.champion_id == jinx)
            .expect("Jinx not found");
        assert_eq!(e.visible_copies, 2);
        assert_eq!(e.remaining, 16); // pool_size=18, visible=2
    }

    #[test]
    fn test_track_bench_copies_counted() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let jinx = champ_id("Jinx");
        let mut state = empty_state();
        state.bench.push(Some(board_slot(jinx)));
        state.bench.push(Some(board_slot(jinx)));
        state.bench.push(None);
        let entries = tracker.track(&state, cat).expect("track failed");
        let e = entries
            .iter()
            .find(|e| e.champion_id == jinx)
            .expect("Jinx not found");
        assert_eq!(e.visible_copies, 2);
    }

    #[test]
    fn test_track_shop_copies_counted() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let caitlyn = champ_id("Caitlyn");
        let mut state = empty_state();
        state.shop.push(shop_slot_for(caitlyn, false));
        state.shop.push(shop_slot_for(caitlyn, false));
        let entries = tracker.track(&state, cat).expect("track failed");
        let e = entries
            .iter()
            .find(|e| e.champion_id == caitlyn)
            .expect("Caitlyn not found");
        assert_eq!(e.visible_copies, 2);
    }

    #[test]
    fn test_track_sold_shop_slot_excluded() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let caitlyn = champ_id("Caitlyn");
        let mut state = empty_state();
        state.shop.push(shop_slot_for(caitlyn, false)); // visible
        state.shop.push(shop_slot_for(caitlyn, true)); // sold — NOT counted
        let entries = tracker.track(&state, cat).expect("track failed");
        let e = entries
            .iter()
            .find(|e| e.champion_id == caitlyn)
            .expect("Caitlyn not found");
        assert_eq!(e.visible_copies, 1);
    }

    #[test]
    fn test_track_empty_shop_slots_ignored() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let mut state = empty_state();
        state.shop.push(empty_shop_slot());
        state.shop.push(empty_shop_slot());
        let entries = tracker.track(&state, cat).expect("track failed");
        for e in &entries {
            assert_eq!(
                e.visible_copies, 0,
                "{} should have 0 visible",
                e.champion_name
            );
        }
    }

    #[test]
    fn test_track_opponent_board_copies_counted() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let jayce = champ_id("Jayce"); // cost-5, pool=10
        let mut state = empty_state();
        state.opponents.push(OpponentSnapshot {
            player_name: "Bob".to_string(),
            hp: 80,
            level: 9,
            board_champions: vec![jayce, jayce],
            active_traits: vec![],
        });
        let entries = tracker.track(&state, cat).expect("track failed");
        let e = entries
            .iter()
            .find(|e| e.champion_id == jayce)
            .expect("Jayce not found");
        assert_eq!(e.visible_copies, 2);
        assert_eq!(e.remaining, 8);
    }

    #[test]
    fn test_track_all_sources_combined() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let jayce = champ_id("Jayce"); // cost-5, pool=10
        let mut state = empty_state();
        state.board.push(board_slot(jayce)); // +1
        state.bench.push(Some(board_slot(jayce))); // +1
        state.shop.push(shop_slot_for(jayce, false)); // +1
        state.opponents.push(OpponentSnapshot {
            player_name: "Alice".to_string(),
            hp: 50,
            level: 9,
            board_champions: vec![jayce, jayce], // +2
            active_traits: vec![],
        });
        let entries = tracker.track(&state, cat).expect("track failed");
        let e = entries
            .iter()
            .find(|e| e.champion_id == jayce)
            .expect("Jayce not found");
        assert_eq!(e.visible_copies, 5);
        assert_eq!(e.remaining, 5);
    }

    #[test]
    fn test_track_saturating_sub_no_overflow() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let jayce = champ_id("Jayce"); // pool=10
        let mut state = empty_state();
        // Put 15 copies visible (more than pool_size of 10)
        state.opponents.push(OpponentSnapshot {
            player_name: "cheater".to_string(),
            hp: 100,
            level: 9,
            board_champions: vec![jayce; 15],
            active_traits: vec![],
        });
        let entries = tracker.track(&state, cat).expect("track failed");
        let e = entries
            .iter()
            .find(|e| e.champion_id == jayce)
            .expect("Jayce not found");
        assert_eq!(e.remaining, 0); // saturating_sub never underflows
    }

    #[test]
    fn test_track_sorted_by_remaining_ascending() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let entries = tracker.track(&empty_state(), cat).expect("track failed");
        // All remaining == pool_size in empty state, so order by pool_size asc (cost-5 first)
        for window in entries.windows(2) {
            assert!(
                window[0].remaining <= window[1].remaining,
                "entries not sorted: {} ({}) > {} ({})",
                window[0].champion_name,
                window[0].remaining,
                window[1].champion_name,
                window[1].remaining,
            );
        }
    }

    #[test]
    fn test_track_status_exhausted_when_all_visible() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let jayce = champ_id("Jayce"); // pool=10
        let mut state = empty_state();
        state.opponents.push(OpponentSnapshot {
            player_name: "P1".to_string(),
            hp: 100,
            level: 9,
            board_champions: vec![jayce; 10],
            active_traits: vec![],
        });
        let entries = tracker.track(&state, cat).expect("track failed");
        let e = entries
            .iter()
            .find(|e| e.champion_id == jayce)
            .expect("Jayce not found");
        assert_eq!(e.status, PoolStatus::Exhausted);
    }

    #[test]
    fn test_track_status_critical_at_two_remaining() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let jayce = champ_id("Jayce"); // pool=10
        let mut state = empty_state();
        state.opponents.push(OpponentSnapshot {
            player_name: "P1".to_string(),
            hp: 100,
            level: 9,
            board_champions: vec![jayce; 8], // 10-8=2 remaining
            active_traits: vec![],
        });
        let entries = tracker.track(&state, cat).expect("track failed");
        let e = entries
            .iter()
            .find(|e| e.champion_id == jayce)
            .expect("Jayce not found");
        assert_eq!(e.status, PoolStatus::Critical);
    }

    #[test]
    fn test_track_status_low_at_5_remaining() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let jayce = champ_id("Jayce"); // pool=10
        let mut state = empty_state();
        state.opponents.push(OpponentSnapshot {
            player_name: "P1".to_string(),
            hp: 100,
            level: 9,
            board_champions: vec![jayce; 5], // 10-5=5 remaining
            active_traits: vec![],
        });
        let entries = tracker.track(&state, cat).expect("track failed");
        let e = entries
            .iter()
            .find(|e| e.champion_id == jayce)
            .expect("Jayce not found");
        assert_eq!(e.status, PoolStatus::Low);
    }

    #[test]
    fn test_track_status_available_when_empty_state() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let entries = tracker.track(&empty_state(), cat).expect("track failed");
        for e in &entries {
            assert_eq!(
                e.status,
                PoolStatus::Available,
                "{} should be Available",
                e.champion_name
            );
        }
    }

    #[test]
    fn test_track_multiple_opponents_all_counted() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let caitlyn = champ_id("Caitlyn"); // cost-1, pool=29
        let mut state = empty_state();
        for i in 0..3 {
            state.opponents.push(OpponentSnapshot {
                player_name: format!("P{}", i),
                hp: 100,
                level: 6,
                board_champions: vec![caitlyn; 3],
                active_traits: vec![],
            });
        }
        // 3 opponents * 3 each = 9 visible
        let entries = tracker.track(&state, cat).expect("track failed");
        let e = entries
            .iter()
            .find(|e| e.champion_id == caitlyn)
            .expect("Caitlyn not found");
        assert_eq!(e.visible_copies, 9);
        assert_eq!(e.remaining, 20);
    }

    #[test]
    fn test_track_entry_fields_populated_correctly() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let entries = tracker.track(&empty_state(), cat).expect("track failed");
        for e in &entries {
            assert!(!e.champion_name.is_empty(), "name should not be empty");
            assert!(e.cost >= 1 && e.cost <= 5, "cost should be 1-5");
            assert_eq!(e.pool_size, pool_size_for_cost(e.cost));
        }
    }

    #[test]
    fn test_track_cost1_pool_size_29() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let caitlyn = champ_id("Caitlyn"); // cost-1
        let entries = tracker.track(&empty_state(), cat).expect("track failed");
        let e = entries
            .iter()
            .find(|e| e.champion_id == caitlyn)
            .expect("Caitlyn not found");
        assert_eq!(e.cost, 1);
        assert_eq!(e.pool_size, 29);
    }

    #[test]
    fn test_track_cost5_pool_size_10() {
        let cat = catalog();
        let tracker = PoolTracker::new();
        let jayce = champ_id("Jayce"); // cost-5
        let entries = tracker.track(&empty_state(), cat).expect("track failed");
        let e = entries
            .iter()
            .find(|e| e.champion_id == jayce)
            .expect("Jayce not found");
        assert_eq!(e.cost, 5);
        assert_eq!(e.pool_size, 10);
    }

    #[test]
    fn test_default_and_new_equivalent() {
        let cat = catalog();
        let a = PoolTracker::new();
        let b = PoolTracker::default();
        let r1 = a.track(&empty_state(), cat).expect("track failed");
        let r2 = b.track(&empty_state(), cat).expect("track failed");
        assert_eq!(r1.len(), r2.len());
    }

    #[test]
    fn test_track_mixed_champions_not_confused() {
        // Ensure counts for one champion don't bleed into another.
        let cat = catalog();
        let tracker = PoolTracker::new();
        let caitlyn = champ_id("Caitlyn");
        let jinx = champ_id("Jinx");
        let mut state = empty_state();
        state.board.push(board_slot(caitlyn));
        state.board.push(board_slot(caitlyn));
        let entries = tracker.track(&state, cat).expect("track failed");
        let jinx_e = entries
            .iter()
            .find(|e| e.champion_id == jinx)
            .expect("Jinx not found");
        assert_eq!(
            jinx_e.visible_copies, 0,
            "Jinx should not be affected by Caitlyn copies"
        );
    }
}

use serde::{Deserialize, Serialize};
use crate::{AugmentId, ChampionId, ChampionSlot};

/// Round information.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoundInfo {
    pub stage: u8,
    pub round: u8,
}

impl RoundInfo {
    pub fn as_float(&self) -> f32 {
        self.stage as f32 + self.round as f32 * 0.1
    }
}

/// A single slot in the shop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopSlot {
    pub champion_id: Option<ChampionId>,
    pub cost: u8,
    pub locked: bool,
    pub sold: bool,
}

/// The complete observable game state at a decision point.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameState {
    pub round: RoundInfo,
    pub board: Vec<ChampionSlot>,
    pub bench: Vec<Option<ChampionSlot>>,
    pub shop: Vec<ShopSlot>,
    pub gold: u8,
    pub hp: u8,
    pub level: u8,
    pub xp: u8,
    pub streak: i8,
    pub current_augments: Vec<AugmentId>,
    pub augment_choices: Option<[AugmentId; 3]>,
    pub active_traits: Vec<(String, u8)>,
}

impl GameState {
    pub fn is_augment_phase(&self) -> bool {
        self.augment_choices.is_some()
    }

    pub fn board_size(&self) -> usize {
        self.board.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_info_as_float() {
        let r = RoundInfo { stage: 3, round: 2 };
        assert!((r.as_float() - 3.2).abs() < 0.001);
    }

    #[test]
    fn test_game_state_default_is_not_augment_phase() {
        let state = GameState::default();
        assert!(!state.is_augment_phase());
    }

    #[test]
    fn test_game_state_augment_phase_detection() {
        let mut state = GameState::default();
        state.augment_choices = Some([AugmentId(0), AugmentId(1), AugmentId(2)]);
        assert!(state.is_augment_phase());
    }

    #[test]
    fn test_game_state_board_size() {
        let mut state = GameState::default();
        state.board.push(ChampionSlot {
            champion_id: ChampionId(1),
            star_level: crate::StarLevel::Two,
            items: vec![],
        });
        assert_eq!(state.board_size(), 1);
    }

    #[test]
    fn test_game_state_serde_roundtrip() {
        let mut state = GameState::default();
        state.gold = 50;
        state.hp = 75;
        state.level = 7;
        let json = serde_json::to_string(&state).expect("serialize failed in test");
        let back: GameState = serde_json::from_str(&json).expect("deserialize failed in test");
        assert_eq!(state.gold, back.gold);
        assert_eq!(state.hp, back.hp);
        assert_eq!(state.level, back.level);
    }

    #[test]
    fn test_shop_slot_default_values() {
        let slot = ShopSlot { champion_id: None, cost: 3, locked: false, sold: false };
        assert!(slot.champion_id.is_none());
        assert!(!slot.locked);
    }
}

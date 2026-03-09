//! MockReader: replays pre-recorded GameState snapshots for testing.

use crate::reader::{GameStateReader, ReaderMode};
use std::sync::{Arc, Mutex};
use tft_types::{
    AugmentId, ChampionId, ChampionSlot, GameState, RoundInfo, ShopSlot, StarLevel, TftError,
};

/// A reader that returns a fixed sequence of GameStates for testing.
pub struct MockReader {
    pub(crate) states: Arc<Mutex<Vec<GameState>>>,
    index: Arc<Mutex<usize>>,
    connected: bool,
}

impl MockReader {
    pub fn new() -> Self {
        Self {
            states: Arc::new(Mutex::new(vec![default_game_state()])),
            index: Arc::new(Mutex::new(0)),
            connected: true,
        }
    }

    pub fn with_states(states: Vec<GameState>) -> Self {
        Self {
            states: Arc::new(Mutex::new(states)),
            index: Arc::new(Mutex::new(0)),
            connected: true,
        }
    }

    pub fn disconnected() -> Self {
        Self {
            states: Arc::new(Mutex::new(vec![])),
            index: Arc::new(Mutex::new(0)),
            connected: false,
        }
    }

    pub fn push_state(&self, state: GameState) {
        if let Ok(mut v) = self.states.lock() {
            v.push(state);
        }
    }
}

impl Default for MockReader {
    fn default() -> Self {
        Self::new()
    }
}

impl GameStateReader for MockReader {
    fn poll(&self) -> Result<Option<GameState>, TftError> {
        if !self.connected {
            return Ok(None);
        }
        let states = self
            .states
            .lock()
            .map_err(|_| TftError::Capture("mock lock poisoned".to_string()))?;
        let mut idx = self
            .index
            .lock()
            .map_err(|_| TftError::Capture("mock index lock poisoned".to_string()))?;
        if states.is_empty() {
            return Ok(None);
        }
        let state = states[*idx % states.len()].clone();
        *idx += 1;
        Ok(Some(state))
    }

    fn mode(&self) -> ReaderMode {
        ReaderMode::Mock
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Returns a sensible default game state for testing.
pub fn default_game_state() -> GameState {
    GameState {
        round: RoundInfo { stage: 2, round: 1 },
        board: vec![
            ChampionSlot {
                champion_id: ChampionId(0),
                star_level: StarLevel::One,
                items: vec![],
            },
            ChampionSlot {
                champion_id: ChampionId(1),
                star_level: StarLevel::Two,
                items: vec![],
            },
        ],
        bench: vec![None; 9],
        shop: vec![
            ShopSlot {
                champion_id: Some(ChampionId(2)),
                cost: 2,
                locked: false,
                sold: false,
            },
            ShopSlot {
                champion_id: None,
                cost: 0,
                locked: false,
                sold: false,
            },
            ShopSlot {
                champion_id: None,
                cost: 0,
                locked: false,
                sold: false,
            },
            ShopSlot {
                champion_id: None,
                cost: 0,
                locked: false,
                sold: false,
            },
            ShopSlot {
                champion_id: None,
                cost: 0,
                locked: false,
                sold: false,
            },
        ],
        gold: 30,
        hp: 80,
        level: 5,
        xp: 20,
        streak: 1,
        current_augments: vec![AugmentId(0)],
        augment_choices: Some([AugmentId(0), AugmentId(1), AugmentId(2)]),
        active_traits: vec![("Arcanist".to_string(), 2)],
        opponents: vec![],
    }
}

/// Returns a game state where augment choice is available.
pub fn augment_phase_state(choices: [AugmentId; 3]) -> GameState {
    let mut state = default_game_state();
    state.augment_choices = Some(choices);
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_reader_poll_returns_state() {
        let reader = MockReader::new();
        let result = reader.poll();
        assert!(result.is_ok());
        assert!(result.expect("poll failed in test").is_some());
    }

    #[test]
    fn test_mock_reader_disconnected_returns_none() {
        let reader = MockReader::disconnected();
        let result = reader.poll().expect("poll failed in test");
        assert!(result.is_none());
    }

    #[test]
    fn test_mock_reader_mode_is_mock() {
        let reader = MockReader::new();
        assert_eq!(reader.mode(), ReaderMode::Mock);
    }

    #[test]
    fn test_mock_reader_is_connected() {
        let connected = MockReader::new();
        let disconnected = MockReader::disconnected();
        assert!(connected.is_connected());
        assert!(!disconnected.is_connected());
    }

    #[test]
    fn test_mock_reader_cycles_through_states() {
        let states = vec![default_game_state(), default_game_state()];
        let reader = MockReader::with_states(states);
        for _ in 0..6 {
            let r = reader.poll().expect("poll failed in test");
            assert!(r.is_some());
        }
    }

    #[test]
    fn test_mock_reader_push_state() {
        let reader = MockReader::new();
        let initial_len = { reader.states.lock().expect("lock failed in test").len() };
        reader.push_state(default_game_state());
        let new_len = { reader.states.lock().expect("lock failed in test").len() };
        assert_eq!(new_len, initial_len + 1);
    }

    #[test]
    fn test_default_game_state_is_augment_phase() {
        let state = default_game_state();
        assert!(state.is_augment_phase());
    }

    #[test]
    fn test_default_game_state_has_board() {
        let state = default_game_state();
        assert!(!state.board.is_empty());
    }

    #[test]
    fn test_augment_phase_state_has_correct_choices() {
        let choices = [AugmentId(3), AugmentId(7), AugmentId(11)];
        let state = augment_phase_state(choices);
        assert_eq!(state.augment_choices, Some(choices));
    }
}

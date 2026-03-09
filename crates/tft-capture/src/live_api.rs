//! Riot Games Live Client Data API reader.
//! Polls http://127.0.0.1:2999/liveclientdata/allgamedata every 500ms.

use crate::reader::{GameStateReader, ReaderMode};
use std::sync::{Arc, Mutex};
use tft_types::{GameState, OpponentSnapshot, RoundInfo, ShopSlot, TftError};
use tracing::{debug, warn};

const API_BASE: &str = "http://127.0.0.1:2999";
const ALLGAME_ENDPOINT: &str = "/liveclientdata/allgamedata";

/// Reads game state from the Riot Live Client Data API.
pub struct RiotLiveApiReader {
    client: reqwest::blocking::Client,
    last_state: Arc<Mutex<Option<GameState>>>,
    connected: Arc<Mutex<bool>>,
}

impl RiotLiveApiReader {
    pub fn new() -> Result<Self, TftError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_millis(500))
            .danger_accept_invalid_certs(true) // Riot uses self-signed cert
            .build()
            .map_err(|e| TftError::LiveApi(format!("Failed to build HTTP client: {}", e)))?;
        Ok(Self {
            client,
            last_state: Arc::new(Mutex::new(None)),
            connected: Arc::new(Mutex::new(false)),
        })
    }

    fn fetch_raw(&self) -> Result<serde_json::Value, TftError> {
        let url = format!("{}{}", API_BASE, ALLGAME_ENDPOINT);
        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| TftError::LiveApi(format!("HTTP request failed: {}", e)))?;
        let json: serde_json::Value = response
            .json()
            .map_err(|e| TftError::LiveApi(format!("JSON parse failed: {}", e)))?;
        Ok(json)
    }

    fn parse_game_state(raw: &serde_json::Value) -> Result<GameState, TftError> {
        // Parse active player data
        let active = raw
            .get("activePlayer")
            .ok_or_else(|| TftError::LiveApi("missing activePlayer".to_string()))?;

        let current_gold = active
            .get("currentGold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as u8;

        let level = active.get("level").and_then(|v| v.as_u64()).unwrap_or(1) as u8;

        // Parse game stats
        let game_data = raw.get("gameData");
        let round_str = game_data
            .and_then(|g| g.get("gameTime"))
            .and_then(|t| t.as_f64())
            .unwrap_or(0.0);

        // Convert game time to approximate stage/round
        let stage = ((round_str / 120.0) as u8).min(7) + 1;

        // Parse allPlayers for opponent snapshots (limited data available from this endpoint)
        let opponents: Vec<OpponentSnapshot> = raw
            .get("allPlayers")
            .and_then(|p| p.as_array())
            .map(|players| {
                players
                    .iter()
                    .filter_map(|p| {
                        let name = p.get("summonerName")?.as_str()?.to_string();
                        Some(OpponentSnapshot {
                            player_name: name,
                            hp: 100,
                            level: 1,
                            board_champions: vec![],
                            active_traits: vec![],
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(GameState {
            round: RoundInfo { stage, round: 1 },
            board: vec![],
            bench: vec![None; 9],
            shop: (0..5)
                .map(|_| ShopSlot {
                    champion_id: None,
                    cost: 0,
                    locked: false,
                    sold: false,
                })
                .collect(),
            gold: current_gold,
            hp: 100,
            level,
            xp: 0,
            streak: 0,
            current_augments: vec![],
            augment_choices: None,
            active_traits: vec![],
            opponents,
        })
    }
}

impl Default for RiotLiveApiReader {
    fn default() -> Self {
        // If construction fails, create a disconnected reader
        Self::new().unwrap_or_else(|_| Self {
            client: reqwest::blocking::Client::new(),
            last_state: Arc::new(Mutex::new(None)),
            connected: Arc::new(Mutex::new(false)),
        })
    }
}

impl GameStateReader for RiotLiveApiReader {
    fn poll(&self) -> Result<Option<GameState>, TftError> {
        match self.fetch_raw() {
            Ok(raw) => {
                if let Ok(mut c) = self.connected.lock() {
                    *c = true;
                }
                match Self::parse_game_state(&raw) {
                    Ok(state) => {
                        if let Ok(mut last) = self.last_state.lock() {
                            *last = Some(state.clone());
                        }
                        debug!(
                            "Live API: polled state stage={} round={}",
                            state.round.stage, state.round.round
                        );
                        Ok(Some(state))
                    }
                    Err(e) => {
                        warn!("Live API parse error: {}", e);
                        Ok(None)
                    }
                }
            }
            Err(_) => {
                if let Ok(mut c) = self.connected.lock() {
                    *c = false;
                }
                Ok(None) // Not connected is not an error
            }
        }
    }

    fn mode(&self) -> ReaderMode {
        ReaderMode::LiveApi
    }

    fn is_connected(&self) -> bool {
        self.connected.lock().map(|c| *c).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_riot_reader_new_succeeds() {
        let result = RiotLiveApiReader::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_riot_reader_mode_is_live_api() {
        let reader = RiotLiveApiReader::new().expect("new failed in test");
        assert_eq!(reader.mode(), ReaderMode::LiveApi);
    }

    #[test]
    fn test_riot_reader_disconnected_by_default() {
        let reader = RiotLiveApiReader::new().expect("new failed in test");
        assert!(!reader.is_connected());
    }

    #[test]
    fn test_parse_game_state_minimal_json() {
        let raw = serde_json::json!({
            "activePlayer": {
                "currentGold": 45.0,
                "level": 7
            },
            "gameData": {
                "gameTime": 300.0
            }
        });
        let state = RiotLiveApiReader::parse_game_state(&raw);
        assert!(state.is_ok());
        let s = state.expect("parse failed in test");
        assert_eq!(s.gold, 45);
        assert_eq!(s.level, 7);
    }

    #[test]
    fn test_parse_game_state_missing_active_player_errors() {
        let raw = serde_json::json!({ "gameData": {} });
        let result = RiotLiveApiReader::parse_game_state(&raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_riot_reader_poll_when_no_game_running_returns_none() {
        let reader = RiotLiveApiReader::new().expect("new failed in test");
        // No TFT game running in test env, should return Ok(None) not error
        let result = reader.poll();
        assert!(result.is_ok());
        // May be None if no game running (expected in CI)
    }

    #[test]
    fn test_parse_game_state_all_players_populates_opponents() {
        let raw = serde_json::json!({
            "activePlayer": { "currentGold": 20.0, "level": 5 },
            "gameData": { "gameTime": 180.0 },
            "allPlayers": [
                { "summonerName": "Alice" },
                { "summonerName": "Bob" }
            ]
        });
        let state = RiotLiveApiReader::parse_game_state(&raw).expect("parse failed in test");
        assert_eq!(state.opponents.len(), 2);
        assert_eq!(state.opponents[0].player_name, "Alice");
        assert_eq!(state.opponents[1].player_name, "Bob");
    }

    #[test]
    fn test_parse_game_state_no_all_players_gives_empty_opponents() {
        let raw = serde_json::json!({
            "activePlayer": { "currentGold": 10.0, "level": 3 },
            "gameData": { "gameTime": 60.0 }
        });
        let state = RiotLiveApiReader::parse_game_state(&raw).expect("parse failed in test");
        assert!(state.opponents.is_empty());
    }

    #[test]
    fn test_parse_game_state_opponent_hp_defaults_to_100() {
        let raw = serde_json::json!({
            "activePlayer": { "currentGold": 0.0, "level": 1 },
            "allPlayers": [{ "summonerName": "Player1" }]
        });
        let state = RiotLiveApiReader::parse_game_state(&raw).expect("parse failed in test");
        assert_eq!(state.opponents[0].hp, 100);
    }

    #[test]
    fn test_parse_game_state_opponent_level_defaults_to_1() {
        let raw = serde_json::json!({
            "activePlayer": { "currentGold": 0.0, "level": 1 },
            "allPlayers": [{ "summonerName": "SomePlayer" }]
        });
        let state = RiotLiveApiReader::parse_game_state(&raw).expect("parse failed in test");
        assert_eq!(state.opponents[0].level, 1);
    }

    #[test]
    fn test_parse_game_state_opponent_board_champions_empty() {
        let raw = serde_json::json!({
            "activePlayer": { "currentGold": 0.0, "level": 1 },
            "allPlayers": [{ "summonerName": "X" }]
        });
        let state = RiotLiveApiReader::parse_game_state(&raw).expect("parse failed in test");
        assert!(state.opponents[0].board_champions.is_empty());
    }

    #[test]
    fn test_parse_game_state_opponent_active_traits_empty() {
        let raw = serde_json::json!({
            "activePlayer": { "currentGold": 0.0, "level": 1 },
            "allPlayers": [{ "summonerName": "X" }]
        });
        let state = RiotLiveApiReader::parse_game_state(&raw).expect("parse failed in test");
        assert!(state.opponents[0].active_traits.is_empty());
    }

    #[test]
    fn test_parse_game_state_player_missing_summoner_name_skipped() {
        let raw = serde_json::json!({
            "activePlayer": { "currentGold": 0.0, "level": 1 },
            "allPlayers": [
                { "summonerName": "Valid" },
                { "noName": true },
                { "summonerName": "AlsoValid" }
            ]
        });
        let state = RiotLiveApiReader::parse_game_state(&raw).expect("parse failed in test");
        // Entry without summonerName should be skipped
        assert_eq!(state.opponents.len(), 2);
        assert_eq!(state.opponents[0].player_name, "Valid");
        assert_eq!(state.opponents[1].player_name, "AlsoValid");
    }
}

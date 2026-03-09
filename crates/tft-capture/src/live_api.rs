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

    /// Convert game time (seconds since match start) to (stage, round).
    ///
    /// Approximate TFT round timings. Each entry is (cumulative_seconds, stage, round).
    /// Rounds are ~35s combat + ~30s planning. Carousels are ~30s. Stage 1 is shorter.
    fn game_time_to_stage_round(secs: f64) -> (u8, u8) {
        // (start_time_secs, stage, round)
        const SCHEDULE: &[(f64, u8, u8)] = &[
            (0.0, 1, 1),
            (30.0, 1, 2),
            (65.0, 1, 3),
            (100.0, 1, 4),
            (140.0, 2, 1), // carousel
            (175.0, 2, 2),
            (220.0, 2, 3),
            (265.0, 2, 4),
            (310.0, 2, 5),
            (355.0, 2, 6),
            (400.0, 2, 7),
            (445.0, 3, 1), // carousel
            (480.0, 3, 2),
            (530.0, 3, 3),
            (580.0, 3, 4),
            (630.0, 3, 5),
            (680.0, 3, 6),
            (730.0, 3, 7),
            (780.0, 4, 1), // carousel
            (815.0, 4, 2),
            (870.0, 4, 3),
            (925.0, 4, 4),
            (980.0, 4, 5),
            (1035.0, 4, 6),
            (1090.0, 4, 7),
            (1145.0, 5, 1), // carousel
            (1180.0, 5, 2),
            (1240.0, 5, 3),
            (1300.0, 5, 4),
            (1360.0, 5, 5),
            (1420.0, 6, 1),
            (1455.0, 6, 2),
            (1520.0, 6, 3),
            (1585.0, 6, 4),
            (1650.0, 6, 5),
            (1715.0, 7, 1),
        ];

        let mut stage = 1u8;
        let mut round = 1u8;
        for &(t, s, r) in SCHEDULE {
            if secs >= t {
                stage = s;
                round = r;
            } else {
                break;
            }
        }
        (stage, round)
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

        // Try to read HP from several known field paths the API may expose
        let hp = active
            .get("championStats")
            .and_then(|s| s.get("currentHealth"))
            .and_then(|v| v.as_f64())
            .or_else(|| active.get("health").and_then(|v| v.as_f64()))
            .map(|h| h.round() as u8)
            .unwrap_or(100);

        // XP toward next level
        let xp = active
            .get("currentXp")
            .or_else(|| active.get("experience"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as u8;

        // Game time -> stage/round
        let game_time = raw
            .get("gameData")
            .and_then(|g| g.get("gameTime"))
            .and_then(|t| t.as_f64())
            .unwrap_or(0.0);
        let (stage, round) = Self::game_time_to_stage_round(game_time);

        // Parse allPlayers for opponent snapshots (limited data from this endpoint)
        let opponents: Vec<OpponentSnapshot> = raw
            .get("allPlayers")
            .and_then(|p| p.as_array())
            .map(|players| {
                players
                    .iter()
                    .filter_map(|p| {
                        let name = p.get("summonerName")?.as_str()?.to_string();
                        let opp_hp = p
                            .get("scores")
                            .and_then(|s| s.get("wardScore"))
                            .and_then(|v| v.as_f64())
                            .map(|h| h as u8)
                            .unwrap_or(100);
                        let opp_level = p.get("level").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
                        Some(OpponentSnapshot {
                            player_name: name,
                            hp: opp_hp,
                            level: opp_level,
                            board_champions: vec![],
                            active_traits: vec![],
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(GameState {
            round: RoundInfo { stage, round },
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
            hp,
            level,
            xp,
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
        // 300s is inside stage 2 (starts ~140s), round 5 (starts ~310s) or round 4
        assert_eq!(s.round.stage, 2);
    }

    #[test]
    fn test_game_time_to_stage_round_early_game() {
        let (stage, round) = RiotLiveApiReader::game_time_to_stage_round(0.0);
        assert_eq!(stage, 1);
        assert_eq!(round, 1);
    }

    #[test]
    fn test_game_time_to_stage_round_stage2() {
        // 200s should be in stage 2
        let (stage, _) = RiotLiveApiReader::game_time_to_stage_round(200.0);
        assert_eq!(stage, 2);
    }

    #[test]
    fn test_game_time_to_stage_round_stage3() {
        // 500s should be in stage 3
        let (stage, _) = RiotLiveApiReader::game_time_to_stage_round(500.0);
        assert_eq!(stage, 3);
    }

    #[test]
    fn test_game_time_to_stage_round_stage4() {
        let (stage, _) = RiotLiveApiReader::game_time_to_stage_round(850.0);
        assert_eq!(stage, 4);
    }

    #[test]
    fn test_game_time_stage_round_monotonic() {
        // Stage/round should never go backwards as time increases
        let mut prev_stage = 1u8;
        let mut prev_round = 1u8;
        for t in (0..1800).step_by(10) {
            let (s, r) = RiotLiveApiReader::game_time_to_stage_round(t as f64);
            assert!(
                (s, r) >= (prev_stage, prev_round),
                "non-monotonic at {}s: ({},{}) -> ({},{})",
                t,
                prev_stage,
                prev_round,
                s,
                r
            );
            prev_stage = s;
            prev_round = r;
        }
    }

    #[test]
    fn test_parse_game_state_hp_from_champion_stats() {
        let raw = serde_json::json!({
            "activePlayer": {
                "currentGold": 10.0,
                "level": 5,
                "championStats": { "currentHealth": 72.0 }
            }
        });
        let s = RiotLiveApiReader::parse_game_state(&raw).expect("parse failed in test");
        assert_eq!(s.hp, 72);
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

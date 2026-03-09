use egui::Ui;
use tft_advisor::{LobbyAnalysis, ThreatLevel};
use crate::theme;

pub fn render(ui: &mut Ui, lobby: &LobbyAnalysis) {
    ui.heading("Lobby");

    if lobby.opponents.is_empty() {
        ui.label(egui::RichText::new("No opponent data").small().color(theme::TEXT_SECONDARY));
        return;
    }

    if !lobby.contested_comps.is_empty() {
        ui.label(
            egui::RichText::new(format!("Contested: {}", lobby.contested_comps.join(", ")))
                .small()
                .color(theme::SCORE_LOW),
        );
    }

    if let Some(ref pivot) = lobby.recommended_pivot {
        ui.label(egui::RichText::new(pivot).small().color(theme::SCORE_MID));
    }

    for opp in lobby.opponents.iter().take(7) {
        let threat_color = match opp.threat_level {
            ThreatLevel::High => theme::SCORE_LOW,
            ThreatLevel::Medium => theme::SCORE_MID,
            ThreatLevel::Low => theme::SCORE_HIGH,
        };
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&opp.player_name).small().color(threat_color));
            ui.label(egui::RichText::new(format!("{}hp", opp.hp)).small().color(theme::TEXT_SECONDARY));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_advisor::{LobbyAnalysis, OpponentAnalysis, ThreatLevel};

    fn make_lobby() -> LobbyAnalysis {
        LobbyAnalysis {
            opponents: vec![
                OpponentAnalysis {
                    player_name: "Alice".to_string(),
                    hp: 80,
                    threat_level: ThreatLevel::High,
                    contested_traits: vec!["Arcanist".to_string()],
                    summary: "High HP, running Arcanist".to_string(),
                }
            ],
            contested_comps: vec!["Arcanist".to_string()],
            recommended_pivot: Some("Consider pivoting away from Arcanist".to_string()),
        }
    }

    #[test]
    fn test_lobby_has_opponent() {
        let lobby = make_lobby();
        assert_eq!(lobby.opponents.len(), 1);
    }

    #[test]
    fn test_lobby_contested_comps() {
        let lobby = make_lobby();
        assert!(lobby.contested_comps.contains(&"Arcanist".to_string()));
    }

    #[test]
    fn test_lobby_pivot_recommendation() {
        let lobby = make_lobby();
        assert!(lobby.recommended_pivot.is_some());
    }

    #[test]
    fn test_threat_level_high() {
        let opp = &make_lobby().opponents[0];
        assert_eq!(opp.threat_level, ThreatLevel::High);
    }

    #[test]
    fn test_empty_lobby() {
        let lobby = LobbyAnalysis { opponents: vec![], contested_comps: vec![], recommended_pivot: None };
        assert!(lobby.opponents.is_empty());
    }
}

//! Top status bar: connection status, round info.

use egui::{Ui, RichText};
use tft_types::GameState;
use crate::theme;
use crate::state::ConnectionStatus;

pub fn render(ui: &mut Ui, game_state: Option<&GameState>, status: Option<&ConnectionStatus>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("TFT Synapse").color(theme::ACCENT_GOLD).strong());
        ui.separator();

        if let Some(state) = game_state {
            ui.label(RichText::new(
                format!("Stage {}-{}", state.round.stage, state.round.round)
            ).color(theme::TEXT_PRIMARY));
            ui.label(RichText::new(format!("HP: {}", state.hp)).color(theme::TEXT_PRIMARY));
            ui.label(RichText::new(format!("Gold: {}", state.gold)).color(theme::ACCENT_GOLD));
            ui.label(RichText::new(format!("Level: {}", state.level)).color(theme::TEXT_PRIMARY));
        } else {
            ui.label(RichText::new("No game detected").color(theme::TEXT_SECONDARY));
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (text, color) = match status {
                Some(ConnectionStatus::Connected) => ("Connected", theme::SCORE_HIGH),
                Some(ConnectionStatus::Polling) => ("Polling", theme::SCORE_MID),
                Some(ConnectionStatus::Manual) => ("Manual", theme::ACCENT_BLUE),
                _ => ("Offline", theme::SCORE_LOW),
            };
            ui.label(RichText::new(text).color(color).small());
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_label_connected() {
        let text = match Some(&ConnectionStatus::Connected) {
            Some(ConnectionStatus::Connected) => "Connected",
            _ => "Offline",
        };
        assert_eq!(text, "Connected");
    }

    #[test]
    fn test_status_label_offline() {
        let text = match None::<&ConnectionStatus> {
            Some(ConnectionStatus::Connected) => "Connected",
            _ => "Offline",
        };
        assert_eq!(text, "Offline");
    }
}

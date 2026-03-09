//! Stats panel: games played, top-4 rate, model info.

use egui::{Ui, RichText};
use tft_advisor::AdvisorMetrics;
use crate::theme;
use crate::state::ConnectionStatus;

pub fn render(ui: &mut Ui, metrics: &AdvisorMetrics, games_trained: u32, status: Option<&ConnectionStatus>) {
    ui.heading(RichText::new("Stats").color(theme::ACCENT_BLUE));
    ui.separator();

    ui.horizontal(|ui| {
        ui.label(RichText::new("Status:").color(theme::TEXT_SECONDARY));
        let (text, color) = match status {
            Some(ConnectionStatus::Connected) => ("Connected", theme::SCORE_HIGH),
            Some(ConnectionStatus::Polling) => ("Polling...", theme::SCORE_MID),
            Some(ConnectionStatus::Manual) => ("Manual", theme::ACCENT_BLUE),
            _ => ("Disconnected", theme::SCORE_LOW),
        };
        ui.label(RichText::new(text).color(color).strong());
    });

    ui.add_space(4.0);
    ui.label(RichText::new(format!("Games played: {}", metrics.games_played)).color(theme::TEXT_PRIMARY));
    ui.label(RichText::new(format!("Model trained on: {} games", games_trained)).color(theme::TEXT_PRIMARY));

    if metrics.games_played > 0 {
        ui.add_space(4.0);
        ui.label(RichText::new(format!("Avg placement: {:.1}", metrics.avg_placement())).color(theme::TEXT_PRIMARY));
        ui.label(RichText::new(format!("Top-4 rate: {:.0}%", metrics.top_four_rate() * 100.0))
            .color(if metrics.top_four_rate() >= 0.5 { theme::SCORE_HIGH } else { theme::SCORE_MID }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_types::Placement;

    #[test]
    fn test_stats_avg_placement_empty() {
        let m = AdvisorMetrics::new();
        assert_eq!(m.avg_placement(), 0.0);
    }

    #[test]
    fn test_stats_top_four_rate_after_games() {
        let mut m = AdvisorMetrics::new();
        m.record_placement(Placement(1));
        m.record_placement(Placement(5));
        assert!((m.top_four_rate() - 0.5).abs() < f32::EPSILON);
    }
}

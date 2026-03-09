use egui::Ui;
use tft_advisor::{BoardLayout, PositionRole};
use crate::theme;

pub fn render(ui: &mut Ui, layout: &BoardLayout) {
    ui.heading("Positioning");
    if layout.positions.is_empty() {
        ui.label(egui::RichText::new("No units on board").small().color(theme::TEXT_SECONDARY));
        return;
    }
    if let Some(ref carry) = layout.carry_champion {
        ui.label(egui::RichText::new(format!("Main carry: {}", carry)).color(theme::SCORE_HIGH).small().strong());
    }
    ui.label(egui::RichText::new(format!(
        "Frontline: {}  Backline: {}",
        layout.frontline_count, layout.backline_count
    )).small().color(theme::TEXT_SECONDARY));

    if let Some(ref warn) = layout.layout_warning {
        ui.label(egui::RichText::new(warn).small().color(theme::SCORE_LOW));
    }

    for rec in &layout.positions {
        let role_color = match rec.role {
            PositionRole::Frontline => theme::TEXT_SECONDARY,
            PositionRole::Carry => theme::SCORE_HIGH,
            PositionRole::SecondaryCarry => theme::SCORE_MID,
            PositionRole::Support => theme::ACCENT_BLUE,
        };
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&rec.champion_name).color(role_color).small());
            ui.label(egui::RichText::new(format!("-> R{}C{}", rec.suggested_position.row, rec.suggested_position.col)).small().color(theme::TEXT_SECONDARY));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_advisor::{BoardLayout, HexPosition, PositionRecommendation, PositionRole};
    use tft_types::ChampionId;

    fn empty_layout() -> BoardLayout {
        BoardLayout::default()
    }

    fn make_rec(name: &str, role: PositionRole, row: u8, col: u8) -> PositionRecommendation {
        PositionRecommendation {
            champion_id: ChampionId(0),
            champion_name: name.to_string(),
            role,
            suggested_position: HexPosition { row, col },
            reason: "test".to_string(),
        }
    }

    #[test]
    fn test_empty_layout_has_no_positions() {
        let layout = empty_layout();
        assert!(layout.positions.is_empty());
    }

    #[test]
    fn test_carry_role_identified() {
        let rec = make_rec("Jinx", PositionRole::Carry, 4, 4);
        assert_eq!(rec.role, PositionRole::Carry);
        assert_eq!(rec.suggested_position.row, 4);
        assert_eq!(rec.suggested_position.col, 4);
    }

    #[test]
    fn test_frontline_role_row1() {
        let rec = make_rec("Vi", PositionRole::Frontline, 1, 3);
        assert_eq!(rec.role, PositionRole::Frontline);
        assert_eq!(rec.suggested_position.row, 1);
    }
}

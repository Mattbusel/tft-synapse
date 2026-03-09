use egui::Ui;
use tft_advisor::CarryCandidate;
use crate::theme;

pub fn render(ui: &mut Ui, carries: &[CarryCandidate]) {
    ui.heading("Carry Targets");
    if carries.is_empty() {
        ui.label(egui::RichText::new("No carry candidates yet").small().color(theme::TEXT_SECONDARY));
        return;
    }
    for (i, c) in carries.iter().enumerate() {
        let label = match i {
            0 => "PRIMARY",
            1 => "2nd",
            _ => "3rd",
        };
        let color = if i == 0 { theme::SCORE_HIGH } else { theme::TEXT_PRIMARY };
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(label).color(color).strong().small());
            ui.label(egui::RichText::new(format!(
                "  {}  ({}/9 copies)",
                c.champion_name, c.copies_held
            )).small());
        });
        ui.label(egui::RichText::new(&c.reason).small().color(theme::TEXT_SECONDARY));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_types::ChampionId;
    use tft_advisor::CarryCandidate;

    #[test]
    fn test_carry_candidate_struct() {
        let c = CarryCandidate {
            champion_id: ChampionId(1),
            champion_name: "Jinx".to_string(),
            copies_held: 5,
            copies_needed: 4,
            score: 0.8,
            reason: "5/9 copies".to_string(),
        };
        assert_eq!(c.copies_held, 5);
        assert_eq!(c.copies_needed, 4);
    }

    #[test]
    fn test_empty_carries_slice() {
        let carries: Vec<CarryCandidate> = vec![];
        assert!(carries.is_empty());
    }

    #[test]
    fn test_carry_score_ordering() {
        let mut carries = vec![
            CarryCandidate { champion_id: ChampionId(1), champion_name: "A".to_string(), copies_held: 3, copies_needed: 6, score: 0.5, reason: String::new() },
            CarryCandidate { champion_id: ChampionId(2), champion_name: "B".to_string(), copies_held: 6, copies_needed: 3, score: 0.9, reason: String::new() },
        ];
        carries.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        assert_eq!(carries[0].champion_name, "B");
    }
}

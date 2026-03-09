use crate::theme;
use egui::Ui;
use tft_advisor::ReviewEntry;

pub fn render(ui: &mut Ui, review: &[ReviewEntry]) {
    ui.heading("Game Review");
    if review.is_empty() {
        ui.label(
            egui::RichText::new("No decisions recorded yet")
                .small()
                .color(theme::TEXT_SECONDARY),
        );
        return;
    }
    for entry in review {
        let score_color = if entry.chosen_score >= 0.7 {
            theme::SCORE_HIGH
        } else {
            theme::SCORE_MID
        };
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{}-{}", entry.stage, entry.round))
                    .small()
                    .color(theme::TEXT_SECONDARY)
                    .monospace(),
            );
            ui.label(
                egui::RichText::new(&entry.chosen_name)
                    .color(score_color)
                    .small()
                    .strong(),
            );
            ui.label(
                egui::RichText::new(format!("{:.0}%", entry.chosen_score * 100.0))
                    .small()
                    .color(score_color),
            );
            if entry.was_top_pick {
                ui.label(egui::RichText::new("*").color(theme::SCORE_HIGH).small());
            }
        });
        if !entry.alternatives.is_empty() {
            let alts: Vec<&str> = entry.alternatives.iter().map(|(n, _)| n.as_str()).collect();
            ui.label(
                egui::RichText::new(format!("  Others: {}", alts.join(", ")))
                    .small()
                    .color(theme::TEXT_SECONDARY),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_advisor::ReviewEntry;

    fn make_entry(stage: u8, round: u8, score: f32, top_pick: bool) -> ReviewEntry {
        ReviewEntry {
            stage,
            round,
            chosen_name: "Cybernetic Heart".to_string(),
            chosen_score: score,
            alternatives: vec![("Backstab".to_string(), 0.0)],
            was_top_pick: top_pick,
        }
    }

    #[test]
    fn test_review_entry_fields() {
        let entry = make_entry(2, 1, 0.8, true);
        assert_eq!(entry.stage, 2);
        assert_eq!(entry.round, 1);
        assert!(entry.was_top_pick);
    }

    #[test]
    fn test_high_score_is_top_pick() {
        let entry = make_entry(3, 2, 0.75, true);
        assert!(entry.chosen_score >= 0.7);
        assert!(entry.was_top_pick);
    }

    #[test]
    fn test_empty_review_slice() {
        let review: Vec<ReviewEntry> = vec![];
        assert!(review.is_empty());
    }
}

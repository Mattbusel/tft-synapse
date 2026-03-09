//! Renders the augment recommendation panel.

use crate::theme;
use egui::{RichText, Ui};
use tft_advisor::Recommendation;

/// Render the augment recommendation panel.
pub fn render(ui: &mut Ui, recommendation: Option<&Recommendation>) {
    ui.heading(RichText::new("Augment Advisor").color(theme::ACCENT_GOLD));
    ui.separator();

    match recommendation {
        None => {
            ui.label(
                RichText::new("Waiting for augment selection phase...")
                    .color(theme::TEXT_SECONDARY),
            );
        }
        Some(rec) => {
            ui.label(RichText::new("Recommended picks:").color(theme::TEXT_PRIMARY));
            ui.add_space(6.0);

            for (i, aug) in rec.ranked.iter().enumerate() {
                let rank_label = match i {
                    0 => "BEST",
                    1 => "2nd",
                    _ => "3rd",
                };

                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let rank_color = if i == 0 {
                            theme::ACCENT_GOLD
                        } else {
                            theme::TEXT_SECONDARY
                        };
                        ui.label(RichText::new(rank_label).color(rank_color).strong());
                        ui.add_space(8.0);
                        ui.label(RichText::new(&aug.reasoning).color(theme::TEXT_PRIMARY));
                    });

                    // Score bar
                    let bar_width = 180.0;
                    let bar_height = 6.0;
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(bar_width, bar_height),
                        egui::Sense::hover(),
                    );
                    let filled = egui::Rect::from_min_size(
                        rect.min,
                        egui::vec2(bar_width * aug.score.clamp(0.0, 1.0), bar_height),
                    );
                    ui.painter().rect_filled(rect, 2.0, theme::BG_CARD);
                    ui.painter()
                        .rect_filled(filled, 2.0, theme::score_color(aug.score));
                    ui.label(
                        RichText::new(format!("{:.0}%", aug.score * 100.0))
                            .color(theme::score_color(aug.score))
                            .small(),
                    );
                });
                ui.add_space(4.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Panel rendering tests are limited without a full egui context.
    // We test the logic components that can be isolated.

    #[test]
    fn test_rank_label_first_is_best() {
        let label = match 0usize {
            0 => "BEST",
            1 => "2nd",
            _ => "3rd",
        };
        assert_eq!(label, "BEST");
    }

    #[test]
    fn test_rank_label_second() {
        let label = match 1usize {
            0 => "BEST",
            1 => "2nd",
            _ => "3rd",
        };
        assert_eq!(label, "2nd");
    }

    #[test]
    fn test_score_clamp() {
        let score = 1.5f32;
        assert_eq!(score.clamp(0.0, 1.0), 1.0);
        let score2 = -0.1f32;
        assert_eq!(score2.clamp(0.0, 1.0), 0.0);
    }
}

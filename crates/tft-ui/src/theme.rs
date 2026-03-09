//! Dark theme constants for TFT Synapse UI.

use egui::Color32;

pub const BG_DARK: Color32 = Color32::from_rgb(15, 15, 20);
pub const BG_PANEL: Color32 = Color32::from_rgb(25, 25, 35);
pub const BG_CARD: Color32 = Color32::from_rgb(35, 35, 50);
pub const ACCENT_GOLD: Color32 = Color32::from_rgb(212, 175, 55);
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(70, 130, 220);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(230, 230, 240);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(150, 150, 170);
pub const SCORE_HIGH: Color32 = Color32::from_rgb(80, 200, 100);
pub const SCORE_MID: Color32 = Color32::from_rgb(220, 180, 50);
pub const SCORE_LOW: Color32 = Color32::from_rgb(200, 80, 80);

/// Get a color for a score value [0.0, 1.0].
pub fn score_color(score: f32) -> Color32 {
    if score >= 0.7 { SCORE_HIGH }
    else if score >= 0.4 { SCORE_MID }
    else { SCORE_LOW }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_color_high() {
        assert_eq!(score_color(0.8), SCORE_HIGH);
    }

    #[test]
    fn test_score_color_mid() {
        assert_eq!(score_color(0.5), SCORE_MID);
    }

    #[test]
    fn test_score_color_low() {
        assert_eq!(score_color(0.2), SCORE_LOW);
    }

    #[test]
    fn test_score_color_boundary_high() {
        assert_eq!(score_color(0.7), SCORE_HIGH);
    }

    #[test]
    fn test_score_color_boundary_mid() {
        assert_eq!(score_color(0.4), SCORE_MID);
    }
}

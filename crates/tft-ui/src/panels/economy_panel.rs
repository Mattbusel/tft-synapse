use crate::theme;
use egui::Ui;
use tft_advisor::{EconomyAction, EconomyAdvice, StreakType};

pub fn render(ui: &mut Ui, advice: &EconomyAdvice) {
    ui.heading("Economy");

    let action_label = match advice.recommended_action {
        EconomyAction::Save => "SAVE GOLD",
        EconomyAction::LevelUp => "LEVEL UP",
        EconomyAction::Roll => "ROLL DOWN",
        EconomyAction::MaintainStreak => "MAINTAIN STREAK",
    };
    let action_color = match advice.recommended_action {
        EconomyAction::Save => theme::TEXT_SECONDARY,
        EconomyAction::LevelUp => theme::SCORE_MID,
        EconomyAction::Roll => theme::SCORE_HIGH,
        EconomyAction::MaintainStreak => theme::SCORE_HIGH,
    };
    ui.label(
        egui::RichText::new(action_label)
            .color(action_color)
            .strong(),
    );
    ui.label(
        egui::RichText::new(&advice.reason)
            .small()
            .color(theme::TEXT_SECONDARY),
    );

    if let Some(ref streak) = advice.streak_type {
        let streak_label = match streak {
            StreakType::Win => "Win streak active",
            StreakType::Loss => "Loss streak active",
        };
        ui.label(
            egui::RichText::new(streak_label)
                .small()
                .color(theme::SCORE_MID),
        );
    }

    if advice.gold_to_interest > 0 {
        ui.label(
            egui::RichText::new(format!(
                "{} gold to next interest ({}g threshold)",
                advice.gold_to_interest, advice.next_interest_threshold
            ))
            .small()
            .color(theme::TEXT_SECONDARY),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_advisor::{EconomyAction, EconomyAdvice};

    #[test]
    fn test_economy_advice_save_action() {
        let advice = EconomyAdvice {
            recommended_action: EconomyAction::Save,
            reason: "Save to 50".to_string(),
            streak_type: None,
            gold_to_interest: 10,
            next_interest_threshold: 50,
        };
        assert!(matches!(advice.recommended_action, EconomyAction::Save));
    }

    #[test]
    fn test_economy_advice_roll_action() {
        let advice = EconomyAdvice {
            recommended_action: EconomyAction::Roll,
            reason: "Low HP".to_string(),
            streak_type: None,
            gold_to_interest: 0,
            next_interest_threshold: 50,
        };
        assert!(matches!(advice.recommended_action, EconomyAction::Roll));
    }

    #[test]
    fn test_economy_advice_streak_type() {
        let advice = EconomyAdvice {
            recommended_action: EconomyAction::MaintainStreak,
            reason: "Win streak".to_string(),
            streak_type: Some(StreakType::Win),
            gold_to_interest: 0,
            next_interest_threshold: 50,
        };
        assert!(matches!(advice.streak_type, Some(StreakType::Win)));
    }
}

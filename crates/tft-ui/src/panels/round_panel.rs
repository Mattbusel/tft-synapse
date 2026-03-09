use crate::theme;
use egui::Ui;
use tft_advisor::{EventType, StageAwareness};

pub fn render(ui: &mut Ui, awareness: &StageAwareness) {
    ui.heading("Stage Awareness");
    ui.label(
        egui::RichText::new(format!(
            "Stage {}-{}",
            awareness.current_stage, awareness.current_round
        ))
        .color(theme::ACCENT_BLUE)
        .strong()
        .small(),
    );

    let level_color = if awareness.is_level_behind {
        theme::SCORE_LOW
    } else {
        theme::SCORE_HIGH
    };
    ui.label(
        egui::RichText::new(format!(
            "Target level: {}{}",
            awareness.recommended_level,
            if awareness.is_level_behind {
                " (behind!)"
            } else {
                ""
            }
        ))
        .small()
        .color(level_color),
    );

    ui.label(
        egui::RichText::new(&awareness.current_priority)
            .small()
            .color(theme::TEXT_PRIMARY),
    );

    if !awareness.upcoming_events.is_empty() {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Upcoming:")
                .small()
                .color(theme::TEXT_SECONDARY),
        );
        for event in awareness.upcoming_events.iter().take(3) {
            let color = match event.event_type {
                EventType::Augment => theme::SCORE_HIGH,
                EventType::Carousel => theme::SCORE_MID,
                EventType::PvE => theme::ACCENT_BLUE,
                EventType::LevelTarget => theme::TEXT_SECONDARY,
            };
            let when = if event.rounds_away == 0 {
                "NOW".to_string()
            } else {
                format!("in {} rounds", event.rounds_away)
            };
            ui.label(
                egui::RichText::new(format!("  {} - {}", when, event.description))
                    .small()
                    .color(color),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_advisor::{EventType, StageAwareness, UpcomingEvent};

    fn make_awareness(stage: u8, round: u8, level_behind: bool) -> StageAwareness {
        StageAwareness {
            current_stage: stage,
            current_round: round,
            recommended_level: 6,
            is_level_behind: level_behind,
            upcoming_events: vec![],
            current_priority: "Test priority".to_string(),
        }
    }

    #[test]
    fn test_stage_awareness_fields() {
        let a = make_awareness(3, 2, false);
        assert_eq!(a.current_stage, 3);
        assert_eq!(a.current_round, 2);
        assert!(!a.is_level_behind);
    }

    #[test]
    fn test_level_behind_flag() {
        let a = make_awareness(2, 1, true);
        assert!(a.is_level_behind);
    }

    #[test]
    fn test_event_type_augment_variant() {
        let event = UpcomingEvent {
            event_type: EventType::Augment,
            description: "First augment choice".to_string(),
            rounds_away: 0,
        };
        assert_eq!(event.event_type, EventType::Augment);
        assert_eq!(event.rounds_away, 0);
    }
}

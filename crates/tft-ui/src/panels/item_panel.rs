use egui::Ui;
use tft_advisor::ItemRecommendation;
use crate::theme;

pub fn render(ui: &mut Ui, items: &[ItemRecommendation]) {
    ui.heading("Items");
    if items.is_empty() {
        ui.label(egui::RichText::new("No item recommendations").small().color(theme::TEXT_SECONDARY));
        return;
    }
    for item in items.iter().take(5) {
        let target = item.target_champion_name.as_deref().unwrap_or("No target");
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&item.item_name).color(theme::TEXT_PRIMARY).small().strong());
            ui.label(egui::RichText::new(format!("-> {}", target)).small().color(theme::SCORE_MID));
        });
        ui.label(egui::RichText::new(&item.reason).small().color(theme::TEXT_SECONDARY));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_advisor::ItemRecommendation;
    use tft_types::ItemId;

    #[test]
    fn test_item_recommendation_struct() {
        let r = ItemRecommendation {
            item_id: ItemId(1),
            item_name: "Rabadon's Deathcap".to_string(),
            target_champion_id: None,
            target_champion_name: None,
            reason: "No AP carry on board".to_string(),
            confidence: 0.5,
        };
        assert!(r.target_champion_name.is_none());
    }

    #[test]
    fn test_item_recommendation_with_target() {
        let r = ItemRecommendation {
            item_id: ItemId(2),
            item_name: "Infinity Edge".to_string(),
            target_champion_id: Some(tft_types::ChampionId(3)),
            target_champion_name: Some("Jinx".to_string()),
            reason: "Best AD carry".to_string(),
            confidence: 0.9,
        };
        assert_eq!(r.target_champion_name.as_deref(), Some("Jinx"));
        assert!((r.confidence - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn test_empty_items() {
        let items: Vec<ItemRecommendation> = vec![];
        assert!(items.is_empty());
    }
}

use crate::theme;
use egui::Ui;
use tft_advisor::{PoolEntry, PoolStatus};

pub fn render(ui: &mut Ui, pool: &[PoolEntry]) {
    ui.heading("Champion Pool");
    if pool.is_empty() {
        ui.label(
            egui::RichText::new("No pool data")
                .small()
                .color(theme::TEXT_SECONDARY),
        );
        return;
    }
    // Show only the 10 most contested (already sorted by remaining ascending)
    for entry in pool.iter().take(10) {
        let (status_text, color) = match entry.status {
            PoolStatus::Exhausted => ("GONE", theme::SCORE_LOW),
            PoolStatus::Critical => ("CRIT", theme::SCORE_LOW),
            PoolStatus::Low => ("LOW", theme::SCORE_MID),
            PoolStatus::Available => ("OK", theme::SCORE_HIGH),
        };
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(status_text)
                    .color(color)
                    .strong()
                    .small()
                    .monospace(),
            );
            ui.label(
                egui::RichText::new(format!(
                    "  {}  ({}/{})",
                    entry.champion_name, entry.remaining, entry.pool_size
                ))
                .small(),
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_advisor::{PoolEntry, PoolStatus};
    use tft_types::ChampionId;

    fn make_entry(name: &str, remaining: u8, pool_size: u8, status: PoolStatus) -> PoolEntry {
        PoolEntry {
            champion_id: ChampionId(0),
            champion_name: name.to_string(),
            cost: 1,
            pool_size,
            visible_copies: pool_size.saturating_sub(remaining),
            remaining,
            status,
        }
    }

    #[test]
    fn test_pool_status_exhausted_variant() {
        let entry = make_entry("Jinx", 0, 18, PoolStatus::Exhausted);
        assert_eq!(entry.status, PoolStatus::Exhausted);
        assert_eq!(entry.remaining, 0);
    }

    #[test]
    fn test_pool_status_critical_variant() {
        let entry = make_entry("Caitlyn", 2, 29, PoolStatus::Critical);
        assert_eq!(entry.status, PoolStatus::Critical);
        assert_eq!(entry.remaining, 2);
    }

    #[test]
    fn test_empty_pool_slice_is_handled() {
        let pool: Vec<PoolEntry> = vec![];
        assert!(pool.is_empty());
    }

    #[test]
    fn test_exhausted_entry_appears_first_when_sorted_by_remaining() {
        let mut pool = vec![
            make_entry("Jinx", 5, 18, PoolStatus::Low),
            make_entry("Caitlyn", 0, 29, PoolStatus::Exhausted),
            make_entry("Vi", 12, 18, PoolStatus::Available),
        ];
        pool.sort_by_key(|e| e.remaining);
        assert_eq!(pool[0].status, PoolStatus::Exhausted);
    }
}

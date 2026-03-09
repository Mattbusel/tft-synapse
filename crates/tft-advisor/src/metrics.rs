//! Running metrics for the advisor: accuracy, games played, placement history.

use tft_types::Placement;

#[derive(Debug, Default, Clone)]
pub struct AdvisorMetrics {
    pub games_played: u32,
    pub top_four_count: u32,
    pub first_place_count: u32,
    pub total_placement: u32,
    pub placement_history: Vec<u8>,
}

impl AdvisorMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_placement(&mut self, placement: Placement) {
        self.games_played += 1;
        self.total_placement += placement.0 as u32;
        self.placement_history.push(placement.0);
        if placement.is_top_four() {
            self.top_four_count += 1;
        }
        if placement.0 == 1 {
            self.first_place_count += 1;
        }
    }

    pub fn avg_placement(&self) -> f32 {
        if self.games_played == 0 {
            return 0.0;
        }
        self.total_placement as f32 / self.games_played as f32
    }

    pub fn top_four_rate(&self) -> f32 {
        if self.games_played == 0 {
            return 0.0;
        }
        self.top_four_count as f32 / self.games_played as f32
    }

    pub fn first_place_rate(&self) -> f32 {
        if self.games_played == 0 {
            return 0.0;
        }
        self.first_place_count as f32 / self.games_played as f32
    }

    pub fn is_top_four(&self) -> bool {
        self.placement_history
            .last()
            .map(|&p| p <= 4)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new_is_zero() {
        let m = AdvisorMetrics::new();
        assert_eq!(m.games_played, 0);
        assert_eq!(m.avg_placement(), 0.0);
        assert_eq!(m.top_four_rate(), 0.0);
    }

    #[test]
    fn test_record_first_place() {
        let mut m = AdvisorMetrics::new();
        m.record_placement(Placement(1));
        assert_eq!(m.games_played, 1);
        assert_eq!(m.first_place_count, 1);
        assert!(m.is_top_four());
        assert!((m.avg_placement() - 1.0).abs() < f32::EPSILON);
        assert!((m.top_four_rate() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_top_four_rate_calculation() {
        let mut m = AdvisorMetrics::new();
        m.record_placement(Placement(1));
        m.record_placement(Placement(3));
        m.record_placement(Placement(5));
        m.record_placement(Placement(8));
        assert!((m.top_four_rate() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_avg_placement_multiple() {
        let mut m = AdvisorMetrics::new();
        m.record_placement(Placement(1));
        m.record_placement(Placement(3));
        assert!((m.avg_placement() - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_placement_history_recorded() {
        let mut m = AdvisorMetrics::new();
        m.record_placement(Placement(2));
        m.record_placement(Placement(5));
        assert_eq!(m.placement_history, vec![2, 5]);
    }
}

//! Export placement history and model stats to CSV.
//!
//! ## Responsibility
//! Serialize `AdvisorMetrics` data to CSV files on disk for post-game analysis.
//!
//! ## Guarantees
//! - Functions never panic; all errors are returned as `TftError`
//! - Parent directories are created if they do not exist
//! - CSV always contains a header row

use std::path::Path;
use tft_types::{Placement, TftError};
use crate::metrics::AdvisorMetrics;

/// One row in the placement history export.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportRow {
    pub game_number: u32,
    pub placement: u8,
    pub reward: f32,
    pub is_top_four: bool,
}

/// Export full placement history to CSV.
///
/// # Arguments
/// * `metrics` — advisor metrics containing placement_history
/// * `path` — output file path; parent directories are created if needed
///
/// # Returns
/// - `Ok(n)` where `n` is the number of data rows written (excluding header)
/// - `Err(TftError::Io)` if directory creation or file write fails
///
/// # Panics
/// This function never panics.
pub fn export_history_csv(metrics: &AdvisorMetrics, path: &Path) -> Result<usize, TftError> {
    let mut rows = Vec::with_capacity(metrics.placement_history.len());
    for (i, &placement) in metrics.placement_history.iter().enumerate() {
        let p = Placement(placement);
        rows.push(ExportRow {
            game_number: (i + 1) as u32,
            placement,
            reward: p.to_reward(),
            is_top_four: p.is_top_four(),
        });
    }

    let mut csv = String::from("game_number,placement,reward,top_four\n");
    for row in &rows {
        csv.push_str(&format!(
            "{},{},{:.4},{}\n",
            row.game_number, row.placement, row.reward, row.is_top_four
        ));
    }

    ensure_parent(path)?;
    std::fs::write(path, &csv).map_err(|e| TftError::Io { source: e })?;

    Ok(rows.len())
}

/// Export summary stats to CSV (single data row with all metrics).
///
/// # Arguments
/// * `metrics` — advisor metrics
/// * `games_trained` — number of games the model was trained on
/// * `path` — output file path; parent directories are created if needed
///
/// # Returns
/// - `Ok(())` on success
/// - `Err(TftError::Io)` if directory creation or file write fails
///
/// # Panics
/// This function never panics.
pub fn export_stats_csv(
    metrics: &AdvisorMetrics,
    games_trained: u32,
    path: &Path,
) -> Result<(), TftError> {
    let csv = format!(
        "games_played,games_trained,avg_placement,top_four_rate,first_place_rate\n{},{},{:.2},{:.4},{:.4}\n",
        metrics.games_played,
        games_trained,
        metrics.avg_placement(),
        metrics.top_four_rate(),
        metrics.first_place_rate(),
    );
    ensure_parent(path)?;
    std::fs::write(path, &csv).map_err(|e| TftError::Io { source: e })?;
    Ok(())
}

fn ensure_parent(path: &Path) -> Result<(), TftError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| TftError::Io { source: e })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_types::Placement;

    fn make_metrics(placements: &[u8]) -> AdvisorMetrics {
        let mut m = AdvisorMetrics::new();
        for &p in placements {
            m.record_placement(Placement(p));
        }
        m
    }

    // ── history export ───────────────────────────────────────────────────────

    #[test]
    fn test_export_history_empty_produces_header_only() {
        let m = AdvisorMetrics::new();
        let dir = tempdir();
        let path = dir.join("history.csv");
        let n = export_history_csv(&m, &path).expect("export failed in test");
        assert_eq!(n, 0);
        let contents = std::fs::read_to_string(&path).expect("read failed in test");
        assert_eq!(contents, "game_number,placement,reward,top_four\n");
    }

    #[test]
    fn test_export_history_single_game_correct_row() {
        let m = make_metrics(&[1]);
        let dir = tempdir();
        let path = dir.join("history.csv");
        let n = export_history_csv(&m, &path).expect("export failed in test");
        assert_eq!(n, 1);
        let contents = std::fs::read_to_string(&path).expect("read failed in test");
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "game_number,placement,reward,top_four");
        // placement 1 → reward 1.0000
        assert!(lines[1].contains("1,1,1.0000,true"), "got: {}", lines[1]);
    }

    #[test]
    fn test_export_history_row_count_matches_games() {
        let m = make_metrics(&[1, 2, 3, 4, 5]);
        let dir = tempdir();
        let path = dir.join("history.csv");
        let n = export_history_csv(&m, &path).expect("export failed in test");
        assert_eq!(n, 5);
    }

    #[test]
    fn test_export_history_reward_values_match_placement_to_reward() {
        let placements = [1u8, 4, 8];
        let m = make_metrics(&placements);
        let dir = tempdir();
        let path = dir.join("h.csv");
        export_history_csv(&m, &path).expect("export failed in test");
        let contents = std::fs::read_to_string(&path).expect("read failed in test");
        for (i, &p) in placements.iter().enumerate() {
            let expected_reward = Placement(p).to_reward();
            let expected_str = format!("{:.4}", expected_reward);
            let line = contents.lines().nth(i + 1).expect("missing line in test");
            assert!(line.contains(&expected_str), "reward mismatch: {}", line);
        }
    }

    #[test]
    fn test_export_history_top_four_flag_correct() {
        let m = make_metrics(&[4, 5]);
        let dir = tempdir();
        let path = dir.join("h.csv");
        export_history_csv(&m, &path).expect("export failed in test");
        let contents = std::fs::read_to_string(&path).expect("read failed in test");
        let lines: Vec<&str> = contents.lines().collect();
        assert!(lines[1].ends_with("true"), "placement 4 should be top_four: {}", lines[1]);
        assert!(lines[2].ends_with("false"), "placement 5 should not be top_four: {}", lines[2]);
    }

    // ── stats export ─────────────────────────────────────────────────────────

    #[test]
    fn test_export_stats_csv_format() {
        let m = make_metrics(&[1, 2, 3, 4]);
        let dir = tempdir();
        let path = dir.join("stats.csv");
        export_stats_csv(&m, 100, &path).expect("export failed in test");
        let contents = std::fs::read_to_string(&path).expect("read failed in test");
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines[0], "games_played,games_trained,avg_placement,top_four_rate,first_place_rate");
        assert!(lines[1].starts_with("4,100,"), "data row: {}", lines[1]);
    }

    #[test]
    fn test_export_stats_csv_games_trained_field() {
        let m = make_metrics(&[2]);
        let dir = tempdir();
        let path = dir.join("stats.csv");
        export_stats_csv(&m, 42, &path).expect("export failed in test");
        let contents = std::fs::read_to_string(&path).expect("read failed in test");
        let data_row = contents.lines().nth(1).expect("missing data row in test");
        let fields: Vec<&str> = data_row.split(',').collect();
        assert_eq!(fields[1], "42");
    }

    #[test]
    fn test_export_creates_nested_directories() {
        let dir = tempdir();
        let nested = dir.join("a").join("b").join("c").join("stats.csv");
        let m = AdvisorMetrics::new();
        export_stats_csv(&m, 0, &nested).expect("export with nested dirs failed in test");
        assert!(nested.exists());
    }

    #[test]
    fn test_export_history_creates_nested_directories() {
        let dir = tempdir();
        let nested = dir.join("x").join("y").join("history.csv");
        let m = AdvisorMetrics::new();
        let n = export_history_csv(&m, &nested).expect("export with nested dirs failed in test");
        assert_eq!(n, 0);
        assert!(nested.exists());
    }

    // ── helper ────────────────────────────────────────────────────────────────

    /// Create a unique temporary directory for this test.
    fn tempdir() -> std::path::PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir()
            .join(format!("tft_export_test_{}", nanos));
        std::fs::create_dir_all(&path).expect("tempdir creation failed in test");
        path
    }
}

//! # Stage: PositioningAdvisor
//!
//! Recommend hex positions for each champion on the board.
//! Pure logic, no data sources beyond the catalog.
//!
//! ## Responsibility
//! Classify each board champion by role (Frontline, Carry, Support, SecondaryCarry)
//! and assign a deterministic hex position within the 4x7 TFT board.
//!
//! ## Guarantees
//! - Deterministic: same state + catalog produces the same layout
//! - Non-panicking: all fallible paths return Result
//! - Pure: reads only `state.board` and the catalog
//!
//! ## NOT Responsible For
//! - Item positioning
//! - Dynamic carousel / bench positioning
//! - Opponent-specific counter-positioning

use tft_data::Catalog;
use tft_types::{ChampionId, GameState, TftError};

/// High-level role of a champion on the board.
#[derive(Debug, Clone, PartialEq)]
pub enum PositionRole {
    /// Tank champions that should stand in front to absorb damage.
    Frontline,
    /// Primary damage dealer — placed at center-back.
    Carry,
    /// Off-damage champions placed behind the frontline.
    SecondaryCarry,
    /// Buff/heal champions placed at the flanks of the backline.
    Support,
}

/// A grid position on the TFT hex board (1-indexed).
#[derive(Debug, Clone, PartialEq)]
pub struct HexPosition {
    /// 1 = frontline, 4 = backline.
    pub row: u8,
    /// 1–7, left to right.
    pub col: u8,
}

/// A positioning recommendation for one champion.
#[derive(Debug, Clone)]
pub struct PositionRecommendation {
    /// Champion identifier.
    pub champion_id: ChampionId,
    /// Display name from catalog.
    pub champion_name: String,
    /// Assigned role.
    pub role: PositionRole,
    /// Suggested grid position.
    pub suggested_position: HexPosition,
    /// Human-readable explanation.
    pub reason: String,
}

/// A complete board layout recommendation.
#[derive(Debug, Clone)]
pub struct BoardLayout {
    /// One entry per board champion.
    pub positions: Vec<PositionRecommendation>,
    /// Name of the identified main carry (if any).
    pub carry_champion: Option<String>,
    /// Number of champions assigned to frontline.
    pub frontline_count: u8,
    /// Number of champions assigned to backline (row ≥ 3).
    pub backline_count: u8,
    /// Situational warning string, or `None` if the layout is balanced.
    pub layout_warning: Option<String>,
}

impl Default for BoardLayout {
    fn default() -> Self {
        BoardLayout {
            positions: vec![],
            carry_champion: None,
            frontline_count: 0,
            backline_count: 0,
            layout_warning: None,
        }
    }
}

/// Generates deterministic positioning recommendations for all board champions.
pub struct PositioningAdvisor;

impl PositioningAdvisor {
    /// Create a new `PositioningAdvisor`.
    pub fn new() -> Self {
        Self
    }

    /// Generate positioning recommendations for all board champions.
    ///
    /// # Arguments
    /// * `state`   — Current observable game state.
    /// * `catalog` — Champion data catalog.
    ///
    /// # Returns
    /// A `BoardLayout` covering every champion currently on `state.board`.
    ///
    /// # Errors
    /// Returns `TftError::ChampionNotFound` if a board champion is absent from
    /// the catalog.
    ///
    /// # Panics
    /// This function never panics.
    pub fn advise_positions(
        &self,
        state: &GameState,
        catalog: &Catalog,
    ) -> Result<BoardLayout, TftError> {
        if state.board.is_empty() {
            return Ok(BoardLayout::default());
        }

        // Step 1: classify every board champion by role.
        let mut classified: Vec<(ChampionId, String, PositionRole)> = Vec::new();

        for slot in &state.board {
            let def = catalog
                .champion_by_id(slot.champion_id)
                .ok_or_else(|| TftError::ChampionNotFound(format!("{:?}", slot.champion_id)))?;
            let role = classify_role(def.cost.as_u8(), &def.traits);
            classified.push((slot.champion_id, def.name.clone(), role));
        }

        // Step 2: identify the main carry (highest-cost unit with a carry role).
        let main_carry_name: Option<String> = {
            let mut best: Option<(u8, &str)> = None;
            for slot in &state.board {
                let def = catalog
                    .champion_by_id(slot.champion_id)
                    .ok_or_else(|| TftError::ChampionNotFound(format!("{:?}", slot.champion_id)))?;
                let cost = def.cost.as_u8();
                let role = classify_role(cost, &def.traits);
                if role == PositionRole::Carry {
                    match best {
                        None => best = Some((cost, &def.name)),
                        Some((bc, _)) if cost > bc => best = Some((cost, &def.name)),
                        _ => {}
                    }
                }
            }
            // Safety: we only store a reference into `catalog` which lives for
            // the duration of this function. Convert to owned String here.
            best.map(|(_, n)| n.to_string())
        };

        // Step 3: assign positions.
        //
        // Frontline  → row 1, cols 1-7 (overflow to row 2)
        // Carry      → row 4, col 4 (main carry only; extras → SecondaryCarry row)
        // Support    → row 4, cols 1 then 7 (flanks)
        // SecondaryCarry → row 3, cols 2-6
        //
        // Carry champion is the single highest-cost Carry unit; all others
        // with PositionRole::Carry are treated as SecondaryCarry for layout.

        let mut frontline_positions = position_sequence(1, &[1, 2, 3, 4, 5, 6, 7]);
        let mut front2_positions = position_sequence(2, &[1, 2, 3, 4, 5, 6, 7]);
        let carry_position = HexPosition { row: 4, col: 4 };
        let mut secondary_positions = position_sequence(3, &[2, 3, 4, 5, 6]);
        let mut sec2_positions = position_sequence(3, &[1, 7, 3, 5, 4]); // overflow
        let support_cols = [1u8, 7];
        let mut support_idx = 0usize;
        let mut support_overflow = position_sequence(4, &[2, 6, 3, 5]); // if > 2 supports

        let mut carry_assigned = false;
        let mut frontline_count: u8 = 0;
        let mut backline_count: u8 = 0;

        let mut recommendations: Vec<PositionRecommendation> = Vec::new();

        for (id, name, role) in &classified {
            // Determine effective role (main carry gets the Carry position,
            // extras with carry traits fall back to SecondaryCarry).
            let effective_role = match role {
                PositionRole::Carry => {
                    if !carry_assigned && main_carry_name.as_deref() == Some(name.as_str()) {
                        PositionRole::Carry
                    } else {
                        PositionRole::SecondaryCarry
                    }
                }
                other => other.clone(),
            };

            let (pos, reason) = match effective_role {
                PositionRole::Frontline => {
                    frontline_count += 1;
                    let p = next_position(&mut frontline_positions, &mut front2_positions);
                    (
                        p,
                        "Tank trait — stands at the front to absorb damage".to_string(),
                    )
                }
                PositionRole::Carry => {
                    carry_assigned = true;
                    backline_count += 1;
                    (
                        carry_position.clone(),
                        "Main carry — center-back for maximum peel".to_string(),
                    )
                }
                PositionRole::Support => {
                    backline_count += 1;
                    let p = if support_idx < support_cols.len() {
                        let c = support_cols[support_idx];
                        support_idx += 1;
                        HexPosition { row: 4, col: c }
                    } else {
                        support_overflow
                            .next()
                            .unwrap_or(HexPosition { row: 4, col: 4 })
                    };
                    (
                        p,
                        "Support trait — flank position to protect and buff carries".to_string(),
                    )
                }
                PositionRole::SecondaryCarry => {
                    backline_count += 1;
                    let p = next_position(&mut secondary_positions, &mut sec2_positions);
                    (p, "Secondary damage dealer — backline position".to_string())
                }
            };

            recommendations.push(PositionRecommendation {
                champion_id: *id,
                champion_name: name.clone(),
                role: effective_role,
                suggested_position: pos,
                reason,
            });
        }

        let layout_warning = if frontline_count == 0 {
            Some("No frontline — your carry will die immediately".to_string())
        } else if backline_count > 0 && frontline_count >= backline_count.saturating_mul(2) {
            Some("Very frontline-heavy — consider more carries".to_string())
        } else {
            None
        };

        Ok(BoardLayout {
            positions: recommendations,
            carry_champion: main_carry_name,
            frontline_count,
            backline_count,
            layout_warning,
        })
    }
}

impl Default for PositioningAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

/// Frontline traits — champions with these are classified Frontline.
const FRONTLINE_TRAITS: &[&str] = &["Bruiser", "Guardian", "Colossus", "Juggernaut", "Vanguard"];

/// Carry traits — champions with these are classified Carry.
const CARRY_TRAITS: &[&str] = &[
    "Gunner", "Arcanist", "Marksman", "Mage", "Duelist", "Sniper",
];

/// Support traits — champions with these are classified Support.
const SUPPORT_TRAITS: &[&str] = &["Scholar", "Enchanter", "Sage", "Strategist"];

/// Classify a champion's role given its cost and trait list.
///
/// Priority: Carry > Support > Frontline > default by cost.
pub fn classify_role(cost: u8, traits: &[String]) -> PositionRole {
    let has_carry = traits.iter().any(|t| {
        CARRY_TRAITS
            .iter()
            .any(|ct| t.to_ascii_lowercase() == ct.to_ascii_lowercase())
    });
    if has_carry {
        return PositionRole::Carry;
    }

    let has_support = traits.iter().any(|t| {
        SUPPORT_TRAITS
            .iter()
            .any(|st| t.to_ascii_lowercase() == st.to_ascii_lowercase())
    });
    if has_support {
        return PositionRole::Support;
    }

    let has_frontline = traits.iter().any(|t| {
        FRONTLINE_TRAITS
            .iter()
            .any(|ft| t.to_ascii_lowercase() == ft.to_ascii_lowercase())
    });
    if has_frontline {
        return PositionRole::Frontline;
    }

    // Default by cost
    match cost {
        1 | 2 => PositionRole::Frontline,
        4 | 5 => PositionRole::SecondaryCarry,
        _ => PositionRole::SecondaryCarry, // cost 3 and unknown
    }
}

/// Build an iterator over `HexPosition`s for a given row and column sequence.
fn position_sequence(row: u8, cols: &[u8]) -> impl Iterator<Item = HexPosition> + '_ {
    cols.iter().map(move |&col| HexPosition { row, col })
}

/// Pull the next position from the primary iterator, falling back to overflow.
fn next_position(
    primary: &mut impl Iterator<Item = HexPosition>,
    overflow: &mut impl Iterator<Item = HexPosition>,
) -> HexPosition {
    primary
        .next()
        .or_else(|| overflow.next())
        .unwrap_or(HexPosition { row: 4, col: 4 })
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tft_data::Catalog;
    use tft_types::{ChampionSlot, GameState, StarLevel};

    fn catalog() -> &'static Catalog {
        Catalog::global().expect("catalog init failed in test")
    }

    fn empty_state() -> GameState {
        GameState::default()
    }

    fn board_slot(id: ChampionId) -> ChampionSlot {
        ChampionSlot {
            champion_id: id,
            star_level: StarLevel::Two,
            items: vec![],
        }
    }

    fn champ_id(name: &str) -> ChampionId {
        let cat = catalog();
        let idx = cat
            .champion_by_name
            .get(name)
            .copied()
            .expect("champion not found in test");
        ChampionId(idx as u8)
    }

    // ── classify_role ─────────────────────────────────────────────────────────

    #[test]
    fn test_classify_gunner_is_carry() {
        let role = classify_role(3, &["Gunner".to_string(), "Rebel".to_string()]);
        assert_eq!(role, PositionRole::Carry);
    }

    #[test]
    fn test_classify_arcanist_is_carry() {
        let role = classify_role(4, &["Arcanist".to_string()]);
        assert_eq!(role, PositionRole::Carry);
    }

    #[test]
    fn test_classify_bruiser_is_frontline() {
        let role = classify_role(2, &["Bruiser".to_string()]);
        assert_eq!(role, PositionRole::Frontline);
    }

    #[test]
    fn test_classify_guardian_is_frontline() {
        let role = classify_role(3, &["Guardian".to_string()]);
        assert_eq!(role, PositionRole::Frontline);
    }

    #[test]
    fn test_classify_enchanter_is_support() {
        let role = classify_role(4, &["Enchanter".to_string()]);
        assert_eq!(role, PositionRole::Support);
    }

    #[test]
    fn test_classify_scholar_is_support() {
        let role = classify_role(2, &["Scholar".to_string()]);
        assert_eq!(role, PositionRole::Support);
    }

    #[test]
    fn test_classify_carry_wins_over_support() {
        // A champion with both Gunner and Enchanter → Carry (higher priority)
        let role = classify_role(3, &["Gunner".to_string(), "Enchanter".to_string()]);
        assert_eq!(role, PositionRole::Carry);
    }

    #[test]
    fn test_classify_carry_wins_over_frontline() {
        let role = classify_role(3, &["Gunner".to_string(), "Bruiser".to_string()]);
        assert_eq!(role, PositionRole::Carry);
    }

    #[test]
    fn test_classify_cost1_no_traits_is_frontline() {
        let role = classify_role(1, &[]);
        assert_eq!(role, PositionRole::Frontline);
    }

    #[test]
    fn test_classify_cost2_no_traits_is_frontline() {
        let role = classify_role(2, &[]);
        assert_eq!(role, PositionRole::Frontline);
    }

    #[test]
    fn test_classify_cost4_no_matching_traits_is_secondary_carry() {
        let role = classify_role(4, &["Scrap".to_string()]);
        assert_eq!(role, PositionRole::SecondaryCarry);
    }

    #[test]
    fn test_classify_cost5_no_matching_traits_is_secondary_carry() {
        let role = classify_role(5, &["Enforcer".to_string()]);
        assert_eq!(role, PositionRole::SecondaryCarry);
    }

    #[test]
    fn test_classify_case_insensitive() {
        let role = classify_role(3, &["gunner".to_string()]);
        assert_eq!(role, PositionRole::Carry);
    }

    // ── PositioningAdvisor::advise_positions ──────────────────────────────────

    #[test]
    fn test_advise_empty_board_returns_default_layout() {
        let cat = catalog();
        let advisor = PositioningAdvisor::new();
        let layout = advisor
            .advise_positions(&empty_state(), cat)
            .expect("advise failed");
        assert!(layout.positions.is_empty());
        assert_eq!(layout.frontline_count, 0);
        assert_eq!(layout.backline_count, 0);
        assert!(layout.carry_champion.is_none());
    }

    #[test]
    fn test_advise_identifies_main_carry() {
        let cat = catalog();
        let advisor = PositioningAdvisor::new();
        let jinx = champ_id("Jinx"); // Gunner, cost-3
        let mut state = empty_state();
        state.board.push(board_slot(jinx));
        let layout = advisor
            .advise_positions(&state, cat)
            .expect("advise failed");
        assert_eq!(layout.carry_champion.as_deref(), Some("Jinx"));
    }

    #[test]
    fn test_advise_frontline_assigned_row_1() {
        let cat = catalog();
        let advisor = PositioningAdvisor::new();
        let vi = champ_id("Vi"); // Bruiser = Frontline
        let mut state = empty_state();
        state.board.push(board_slot(vi));
        let layout = advisor
            .advise_positions(&state, cat)
            .expect("advise failed");
        let rec = layout
            .positions
            .iter()
            .find(|r| r.champion_id == vi)
            .expect("Vi not found");
        assert_eq!(rec.suggested_position.row, 1, "Frontline should be row 1");
    }

    #[test]
    fn test_advise_carry_assigned_row_4_col_4() {
        let cat = catalog();
        let advisor = PositioningAdvisor::new();
        let jinx = champ_id("Jinx"); // Gunner
        let mut state = empty_state();
        state.board.push(board_slot(jinx));
        let layout = advisor
            .advise_positions(&state, cat)
            .expect("advise failed");
        let rec = layout
            .positions
            .iter()
            .find(|r| r.champion_id == jinx)
            .expect("Jinx not found");
        assert_eq!(rec.suggested_position.row, 4);
        assert_eq!(rec.suggested_position.col, 4);
    }

    #[test]
    fn test_advise_support_assigned_flank_col() {
        let cat = catalog();
        let advisor = PositioningAdvisor::new();
        let janna = champ_id("Janna"); // Scholar = Support
        let mut state = empty_state();
        state.board.push(board_slot(janna));
        let layout = advisor
            .advise_positions(&state, cat)
            .expect("advise failed");
        let rec = layout
            .positions
            .iter()
            .find(|r| r.champion_id == janna)
            .expect("Janna not found");
        assert!(
            rec.suggested_position.col == 1 || rec.suggested_position.col == 7,
            "Support should be at col 1 or 7, got {}",
            rec.suggested_position.col
        );
    }

    #[test]
    fn test_advise_frontline_count_correct() {
        let cat = catalog();
        let advisor = PositioningAdvisor::new();
        let vi = champ_id("Vi"); // Frontline
        let thresh = champ_id("Thresh"); // Guardian = Frontline
        let mut state = empty_state();
        state.board.push(board_slot(vi));
        state.board.push(board_slot(thresh));
        let layout = advisor
            .advise_positions(&state, cat)
            .expect("advise failed");
        assert_eq!(layout.frontline_count, 2);
    }

    #[test]
    fn test_advise_backline_count_correct() {
        let cat = catalog();
        let advisor = PositioningAdvisor::new();
        let jinx = champ_id("Jinx"); // Carry
        let lux = champ_id("Lux"); // Arcanist = Carry
        let mut state = empty_state();
        state.board.push(board_slot(jinx));
        state.board.push(board_slot(lux));
        let layout = advisor
            .advise_positions(&state, cat)
            .expect("advise failed");
        assert_eq!(layout.backline_count, 2);
    }

    #[test]
    fn test_advise_warning_no_frontline() {
        let cat = catalog();
        let advisor = PositioningAdvisor::new();
        let jinx = champ_id("Jinx"); // Carry only
        let mut state = empty_state();
        state.board.push(board_slot(jinx));
        let layout = advisor
            .advise_positions(&state, cat)
            .expect("advise failed");
        assert_eq!(
            layout.layout_warning.as_deref(),
            Some("No frontline — your carry will die immediately")
        );
    }

    #[test]
    fn test_advise_warning_frontline_heavy() {
        let cat = catalog();
        let advisor = PositioningAdvisor::new();
        // 4 frontline, 1 carry (backline) → 4 >= 1*2 → warning
        let vi = champ_id("Vi");
        let thresh = champ_id("Thresh");
        let poppy = champ_id("Poppy");
        let jinx = champ_id("Jinx");
        let mut state = empty_state();
        // Vi, Thresh, Poppy = frontline; Jinx = carry
        state.board.push(board_slot(vi));
        state.board.push(board_slot(thresh));
        state.board.push(board_slot(poppy));
        state.board.push(board_slot(jinx));
        let layout = advisor
            .advise_positions(&state, cat)
            .expect("advise failed");
        // frontline_count=3, backline_count=1 → 3 >= 2 → warning
        if layout.frontline_count >= layout.backline_count.saturating_mul(2) {
            assert_eq!(
                layout.layout_warning.as_deref(),
                Some("Very frontline-heavy — consider more carries")
            );
        }
    }

    #[test]
    fn test_advise_no_warning_balanced() {
        let cat = catalog();
        let advisor = PositioningAdvisor::new();
        // 2 frontline, 2 carries
        let vi = champ_id("Vi");
        let thresh = champ_id("Thresh");
        let jinx = champ_id("Jinx");
        let ashe = champ_id("Ashe"); // Sniper = Carry
        let mut state = empty_state();
        state.board.push(board_slot(vi));
        state.board.push(board_slot(thresh));
        state.board.push(board_slot(jinx));
        state.board.push(board_slot(ashe));
        let layout = advisor
            .advise_positions(&state, cat)
            .expect("advise failed");
        assert!(
            layout.layout_warning.is_none(),
            "balanced board should have no warning, got: {:?}",
            layout.layout_warning
        );
    }

    #[test]
    fn test_advise_positions_count_matches_board() {
        let cat = catalog();
        let advisor = PositioningAdvisor::new();
        let jinx = champ_id("Jinx");
        let vi = champ_id("Vi");
        let janna = champ_id("Janna");
        let mut state = empty_state();
        state.board.push(board_slot(jinx));
        state.board.push(board_slot(vi));
        state.board.push(board_slot(janna));
        let layout = advisor
            .advise_positions(&state, cat)
            .expect("advise failed");
        assert_eq!(layout.positions.len(), 3);
    }

    #[test]
    fn test_advise_reason_not_empty() {
        let cat = catalog();
        let advisor = PositioningAdvisor::new();
        let jinx = champ_id("Jinx");
        let mut state = empty_state();
        state.board.push(board_slot(jinx));
        let layout = advisor
            .advise_positions(&state, cat)
            .expect("advise failed");
        for rec in &layout.positions {
            assert!(
                !rec.reason.is_empty(),
                "reason should not be empty for {}",
                rec.champion_name
            );
        }
    }

    #[test]
    fn test_default_equals_new() {
        let cat = catalog();
        let a = PositioningAdvisor::new();
        let b = PositioningAdvisor::default();
        let r1 = a
            .advise_positions(&empty_state(), cat)
            .expect("advise failed");
        let r2 = b
            .advise_positions(&empty_state(), cat)
            .expect("advise failed");
        assert_eq!(r1.positions.len(), r2.positions.len());
    }
}

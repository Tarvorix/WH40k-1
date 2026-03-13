//! Formation generator for spreading unit models into coherent formations.
//!
//! When deploying a unit or arriving from reserves, models need to be arranged
//! in a formation that satisfies:
//! - Unit coherency (each model within 2" of another; 7+ models need 2 neighbors)
//! - No base overlap between models (within the unit or with other units)
//! - All models wholly within board bounds
//! - All models wholly within deployment zone (when deploying)
//! - No overlap with impassable terrain
//!
//! Source: 40k_revised.md - Unit Coherency, Base Sizes, Engagement Range
//! Source: CP_Rules.md - Deployment rules

use wh40k_core_types::{BaseSize, Inches, Position};

use crate::{Board, DeploymentZone};

// ---------------------------------------------------------------------------
// Formation generation
// ---------------------------------------------------------------------------

/// Result of formation generation.
#[derive(Debug, Clone)]
pub enum FormationResult {
    /// Successfully generated positions for all models.
    Success(Vec<Position>),
    /// Could not generate a valid formation at this center point.
    /// The center is too close to a boundary or other models.
    CannotFit(String),
}

/// Generate a coherent formation of model positions around a center point.
///
/// Models are arranged in concentric rings:
/// - 1 model: placed at center
/// - 2-6 models: first model at center, rest on a single ring
/// - 7-10 models: first model at center, 3 on inner ring, rest on outer ring
///
/// The formation satisfies:
/// - Unit coherency (2" base-to-base, 2 neighbors for 7+ models)
/// - No base overlap within the formation
/// - No base overlap with `other_models` from other units
/// - All models wholly within board bounds
/// - All models wholly within deployment zone (if provided)
/// - No overlap with impassable terrain
///
/// # Arguments
/// * `center` - Center point for the formation
/// * `base_sizes` - Base sizes for each model (in order). Length = number of models.
/// * `board` - Board for bounds/terrain checking
/// * `zone` - Optional deployment zone constraint
/// * `other_models` - Positions and base sizes of already-placed models from other units
pub fn generate_formation(
    center: Position,
    base_sizes: &[BaseSize],
    board: &Board,
    zone: Option<&DeploymentZone>,
    other_models: &[(Position, BaseSize)],
) -> FormationResult {
    let count = base_sizes.len();

    if count == 0 {
        return FormationResult::Success(Vec::new());
    }

    // Single model: just place at center
    if count == 1 {
        if let Some(reason) = check_position_valid(center, base_sizes[0], board, zone, other_models) {
            return FormationResult::CannotFit(reason);
        }
        return FormationResult::Success(vec![center]);
    }

    // Find the largest base size in the unit for spacing calculations
    let max_base_radius = base_sizes.iter().map(|b| b.radius_inches().mils()).max().unwrap_or(0);
    let typical_radius = Inches::from_mils(max_base_radius);

    // Gap between adjacent bases (0.2" = 200 mils for visual clarity)
    let gap = Inches::from_mils(200);

    // Try the formation at the given center. If it doesn't fit, try shifting.
    let result = try_generate_at_center(center, base_sizes, typical_radius, gap, board, zone, other_models);
    if let FormationResult::Success(_) = &result {
        return result;
    }

    // Try compacted formation (smaller gap)
    let compact_gap = Inches::from_mils(50);
    let result = try_generate_at_center(center, base_sizes, typical_radius, compact_gap, board, zone, other_models);
    if let FormationResult::Success(_) = &result {
        return result;
    }

    // Calculate toward-center unit direction (scaled to 1000)
    let board_center = board.center();
    let dx_to_center = board_center.x.mils() - center.x.mils();
    let dy_to_center = board_center.y.mils() - center.y.mils();
    let toward_center = if dx_to_center != 0 || dy_to_center != 0 {
        let mag = isqrt_i32(
            dx_to_center as i64 * dx_to_center as i64
                + dy_to_center as i64 * dy_to_center as i64,
        );
        if mag > 0 {
            Some((
                (dx_to_center as i64 * 1000 / mag) as i32,
                (dy_to_center as i64 * 1000 / mag) as i32,
            ))
        } else {
            None
        }
    } else {
        None
    };

    // Unit direction vectors (scaled to 1000) for 8 cardinal/diagonal directions
    const DIRECTIONS: [(i32, i32); 8] = [
        (1000, 0),
        (-1000, 0),
        (0, 1000),
        (0, -1000),
        (707, 707),
        (-707, 707),
        (707, -707),
        (-707, -707),
    ];

    // Try progressively larger shifts with both normal and compact gaps.
    // Formations can extend ~3" from center (10-model double ring), so we need
    // shifts up to 4" to pull them away from zone edges.
    let shift_distances: [i32; 5] = [1000, 2000, 3000, 4000, 5000];
    let gaps = [gap, compact_gap];

    for &shift_dist in &shift_distances {
        // Try shifting toward board center first (most likely to help for zone-edge cases)
        if let Some((nx, ny)) = toward_center {
            let sx = (nx as i64 * shift_dist as i64 / 1000) as i32;
            let sy = (ny as i64 * shift_dist as i64 / 1000) as i32;
            let shifted = Position::new(
                Inches::from_mils(center.x.mils() + sx),
                Inches::from_mils(center.y.mils() + sy),
            );
            for &g in &gaps {
                let result = try_generate_at_center(
                    shifted, base_sizes, typical_radius, g, board, zone, other_models,
                );
                if let FormationResult::Success(_) = &result {
                    return result;
                }
            }
        }

        // Try 8 cardinal/diagonal directions at this distance
        for &(dx, dy) in &DIRECTIONS {
            let sx = (dx as i64 * shift_dist as i64 / 1000) as i32;
            let sy = (dy as i64 * shift_dist as i64 / 1000) as i32;
            let shifted = Position::new(
                Inches::from_mils(center.x.mils() + sx),
                Inches::from_mils(center.y.mils() + sy),
            );
            for &g in &gaps {
                let result = try_generate_at_center(
                    shifted, base_sizes, typical_radius, g, board, zone, other_models,
                );
                if let FormationResult::Success(_) = &result {
                    return result;
                }
            }
        }
    }

    // All attempts failed
    FormationResult::CannotFit(
        "Cannot fit formation at or near the selected position. Try a location further from edges and other units.".to_string()
    )
}

/// Integer square root helper.
fn isqrt_i32(n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Try generating a formation at a specific center point.
fn try_generate_at_center(
    center: Position,
    base_sizes: &[BaseSize],
    typical_radius: Inches,
    gap: Inches,
    board: &Board,
    zone: Option<&DeploymentZone>,
    other_models: &[(Position, BaseSize)],
) -> FormationResult {
    let count = base_sizes.len();

    let positions = if count <= 5 {
        // Single horizontal rank (line abreast)
        generate_single_rank(center, base_sizes, typical_radius, gap)
    } else {
        // Two ranks (front + back row)
        generate_two_ranks(center, base_sizes, typical_radius, gap)
    };

    // Validate all positions
    let mut placed: Vec<(Position, BaseSize)> = Vec::with_capacity(count);

    for (i, &pos) in positions.iter().enumerate() {
        let base = base_sizes[i];

        // Check board bounds
        if !board.contains_model(pos, base) {
            return FormationResult::CannotFit("Model would be outside board bounds".to_string());
        }

        // Check deployment zone
        if let Some(z) = zone {
            if !z.wholly_contains_model(pos, base) {
                return FormationResult::CannotFit("Model would be outside deployment zone".to_string());
            }
        }

        // Check impassable terrain
        if board.overlaps_impassable_terrain(pos, base) {
            return FormationResult::CannotFit("Model overlaps impassable terrain".to_string());
        }

        // Check overlap with other units' models
        if crate::overlaps_any_model(pos, base, other_models) {
            return FormationResult::CannotFit("Model overlaps another unit's model".to_string());
        }

        // Check overlap with already-placed models in this formation
        if crate::overlaps_any_model(pos, base, &placed) {
            return FormationResult::CannotFit("Models within formation overlap each other".to_string());
        }

        placed.push((pos, base));
    }

    FormationResult::Success(positions)
}

/// Generate positions in a single-rank line abreast formation (2-5 models).
///
/// Layout: models arranged in a horizontal row centered on `center`.
/// Model 0 (leader) is placed at center. Remaining models alternate
/// left and right to build a symmetric line.
///
/// ```text
///   4  2  0  1  3
/// ```
fn generate_single_rank(
    center: Position,
    base_sizes: &[BaseSize],
    typical_radius: Inches,
    gap: Inches,
) -> Vec<Position> {
    let count = base_sizes.len();
    let mut positions = vec![Position::default(); count];

    // Spacing between adjacent model centers (diameter + gap)
    let spacing = typical_radius.mils() * 2 + gap.mils();

    // Model 0 (leader) at center. Models 1,2,3,4... go to slots +1,-1,+2,-2...
    for (i, pos) in positions.iter_mut().enumerate() {
        if i == 0 {
            *pos = center;
            continue;
        }
        let slot = if i % 2 == 1 {
            i.div_ceil(2) as i32
        } else {
            -(i as i32 / 2)
        };
        *pos = Position::new(
            Inches::from_mils(center.x.mils() + slot * spacing),
            center.y,
        );
    }

    positions
}

/// Generate positions in a two-rank formation (6-10 models).
///
/// Layout: two horizontal rows. Front rank is centered on `center`,
/// back rank is offset behind (positive Y = downward on board).
/// Model 0 (leader) is at center of front rank.
///
/// For 10 models (5 front, 5 back):
/// ```text
///   Front:  8  6  0  5  7
///   Back:   9  3  1  2  4
/// ```
///
/// This gives every model at least 2 neighbors within coherency (2"):
/// - Front rank models are adjacent to left/right neighbors
/// - Back rank models are adjacent to left/right AND to front rank models above
fn generate_two_ranks(
    center: Position,
    base_sizes: &[BaseSize],
    typical_radius: Inches,
    gap: Inches,
) -> Vec<Position> {
    let count = base_sizes.len();
    let mut positions = vec![Position::default(); count];

    // Column spacing (horizontal distance between adjacent model centers)
    let col_spacing = typical_radius.mils() * 2 + gap.mils();

    // Row spacing (vertical distance between front and back rank centers)
    // Must satisfy coherency: diagonal distance from front to back rank ≤ 2" + 2*radius
    // With col_spacing offset of half a column, diagonal = sqrt(row^2 + (col/2)^2)
    // Keep it simple: same as column spacing for a nice grid
    let row_spacing = typical_radius.mils() * 2 + gap.mils();

    // Split into front rank and back rank
    let front_count = count.div_ceil(2); // 5 for 10, 5 for 9, 4 for 8, etc.
    let back_count = count - front_count;

    // Place front rank centered on `center`
    // Model 0 (leader) at center. Others alternate right/left.
    positions[0] = center;

    // Model 0 = leader center front
    // Models 1..back_count = back rank
    // Models back_count+1..count-1 = rest of front rank
    for i in 1..front_count {
        let slot = if i % 2 == 1 {
            i.div_ceil(2) as i32
        } else {
            -(i as i32 / 2)
        };
        positions[back_count + i] = Position::new(
            Inches::from_mils(center.x.mils() + slot * col_spacing),
            center.y,
        );
    }

    // Place back rank behind front rank (positive Y = further from enemy)
    // Slots: 0, -1, +1, -2, +2 ... (centered behind front rank)
    for i in 0..back_count {
        let slot = if i % 2 == 0 {
            (i as i32 + 1) / 2   // 0->0, 2->1, 4->2
        } else {
            -(i.div_ceil(2) as i32) // 1->-1, 3->-2
        };
        positions[1 + i] = Position::new(
            Inches::from_mils(center.x.mils() + slot * col_spacing),
            Inches::from_mils(center.y.mils() + row_spacing),
        );
    }

    positions
}

/// Check if a single position is valid for placement.
/// Returns None if valid, Some(reason) if invalid.
fn check_position_valid(
    pos: Position,
    base: BaseSize,
    board: &Board,
    zone: Option<&DeploymentZone>,
    other_models: &[(Position, BaseSize)],
) -> Option<String> {
    if !board.contains_model(pos, base) {
        return Some("Position outside board bounds".to_string());
    }
    if let Some(z) = zone {
        if !z.wholly_contains_model(pos, base) {
            return Some("Position outside deployment zone".to_string());
        }
    }
    if board.overlaps_impassable_terrain(pos, base) {
        return Some("Position overlaps impassable terrain".to_string());
    }
    if crate::overlaps_any_model(pos, base, other_models) {
        return Some("Position overlaps another model".to_string());
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_core_types::BoardDimensions;

    fn make_board() -> Board {
        Board::new(BoardDimensions::COMBAT_PATROL)
    }

    #[test]
    fn test_single_model_formation() {
        let board = make_board();
        let center = Position::from_inches(22, 5);
        let result = generate_formation(center, &[BaseSize::MM60], &board, None, &[]);
        match result {
            FormationResult::Success(positions) => {
                assert_eq!(positions.len(), 1);
                assert_eq!(positions[0], center);
            }
            FormationResult::CannotFit(reason) => panic!("Should fit: {}", reason),
        }
    }

    #[test]
    fn test_three_model_formation() {
        let board = make_board();
        let center = Position::from_inches(22, 15);
        let bases = [BaseSize::MM32, BaseSize::MM32, BaseSize::MM32];
        let result = generate_formation(center, &bases, &board, None, &[]);
        match result {
            FormationResult::Success(positions) => {
                assert_eq!(positions.len(), 3);
                // First model should be at center
                assert_eq!(positions[0], center);
                // All positions should be different
                assert_ne!(positions[1], positions[0]);
                assert_ne!(positions[2], positions[0]);
                assert_ne!(positions[1], positions[2]);
                // All should be on the board
                for (i, &pos) in positions.iter().enumerate() {
                    assert!(board.contains_model(pos, bases[i]),
                        "Model {} at {:?} should be on board", i, pos);
                }
                // Check no overlap between models
                for i in 0..positions.len() {
                    for j in (i+1)..positions.len() {
                        let dist = crate::distance_between_models(
                            positions[i], bases[i],
                            positions[j], bases[j],
                        );
                        assert!(dist.mils() >= 0,
                            "Models {} and {} overlap: dist={}", i, j, dist);
                    }
                }
            }
            FormationResult::CannotFit(reason) => panic!("Should fit: {}", reason),
        }
    }

    #[test]
    fn test_ten_model_formation_berzerkers() {
        let board = make_board();
        let center = Position::from_inches(22, 5);
        let bases = [BaseSize::MM32; 10];
        let result = generate_formation(center, &bases, &board, None, &[]);
        match result {
            FormationResult::Success(positions) => {
                assert_eq!(positions.len(), 10);
                // All should be on the board
                for (i, &pos) in positions.iter().enumerate() {
                    assert!(board.contains_model(pos, bases[i]),
                        "Model {} at {:?} should be on board", i, pos);
                }
                // No overlap between any pair
                for i in 0..positions.len() {
                    for j in (i+1)..positions.len() {
                        let dist = crate::distance_between_models(
                            positions[i], bases[i],
                            positions[j], bases[j],
                        );
                        assert!(dist.mils() >= 0,
                            "Models {} and {} overlap: dist={} mils", i, j, dist.mils());
                    }
                }
                // Check coherency using the geometry crate's check
                let model_positions: Vec<crate::ModelPosition> = positions.iter().enumerate()
                    .map(|(i, &pos)| crate::ModelPosition {
                        position: pos,
                        base_size: bases[i],
                        height: Inches::ZERO,
                    })
                    .collect();
                let coherency = crate::check_coherency(&model_positions);
                assert!(coherency.is_coherent(),
                    "10-model formation should be coherent, got: {:?}", coherency);
            }
            FormationResult::CannotFit(reason) => panic!("Should fit 10 Berzerkers: {}", reason),
        }
    }

    #[test]
    fn test_ten_model_formation_jakhals() {
        let board = make_board();
        let center = Position::from_inches(22, 5);
        let bases = [BaseSize::MM28; 10];
        let result = generate_formation(center, &bases, &board, None, &[]);
        match result {
            FormationResult::Success(positions) => {
                assert_eq!(positions.len(), 10);
                // Check coherency
                let model_positions: Vec<crate::ModelPosition> = positions.iter().enumerate()
                    .map(|(i, &pos)| crate::ModelPosition {
                        position: pos,
                        base_size: bases[i],
                        height: Inches::ZERO,
                    })
                    .collect();
                let coherency = crate::check_coherency(&model_positions);
                assert!(coherency.is_coherent(),
                    "10 Jakhals formation should be coherent, got: {:?}", coherency);
            }
            FormationResult::CannotFit(reason) => panic!("Should fit 10 Jakhals: {}", reason),
        }
    }

    #[test]
    fn test_two_model_formation() {
        let board = make_board();
        let center = Position::from_inches(22, 15);
        let bases = [BaseSize::MM40, BaseSize::MM40];
        let result = generate_formation(center, &bases, &board, None, &[]);
        match result {
            FormationResult::Success(positions) => {
                assert_eq!(positions.len(), 2);
                // Models should not overlap
                let dist = crate::distance_between_models(
                    positions[0], bases[0],
                    positions[1], bases[1],
                );
                assert!(dist.mils() >= 0, "Models overlap: dist={}", dist);
                // Within coherency (2")
                assert!(dist.mils() <= 2000, "Models too far apart for coherency: {}mils", dist.mils());
            }
            FormationResult::CannotFit(reason) => panic!("Should fit: {}", reason),
        }
    }

    #[test]
    fn test_formation_near_board_edge() {
        let board = make_board();
        // Place near bottom-left corner
        let center = Position::from_inches(2, 2);
        let bases = [BaseSize::MM32; 3];
        let result = generate_formation(center, &bases, &board, None, &[]);
        match result {
            FormationResult::Success(positions) => {
                for (i, &pos) in positions.iter().enumerate() {
                    assert!(board.contains_model(pos, bases[i]),
                        "Model {} at {:?} should be on board", i, pos);
                }
            }
            FormationResult::CannotFit(_) => {
                // It's acceptable to fail near a corner — formation shifts should help
                // but very tight corners may not fit
            }
        }
    }

    #[test]
    fn test_formation_avoids_other_models() {
        let board = make_board();
        let center = Position::from_inches(22, 15);
        let bases = [BaseSize::MM32; 3];
        // Place an obstacle model right next to center
        let other_models = vec![
            (Position::new(
                Inches::from_inches(22) + Inches::from_mils(1500),
                Inches::from_inches(15),
            ), BaseSize::MM32),
        ];
        let result = generate_formation(center, &bases, &board, None, &other_models);
        match result {
            FormationResult::Success(positions) => {
                // None of our models should overlap the obstacle
                for (i, &pos) in positions.iter().enumerate() {
                    assert!(!crate::overlaps_any_model(pos, bases[i], &other_models),
                        "Model {} at {:?} overlaps obstacle", i, pos);
                }
            }
            FormationResult::CannotFit(_) => {
                // Acceptable if we can't fit due to obstacle
            }
        }
    }

    #[test]
    fn test_empty_formation() {
        let board = make_board();
        let center = Position::from_inches(22, 15);
        let result = generate_formation(center, &[], &board, None, &[]);
        match result {
            FormationResult::Success(positions) => assert_eq!(positions.len(), 0),
            FormationResult::CannotFit(reason) => panic!("Empty should always succeed: {}", reason),
        }
    }

    #[test]
    fn test_six_model_formation() {
        let board = make_board();
        let center = Position::from_inches(22, 15);
        let bases = [BaseSize::MM32; 6];
        let result = generate_formation(center, &bases, &board, None, &[]);
        match result {
            FormationResult::Success(positions) => {
                assert_eq!(positions.len(), 6);
                // No overlap
                for i in 0..positions.len() {
                    for j in (i+1)..positions.len() {
                        let dist = crate::distance_between_models(
                            positions[i], bases[i],
                            positions[j], bases[j],
                        );
                        assert!(dist.mils() >= 0,
                            "Models {} and {} overlap: dist={}", i, j, dist.mils());
                    }
                }
            }
            FormationResult::CannotFit(reason) => panic!("Should fit 6 models: {}", reason),
        }
    }

    #[test]
    fn test_jakhals_near_attacker_zone_edge() {
        // Reproduces production bug: 10 Jakhals (28mm) deployed near the
        // attacker zone boundary (y=9"). The formation extends ~2.6" from
        // center, so if the AI picks y=7", models at y=7"+2.6"=9.6" are
        // outside the zone. The generator must shift to find a valid fit.
        let board = make_board();
        let config = crate::create_standard_deployment(
            &wh40k_core_types::BoardDimensions::COMBAT_PATROL,
            wh40k_core_types::PlayerId::new(0),
            wh40k_core_types::PlayerId::new(1),
        );
        let zone = &config.attacker_zone;
        // AI picks a point near the top edge of the attacker zone
        let center = Position::new(
            Inches::from_inches(22),
            Inches::from_mils(7500), // 7.5" out of 9" deep zone
        );
        let bases = [BaseSize::MM28; 10];
        let result = generate_formation(center, &bases, &board, Some(zone), &[]);
        match result {
            FormationResult::Success(positions) => {
                assert_eq!(positions.len(), 10);
                // All models must be in the zone
                for (i, &pos) in positions.iter().enumerate() {
                    assert!(zone.wholly_contains_model(pos, bases[i]),
                        "Model {} at {:?} should be in attacker zone", i, pos);
                }
                // Check coherency
                let model_positions: Vec<crate::ModelPosition> = positions.iter().enumerate()
                    .map(|(i, &pos)| crate::ModelPosition {
                        position: pos,
                        base_size: bases[i],
                        height: Inches::ZERO,
                    })
                    .collect();
                let coherency = crate::check_coherency(&model_positions);
                assert!(coherency.is_coherent(),
                    "Formation should be coherent: {:?}", coherency);
            }
            FormationResult::CannotFit(reason) => {
                panic!("Should fit 10 Jakhals near zone edge: {}", reason);
            }
        }
    }

    #[test]
    fn test_custodian_guard_near_defender_zone_edge() {
        // 3 Custodian Guard (32mm) deployed near the defender zone boundary.
        // Defender zone: y=21" to y=30". If AI picks y=21.5", formation
        // ring extends below y=21", outside the zone.
        let board = make_board();
        let config = crate::create_standard_deployment(
            &wh40k_core_types::BoardDimensions::COMBAT_PATROL,
            wh40k_core_types::PlayerId::new(0),
            wh40k_core_types::PlayerId::new(1),
        );
        let zone = &config.defender_zone;
        let center = Position::new(
            Inches::from_inches(22),
            Inches::from_mils(21500), // 0.5" inside defender zone
        );
        let bases = [BaseSize::MM32; 3];
        let result = generate_formation(center, &bases, &board, Some(zone), &[]);
        match result {
            FormationResult::Success(positions) => {
                assert_eq!(positions.len(), 3);
                for (i, &pos) in positions.iter().enumerate() {
                    assert!(zone.wholly_contains_model(pos, bases[i]),
                        "Model {} at {:?} should be in defender zone", i, pos);
                }
            }
            FormationResult::CannotFit(reason) => {
                panic!("Should fit 3 Custodian Guard near zone edge: {}", reason);
            }
        }
    }

    #[test]
    fn test_berzerkers_in_attacker_zone_center() {
        // 10 Berzerkers (32mm) placed well inside attacker zone — should
        // always succeed without shifting.
        let board = make_board();
        let config = crate::create_standard_deployment(
            &wh40k_core_types::BoardDimensions::COMBAT_PATROL,
            wh40k_core_types::PlayerId::new(0),
            wh40k_core_types::PlayerId::new(1),
        );
        let zone = &config.attacker_zone;
        let center = Position::from_inches(22, 4); // well inside the 9" zone
        let bases = [BaseSize::MM32; 10];
        let result = generate_formation(center, &bases, &board, Some(zone), &[]);
        match result {
            FormationResult::Success(positions) => {
                assert_eq!(positions.len(), 10);
                for (i, &pos) in positions.iter().enumerate() {
                    assert!(zone.wholly_contains_model(pos, bases[i]),
                        "Model {} at {:?} should be in attacker zone", i, pos);
                }
                let model_positions: Vec<crate::ModelPosition> = positions.iter().enumerate()
                    .map(|(i, &pos)| crate::ModelPosition {
                        position: pos,
                        base_size: bases[i],
                        height: Inches::ZERO,
                    })
                    .collect();
                let coherency = crate::check_coherency(&model_positions);
                assert!(coherency.is_coherent(),
                    "Formation should be coherent: {:?}", coherency);
            }
            FormationResult::CannotFit(reason) => {
                panic!("Should fit 10 Berzerkers in zone center: {}", reason);
            }
        }
    }

}

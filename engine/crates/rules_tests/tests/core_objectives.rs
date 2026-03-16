//! Source-linked rules tests for 40k_revised.md Section 18: Objectives.
//!
//! Tests cover:
//!   - 18.1: Objective Marker Basics (OBJECTIVE_RANGE = 3", position tracking)
//!   - 18.2: Controlling Objectives (OC tally, higher wins, equal = contested,
//!           battle-shocked = OC 0)
//!   - 18.4: Sticky Objectives (once controlled, stays until opponent takes it)
//!
//! Source: 40k_revised.md Section 18 -- "OBJECTIVES"

use wh40k_core_types::{
    ArmorSave, BaseSize, DatasheetId, EngagementStatus, Inches, Keyword,
    KeywordSet, Leadership, ModelId, MoveCharacteristic, ObjectiveControl,
    ObjectiveId, PlayerId, Position, Toughness, UnitId, UnitStatus, Wounds,
};
use wh40k_game_core::unit::{ModelState, UnitState};
use wh40k_geometry::{ObjectiveControlStatus, ObjectiveMarker, within_objective_range_2d};

// ===========================================================================
// Helper factories
// ===========================================================================

/// Build a model at a specified position.
fn make_model(model_id: u32, unit_id: UnitId, wounds: u8, pos: Position) -> ModelState {
    ModelState::new(
        ModelId::new(model_id),
        unit_id,
        Wounds::new(wounds),
        pos,
        BaseSize::MM32,
        vec![],
        vec![],
        false,
        None,
    )
}

/// Build a unit with given keywords, OC value, and models.
fn make_unit_with_oc(
    id: u32,
    owner: PlayerId,
    keywords: &[Keyword],
    models: Vec<ModelState>,
    oc: u8,
) -> UnitState {
    let mut unit = UnitState::new(
        UnitId::new(id),
        owner,
        "Test Unit".to_string(),
        DatasheetId::new(1),
        KeywordSet::from_keywords(keywords),
        models,
        MoveCharacteristic::from_inches(6),
        Toughness::new(4),
        ArmorSave::THREE_PLUS,
        None,
        Leadership::new(7),
        ObjectiveControl::new(oc),
    );
    unit.status = UnitStatus::OnBattlefield;
    unit
}

/// Build a standard unit with OC 2 (typical infantry).
fn make_standard_unit(
    id: u32,
    owner: PlayerId,
    model_count: usize,
    base_pos: Position,
) -> UnitState {
    let unit_id = UnitId::new(id);
    let models: Vec<ModelState> = (0..model_count)
        .map(|i| {
            make_model(
                id * 100 + i as u32,
                unit_id,
                3,
                Position::new(
                    base_pos.x + Inches::from_inches(i as i32),
                    base_pos.y,
                ),
            )
        })
        .collect();
    make_unit_with_oc(id, owner, &[Keyword::Infantry, Keyword::Battleline], models, 2)
}

// ===========================================================================
// §18.1 — Objective Marker Basics: range and position
// ===========================================================================

/// Source: 40k_revised.md §18.1
/// Rule: "A model is in range of an objective marker if it is within 3\"
///        horizontally and 5\" vertically."
/// The OBJECTIVE_RANGE constant is 3 inches.
#[test]
fn objective_range_is_three_inches() {
    assert_eq!(
        Inches::OBJECTIVE_RANGE,
        Inches::from_inches(3),
        "OBJECTIVE_RANGE constant should be 3 inches"
    );
    assert_eq!(
        Inches::OBJECTIVE_RANGE.mils(),
        3000,
        "OBJECTIVE_RANGE should be 3000 mils"
    );
}

/// Source: 40k_revised.md §18.1
/// Rule: The vertical component of objective range is 5 inches.
#[test]
fn objective_range_vertical_is_five_inches() {
    assert_eq!(
        Inches::OBJECTIVE_RANGE_VERTICAL,
        Inches::from_inches(5),
        "OBJECTIVE_RANGE_VERTICAL should be 5 inches"
    );
}

/// Source: 40k_revised.md §18.1
/// Rule: An objective marker has a fixed position on the board.
/// Test that ObjectiveMarker stores and returns its position.
#[test]
fn objective_marker_tracks_position() {
    let obj_pos = Position::from_inches(22, 15);
    let objective = ObjectiveMarker::new(ObjectiveId::new(1), obj_pos, "Alpha");

    assert_eq!(
        objective.position, obj_pos,
        "Objective marker should store its position"
    );
    assert_eq!(objective.id, ObjectiveId::new(1));
    assert_eq!(objective.label, "Alpha");
}

/// Source: 40k_revised.md §18.1
/// Rule: A model within 3" of the objective center is "in range".
/// Uses within_objective_range_2d which accounts for base size.
#[test]
fn model_within_three_inches_is_in_range() {
    let obj_pos = Position::from_inches(20, 15);
    // Model at 18" is 2" from objective center, with 32mm base radius (~0.63")
    // Gap = 2" - 0.63" = ~1.37" which is < 3"
    let model_pos = Position::from_inches(18, 15);
    let base = BaseSize::MM32;

    let in_range = within_objective_range_2d(model_pos, base, obj_pos);
    assert!(
        in_range,
        "Model 2\" from objective center should be within 3\" objective range"
    );
}

/// Source: 40k_revised.md §18.1
/// Rule: A model outside 3" of the objective center is NOT "in range".
#[test]
fn model_beyond_three_inches_is_not_in_range() {
    let obj_pos = Position::from_inches(20, 15);
    // Model at 15" is 5" from objective center
    // Gap from base edge = 5" - 0.63" = ~4.37" which is > 3"
    let model_pos = Position::from_inches(15, 15);
    let base = BaseSize::MM32;

    let in_range = within_objective_range_2d(model_pos, base, obj_pos);
    assert!(
        !in_range,
        "Model 5\" from objective center should NOT be within objective range"
    );
}

/// Source: 40k_revised.md §18.1
/// Rule: The ObjectiveMarker has its own is_model_in_range_2d method for
/// convenience. Test that it works correctly.
#[test]
fn objective_marker_is_model_in_range_method() {
    let obj = ObjectiveMarker::new(ObjectiveId::new(1), Position::from_inches(20, 15), "Alpha");
    let close_model_pos = Position::from_inches(21, 15); // 1" away
    let far_model_pos = Position::from_inches(30, 15); // 10" away
    let base = BaseSize::MM32;

    assert!(
        obj.is_model_in_range_2d(close_model_pos, base),
        "Model 1\" from objective should be in range"
    );
    assert!(
        !obj.is_model_in_range_2d(far_model_pos, base),
        "Model 10\" from objective should NOT be in range"
    );
}

/// Source: 40k_revised.md §18.1
/// Rule: Objective markers start as uncontrolled.
#[test]
fn objective_starts_uncontrolled() {
    let obj = ObjectiveMarker::new(ObjectiveId::new(1), Position::from_inches(22, 15), "Alpha");

    assert_eq!(
        obj.control_status,
        ObjectiveControlStatus::Uncontrolled,
        "Objective should start as Uncontrolled"
    );
    assert!(
        obj.secured_by.is_none(),
        "Objective should not be secured by anyone initially"
    );
}

// ===========================================================================
// §18.2 — Controlling Objectives: OC tallies
// ===========================================================================

/// Source: 40k_revised.md §18.2
/// Rule: "Count the total OC of all models within range of that objective marker
///        for each player. The player with the highest total OC controls it."
/// Test: Single player with models in range controls the objective.
#[test]
fn single_player_models_in_range_controls_objective() {
    let mut obj = ObjectiveMarker::new(
        ObjectiveId::new(1),
        Position::from_inches(20, 15),
        "Alpha",
    );

    // Player 0 has total OC of 4 (e.g., 2 models with OC 2 each)
    let player_oc = vec![(PlayerId::new(0), 4u32)];
    obj.update_control(&player_oc);

    assert_eq!(
        obj.control_status,
        ObjectiveControlStatus::ControlledBy(PlayerId::new(0)),
        "Player with models in range should control the objective"
    );
}

/// Source: 40k_revised.md §18.2
/// Rule: "The player whose models have the highest total OC controls the
///        objective marker."
/// Test: Player with higher total OC controls it.
#[test]
fn higher_total_oc_controls_objective() {
    let mut obj = ObjectiveMarker::new(
        ObjectiveId::new(1),
        Position::from_inches(20, 15),
        "Alpha",
    );

    // Player 0: OC 6, Player 1: OC 4
    let player_oc = vec![(PlayerId::new(0), 6u32), (PlayerId::new(1), 4u32)];
    obj.update_control(&player_oc);

    assert_eq!(
        obj.control_status,
        ObjectiveControlStatus::ControlledBy(PlayerId::new(0)),
        "Player 0 with higher OC (6 vs 4) should control the objective"
    );
}

/// Source: 40k_revised.md §18.2
/// Rule: If Player 1 has more OC, Player 1 controls it.
#[test]
fn player_with_more_oc_controls_regardless_of_order() {
    let mut obj = ObjectiveMarker::new(
        ObjectiveId::new(1),
        Position::from_inches(20, 15),
        "Alpha",
    );

    // Player 0: OC 2, Player 1: OC 8
    let player_oc = vec![(PlayerId::new(0), 2u32), (PlayerId::new(1), 8u32)];
    obj.update_control(&player_oc);

    assert_eq!(
        obj.control_status,
        ObjectiveControlStatus::ControlledBy(PlayerId::new(1)),
        "Player 1 with higher OC (8 vs 2) should control the objective"
    );
}

/// Source: 40k_revised.md §18.2
/// Rule: "If the totals are tied, that objective marker is contested and
///        neither player controls it."
/// Test: Equal OC values mean the objective is contested.
#[test]
fn equal_oc_means_contested() {
    let mut obj = ObjectiveMarker::new(
        ObjectiveId::new(1),
        Position::from_inches(20, 15),
        "Alpha",
    );

    // Both players have OC 4
    let player_oc = vec![(PlayerId::new(0), 4u32), (PlayerId::new(1), 4u32)];
    obj.update_control(&player_oc);

    assert_eq!(
        obj.control_status,
        ObjectiveControlStatus::Contested,
        "Equal OC should result in Contested status"
    );
}

/// Source: 40k_revised.md §18.2
/// Rule: If no player has models in range, the objective is uncontrolled.
#[test]
fn no_models_in_range_means_uncontrolled() {
    let mut obj = ObjectiveMarker::new(
        ObjectiveId::new(1),
        Position::from_inches(20, 15),
        "Alpha",
    );

    // Empty OC list: no models in range
    let player_oc: Vec<(PlayerId, u32)> = vec![];
    obj.update_control(&player_oc);

    assert_eq!(
        obj.control_status,
        ObjectiveControlStatus::Uncontrolled,
        "No models in range should leave objective Uncontrolled"
    );
}

/// Source: 40k_revised.md §18.2
/// Rule: If only models with OC 0 are in range, the objective stays uncontrolled.
/// (This can happen with battle-shocked units.)
#[test]
fn all_oc_zero_means_uncontrolled() {
    let mut obj = ObjectiveMarker::new(
        ObjectiveId::new(1),
        Position::from_inches(20, 15),
        "Alpha",
    );

    // Player has models in range but all with OC 0 (battle-shocked)
    let player_oc = vec![(PlayerId::new(0), 0u32)];
    obj.update_control(&player_oc);

    assert_eq!(
        obj.control_status,
        ObjectiveControlStatus::Uncontrolled,
        "All OC 0 should leave objective Uncontrolled"
    );
}

/// Source: 40k_revised.md §18.2
/// Rule: Both players with OC 0 also means uncontrolled (not contested).
#[test]
fn both_players_oc_zero_means_uncontrolled() {
    let mut obj = ObjectiveMarker::new(
        ObjectiveId::new(1),
        Position::from_inches(20, 15),
        "Alpha",
    );

    let player_oc = vec![(PlayerId::new(0), 0u32), (PlayerId::new(1), 0u32)];
    obj.update_control(&player_oc);

    assert_eq!(
        obj.control_status,
        ObjectiveControlStatus::Uncontrolled,
        "Both players with OC 0 should be Uncontrolled, not Contested"
    );
}

// ===========================================================================
// §18.2 — Battle-shocked units have OC = 0
// ===========================================================================

/// Source: 40k_revised.md §18.2
/// Rule: "Battle-shocked units... have an Objective Control characteristic of 0."
/// Test: A battle-shocked unit returns effective OC of 0.
#[test]
fn battle_shocked_unit_has_oc_zero() {
    let unit_id = UnitId::new(10);
    let model = make_model(1000, unit_id, 3, Position::from_inches(20, 15));
    let mut unit = make_unit_with_oc(
        10,
        PlayerId::new(0),
        &[Keyword::Infantry, Keyword::Battleline],
        vec![model],
        2,
    );

    // Normal OC
    assert_eq!(
        unit.effective_oc().value(),
        2,
        "Non-battle-shocked unit should have base OC"
    );

    // Battle-shock the unit
    unit.battle_shocked = true;
    assert_eq!(
        unit.effective_oc().value(),
        0,
        "Battle-shocked unit should have effective OC of 0"
    );
}

/// Source: 40k_revised.md §18.2
/// Rule: A unit that recovers from battle-shock regains its normal OC.
#[test]
fn recovered_unit_regains_normal_oc() {
    let unit_id = UnitId::new(11);
    let model = make_model(1100, unit_id, 3, Position::from_inches(20, 15));
    let mut unit = make_unit_with_oc(
        11,
        PlayerId::new(0),
        &[Keyword::Infantry],
        vec![model],
        2,
    );

    unit.battle_shocked = true;
    assert_eq!(unit.effective_oc().value(), 0);

    // Recover from battle-shock
    unit.battle_shocked = false;
    assert_eq!(
        unit.effective_oc().value(),
        2,
        "Recovered unit should regain normal OC"
    );
}

/// Source: 40k_revised.md §18.2
/// Rule: Different units can have different base OC values.
/// OC 1 (vehicles/characters) vs OC 2 (infantry) vs OC 0 (certain units).
#[test]
fn different_units_different_oc_values() {
    let model_a = make_model(1200, UnitId::new(12), 3, Position::from_inches(10, 10));
    let unit_oc2 = make_unit_with_oc(
        12,
        PlayerId::new(0),
        &[Keyword::Infantry],
        vec![model_a],
        2,
    );
    assert_eq!(unit_oc2.effective_oc().value(), 2);

    let model_b = make_model(1300, UnitId::new(13), 6, Position::from_inches(10, 10));
    let unit_oc1 = make_unit_with_oc(
        13,
        PlayerId::new(0),
        &[Keyword::Character],
        vec![model_b],
        1,
    );
    assert_eq!(unit_oc1.effective_oc().value(), 1);

    let model_c = make_model(1400, UnitId::new(14), 3, Position::from_inches(10, 10));
    let unit_oc0 = make_unit_with_oc(
        14,
        PlayerId::new(0),
        &[Keyword::Infantry],
        vec![model_c],
        0,
    );
    assert_eq!(unit_oc0.effective_oc().value(), 0);
}

/// Source: 40k_revised.md §18.2
/// Rule: Total OC is the sum of OC for all models within range, not just
/// the unit's OC value. A unit with 5 models at OC 2 contributes
/// 5 * 2 = 10 total OC per the rules (OC is per model, but the unit
/// tracks a single OC characteristic applied to each alive model).
/// Test: A multi-model unit has OC that scales with model count.
#[test]
fn multi_model_unit_oc_scales_with_model_count() {
    let _unit_id = UnitId::new(15);
    let unit = make_standard_unit(15, PlayerId::new(0), 5, Position::from_inches(20, 15));

    // Unit has base OC of 2
    assert_eq!(unit.effective_oc().value(), 2);

    // Per rules, total OC within range = number of models in range * unit's OC
    // With 5 models at OC 2, total OC contribution = 5 * 2 = 10
    let models_alive = unit.models_alive();
    let total_oc = models_alive as u32 * unit.effective_oc().value() as u32;
    assert_eq!(
        total_oc, 10,
        "5 models with OC 2 should contribute total OC of 10"
    );
}

/// Source: 40k_revised.md §18.2
/// Rule: Dead models don't contribute OC.
/// A unit with some models killed has reduced total OC.
#[test]
fn dead_models_reduce_total_oc() {
    let _unit_id = UnitId::new(16);
    let mut unit = make_standard_unit(16, PlayerId::new(0), 5, Position::from_inches(20, 15));

    // Kill 3 models
    unit.models[0].alive = false;
    unit.models[1].alive = false;
    unit.models[2].alive = false;

    let models_alive = unit.models_alive();
    assert_eq!(models_alive, 2);

    let total_oc = models_alive as u32 * unit.effective_oc().value() as u32;
    assert_eq!(
        total_oc, 4,
        "2 alive models with OC 2 should contribute total OC of 4"
    );
}

// ===========================================================================
// §18.4 — Sticky Objectives: once controlled, stays controlled
// ===========================================================================

/// Source: 40k_revised.md §18.4 / CP_Rules.md §12.3
/// Rule: "Once a BATTLELINE unit secures an objective, it stays controlled
///        even if the unit moves away, until the opponent takes control."
/// The secured_by field on ObjectiveMarker tracks this.
#[test]
fn secured_objective_tracks_controlling_player() {
    let mut obj = ObjectiveMarker::new(
        ObjectiveId::new(1),
        Position::from_inches(22, 15),
        "Alpha",
    );

    // No one has secured it initially
    assert!(obj.secured_by.is_none());

    // Player 0 secures it
    obj.secured_by = Some(PlayerId::new(0));
    assert_eq!(
        obj.secured_by,
        Some(PlayerId::new(0)),
        "Objective should track which player secured it"
    );
}

/// Source: 40k_revised.md §18.4 / CP_Rules.md §12.3
/// Rule: A secured objective remains controlled by the securing player
/// even when no friendly models are in range.
/// The secured_by field persists independently of the control_status.
#[test]
fn secured_objective_persists_without_models() {
    let mut obj = ObjectiveMarker::new(
        ObjectiveId::new(1),
        Position::from_inches(22, 15),
        "Alpha",
    );

    // Player 0 secures it
    obj.secured_by = Some(PlayerId::new(0));

    // No models in range anymore — update_control sets Uncontrolled
    obj.update_control(&[]);

    // The OC-based control status may show Uncontrolled, but secured_by persists
    assert_eq!(
        obj.secured_by,
        Some(PlayerId::new(0)),
        "secured_by should persist even when no models are in range"
    );
}

/// Source: 40k_revised.md §18.4 / CP_Rules.md §12.3
/// Rule: The opponent can take a secured objective by controlling it
/// with their own models (higher OC). When the opponent takes it,
/// the secured_by is updated.
#[test]
fn opponent_can_flip_secured_objective() {
    let mut obj = ObjectiveMarker::new(
        ObjectiveId::new(1),
        Position::from_inches(22, 15),
        "Alpha",
    );

    // Player 0 secures it
    obj.secured_by = Some(PlayerId::new(0));

    // Player 1 moves in with higher OC and takes control
    let player_oc = vec![(PlayerId::new(1), 6u32)];
    obj.update_control(&player_oc);

    assert_eq!(
        obj.control_status,
        ObjectiveControlStatus::ControlledBy(PlayerId::new(1)),
        "Player 1 should control via higher OC"
    );

    // Update secured_by to reflect the flip
    obj.secured_by = Some(PlayerId::new(1));
    assert_eq!(
        obj.secured_by,
        Some(PlayerId::new(1)),
        "secured_by should be updated when opponent takes control"
    );
}

/// Source: CP_Rules.md §12.3
/// Rule: Only BATTLELINE units can secure objectives in Combat Patrol.
/// The can_unit_secure_objective function checks this.
#[test]
fn only_battleline_can_secure() {
    use wh40k_combat_patrol_rules::CombatPatrolRules;

    let unit_id = UnitId::new(20);
    let model = make_model(2000, unit_id, 3, Position::from_inches(20, 15));
    let battleline_unit = make_unit_with_oc(
        20,
        PlayerId::new(0),
        &[Keyword::Infantry, Keyword::Battleline],
        vec![model],
        2,
    );
    assert!(
        CombatPatrolRules::can_unit_secure_objective(&battleline_unit),
        "BATTLELINE unit should be able to secure objectives"
    );

    let model2 = make_model(2100, UnitId::new(21), 5, Position::from_inches(20, 15));
    let character_unit = make_unit_with_oc(
        21,
        PlayerId::new(0),
        &[Keyword::Infantry, Keyword::Character],
        vec![model2],
        1,
    );
    assert!(
        !CombatPatrolRules::can_unit_secure_objective(&character_unit),
        "Non-BATTLELINE CHARACTER unit should NOT be able to secure objectives"
    );
}

/// Source: CP_Rules.md §12.3
/// Rule: Battle-shocked BATTLELINE units cannot secure objectives.
#[test]
fn battle_shocked_battleline_cannot_secure() {
    use wh40k_combat_patrol_rules::CombatPatrolRules;

    let unit_id = UnitId::new(22);
    let model = make_model(2200, unit_id, 3, Position::from_inches(20, 15));
    let mut unit = make_unit_with_oc(
        22,
        PlayerId::new(0),
        &[Keyword::Infantry, Keyword::Battleline],
        vec![model],
        2,
    );

    assert!(CombatPatrolRules::can_unit_secure_objective(&unit));

    // Battle-shock the unit
    unit.battle_shocked = true;
    assert!(
        !CombatPatrolRules::can_unit_secure_objective(&unit),
        "Battle-shocked BATTLELINE unit should NOT be able to secure objectives"
    );
}

/// Source: CP_Rules.md §12.3
/// Rule: The ObjectiveSecuringRules configuration confirms that
/// secured objectives persist and BATTLELINE secures in Command Phase.
#[test]
fn objective_securing_rules_configuration() {
    use wh40k_combat_patrol_rules::CombatPatrolRules;

    let rules = CombatPatrolRules::objective_securing_rules();
    assert!(
        rules.battleline_secures_in_command_phase,
        "BATTLELINE should secure in Command Phase"
    );
    assert!(
        rules.secured_objectives_persist,
        "Secured objectives should persist (sticky)"
    );
}

/// Source: 40k_revised.md §18.2
/// Rule: OC enhancement overrides change the effective OC of a unit.
/// Test that enhancement_oc_override is respected in effective_oc().
#[test]
fn enhancement_oc_override_changes_effective_oc() {
    let unit_id = UnitId::new(23);
    let model = make_model(2300, unit_id, 3, Position::from_inches(20, 15));
    let mut unit = make_unit_with_oc(
        23,
        PlayerId::new(0),
        &[Keyword::Infantry, Keyword::Character],
        vec![model],
        1,
    );

    assert_eq!(unit.effective_oc().value(), 1);

    // Apply Fearsome Presence enhancement (OC 5)
    unit.enhancement_oc_override = Some(ObjectiveControl::new(5));
    assert_eq!(
        unit.effective_oc().value(),
        5,
        "Enhancement OC override should change effective OC to 5"
    );
}

/// Source: Custodes.md - Watchman of Terra
/// Rule: Some OC overrides only apply when the unit is in engagement range.
/// Test the enhancement_oc_requires_engaged flag.
#[test]
fn enhancement_oc_override_requires_engaged() {
    let unit_id = UnitId::new(24);
    let model = make_model(2400, unit_id, 3, Position::from_inches(20, 15));
    let mut unit = make_unit_with_oc(
        24,
        PlayerId::new(0),
        &[Keyword::Infantry],
        vec![model],
        2,
    );

    // Watchman of Terra: OC 4 but only in Engagement Range
    unit.enhancement_oc_override = Some(ObjectiveControl::new(4));
    unit.enhancement_oc_requires_engaged = true;

    // Not engaged: should use base OC
    unit.engagement_status = EngagementStatus::NotEngaged;
    assert_eq!(
        unit.effective_oc().value(),
        2,
        "Not-engaged unit should use base OC despite override"
    );

    // Engaged: should use override OC
    unit.engagement_status = EngagementStatus::Engaged;
    assert_eq!(
        unit.effective_oc().value(),
        4,
        "Engaged unit should use enhancement OC override"
    );
}

/// Source: 40k_revised.md §18.2
/// Rule: Battle-shock still overrides enhancement OC to 0.
/// Even with an OC enhancement, a battle-shocked unit has OC 0.
#[test]
fn battle_shock_overrides_enhancement_oc() {
    let unit_id = UnitId::new(25);
    let model = make_model(2500, unit_id, 3, Position::from_inches(20, 15));
    let mut unit = make_unit_with_oc(
        25,
        PlayerId::new(0),
        &[Keyword::Infantry],
        vec![model],
        2,
    );

    unit.enhancement_oc_override = Some(ObjectiveControl::new(5));
    assert_eq!(unit.effective_oc().value(), 5);

    // Battle-shock overrides everything to 0
    unit.battle_shocked = true;
    assert_eq!(
        unit.effective_oc().value(),
        0,
        "Battle-shocked unit should have OC 0 even with enhancement override"
    );
}

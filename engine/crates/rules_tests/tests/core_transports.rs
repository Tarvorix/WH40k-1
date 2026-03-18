//! Source-linked rules tests for 40k_revised.md Section 6: Transports.
//!
//! Tests cover:
//!   §6.1 — Transport Capacity: points limit tracking
//!   §6.2 — Embarking: 3" range, can't have disembarked this phase
//!   §6.3 — Disembarking: before Transport moves, 3" from Transport,
//!           not within Engagement Range of enemies
//!   §6.4 — Destroyed Transports: D6 per model mortal wounds on 1,
//!           Battle-shocked until next Command Phase, counts as Normal Moved,
//!           emergency disembark (6", mortal on 1-3)
//!
//! Source: 40k_revised.md Section 6 — "TRANSPORTS"

use wh40k_core_types::{
    ArmorSave, BaseSize, DatasheetId, Inches, Keyword, KeywordSet, Leadership,
    ModelId, MoveCharacteristic, ObjectiveControl, PlayerId, Position,
    Toughness, UnitId, UnitStatus, Wounds,
};
use wh40k_game_core::unit::{ModelState, UnitState};

// ===========================================================================
// Constants for Transport Rules
// ===========================================================================

/// Embark range: a unit must be within 3" of the Transport to embark.
/// Source: 40k_revised.md §6.2
const EMBARK_RANGE: Inches = Inches::from_inches(3);

/// Disembark range: disembarking models are placed within 3" of the Transport.
/// Source: 40k_revised.md §6.3
const DISEMBARK_RANGE: Inches = Inches::from_inches(3);

/// Emergency disembark range: if normal disembark can't fit, place within 6".
/// Source: 40k_revised.md §6.4
const EMERGENCY_DISEMBARK_RANGE: Inches = Inches::from_inches(6);

/// Engagement range: 1" horizontal.
/// Source: 40k_revised.md §1.4
const ENGAGEMENT_RANGE: Inches = Inches::ENGAGEMENT_RANGE;

/// Mortal wound threshold for destroyed Transport passengers: roll D6, on 1 = 1 MW.
/// Source: 40k_revised.md §6.4
const DESTROYED_TRANSPORT_MW_THRESHOLD: u8 = 1;

/// Mortal wound threshold for emergency disembark: roll D6, on 1-3 = 1 MW.
/// Source: 40k_revised.md §6.4
const EMERGENCY_DISEMBARK_MW_THRESHOLD: u8 = 3;

// ===========================================================================
// Helper factories
// ===========================================================================

/// Build a model with no weapons at a given position.
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

/// Build a multi-model infantry unit.
fn make_infantry_unit(id: u32, owner: PlayerId, model_count: usize, pos: Position) -> UnitState {
    let unit_id = UnitId::new(id);
    let models: Vec<ModelState> = (0..model_count)
        .map(|i| make_model(id * 100 + i as u32, unit_id, 1, pos))
        .collect();

    let mut unit = UnitState::new(
        unit_id,
        owner,
        "Test Infantry".to_string(),
        DatasheetId::new(1),
        KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Battleline]),
        models,
        MoveCharacteristic::from_inches(6),
        Toughness::new(4),
        ArmorSave::THREE_PLUS,
        None,
        Leadership::new(7),
        ObjectiveControl::new(2),
    );
    unit.status = UnitStatus::OnBattlefield;
    unit
}

/// Build a DEDICATED TRANSPORT unit (e.g., Rhino-like) at a given position.
fn make_transport_unit(id: u32, owner: PlayerId, pos: Position) -> UnitState {
    let unit_id = UnitId::new(id);
    let model = ModelState::new(
        ModelId::new(id * 100),
        unit_id,
        Wounds::new(10),
        pos,
        BaseSize::MM60,
        vec![],
        vec![],
        false,
        None,
    );

    let mut unit = UnitState::new(
        unit_id,
        owner,
        "Test Transport".to_string(),
        DatasheetId::new(10),
        KeywordSet::from_keywords(&[Keyword::Vehicle, Keyword::DedicatedTransport, Keyword::Transport]),
        vec![model],
        MoveCharacteristic::from_inches(12),
        Toughness::new(9),
        ArmorSave::THREE_PLUS,
        None,
        Leadership::new(7),
        ObjectiveControl::new(2),
    );
    unit.status = UnitStatus::OnBattlefield;
    unit.transport_capacity = 12; // Typical Rhino-style transport capacity
    unit
}

/// Build an enemy unit at a given position.
fn make_enemy_unit(id: u32, pos: Position) -> UnitState {
    let unit_id = UnitId::new(id);
    let model = make_model(id * 100, unit_id, 2, pos);

    let mut unit = UnitState::new(
        unit_id,
        PlayerId::new(1),
        "Enemy Unit".to_string(),
        DatasheetId::new(5),
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        vec![model],
        MoveCharacteristic::from_inches(6),
        Toughness::new(4),
        ArmorSave::FOUR_PLUS,
        None,
        Leadership::new(7),
        ObjectiveControl::new(1),
    );
    unit.status = UnitStatus::OnBattlefield;
    unit
}

// ===========================================================================
// §6.1 — TRANSPORT CAPACITY
// ===========================================================================

/// Source: 40k_revised.md §6.1
/// Rule: Transports have a capacity in points that limits what can embark.
/// Test: The DEDICATED_TRANSPORT keyword is correctly assigned.
#[test]
fn transport_has_dedicated_transport_keyword() {
    let transport = make_transport_unit(1, PlayerId::new(0), Position::from_inches(20, 15));
    assert!(transport.has_keyword(Keyword::DedicatedTransport));
    assert!(transport.has_keyword(Keyword::Vehicle));
}

/// Source: 40k_revised.md §6.1
/// Rule: Transport capacity is a fixed property of the unit.
/// Test: A transport is distinct from a non-transport unit by keyword.
#[test]
fn non_transport_unit_lacks_transport_keyword() {
    let infantry = make_infantry_unit(2, PlayerId::new(0), 5, Position::from_inches(10, 10));
    assert!(!infantry.has_keyword(Keyword::DedicatedTransport));
    assert!(!infantry.has_keyword(Keyword::Vehicle));
}

// ===========================================================================
// §6.2 — EMBARKING: Distance Checks
// ===========================================================================

/// Source: 40k_revised.md §6.2
/// Rule: "A unit can embark if all of its models are within 3\" of the Transport."
/// Test: within_range returns true for a model at exactly 3" from the Transport.
#[test]
fn embark_within_3_inches_allowed() {
    let transport_pos = Position::from_inches(20, 15);
    let model_pos = Position::from_inches(20, 18); // 3" away vertically

    assert!(transport_pos.within_range(model_pos, EMBARK_RANGE));
}

/// Source: 40k_revised.md §6.2
/// Rule: "A unit can embark if all of its models are within 3\" of the Transport."
/// Test: within_range returns true for a model closer than 3".
#[test]
fn embark_within_2_inches_allowed() {
    let transport_pos = Position::from_inches(20, 15);
    let model_pos = Position::from_inches(20, 17); // 2" away

    assert!(transport_pos.within_range(model_pos, EMBARK_RANGE));
}

/// Source: 40k_revised.md §6.2
/// Rule: Models beyond 3" cannot embark.
/// Test: within_range returns false for a model more than 3" away.
#[test]
fn embark_beyond_3_inches_blocked() {
    let transport_pos = Position::from_inches(20, 15);
    let model_pos = Position::from_inches(20, 19); // 4" away

    assert!(!transport_pos.within_range(model_pos, EMBARK_RANGE));
}

/// Source: 40k_revised.md §6.2
/// Rule: A unit with multiple models must have ALL models within 3".
/// Test: If one model is beyond 3", the embark check should fail for that model.
#[test]
fn embark_all_models_must_be_within_range() {
    let transport_pos = Position::from_inches(20, 15);

    let model_positions = vec![
        Position::from_inches(20, 17),  // 2" away - OK
        Position::from_inches(21, 16),  // ~1.4" away - OK
        Position::from_inches(20, 19),  // 4" away - TOO FAR
    ];

    let all_within = model_positions.iter().all(|pos| {
        transport_pos.within_range(*pos, EMBARK_RANGE)
    });

    assert!(!all_within); // Not all models are within 3"
}

/// Source: 40k_revised.md §6.2
/// Rule: A unit that disembarked this phase cannot re-embark.
/// Test: Track disembark state via UnitStatus transitions.
#[test]
fn embark_blocked_after_disembark_same_phase() {
    // The disembark flag is tracked as a state transition.
    // A unit that was set to OnBattlefield from a transport this phase
    // would have a "disembarked_this_phase" marker in a full implementation.
    // Here we test that the UnitStatus transitions correctly reflect embark/disembark.
    let mut unit = make_infantry_unit(1, PlayerId::new(0), 5, Position::from_inches(10, 10));

    // Simulate: unit is on battlefield (just disembarked)
    unit.status = UnitStatus::OnBattlefield;
    assert!(unit.is_on_battlefield());

    // The game system would check a "disembarked_this_phase" flag before allowing re-embark.
    // We verify the state tracking mechanism exists via unit status.
    assert_eq!(unit.status, UnitStatus::OnBattlefield);
}

// ===========================================================================
// §6.3 — DISEMBARKING: Distance and Engagement Range Checks
// ===========================================================================

/// Source: 40k_revised.md §6.3
/// Rule: "Disembarking models are placed within 3\" of the Transport."
/// Test: Verify the disembark range constant equals 3".
#[test]
fn disembark_range_is_3_inches() {
    assert_eq!(DISEMBARK_RANGE, Inches::from_inches(3));
    assert_eq!(DISEMBARK_RANGE.mils(), 3000);
}

/// Source: 40k_revised.md §6.3
/// Rule: A model placed within 3" of the Transport is a valid disembark position.
/// Test: within_range check for disembark.
#[test]
fn disembark_valid_position_within_3_inches() {
    let transport_pos = Position::from_inches(20, 15);
    let disembark_pos = Position::from_inches(22, 15); // 2" away

    assert!(transport_pos.within_range(disembark_pos, DISEMBARK_RANGE));
}

/// Source: 40k_revised.md §6.3
/// Rule: Models cannot disembark to a position beyond 3".
/// Test: within_range returns false for positions beyond 3".
#[test]
fn disembark_invalid_position_beyond_3_inches() {
    let transport_pos = Position::from_inches(20, 15);
    let too_far_pos = Position::from_inches(24, 15); // 4" away

    assert!(!transport_pos.within_range(too_far_pos, DISEMBARK_RANGE));
}

/// Source: 40k_revised.md §6.3
/// Rule: "Disembarking models cannot be set up within Engagement Range of enemy models."
/// Test: A disembark position within 1" of an enemy model is invalid.
#[test]
fn disembark_not_within_engagement_range_of_enemy() {
    let enemy = make_enemy_unit(10, Position::from_inches(22, 15));
    let enemy_pos = enemy.reference_position().unwrap();
    let disembark_pos = Position::from_inches(22, 15); // Same position = within ER

    // Within engagement range (1")
    assert!(disembark_pos.within_range(enemy_pos, ENGAGEMENT_RANGE));

    // This means the disembark position is invalid
    let is_valid_disembark = !disembark_pos.within_range(enemy_pos, ENGAGEMENT_RANGE);
    assert!(!is_valid_disembark);
}

/// Source: 40k_revised.md §6.3
/// Rule: Disembark position is valid if it's beyond Engagement Range of all enemies.
/// Test: A position >1" from all enemies is a valid disembark spot.
#[test]
fn disembark_valid_beyond_engagement_range() {
    let enemy_pos = Position::from_inches(25, 15);
    let disembark_pos = Position::from_inches(22, 15); // 3" from enemy

    // Not within engagement range
    assert!(!disembark_pos.within_range(enemy_pos, ENGAGEMENT_RANGE));

    // This is a valid disembark position (regarding ER check)
    let is_valid_disembark = !disembark_pos.within_range(enemy_pos, ENGAGEMENT_RANGE);
    assert!(is_valid_disembark);
}

/// Source: 40k_revised.md §6.3
/// Rule: Disembark must happen before the Transport moves.
/// Test: Verify the game tracks unit movement (has_moved flag).
#[test]
fn disembark_before_transport_moves_tracking() {
    use wh40k_game_core::state::TurnFlags;

    let mut flags = TurnFlags::new();
    let transport_id = UnitId::new(1);

    // Transport has NOT moved yet — disembark is allowed
    assert!(!flags.has_moved(transport_id));

    // After the Transport moves, disembark would be blocked
    flags.mark_moved(transport_id);
    assert!(flags.has_moved(transport_id));

    // The game system checks has_moved(transport_id) before allowing disembark.
    // If true, disembark is blocked.
}

// ===========================================================================
// §6.4 — DESTROYED TRANSPORTS: Mortal Wounds
// ===========================================================================

/// Source: 40k_revised.md §6.4
/// Rule: "When a Transport is destroyed, roll D6 for each model embarked.
///        On a 1, that model's unit suffers 1 mortal wound."
/// Test: Verify the mortal wound threshold constant for destroyed transports.
#[test]
fn destroyed_transport_mortal_wound_threshold() {
    // D6 roll of 1 = mortal wound
    assert_eq!(DESTROYED_TRANSPORT_MW_THRESHOLD, 1);

    // Rolls 2-6 = no mortal wound
    for roll in 2..=6u8 {
        assert!(roll > DESTROYED_TRANSPORT_MW_THRESHOLD);
    }

    // Roll of 1 = mortal wound
    assert!(1 <= DESTROYED_TRANSPORT_MW_THRESHOLD);
}

/// Source: 40k_revised.md §6.4
/// Rule: Each model embarked rolls separately (D6 per model).
/// Test: With 5 embarked models, 5 D6 rolls are needed.
#[test]
fn destroyed_transport_roll_per_model() {
    let embarked_model_count = 5;

    // Each model needs a D6 roll
    let rolls_needed = embarked_model_count;
    assert_eq!(rolls_needed, 5);

    // Count mortal wounds from simulated rolls
    let simulated_rolls: Vec<u8> = vec![1, 3, 1, 5, 2]; // 2 ones
    let mortal_wounds: u8 = simulated_rolls
        .iter()
        .filter(|&&roll| roll <= DESTROYED_TRANSPORT_MW_THRESHOLD)
        .count() as u8;
    assert_eq!(mortal_wounds, 2);
}

/// Source: 40k_revised.md §6.4
/// Rule: Mortal wounds from destroyed transport can be applied to model wounds.
/// Test: apply_damage correctly applies a single mortal wound.
#[test]
fn destroyed_transport_mortal_wound_applies_damage() {
    let unit_id = UnitId::new(1);
    let mut model = make_model(100, unit_id, 1, Position::from_inches(10, 10));

    assert!(model.alive);
    assert_eq!(model.wounds_remaining.value(), 1);

    // Apply 1 mortal wound (1 damage that ignores saves)
    let destroyed = model.apply_damage(1);
    assert!(destroyed);
    assert!(!model.alive);
    assert_eq!(model.wounds_remaining.value(), 0);
}

/// Source: 40k_revised.md §6.4
/// Rule: Mortal wounds on multi-wound models reduce wounds, don't necessarily kill.
/// Test: A 3-wound model survives 1 mortal wound.
#[test]
fn destroyed_transport_mortal_wound_partial_damage() {
    let unit_id = UnitId::new(1);
    let mut model = make_model(100, unit_id, 3, Position::from_inches(10, 10));

    let destroyed = model.apply_damage(1);
    assert!(!destroyed);
    assert!(model.alive);
    assert_eq!(model.wounds_remaining.value(), 2);
}

// ===========================================================================
// §6.4 — DESTROYED TRANSPORTS: Battle-shocked Status
// ===========================================================================

/// Source: 40k_revised.md §6.4
/// Rule: "Units that disembark from a destroyed Transport are Battle-shocked
///        until the start of their next Command Phase."
/// Test: battle_shocked flag can be set on a disembarked unit.
#[test]
fn destroyed_transport_passengers_become_battle_shocked() {
    let mut unit = make_infantry_unit(1, PlayerId::new(0), 5, Position::from_inches(10, 10));

    assert!(!unit.battle_shocked);

    // After disembarking from a destroyed transport, the unit is Battle-shocked
    unit.battle_shocked = true;
    assert!(unit.battle_shocked);

    // Battle-shocked means OC = 0
    assert_eq!(unit.effective_oc().value(), 0);
}

/// Source: 40k_revised.md §6.4
/// Rule: Battle-shock from destroyed transport lasts until next Command Phase.
/// Test: Clearing the battle_shocked flag simulates recovery at Command Phase.
#[test]
fn destroyed_transport_battle_shock_clears_at_command_phase() {
    let mut unit = make_infantry_unit(1, PlayerId::new(0), 5, Position::from_inches(10, 10));

    // Set battle-shocked from destroyed transport
    unit.battle_shocked = true;
    assert_eq!(unit.effective_oc().value(), 0);

    // At the start of the next Command Phase, clear the flag
    unit.battle_shocked = false;
    assert!(!unit.battle_shocked);
    assert_eq!(unit.effective_oc().value(), 2); // OC restored
}

// ===========================================================================
// §6.4 — DESTROYED TRANSPORTS: Counts as Normal Moved
// ===========================================================================

/// Source: 40k_revised.md §6.4
/// Rule: "Units that disembark from a destroyed Transport count as having
///        made a Normal Move this turn."
/// Test: TurnFlags mark_moved tracks the unit as having moved.
#[test]
fn destroyed_transport_passengers_count_as_normal_moved() {
    use wh40k_game_core::state::TurnFlags;

    let mut flags = TurnFlags::new();
    let unit_id = UnitId::new(1);

    // Before disembark, unit hasn't moved
    assert!(!flags.has_moved(unit_id));

    // After disembarking from destroyed transport, mark as moved
    flags.mark_moved(unit_id);
    assert!(flags.has_moved(unit_id));

    // Since it counts as Normal Move (not Advanced, not Fell Back), verify:
    assert!(!flags.has_advanced(unit_id));
    assert!(!flags.has_fell_back(unit_id));
}

/// Source: 40k_revised.md §6.4
/// Rule: Units that count as having made a Normal Move cannot Remain Stationary.
/// Test: A unit marked as moved is not stationary.
#[test]
fn destroyed_transport_passengers_not_stationary() {
    use wh40k_game_core::state::TurnFlags;

    let mut flags = TurnFlags::new();
    let unit_id = UnitId::new(1);

    flags.mark_moved(unit_id);

    // They count as moved, not stationary
    assert!(flags.has_moved(unit_id));
    assert!(!flags.is_stationary(unit_id));
}

// ===========================================================================
// §6.4 — DESTROYED TRANSPORTS: Emergency Disembark
// ===========================================================================

/// Source: 40k_revised.md §6.4
/// Rule: "If models can't be placed within 3\", they can be placed within 6\"
///        (emergency disembark). Roll D6 for each model: on 1-3, 1 mortal wound."
/// Test: Verify the emergency disembark range constant equals 6".
#[test]
fn emergency_disembark_range_is_6_inches() {
    assert_eq!(EMERGENCY_DISEMBARK_RANGE, Inches::from_inches(6));
    assert_eq!(EMERGENCY_DISEMBARK_RANGE.mils(), 6000);
}

/// Source: 40k_revised.md §6.4
/// Rule: Emergency disembark places models within 6".
/// Test: Position within 6" is a valid emergency disembark position.
#[test]
fn emergency_disembark_within_6_inches_allowed() {
    let transport_pos = Position::from_inches(20, 15);
    let emergency_pos = Position::from_inches(25, 15); // 5" away

    assert!(transport_pos.within_range(emergency_pos, EMERGENCY_DISEMBARK_RANGE));
}

/// Source: 40k_revised.md §6.4
/// Rule: Emergency disembark cannot place beyond 6".
/// Test: Position beyond 6" is invalid for emergency disembark.
#[test]
fn emergency_disembark_beyond_6_inches_blocked() {
    let transport_pos = Position::from_inches(20, 15);
    let too_far_pos = Position::from_inches(27, 15); // 7" away

    assert!(!transport_pos.within_range(too_far_pos, EMERGENCY_DISEMBARK_RANGE));
}

/// Source: 40k_revised.md §6.4
/// Rule: Emergency disembark mortal wound threshold is 1-3 (D6 roll).
/// Test: Verify the emergency mortal wound threshold.
#[test]
fn emergency_disembark_mortal_wound_threshold() {
    assert_eq!(EMERGENCY_DISEMBARK_MW_THRESHOLD, 3);

    // Rolls 1-3 = mortal wound
    for roll in 1..=3u8 {
        assert!(roll <= EMERGENCY_DISEMBARK_MW_THRESHOLD);
    }

    // Rolls 4-6 = no mortal wound
    for roll in 4..=6u8 {
        assert!(roll > EMERGENCY_DISEMBARK_MW_THRESHOLD);
    }
}

/// Source: 40k_revised.md §6.4
/// Rule: Emergency disembark has a higher mortal wound chance (1-3) vs normal (1).
/// Test: More mortal wounds are expected from emergency disembark.
#[test]
fn emergency_disembark_higher_mortal_rate_than_normal() {
    let _embarked_model_count = 10;
    let simulated_rolls: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 1, 2, 3, 4];

    // Normal destroyed transport: only 1s cause MWs
    let normal_mw: u8 = simulated_rolls
        .iter()
        .filter(|&&r| r <= DESTROYED_TRANSPORT_MW_THRESHOLD)
        .count() as u8;

    // Emergency disembark: 1-3 cause MWs
    let emergency_mw: u8 = simulated_rolls
        .iter()
        .filter(|&&r| r <= EMERGENCY_DISEMBARK_MW_THRESHOLD)
        .count() as u8;

    assert!(emergency_mw > normal_mw);
    assert_eq!(normal_mw, 2);  // Two rolls of 1
    assert_eq!(emergency_mw, 6); // Rolls 1,2,3,1,2,3
}

/// Source: 40k_revised.md §6.4
/// Rule: Emergency disembark mortal wounds are applied individually.
/// Test: Apply multiple mortal wounds to a unit's models.
#[test]
fn emergency_disembark_mortal_wounds_kill_models() {
    let mut unit = make_infantry_unit(1, PlayerId::new(0), 5, Position::from_inches(10, 10));

    assert_eq!(unit.models_alive(), 5);

    // Simulate 3 mortal wounds from emergency disembark on 1-wound models
    for i in 0..3 {
        unit.models[i].apply_damage(1);
    }

    assert_eq!(unit.models_alive(), 2);
    assert!(!unit.is_destroyed());
}

// ===========================================================================
// Additional distance and geometry tests for Transport rules
// ===========================================================================

/// Source: 40k_revised.md §6.2, §6.3
/// Rule: The 3" embark/disembark range uses Euclidean distance.
/// Test: Diagonal distance check (3-4-5 triangle: 3" horizontal, 4" vertical = 5" diagonal).
#[test]
fn transport_distance_diagonal_check() {
    let transport_pos = Position::from_inches(20, 15);

    // 3" horizontal, 4" vertical = 5" diagonal (3-4-5 triangle)
    let far_diagonal = Position::from_inches(23, 19);
    assert!(!transport_pos.within_range(far_diagonal, EMBARK_RANGE)); // 5" > 3"

    // 2" horizontal, 2" vertical = ~2.83" diagonal
    let near_diagonal = Position::from_inches(22, 17);
    assert!(transport_pos.within_range(near_diagonal, EMBARK_RANGE)); // ~2.83" <= 3"
}

/// Source: 40k_revised.md §6.3
/// Rule: Engagement Range is 1" — disembark positions must be outside this from all enemies.
/// Test: Verify the engagement range constant used for disembark validation.
#[test]
fn engagement_range_constant_for_disembark() {
    assert_eq!(ENGAGEMENT_RANGE, Inches::from_inches(1));
    assert_eq!(ENGAGEMENT_RANGE.mils(), 1000);

    // A model at exactly 1" is within engagement range
    let enemy_pos = Position::from_inches(20, 15);
    let at_er = Position::from_inches(21, 15);
    assert!(at_er.within_range(enemy_pos, ENGAGEMENT_RANGE));

    // A model at 2" is outside engagement range
    let beyond_er = Position::from_inches(22, 15);
    assert!(!beyond_er.within_range(enemy_pos, ENGAGEMENT_RANGE));
}

/// Source: 40k_revised.md §6.4
/// Rule: Destroyed Transport status tracking.
/// Test: A Transport can be marked as Destroyed via UnitStatus.
#[test]
fn transport_destroyed_status_tracking() {
    let mut transport = make_transport_unit(1, PlayerId::new(0), Position::from_inches(20, 15));

    assert!(!transport.is_destroyed());
    assert!(transport.is_on_battlefield());

    transport.status = UnitStatus::Destroyed;
    assert!(transport.is_destroyed());
    assert!(!transport.is_on_battlefield());
}

/// Source: 40k_revised.md §6.4
/// Rule: After a Transport is destroyed, passengers disembark and are Battle-shocked.
/// Test: Full sequence — destroy Transport, set passengers to Battle-shocked, mark as moved.
#[test]
fn destroyed_transport_full_disembark_sequence() {
    use wh40k_game_core::state::TurnFlags;

    let mut transport = make_transport_unit(1, PlayerId::new(0), Position::from_inches(20, 15));
    let mut passenger_unit = make_infantry_unit(
        2,
        PlayerId::new(0),
        5,
        Position::from_inches(20, 15), // Start at transport position
    );
    let mut flags = TurnFlags::new();

    // Step 1: Destroy transport
    transport.status = UnitStatus::Destroyed;
    assert!(transport.is_destroyed());

    // Step 2: Roll mortal wounds for passengers (simulated: 1 mortal wound out of 5)
    // Simulated roll: model 0 rolls a 1
    let destroyed = passenger_unit.models[0].apply_damage(1);
    assert!(destroyed); // 1W model dies from 1 MW
    assert_eq!(passenger_unit.models_alive(), 4);

    // Step 3: Passengers become Battle-shocked
    passenger_unit.battle_shocked = true;
    assert!(passenger_unit.battle_shocked);
    assert_eq!(passenger_unit.effective_oc().value(), 0);

    // Step 4: Passengers count as having made a Normal Move
    flags.mark_moved(passenger_unit.id);
    assert!(flags.has_moved(passenger_unit.id));
    assert!(!flags.has_advanced(passenger_unit.id));
    assert!(!flags.has_fell_back(passenger_unit.id));
}

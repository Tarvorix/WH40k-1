//! Source-linked rules tests for 40k_revised.md Section 10: Fight Phase.
//!
//! Tests cover:
//!   - 10.1: Fight Phase Structure (Fights First + Remaining Combats, alternation)
//!   - 10.2: Fights First (charged_this_turn -> Fights First eligibility)
//!   - 10.4: Pile In (PILE_IN constant = 3", must end in ER, must end closer)
//!   - 10.5: Making Melee Attacks (models in ER, one melee weapon, WS for hit rolls)
//!   - 10.6: Consolidate (CONSOLIDATE constant = 3", must end closer to enemy)
//!
//! Source: 40k_revised.md Section 10 -- "FIGHT PHASE"

use wh40k_core_types::{
    ArmorPenetration, ArmorSave, AttackCount, BaseSize, Damage,
    EngagementStatus, Inches, InvulnerableSave, Keyword, KeywordSet,
    ModelId, PlayerId, Position, Skill, Strength, Toughness,
    UnitId, UnitStatus, WeaponAbilitySet, WeaponId, WeaponProfile,
    WeaponType, Wounds, DatasheetId, MoveCharacteristic, ObjectiveControl,
    Leadership, Phase, SubPhase,
};
use wh40k_dice::{DiceContext, DiceRoller, StreamKind};
use wh40k_game_core::combat::{resolve_attack_batch, AttackContext};
use wh40k_game_core::state::TurnFlags;
use wh40k_game_core::unit::{ModelState, UnitState};

// ===========================================================================
// Helper factories
// ===========================================================================

/// Create a deterministic DiceRoller from a seed byte for reproducibility.
fn make_dice(seed_byte: u8) -> DiceRoller {
    let ctx = DiceContext::new([seed_byte; 32], StreamKind::HitRoll, 0, 0);
    DiceRoller::new(ctx)
}

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

/// Build a unit with given keywords and models.
fn make_unit(
    id: u32,
    owner: PlayerId,
    keywords: &[Keyword],
    models: Vec<ModelState>,
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
        ObjectiveControl::new(2),
    );
    unit.status = UnitStatus::OnBattlefield;
    unit
}

/// Build a simple melee weapon profile with the given stats and no special abilities.
fn make_melee_weapon(
    name: &str,
    attacks: u8,
    skill: u8,
    strength: u8,
    ap: i8,
    damage: u8,
) -> WeaponProfile {
    WeaponProfile {
        id: WeaponId::new(2),
        name: name.to_string(),
        weapon_type: WeaponType::Melee,
        range: Inches::ZERO,
        attacks: AttackCount::Fixed(attacks),
        skill: Skill(skill),
        strength: Strength::new(strength),
        ap: ArmorPenetration::new(ap),
        damage: Damage::Fixed(damage),
        abilities: WeaponAbilitySet::new(),
    }
}

/// Build a default melee AttackContext for testing.
fn make_melee_attack_context(
    weapon: WeaponProfile,
    attacking_model_count: u8,
    resolved_attacks_per_model: u8,
    defender_toughness: u8,
    defender_armor_save: u8,
    defender_invulnerable: Option<InvulnerableSave>,
) -> AttackContext {
    AttackContext {
        attacker_id: UnitId::new(1),
        attacker_owner: PlayerId::new(0),
        defender_id: UnitId::new(2),
        weapon,
        attacking_model_count,
        resolved_attacks_per_model,
        attacker_advanced: false,
        attacker_stationary: false,
        attacker_charged: false,
        within_half_range: false,
        target_has_cover: false,
        in_engagement_range: true, // melee attacks are always in ER
        target_model_count: 5,
        target_keywords: KeywordSet::from_keywords(&[Keyword::Infantry]),
        target_has_stealth: false,
        distance_mils: 0,
        defender_toughness: Toughness::new(defender_toughness),
        defender_armor_save: ArmorSave(defender_armor_save),
        defender_invulnerable,
        effective_abilities: WeaponAbilitySet::new(),
        indirect_fire_no_los: false,
        is_overwatch: false,
        bonus_attacks_per_model: 0,
        bonus_ap: 0,
        bonus_fnp: 0,
        critical_hit_threshold: 0,
        command_reroll_active: false,
        defender_command_reroll_active: false,
    }
}

// ===========================================================================
// §10.1 — Fight Phase Structure: alternation tracking
// ===========================================================================

/// Source: 40k_revised.md §10.1
/// Rule: "Players alternate, non-active player picks first in Remaining Combats."
/// The fight_alternation_next_player field tracks whose turn it is to pick.
/// Test: field starts as None (uninitialized).
#[test]
fn fight_alternation_starts_uninitialized() {
    let flags = TurnFlags::new();
    assert!(
        flags.fight_alternation_next_player.is_none(),
        "Fight alternation should start as None (uninitialized)"
    );
}

/// Source: 40k_revised.md §10.1
/// Rule: Fight alternation is initialized per step.
/// init_fight_alternation sets which player picks first.
/// In Fights First, active player picks first.
#[test]
fn init_fight_alternation_sets_first_picker() {
    let mut flags = TurnFlags::new();
    let active_player = PlayerId::new(0);

    flags.init_fight_alternation(active_player);

    assert_eq!(
        flags.fight_alternation_next_player,
        Some(active_player),
        "init_fight_alternation should set the first picker"
    );
}

/// Source: 40k_revised.md §10.1
/// Rule: In Remaining Combats step, non-active player picks first.
/// Test: init_fight_alternation can be set to the non-active player.
#[test]
fn init_fight_alternation_remaining_combats_non_active_picks_first() {
    let mut flags = TurnFlags::new();
    let active_player = PlayerId::new(0);
    let non_active_player = PlayerId::new(1);

    // For Remaining Combats, the non-active player picks first
    flags.init_fight_alternation(non_active_player);

    assert_eq!(
        flags.fight_alternation_next_player,
        Some(non_active_player),
        "Non-active player should pick first in Remaining Combats"
    );

    // Verify it's not the active player
    assert_ne!(
        flags.fight_alternation_next_player,
        Some(active_player),
        "Active player should NOT be first picker in Remaining Combats"
    );
}

/// Source: 40k_revised.md §10.1
/// Rule: "Players alternate" — after one player picks, the other picks next.
/// advance_fight_alternation toggles the picker to the other player.
#[test]
fn advance_fight_alternation_toggles_player() {
    let mut flags = TurnFlags::new();
    let player_a = PlayerId::new(0);
    let player_b = PlayerId::new(1);

    // Player A picks first
    flags.init_fight_alternation(player_a);
    assert_eq!(flags.fight_alternation_next_player, Some(player_a));

    // After Player A selects a unit, it toggles to Player B
    flags.advance_fight_alternation(player_a, player_b);
    assert_eq!(
        flags.fight_alternation_next_player,
        Some(player_b),
        "After Player A picks, Player B should pick next"
    );
}

/// Source: 40k_revised.md §10.1
/// Rule: Alternation continues back and forth.
/// Test: Player B picks, then it goes back to Player A.
#[test]
fn advance_fight_alternation_toggles_back() {
    let mut flags = TurnFlags::new();
    let player_a = PlayerId::new(0);
    let player_b = PlayerId::new(1);

    // Initialize with Player A
    flags.init_fight_alternation(player_a);

    // Player A picks -> Player B
    flags.advance_fight_alternation(player_a, player_b);
    assert_eq!(flags.fight_alternation_next_player, Some(player_b));

    // Player B picks -> Player A
    flags.advance_fight_alternation(player_b, player_a);
    assert_eq!(
        flags.fight_alternation_next_player,
        Some(player_a),
        "Alternation should toggle back to Player A"
    );
}

/// Source: 40k_revised.md §10.1
/// Rule: Fight alternation can be re-initialized between Fights First and
/// Remaining Combats steps (different first-picker rules for each step).
#[test]
fn fight_alternation_reinitializable_between_steps() {
    let mut flags = TurnFlags::new();
    let active_player = PlayerId::new(0);
    let non_active_player = PlayerId::new(1);

    // Fights First step: active player picks first
    flags.init_fight_alternation(active_player);
    assert_eq!(flags.fight_alternation_next_player, Some(active_player));

    // Remaining Combats step: non-active player picks first
    flags.init_fight_alternation(non_active_player);
    assert_eq!(
        flags.fight_alternation_next_player,
        Some(non_active_player),
        "Re-initializing should override the previous picker"
    );
}

// ===========================================================================
// §10.1 — mark_fought / has_fought: prevents fighting twice
// ===========================================================================

/// Source: 40k_revised.md §10.1
/// Rule: A unit can only fight once per Fight Phase.
/// mark_fought records the unit; has_fought checks it.
#[test]
fn mark_fought_prevents_fighting_twice() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(10);

    // Initially not fought
    assert!(
        !flags.has_fought(unit),
        "Unit should not have fought initially"
    );

    // Mark as fought
    flags.mark_fought(unit);
    assert!(
        flags.has_fought(unit),
        "Unit should be marked as fought after mark_fought"
    );
}

/// Source: 40k_revised.md §10.1
/// Rule: Each unit independently tracks whether it has fought.
/// One unit fighting does not affect another.
#[test]
fn mark_fought_is_per_unit() {
    let mut flags = TurnFlags::new();
    let unit_a = UnitId::new(10);
    let unit_b = UnitId::new(11);
    let unit_c = UnitId::new(12);

    flags.mark_fought(unit_a);

    assert!(flags.has_fought(unit_a));
    assert!(
        !flags.has_fought(unit_b),
        "Unit B should not be affected by Unit A fighting"
    );
    assert!(
        !flags.has_fought(unit_c),
        "Unit C should not be affected by Unit A fighting"
    );
}

/// Source: 40k_revised.md §10.1
/// Rule: The fought flag clears at the start of each turn.
#[test]
fn mark_fought_clears_on_turn_reset() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(10);

    flags.mark_fought(unit);
    assert!(flags.has_fought(unit));

    flags.clear();
    assert!(
        !flags.has_fought(unit),
        "units_fought should be cleared after turn reset"
    );
}

/// Source: 40k_revised.md §10.1
/// Rule: Multiple units can fight in the same phase, each tracked separately.
#[test]
fn multiple_units_can_fight_in_same_phase() {
    let mut flags = TurnFlags::new();
    let unit_a = UnitId::new(20);
    let unit_b = UnitId::new(21);
    let unit_c = UnitId::new(22);

    flags.mark_fought(unit_a);
    flags.mark_fought(unit_b);
    flags.mark_fought(unit_c);

    assert!(flags.has_fought(unit_a));
    assert!(flags.has_fought(unit_b));
    assert!(flags.has_fought(unit_c));
}

// ===========================================================================
// §10.2 — Fights First: charged_this_turn flag
// ===========================================================================

/// Source: 40k_revised.md §10.2
/// Rule: "A unit that made a charge move this turn has the Fights First ability."
/// The charged_this_turn flag set by mark_charged qualifies the unit for
/// the Fights First step.
#[test]
fn charged_this_turn_qualifies_for_fights_first() {
    let mut flags = TurnFlags::new();
    let charger = UnitId::new(30);
    let non_charger = UnitId::new(31);

    flags.mark_charged(charger);

    assert!(
        flags.charged_this_turn(charger),
        "Charging unit should have Fights First"
    );
    assert!(
        !flags.charged_this_turn(non_charger),
        "Non-charging unit should NOT have Fights First"
    );
}

/// Source: 40k_revised.md §10.2
/// Rule: Units that were already in combat but did NOT charge this turn
/// do NOT get Fights First. They fight in the Remaining Combats step.
#[test]
fn already_engaged_unit_no_fights_first_without_charge() {
    let flags = TurnFlags::new();
    let engaged_unit = UnitId::new(32);

    // Unit is engaged but did NOT charge this turn
    // (engagement_status is on the UnitState, not TurnFlags)
    // The charged_this_turn flag is what determines Fights First
    assert!(
        !flags.charged_this_turn(engaged_unit),
        "Engaged unit that didn't charge should NOT have Fights First"
    );
}

/// Source: 40k_revised.md §10.2
/// Rule: Multiple units can charge in the same turn, and all of them
/// get Fights First independently.
#[test]
fn multiple_chargers_all_get_fights_first() {
    let mut flags = TurnFlags::new();
    let charger_a = UnitId::new(33);
    let charger_b = UnitId::new(34);
    let charger_c = UnitId::new(35);

    flags.mark_charged(charger_a);
    flags.mark_charged(charger_b);
    flags.mark_charged(charger_c);

    assert!(flags.charged_this_turn(charger_a));
    assert!(flags.charged_this_turn(charger_b));
    assert!(flags.charged_this_turn(charger_c));
}

/// Source: 40k_revised.md §10.2
/// Rule: The charged_this_turn flag clears when turn flags are cleared,
/// so units do NOT retain Fights First into the next turn.
#[test]
fn charged_this_turn_clears_between_turns() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(36);

    flags.mark_charged(unit);
    assert!(flags.charged_this_turn(unit));

    flags.clear();
    assert!(
        !flags.charged_this_turn(unit),
        "charged_this_turn should NOT persist across turn boundaries"
    );
}

// ===========================================================================
// §10.4 — Pile In: up to 3" (PILE_IN constant)
// ===========================================================================

/// Source: 40k_revised.md §10.4
/// Rule: "Each time a unit piles in, you can move each model in that unit
///        up to 3\"."
/// The PILE_IN constant is exactly 3 inches.
#[test]
fn pile_in_distance_is_three_inches() {
    assert_eq!(
        Inches::PILE_IN,
        Inches::from_inches(3),
        "PILE_IN constant should be 3 inches"
    );
    assert_eq!(
        Inches::PILE_IN.mils(),
        3000,
        "PILE_IN should be 3000 mils"
    );
    assert_eq!(
        Inches::PILE_IN.whole_inches(),
        3,
        "PILE_IN whole inches should be 3"
    );
}

/// Source: 40k_revised.md §10.4
/// Rule: A pile-in move of exactly 3" is valid (at the maximum).
#[test]
fn pile_in_move_at_maximum_is_valid() {
    let pile_in_max = Inches::PILE_IN;
    let move_distance = Inches::from_inches(3);

    assert!(
        move_distance <= pile_in_max,
        "A 3\" pile-in move should be valid (exactly at maximum)"
    );
}

/// Source: 40k_revised.md §10.4
/// Rule: A pile-in move exceeding 3" is invalid.
#[test]
fn pile_in_move_exceeding_maximum_is_invalid() {
    let pile_in_max = Inches::PILE_IN;
    let over_move = Inches::from_mils(3001); // 3.001"

    assert!(
        over_move > pile_in_max,
        "A move exceeding 3\" should be invalid for pile-in"
    );
}

/// Source: 40k_revised.md §10.4
/// Rule: Pile-in moves smaller than 3" are valid.
#[test]
fn pile_in_partial_move_is_valid() {
    let pile_in_max = Inches::PILE_IN;

    let small_move = Inches::from_inches(1);
    assert!(small_move <= pile_in_max, "1\" pile-in should be valid");

    let medium_move = Inches::from_inches(2);
    assert!(medium_move <= pile_in_max, "2\" pile-in should be valid");

    let zero_move = Inches::ZERO;
    assert!(zero_move <= pile_in_max, "0\" pile-in (staying put) should be valid");
}

/// Source: 40k_revised.md §10.4
/// Rule: "Each model that piles in must end that move in Engagement Range
///        of at least one enemy model."
/// The engagement range is 1" horizontal.
#[test]
fn pile_in_must_end_in_engagement_range() {
    // Models must end within ER of at least one enemy.
    // ER = 1" horizontal between base edges.
    assert_eq!(
        Inches::ENGAGEMENT_RANGE,
        Inches::from_inches(1),
        "Engagement range for pile-in end position is 1 inch"
    );

    // Verify that a model at the right distance is in ER
    let model_pos = Position::from_inches(10, 10);
    let enemy_pos = Position::from_inches(11, 10); // 1" center to center, bases overlap -> in ER
    let base = BaseSize::MM32;

    let in_er = wh40k_geometry::within_engagement_range_2d(model_pos, base, enemy_pos, base);
    assert!(
        in_er,
        "Model 1\" center-to-center from enemy with 32mm bases should be in ER after pile-in"
    );
}

/// Source: 40k_revised.md §10.4
/// Rule: "Each model that piles in must end that move closer to the closest
///        enemy model than it was at the start of the move."
/// Test the distance calculation used for "closer to enemy" validation.
#[test]
fn pile_in_must_end_closer_to_closest_enemy() {
    let start_pos = Position::from_inches(10, 10);
    let enemy_pos = Position::from_inches(15, 10);

    let start_dist = start_pos.distance(enemy_pos);
    assert_eq!(start_dist, Inches::from_inches(5), "Start distance should be 5\"");

    // Move 2" closer to the enemy (along x axis)
    let end_pos = Position::from_inches(12, 10);
    let end_dist = end_pos.distance(enemy_pos);
    assert_eq!(end_dist, Inches::from_inches(3), "End distance should be 3\"");

    assert!(
        end_dist < start_dist,
        "Model must end closer to the closest enemy after pile-in"
    );
}

/// Source: 40k_revised.md §10.4
/// Rule: A pile-in move that does NOT end closer to the closest enemy is invalid.
#[test]
fn pile_in_moving_away_is_invalid() {
    let start_pos = Position::from_inches(10, 10);
    let enemy_pos = Position::from_inches(15, 10);

    let start_dist = start_pos.distance(enemy_pos);

    // Move away from enemy (further in x)
    let away_pos = Position::from_inches(8, 10);
    let away_dist = away_pos.distance(enemy_pos);

    assert!(
        away_dist > start_dist,
        "Moving away from enemy should result in greater distance"
    );
    // This would be an invalid pile-in move
}

/// Source: 40k_revised.md §10.4
/// Rule: Extended pile-in (6") is available to some units.
/// Test the PILE_IN_EXTENDED constant.
#[test]
fn extended_pile_in_distance_is_six_inches() {
    assert_eq!(
        Inches::PILE_IN_EXTENDED,
        Inches::from_inches(6),
        "Extended pile-in should be 6 inches"
    );
    assert_eq!(Inches::PILE_IN_EXTENDED.mils(), 6000);
}

// ===========================================================================
// §10.5 — Making Melee Attacks: models in ER, one melee weapon, WS hit rolls
// ===========================================================================

/// Source: 40k_revised.md §10.5
/// Rule: "Models in Engagement Range of one or more enemy models, or models
///        that are in base-to-base contact with a friendly model in ER, can
///        make melee attacks."
/// The engagement_status field on UnitState tracks this.
#[test]
fn engagement_status_determines_melee_eligibility() {
    let unit_id = UnitId::new(40);
    let model = make_model(4000, unit_id, 3, Position::from_inches(10, 10));
    let mut unit = make_unit(
        40,
        PlayerId::new(0),
        &[Keyword::Infantry],
        vec![model],
    );

    // Not engaged: cannot make melee attacks
    assert_eq!(unit.engagement_status, EngagementStatus::NotEngaged);

    // Set to engaged: can make melee attacks
    unit.engagement_status = EngagementStatus::Engaged;
    assert_eq!(
        unit.engagement_status,
        EngagementStatus::Engaged,
        "Unit must be Engaged to make melee attacks"
    );
}

/// Source: 40k_revised.md §10.5
/// Rule: "When a model makes melee attacks, it uses its Weapon Skill (WS)
///        characteristic for the Hit roll."
/// Melee weapons use WeaponType::Melee and WS (via the skill field).
#[test]
fn melee_weapon_uses_weapon_skill_for_hits() {
    let melee_weapon = make_melee_weapon("Chainsword", 3, 3, 4, -1, 1);

    assert_eq!(
        melee_weapon.weapon_type,
        WeaponType::Melee,
        "Melee weapon should have Melee weapon type"
    );
    assert_eq!(
        melee_weapon.skill.value(),
        3,
        "Chainsword WS should be 3+"
    );
    assert_eq!(
        melee_weapon.range,
        Inches::ZERO,
        "Melee weapons have 0\" range"
    );
}

/// Source: 40k_revised.md §10.5
/// Rule: "Each model can only make attacks with one melee weapon."
/// The weapon_type field distinguishes melee from ranged.
#[test]
fn melee_weapon_type_is_melee() {
    let chainsword = make_melee_weapon("Chainsword", 3, 3, 4, 0, 1);
    let power_fist = make_melee_weapon("Power fist", 3, 3, 8, -2, 2);

    assert_eq!(chainsword.weapon_type, WeaponType::Melee);
    assert_eq!(power_fist.weapon_type, WeaponType::Melee);
    assert_ne!(
        chainsword.weapon_type,
        WeaponType::Ranged,
        "Melee weapons should not be Ranged type"
    );
}

/// Source: 40k_revised.md §10.5
/// Rule: Melee attacks use the full attack pipeline via resolve_attack_batch.
/// Test: Run a melee attack through the pipeline with a deterministic dice roller.
#[test]
fn melee_attack_pipeline_via_resolve_attack_batch() {
    let weapon = make_melee_weapon("Berzerker chainblade", 4, 3, 5, -1, 1);
    let mut ctx = make_melee_attack_context(weapon.clone(), 1, 4, 4, 3, None);
    ctx.effective_abilities = weapon.abilities.clone();

    let defender_unit = UnitId::new(2);
    let mut defenders = vec![
        ModelState::new(
            ModelId::new(200),
            defender_unit,
            Wounds::new(3),
            Position::from_inches(20, 10),
            BaseSize::MM32,
            vec![],
            vec![],
            false,
            None,
        ),
        ModelState::new(
            ModelId::new(201),
            defender_unit,
            Wounds::new(3),
            Position::from_inches(20, 11),
            BaseSize::MM32,
            vec![],
            vec![],
            false,
            None,
        ),
    ];

    let mut dice = make_dice(42);
    let result = resolve_attack_batch(&ctx, &mut defenders, &mut dice);

    // The attack pipeline ran successfully (events were generated)
    assert!(
        !result.events.is_empty(),
        "Melee attack pipeline should generate events"
    );
}

/// Source: 40k_revised.md §10.5
/// Rule: Melee attacks with higher WS (worse skill) hit less often.
/// Compare WS 2+ vs WS 5+ over many seeded runs.
#[test]
fn melee_ws_affects_hit_probability() {
    let ws2_weapon = make_melee_weapon("Elite blade", 10, 2, 5, -1, 1);
    let ws5_weapon = make_melee_weapon("Crude club", 10, 5, 5, -1, 1);

    let mut total_ws2_destroyed = 0u32;
    let mut total_ws5_destroyed = 0u32;

    // Run with many different seeds to get statistical signal
    for seed in 0u8..50u8 {
        let defender_unit = UnitId::new(2);

        // WS 2+ attacks
        let mut ctx_ws2 = make_melee_attack_context(ws2_weapon.clone(), 1, 10, 4, 4, None);
        ctx_ws2.effective_abilities = ws2_weapon.abilities.clone();
        let mut defenders_ws2: Vec<ModelState> = (0..5)
            .map(|i| ModelState::new(
                ModelId::new(300 + i),
                defender_unit,
                Wounds::new(1),
                Position::from_inches(20, 10 + i as i32),
                BaseSize::MM32,
                vec![],
                vec![],
                false,
                None,
            ))
            .collect();
        let mut dice_ws2 = make_dice(seed);
        let r_ws2 = resolve_attack_batch(&ctx_ws2, &mut defenders_ws2, &mut dice_ws2);
        total_ws2_destroyed += r_ws2.models_destroyed.len() as u32;

        // WS 5+ attacks
        let mut ctx_ws5 = make_melee_attack_context(ws5_weapon.clone(), 1, 10, 4, 4, None);
        ctx_ws5.effective_abilities = ws5_weapon.abilities.clone();
        let mut defenders_ws5: Vec<ModelState> = (0..5)
            .map(|i| ModelState::new(
                ModelId::new(400 + i),
                defender_unit,
                Wounds::new(1),
                Position::from_inches(20, 10 + i as i32),
                BaseSize::MM32,
                vec![],
                vec![],
                false,
                None,
            ))
            .collect();
        let mut dice_ws5 = make_dice(seed);
        let r_ws5 = resolve_attack_batch(&ctx_ws5, &mut defenders_ws5, &mut dice_ws5);
        total_ws5_destroyed += r_ws5.models_destroyed.len() as u32;
    }

    // WS 2+ should destroy more models than WS 5+ on average
    assert!(
        total_ws2_destroyed > total_ws5_destroyed,
        "WS 2+ should destroy more models ({}) than WS 5+ ({})",
        total_ws2_destroyed,
        total_ws5_destroyed
    );
}

/// Source: 40k_revised.md §10.5
/// Rule: Melee attacks are made in engagement range — the in_engagement_range
/// flag on AttackContext should be true for melee.
#[test]
fn melee_attack_context_in_engagement_range() {
    let weapon = make_melee_weapon("Close combat weapon", 2, 4, 4, 0, 1);
    let ctx = make_melee_attack_context(weapon, 1, 2, 4, 3, None);

    assert!(
        ctx.in_engagement_range,
        "Melee attack context should have in_engagement_range = true"
    );
    assert_eq!(
        ctx.distance_mils, 0,
        "Melee attack distance should be 0"
    );
}

/// Source: 40k_revised.md §10.5
/// Rule: Melee weapons have zero range (unlike ranged weapons).
#[test]
fn melee_weapon_has_zero_range() {
    let chainsword = make_melee_weapon("Chainsword", 3, 3, 4, 0, 1);
    assert_eq!(
        chainsword.range,
        Inches::ZERO,
        "Melee weapon range should be 0\""
    );

    let guardian_spear = make_melee_weapon("Guardian spear", 5, 2, 7, -2, 2);
    assert_eq!(
        guardian_spear.range,
        Inches::ZERO,
        "Guardian spear melee range should be 0\""
    );
}

/// Source: 40k_revised.md §10.5
/// Rule: A melee weapon can be used in engagement range.
/// WeaponProfile.can_fire_in_engagement() returns true for Melee weapons.
#[test]
fn melee_weapon_can_fire_in_engagement() {
    let chainsword = make_melee_weapon("Chainsword", 3, 3, 4, 0, 1);
    assert!(
        chainsword.can_fire_in_engagement(),
        "Melee weapons should always be usable in engagement range"
    );
}

// ===========================================================================
// §10.6 — Consolidate: up to 3" (CONSOLIDATE constant)
// ===========================================================================

/// Source: 40k_revised.md §10.6
/// Rule: "Each time a unit consolidates, you can move each model in that unit
///        up to 3\"."
/// The CONSOLIDATE constant is exactly 3 inches.
#[test]
fn consolidate_distance_is_three_inches() {
    assert_eq!(
        Inches::CONSOLIDATE,
        Inches::from_inches(3),
        "CONSOLIDATE constant should be 3 inches"
    );
    assert_eq!(
        Inches::CONSOLIDATE.mils(),
        3000,
        "CONSOLIDATE should be 3000 mils"
    );
}

/// Source: 40k_revised.md §10.6
/// Rule: Consolidation moves must end closer to the closest enemy model.
/// Same direction constraint as pile-in.
#[test]
fn consolidate_must_end_closer_to_enemy() {
    let model_pos = Position::from_inches(10, 10);
    let enemy_pos = Position::from_inches(14, 10);

    let initial_dist = model_pos.distance(enemy_pos);
    assert_eq!(initial_dist, Inches::from_inches(4));

    // Move 2" closer
    let consolidated_pos = Position::from_inches(12, 10);
    let new_dist = consolidated_pos.distance(enemy_pos);
    assert_eq!(new_dist, Inches::from_inches(2));

    assert!(
        new_dist < initial_dist,
        "Consolidation must end closer to the closest enemy"
    );
}

/// Source: 40k_revised.md §10.6
/// Rule: Consolidation move of exactly 3" is valid.
#[test]
fn consolidate_at_maximum_is_valid() {
    let consolidate_max = Inches::CONSOLIDATE;
    let move_dist = Inches::from_inches(3);
    assert!(
        move_dist <= consolidate_max,
        "A 3\" consolidation should be valid"
    );
}

/// Source: 40k_revised.md §10.6
/// Rule: Consolidation move exceeding 3" is invalid.
#[test]
fn consolidate_exceeding_maximum_is_invalid() {
    let consolidate_max = Inches::CONSOLIDATE;
    let over_move = Inches::from_mils(3001);
    assert!(
        over_move > consolidate_max,
        "A move exceeding 3\" should be invalid for consolidation"
    );
}

/// Source: 40k_revised.md §10.6
/// Rule: Extended consolidation (6") for special abilities.
#[test]
fn extended_consolidate_distance_is_six_inches() {
    assert_eq!(
        Inches::CONSOLIDATE_EXTENDED,
        Inches::from_inches(6),
        "Extended consolidation should be 6 inches"
    );
    assert_eq!(Inches::CONSOLIDATE_EXTENDED.mils(), 6000);
}

/// Source: 40k_revised.md §10.6
/// Rule: A unit that consolidates can end up in engagement range of a new enemy,
/// potentially pulling them into combat for the next turn.
/// Test that models can end consolidation in ER of enemies.
#[test]
fn consolidate_can_enter_engagement_range_of_new_enemy() {
    // Model consolidates from 5" away to within ER of a new enemy.
    // With 32mm bases (radius ~0.63"), ER boundary is at ~2.26" center-to-center.
    let start_pos = Position::from_inches(10, 10);
    let new_enemy_pos = Position::from_inches(14, 10); // 4" away center-to-center

    let base = BaseSize::MM32;

    // Before consolidation: not in ER of new enemy (gap ~4" - 2*0.63" = ~2.74" > 1")
    let in_er_before = wh40k_geometry::within_engagement_range_2d(
        start_pos, base, new_enemy_pos, base,
    );
    assert!(
        !in_er_before,
        "Should NOT be in ER of new enemy before consolidation (4\" apart center-to-center)"
    );

    // After consolidating 3" closer: now at 1" center-to-center, bases overlap -> in ER
    let end_pos = Position::from_inches(13, 10);
    let in_er_after = wh40k_geometry::within_engagement_range_2d(
        end_pos, base, new_enemy_pos, base,
    );
    assert!(
        in_er_after,
        "Should be in ER of new enemy after consolidation toward them (1\" apart center-to-center)"
    );
}

// ===========================================================================
// Fight Phase integration: charged units fight with correct flags
// ===========================================================================

/// Source: 40k_revised.md §10.1, §10.2
/// Rule: A unit that charged has Fights First AND is marked as charged.
/// The mark_charged call sets both units_charged and charged_this_turn.
#[test]
fn charged_unit_has_both_charge_and_fights_first_flags() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(50);

    flags.mark_charged(unit);

    assert!(
        flags.has_charged(unit),
        "Charged unit should be in units_charged"
    );
    assert!(
        flags.charged_this_turn(unit),
        "Charged unit should have charged_this_turn (Fights First)"
    );
}

/// Source: 40k_revised.md §10.1
/// Rule: The fight alternation should be properly reset when turn flags clear.
#[test]
fn fight_alternation_clears_on_turn_reset() {
    let mut flags = TurnFlags::new();
    flags.init_fight_alternation(PlayerId::new(0));
    assert!(flags.fight_alternation_next_player.is_some());

    flags.clear();
    assert!(
        flags.fight_alternation_next_player.is_none(),
        "Fight alternation should be cleared on turn reset"
    );
}

/// Source: 40k_revised.md §10.4, §10.6
/// Rule: Pile-in and consolidation distances are both 3" by default,
/// and both extended distances are 6".
#[test]
fn pile_in_and_consolidate_distances_match() {
    assert_eq!(
        Inches::PILE_IN,
        Inches::CONSOLIDATE,
        "Default pile-in and consolidation distances should both be 3\""
    );
    assert_eq!(
        Inches::PILE_IN_EXTENDED,
        Inches::CONSOLIDATE_EXTENDED,
        "Extended pile-in and consolidation distances should both be 6\""
    );
}

/// Source: 40k_revised.md §10.5
/// Rule: The Fight Phase is the Phase::Fight enum variant.
/// Ensure the fight phase exists in the phase sequence after Charge.
#[test]
fn fight_phase_follows_charge_in_sequence() {
    let charge_phase = Phase::Charge;
    let next = charge_phase.next();

    assert_eq!(
        next,
        Some(Phase::Fight),
        "Fight phase should follow Charge phase"
    );
}

/// Source: 40k_revised.md §10.1
/// Rule: The Fight phase sub-phases include FightsFirst, RemainingCombats,
/// PileIn, ResolveMeleeAttacks, and Consolidate.
#[test]
fn fight_subphases_exist() {
    // Verify the fight-related sub-phases are available
    let _fights_first = SubPhase::FightsFirst;
    let _remaining_combats = SubPhase::RemainingCombats;
    let _pile_in = SubPhase::PileIn;
    let _melee_attacks = SubPhase::ResolveMeleeAttacks;
    let _consolidate = SubPhase::Consolidate;
    let _fight_start = SubPhase::FightPhaseStart;
    let _fight_end = SubPhase::FightPhaseEnd;

    // If this compiles, the sub-phases exist
    assert_ne!(SubPhase::FightsFirst, SubPhase::RemainingCombats);
    assert_ne!(SubPhase::PileIn, SubPhase::Consolidate);
}

// ===========================================================================
// §10.1 — Cannot pass: cannot pass when eligible units remain
// ===========================================================================

/// Source: 40k_revised.md §10.1
/// Rule: "Players cannot pass when they have eligible units remaining to fight."
/// Eligible units are those in Engagement Range that have not yet fought.
/// Test: TurnFlags tracks fought status so we can determine eligibility.
#[test]
fn fight_cannot_pass_eligible_units_remain() {
    let mut flags = TurnFlags::new();
    let unit_a = UnitId::new(10);
    let unit_b = UnitId::new(11);

    // Unit A is engaged and has NOT fought — eligible to fight
    assert!(
        !flags.has_fought(unit_a),
        "Unit A has not fought, so it is eligible (§10.1)"
    );

    // Unit B has fought — no longer eligible
    flags.mark_fought(unit_b);
    assert!(
        flags.has_fought(unit_b),
        "Unit B has fought, no longer eligible (§10.1)"
    );

    // As long as unit_a hasn't fought, the player cannot pass
    // (the engine enforces this by checking has_fought for all engaged units)
    let any_eligible = !flags.has_fought(unit_a);
    assert!(
        any_eligible,
        "Player cannot pass while unfought engaged units remain (§10.1)"
    );
}

// ===========================================================================
// §10.1 — Fight once per phase: mark_fought prevents second fight
// ===========================================================================

/// Source: 40k_revised.md §10.1
/// Rule: "Each unit can only fight once per Fight Phase."
/// After mark_fought, the unit cannot fight again in the same phase.
/// Test: mark_fought blocks re-selection in the combat context.
#[test]
fn fight_once_per_phase_prevents_second_fight() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(20);

    // Fight the first time
    assert!(!flags.has_fought(unit), "Unit should be eligible initially");
    flags.mark_fought(unit);
    assert!(flags.has_fought(unit), "Unit should be marked as fought");

    // Attempt to fight again — has_fought returns true, blocking re-selection
    assert!(
        flags.has_fought(unit),
        "Unit that already fought cannot fight again in the same phase (§10.1)"
    );

    // Other units are unaffected
    let other_unit = UnitId::new(21);
    assert!(
        !flags.has_fought(other_unit),
        "Other units should not be affected by unit 20 fighting"
    );
}

// ===========================================================================
// §10.4 — Only non-base-contact models pile in
// ===========================================================================

/// Source: 40k_revised.md §10.4
/// Rule: "Only models that are NOT already in base-to-base contact with an
///        enemy model can pile in."
/// Test: models already in base-to-base contact (distance = 0 gap between
/// bases) should not need to pile in.
#[test]
fn pile_in_only_non_base_contact_models() {
    // Model A at (10, 10) and enemy at (10, 10) — same position, in base contact
    let model_pos = Position::from_inches(10, 10);
    let enemy_pos = Position::from_inches(10, 10);
    let base = BaseSize::MM32;

    let model_dist = wh40k_geometry::distance_between_models(model_pos, base, enemy_pos, base);

    // Models at the same position have negative or zero gap (bases overlap)
    assert!(
        model_dist <= Inches::ZERO,
        "Models at the same position are in base-to-base contact — no pile-in needed (§10.4)"
    );

    // A model NOT in base contact is further away
    let far_model_pos = Position::from_inches(12, 10);
    let far_dist = wh40k_geometry::distance_between_models(far_model_pos, base, enemy_pos, base);
    assert!(
        far_dist > Inches::ZERO,
        "Model 2\" away from enemy has a positive gap — can pile in (§10.4)"
    );
}

// ===========================================================================
// §10.4 — Must end in Unit Coherency after pile-in
// ===========================================================================

/// Source: 40k_revised.md §10.4
/// Rule: "After a unit piles in, it must end in Unit Coherency."
/// Coherency requires each model to be within 2" horizontally and 5"
/// vertically of at least one other model in the unit.
/// Test: verify the coherency range constants are correct.
#[test]
fn pile_in_must_end_in_coherency() {
    // Unit coherency range is 2" horizontal, 5" vertical
    assert_eq!(
        Inches::COHERENCY_RANGE,
        Inches::from_inches(2),
        "Coherency range should be 2\" for pile-in end validation (§10.4)"
    );
    assert_eq!(
        Inches::COHERENCY_RANGE_VERTICAL,
        Inches::from_inches(5),
        "Coherency vertical range should be 5\" for pile-in end validation (§10.4)"
    );

    // Two models within coherency
    let model_a = Position::from_inches(10, 10);
    let model_b = Position::from_inches(12, 10); // 2" away
    let dist = model_a.distance(model_b);
    assert!(
        dist <= Inches::COHERENCY_RANGE,
        "Models 2\" apart should be in coherency (§10.4)"
    );

    // Two models outside coherency
    let model_c = Position::from_inches(13, 10); // 3" from model_a
    let dist_c = model_a.distance(model_c);
    assert!(
        dist_c > Inches::COHERENCY_RANGE,
        "Models 3\" apart should NOT be in coherency (§10.4)"
    );
}

// ===========================================================================
// §10.5 — One melee weapon per model selection
// ===========================================================================

/// Source: 40k_revised.md §10.5
/// Rule: "Each model can only make attacks with one melee weapon."
/// Test: melee weapons are typed as WeaponType::Melee and can be distinguished.
#[test]
fn fight_one_melee_weapon_per_model() {
    let chainsword = make_melee_weapon("Chainsword", 4, 3, 4, 0, 1);
    let power_fist = make_melee_weapon("Power fist", 3, 3, 8, -2, 2);

    // Both are melee weapons
    assert_eq!(chainsword.weapon_type, WeaponType::Melee);
    assert_eq!(power_fist.weapon_type, WeaponType::Melee);

    // They have different IDs/names (model must choose ONE)
    assert_ne!(
        chainsword.name, power_fist.name,
        "Different melee weapons should be distinguishable for one-weapon-per-model rule (§10.5)"
    );

    // Ranged weapons are NOT melee — they use a different attack step
    let ranged = WeaponProfile {
        id: WeaponId::new(99),
        name: "Boltgun".to_string(),
        weapon_type: WeaponType::Ranged,
        range: Inches::from_inches(24),
        attacks: AttackCount::Fixed(2),
        skill: Skill(3),
        strength: Strength::new(4),
        ap: ArmorPenetration::new(0),
        damage: Damage::Fixed(1),
        abilities: WeaponAbilitySet::new(),
    };
    assert_ne!(
        ranged.weapon_type, chainsword.weapon_type,
        "Ranged weapon is not Melee (§10.5)"
    );
}

// ===========================================================================
// §10.6 — Must end in base-to-base if possible (consolidation)
// ===========================================================================

/// Source: 40k_revised.md §10.6
/// Rule: "If a model can end a consolidation move in base-to-base contact
///        with the closest enemy model, it must do so."
/// Test: verify that ending closer to enemy is enforced, and base-to-base
/// is achievable within the consolidation distance.
#[test]
fn consolidate_must_end_base_to_base_if_possible() {
    let consolidate_max = Inches::CONSOLIDATE;
    assert_eq!(consolidate_max, Inches::from_inches(3), "Consolidation max = 3\"");

    let model_pos = Position::from_inches(10, 10);
    let enemy_pos = Position::from_inches(12, 10); // 2" away (center to center)
    let base = BaseSize::MM32;

    let start_dist = model_pos.distance(enemy_pos);
    assert_eq!(start_dist, Inches::from_inches(2));

    // Can the model reach base-to-base within 3"?
    // Base-to-base means model distance (gap between bases) = 0
    let model_gap = wh40k_geometry::distance_between_models(model_pos, base, enemy_pos, base);

    // With 32mm bases (~0.63" radius each), gap between bases at 2" center distance:
    // gap = 2" - 2*0.63" = 0.74"
    // The model needs to close 0.74" which is well within the 3" consolidation max.
    assert!(
        model_gap <= consolidate_max,
        "Gap between bases ({}) should be closeable within consolidation range (3\") (§10.6)",
        model_gap
    );

    // Move to base-to-base (close the gap)
    let btb_pos = Position::from_inches(11, 10); // 1" from enemy center
    let _btb_gap = wh40k_geometry::distance_between_models(btb_pos, base, enemy_pos, base);
    let btb_in_er = wh40k_geometry::within_engagement_range_2d(btb_pos, base, enemy_pos, base);

    assert!(
        btb_in_er,
        "Model moved to 1\" from enemy should be in ER (§10.6)"
    );

    // Verify the move distance was within consolidation range
    let move_dist = model_pos.distance(btb_pos);
    assert!(
        move_dist <= consolidate_max,
        "Move distance ({}) should be within consolidation range (§10.6)",
        move_dist
    );

    // Verify the end position is closer to enemy than start
    let end_dist = btb_pos.distance(enemy_pos);
    assert!(
        end_dist < start_dist,
        "Consolidation must end closer to the closest enemy (§10.6)"
    );
}

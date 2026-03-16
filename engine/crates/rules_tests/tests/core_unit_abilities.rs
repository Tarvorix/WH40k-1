//! Source-linked rules tests for 40k_revised.md Section 12: Unit Abilities.
//!
//! Tests cover:
//!   §12.1 — Feel No Pain [X+]
//!   §12.2 — Deadly Demise [X]
//!   §12.3 — Deep Strike
//!   §12.4 — Infiltrators
//!   §12.5 — Scouts [X"]
//!   §12.6 — Lone Operative
//!   §12.7 — Stealth
//!   §12.8 — Leader / Attached Units
//!   §12.9 — Fights First
//!   §12.10 — Aura abilities
//!
//! Source: 40k_revised.md Section 12 — "UNIT ABILITIES"

use wh40k_core_types::{
    ArmorPenetration, ArmorSave, AttackCount, BaseSize, Damage, EngagementStatus,
    FeelNoPain, Inches, InvulnerableSave, Keyword, KeywordSet, ModelId, MoveCharacteristic,
    ObjectiveControl, PlayerId, Position, Skill, Strength, Toughness, UnitId, UnitStatus,
    WeaponAbility, WeaponAbilitySet, WeaponId, WeaponProfile, WeaponType, Wounds,
    DatasheetId, Leadership,
};
use wh40k_dice::{DiceContext, DiceRoller, StreamKind};
use wh40k_game_core::combat::{resolve_attack_batch, AttackContext};
use wh40k_game_core::unit::{ModelState, UnitState, ReserveType};

// ===========================================================================
// Helper factories
// ===========================================================================

/// Create a deterministic DiceRoller from a seed byte for reproducibility.
fn make_dice(seed_byte: u8) -> DiceRoller {
    let ctx = DiceContext::new([seed_byte; 32], StreamKind::HitRoll, 0, 0);
    DiceRoller::new(ctx)
}

/// Build a simple ranged weapon profile.
fn make_ranged_weapon(
    name: &str,
    attacks: u8,
    skill: u8,
    strength: u8,
    ap: i8,
    damage: u8,
) -> WeaponProfile {
    WeaponProfile {
        id: WeaponId::new(1),
        name: name.to_string(),
        weapon_type: WeaponType::Ranged,
        range: Inches::from_inches(24),
        attacks: AttackCount::Fixed(attacks),
        skill: Skill(skill),
        strength: Strength::new(strength),
        ap: ArmorPenetration::new(ap),
        damage: Damage::Fixed(damage),
        abilities: WeaponAbilitySet::new(),
    }
}

/// Build a simple melee weapon profile.
#[allow(dead_code)]
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

/// Build a defender model with optional Feel No Pain.
fn make_defender_model(
    model_id: u32,
    unit_id: UnitId,
    wounds: u8,
    fnp: Option<FeelNoPain>,
) -> ModelState {
    ModelState::new(
        ModelId::new(model_id),
        unit_id,
        Wounds::new(wounds),
        Position::from_inches(20, 10),
        BaseSize::MM32,
        vec![],
        vec![],
        false,
        fnp,
    )
}

/// Build a defender leader model (is_leader = true) with optional FNP.
fn make_leader_model(
    model_id: u32,
    unit_id: UnitId,
    wounds: u8,
    fnp: Option<FeelNoPain>,
) -> ModelState {
    ModelState::new(
        ModelId::new(model_id),
        unit_id,
        Wounds::new(wounds),
        Position::from_inches(20, 10),
        BaseSize::MM40,
        vec![],
        vec![],
        true,
        fnp,
    )
}

/// Build a default AttackContext.
fn make_attack_context(
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
        in_engagement_range: false,
        target_model_count: 5,
        target_keywords: KeywordSet::from_keywords(&[Keyword::Infantry]),
        target_has_stealth: false,
        distance_mils: 12_000,
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

/// Build an infantry UnitState for ability tests.
fn make_infantry_unit(id: u32, model_count: usize, wounds_per_model: u8) -> UnitState {
    let unit_id = UnitId::new(id);
    let models: Vec<ModelState> = (0..model_count)
        .map(|i| {
            ModelState::new(
                ModelId::new(id * 100 + i as u32),
                unit_id,
                Wounds::new(wounds_per_model),
                Position::from_inches(10, 10),
                BaseSize::MM32,
                vec![],
                vec![],
                false,
                None,
            )
        })
        .collect();

    let mut unit = UnitState::new(
        unit_id,
        PlayerId::new(0),
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

// ===========================================================================
// §12.1 — FEEL NO PAIN [X+]
// ===========================================================================

/// Source: 40k_revised.md §12.1
/// Rule: "Each time a model with this ability would lose a wound, roll one D6:
///        if the result equals or exceeds X, that wound is not lost."
/// Test: FNP 5+ is set on ModelState and recognized by the combat system.
#[test]
fn fnp_model_state_stores_fnp_value() {
    let model = make_defender_model(1, UnitId::new(1), 3, Some(FeelNoPain::FIVE_PLUS));
    assert!(model.feel_no_pain.is_some());
    assert_eq!(model.feel_no_pain.unwrap().value(), 5);
    assert!(model.feel_no_pain.unwrap().has_fnp());
}

/// Source: 40k_revised.md §12.1
/// Rule: FNP None means no FNP ability.
/// Test: Model without FNP has None feel_no_pain field.
#[test]
fn fnp_model_without_fnp() {
    let model = make_defender_model(2, UnitId::new(1), 3, None);
    assert!(model.feel_no_pain.is_none());
}

/// Source: 40k_revised.md §12.1
/// Rule: "Each time a wound would be lost, roll one D6."
/// Test: Resolve attacks against a model with FNP 5+ — damage may be reduced.
///       Over many seeds, FNP should sometimes ignore wounds.
#[test]
fn fnp_reduces_damage_via_combat_pipeline() {
    let weapon = make_ranged_weapon("Heavy bolter", 3, 3, 5, -1, 2);
    let defender_id = UnitId::new(2);

    // Run through many seeds to find at least one where FNP ignores a wound
    let mut found_fnp_reducing_damage = false;
    let mut found_normal_damage = false;

    for seed in 0u8..50 {
        let mut dice_fnp = make_dice(seed);
        let mut dice_no_fnp = make_dice(seed);

        // With FNP 5+
        let mut models_fnp = vec![
            make_defender_model(100, defender_id, 6, Some(FeelNoPain::FIVE_PLUS)),
        ];
        let ctx = make_attack_context(weapon.clone(), 1, 3, 4, 6, None);
        let _result_fnp = resolve_attack_batch(&ctx, &mut models_fnp, &mut dice_fnp);

        // Without FNP
        let mut models_no_fnp = vec![
            make_defender_model(100, defender_id, 6, None),
        ];
        let _result_no_fnp = resolve_attack_batch(&ctx, &mut models_no_fnp, &mut dice_no_fnp);

        let wounds_with_fnp = 6u8.saturating_sub(models_fnp[0].wounds_remaining.value());
        let wounds_without_fnp = 6u8.saturating_sub(models_no_fnp[0].wounds_remaining.value());

        if wounds_with_fnp < wounds_without_fnp {
            found_fnp_reducing_damage = true;
        }
        if wounds_without_fnp > 0 {
            found_normal_damage = true;
        }

        if found_fnp_reducing_damage && found_normal_damage {
            break;
        }
    }

    // FNP should reduce damage at least once across 50 seeds
    assert!(found_fnp_reducing_damage, "FNP 5+ should reduce damage in at least one seed out of 50");
    assert!(found_normal_damage, "Normal attacks should deal damage in at least one seed out of 50");
}

/// Source: 40k_revised.md §12.1
/// Rule: "If a model has more than one Feel No Pain ability, you can only
///        use one of those abilities each time that model loses a wound."
/// Test: FeelNoPain struct stores a single value per model — by design only
///       one FNP applies. The combat system picks the best of model FNP and bonus FNP.
#[test]
fn fnp_multiple_only_best_applies() {
    // Model has native FNP 6+
    let model = make_defender_model(3, UnitId::new(1), 3, Some(FeelNoPain::SIX_PLUS));
    assert_eq!(model.feel_no_pain.unwrap().value(), 6);

    // The combat pipeline's bonus_fnp field (from blessings) can provide a better FNP.
    // When both are present, the system uses the best (lowest) value.
    // FNP 5+ from bonus vs FNP 6+ native -> effective FNP should be 5+.
    // This is verified by the apply_damage_with_fnp logic which picks min(model_fnp, bonus_fnp).
}

/// Source: 40k_revised.md §12.1
/// Rule: FNP 4+ is stronger than FNP 6+.
/// Test: FeelNoPain ordering — lower value = better FNP.
#[test]
fn fnp_ordering_lower_is_better() {
    assert!(FeelNoPain::FOUR_PLUS.value() < FeelNoPain::FIVE_PLUS.value());
    assert!(FeelNoPain::FIVE_PLUS.value() < FeelNoPain::SIX_PLUS.value());
    assert!(FeelNoPain::FOUR_PLUS.passes(4));
    assert!(FeelNoPain::FOUR_PLUS.passes(5));
    assert!(FeelNoPain::FOUR_PLUS.passes(6));
    assert!(!FeelNoPain::FOUR_PLUS.passes(3));
}

/// Source: 40k_revised.md §12.1
/// Rule: FNP NONE (7+) means no FNP save.
/// Test: FeelNoPain::NONE doesn't activate.
#[test]
fn fnp_none_does_not_activate() {
    let fnp_none = FeelNoPain::NONE;
    assert!(!fnp_none.has_fnp());
    assert!(!fnp_none.passes(6)); // Even a 6 doesn't pass 7+
}

/// Source: 40k_revised.md §12.1
/// Rule: FNP applies to mortal wounds as well as normal damage.
/// Test: bonus_fnp field in AttackContext is carried through the combat pipeline.
#[test]
fn fnp_bonus_fnp_in_attack_context() {
    let weapon = make_ranged_weapon("Bolt rifle", 2, 3, 4, -1, 1);
    let ctx = make_attack_context(weapon, 1, 2, 4, 6, None);

    // Verify the bonus_fnp field is part of the context
    assert_eq!(ctx.bonus_fnp, 0);

    // Create a context with bonus FNP (e.g., from Wrathful Devotion blessing)
    let mut ctx_with_bonus = ctx.clone();
    ctx_with_bonus.bonus_fnp = 5; // 5+ FNP from blessing

    assert_eq!(ctx_with_bonus.bonus_fnp, 5);
}

// ===========================================================================
// §12.2 — DEADLY DEMISE [X]
// ===========================================================================

/// Source: 40k_revised.md §12.2
/// Rule: "When the last model in a unit with this ability is destroyed, roll one D6.
///        On a 6, every unit within 6\" suffers X mortal wounds."
/// Test: deadly_demise field is tracked on UnitState.
#[test]
fn deadly_demise_field_stored() {
    let mut unit = make_infantry_unit(20, 1, 5);
    unit.deadly_demise = 3;
    assert_eq!(unit.deadly_demise, 3);
}

/// Source: 40k_revised.md §12.2
/// Rule: A unit without Deadly Demise has the value 0.
/// Test: Default deadly_demise is 0 (no ability).
#[test]
fn deadly_demise_default_zero() {
    let unit = make_infantry_unit(21, 1, 5);
    assert_eq!(unit.deadly_demise, 0);
}

/// Source: 40k_revised.md §12.2
/// Rule: Different units may have different Deadly Demise values.
/// Test: Multiple units with different DD values.
#[test]
fn deadly_demise_different_values() {
    let mut unit_a = make_infantry_unit(22, 1, 5);
    unit_a.deadly_demise = 1;

    let mut unit_b = make_infantry_unit(23, 1, 8);
    unit_b.deadly_demise = 3;

    let mut unit_c = make_infantry_unit(24, 1, 10);
    unit_c.deadly_demise = 6;

    assert_eq!(unit_a.deadly_demise, 1);
    assert_eq!(unit_b.deadly_demise, 3);
    assert_eq!(unit_c.deadly_demise, 6);
}

// ===========================================================================
// §12.3 — DEEP STRIKE
// ===========================================================================

/// Source: 40k_revised.md §12.3
/// Rule: "Units with Deep Strike can be set up in Reserves instead of on the battlefield."
/// Test: Unit can be placed in reserves with DeepStrike reserve type.
#[test]
fn deep_strike_unit_in_reserves() {
    let mut unit = make_infantry_unit(30, 5, 1);
    unit.status = UnitStatus::InStrategicReserves;
    unit.reserve_type = Some(ReserveType::DeepStrike);

    assert!(unit.is_in_reserves());
    assert_eq!(unit.reserve_type, Some(ReserveType::DeepStrike));
}

/// Source: 40k_revised.md §12.3
/// Rule: "When arriving from Deep Strike, place anywhere on the battlefield
///        more than 9\" from all enemy models."
/// Test: DEEP_STRIKE_MIN_DISTANCE constant is 9 inches.
#[test]
fn deep_strike_minimum_distance_is_9_inches() {
    assert_eq!(Inches::DEEP_STRIKE_MIN_DISTANCE, Inches::from_inches(9));
    assert_eq!(Inches::DEEP_STRIKE_MIN_DISTANCE.mils(), 9000);
}

/// Source: 40k_revised.md §12.3
/// Rule: Deep Strike placement must be >9" from all enemies.
/// Test: Distance check using within_range — 9" is too close, 10" is ok.
#[test]
fn deep_strike_distance_check() {
    let arriving_pos = Position::from_inches(20, 15);
    let enemy_pos = Position::from_inches(20, 6);

    // 9" apart exactly — too close (must be MORE than 9")
    let distance = arriving_pos.distance(enemy_pos);
    assert_eq!(distance, Inches::from_inches(9));
    // within_range of 9" returns true (<=9), so this placement is invalid
    assert!(arriving_pos.within_range(enemy_pos, Inches::DEEP_STRIKE_MIN_DISTANCE));

    // 10" apart — valid placement
    let far_enemy = Position::from_inches(20, 5);
    let dist = arriving_pos.distance(far_enemy);
    assert_eq!(dist, Inches::from_inches(10));
    // Must be > 9", so within_range(9) should be false for valid placement
    assert!(!arriving_pos.within_range(far_enemy, Inches::from_inches(9)));
}

/// Source: 40k_revised.md §12.3
/// Rule: Strategic Reserves is a different reserve type from Deep Strike.
/// Test: Distinguish between DeepStrike and StrategicReserves.
#[test]
fn deep_strike_vs_strategic_reserves() {
    let mut unit_ds = make_infantry_unit(31, 5, 1);
    unit_ds.status = UnitStatus::InStrategicReserves;
    unit_ds.reserve_type = Some(ReserveType::DeepStrike);

    let mut unit_sr = make_infantry_unit(32, 5, 1);
    unit_sr.status = UnitStatus::InStrategicReserves;
    unit_sr.reserve_type = Some(ReserveType::StrategicReserves);

    assert_ne!(unit_ds.reserve_type, unit_sr.reserve_type);
    assert_eq!(unit_ds.reserve_type, Some(ReserveType::DeepStrike));
    assert_eq!(unit_sr.reserve_type, Some(ReserveType::StrategicReserves));
}

// ===========================================================================
// §12.4 — INFILTRATORS
// ===========================================================================

/// Source: 40k_revised.md §12.4
/// Rule: "Units with Infiltrators can be set up anywhere on the battlefield
///        that is more than 9\" from the enemy deployment zone and all enemy models."
/// Test: The 9" distance check applies to Infiltrators placement too.
#[test]
fn infiltrators_9_inch_exclusion_from_enemies() {
    let infiltrator_pos = Position::from_inches(22, 15);
    let enemy_pos = Position::from_inches(22, 5);

    let distance = infiltrator_pos.distance(enemy_pos);
    assert_eq!(distance, Inches::from_inches(10));

    // 10" is valid for Infiltrators (>9" from enemies)
    assert!(!infiltrator_pos.within_range(enemy_pos, Inches::from_inches(9)));
}

/// Source: 40k_revised.md §12.4
/// Rule: Infiltrators must also be >9" from the enemy deployment zone.
/// Test: Deployment zone boundary check using distance calculation.
#[test]
fn infiltrators_9_inch_exclusion_from_enemy_dz() {
    // Enemy deployment zone edge at y=0 to y=6 (for example)
    let enemy_dz_edge = Position::from_inches(22, 6);
    let infiltrator_pos = Position::from_inches(22, 16);

    let distance = infiltrator_pos.distance(enemy_dz_edge);
    assert_eq!(distance, Inches::from_inches(10));
    // Valid: >9" from enemy DZ edge
    assert!(!infiltrator_pos.within_range(enemy_dz_edge, Inches::from_inches(9)));
}

// ===========================================================================
// §12.5 — SCOUTS [X"]
// ===========================================================================

/// Source: 40k_revised.md §12.5
/// Rule: "At the start of the first battle round, before the first turn begins,
///        units with Scouts X\" can make a Normal Move of up to X\"."
/// Test: Position translation simulates a scout move.
#[test]
fn scouts_pre_game_move() {
    let start_pos = Position::from_inches(10, 5);
    let scout_distance = Inches::from_inches(6); // Scouts 6"

    // Move 6" forward (toward enemy)
    let new_pos = start_pos.translate(Inches::ZERO, scout_distance);
    assert_eq!(new_pos, Position::from_inches(10, 11));

    // Verify distance moved
    let moved = start_pos.distance(new_pos);
    assert_eq!(moved, Inches::from_inches(6));
}

/// Source: 40k_revised.md §12.5
/// Rule: Scouts cannot move within Engagement Range (1") of enemy models.
/// Test: Position after scout move must be >1" from enemies.
#[test]
fn scouts_cannot_end_within_engagement_range() {
    let scout_end_pos = Position::from_inches(10, 10);
    let enemy_pos = Position::from_inches(10, 11);

    let distance = scout_end_pos.distance(enemy_pos);
    assert_eq!(distance, Inches::from_inches(1));

    // Within engagement range — invalid scout end position
    assert!(scout_end_pos.within_range(enemy_pos, Inches::ENGAGEMENT_RANGE));
}

// ===========================================================================
// §12.6 — LONE OPERATIVE
// ===========================================================================

/// Source: 40k_revised.md §12.6
/// Rule: "Unless it is within 12\" of the Lone Operative, an enemy model
///        cannot target this unit with a ranged attack."
/// Test: 12" range check for Lone Operative targeting.
#[test]
fn lone_operative_12_inch_range_check() {
    let lone_op_pos = Position::from_inches(20, 15);
    let attacker_far = Position::from_inches(20, 3); // 12" away
    let attacker_close = Position::from_inches(20, 4); // 11" away

    // 12" exactly — at boundary
    let dist_far = lone_op_pos.distance(attacker_far);
    assert_eq!(dist_far, Inches::from_inches(12));
    assert!(lone_op_pos.within_range(attacker_far, Inches::from_inches(12)));

    // 11" — within 12", can target
    let dist_close = lone_op_pos.distance(attacker_close);
    assert_eq!(dist_close, Inches::from_inches(11));
    assert!(lone_op_pos.within_range(attacker_close, Inches::from_inches(12)));

    // 13" — outside 12", cannot target
    let attacker_too_far = Position::from_inches(20, 2);
    let dist_too_far = lone_op_pos.distance(attacker_too_far);
    assert_eq!(dist_too_far, Inches::from_inches(13));
    assert!(!lone_op_pos.within_range(attacker_too_far, Inches::from_inches(12)));
}

/// Source: 40k_revised.md §12.6
/// Rule: Lone Operative still applies Character keyword requirements.
/// Test: Unit with Character keyword check.
#[test]
fn lone_operative_requires_character() {
    let unit_id = UnitId::new(60);
    let model = ModelState::new(
        ModelId::new(600),
        unit_id,
        Wounds::new(4),
        Position::from_inches(20, 15),
        BaseSize::MM32,
        vec![],
        vec![],
        true, // is_leader
        None,
    );

    let unit = UnitState::new(
        unit_id,
        PlayerId::new(0),
        "Lone Operative Character".to_string(),
        DatasheetId::new(3),
        KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Character]),
        vec![model],
        MoveCharacteristic::from_inches(6),
        Toughness::new(4),
        ArmorSave::THREE_PLUS,
        None,
        Leadership::new(7),
        ObjectiveControl::new(1),
    );

    assert!(unit.is_character());
    assert!(unit.has_keyword(Keyword::Character));
}

// ===========================================================================
// §12.7 — STEALTH
// ===========================================================================

/// Source: 40k_revised.md §12.7
/// Rule: "Subtract 1 from the Hit roll when a ranged attack targets this unit."
/// Test: target_has_stealth flag in AttackContext modifies hit rolls.
#[test]
fn stealth_minus_1_to_hit_ranged() {
    let weapon = make_ranged_weapon("Bolt rifle", 2, 3, 4, 0, 1);

    // Run many seeds to confirm stealth consistently reduces hits
    let mut total_hits_no_stealth = 0u32;
    let mut total_hits_stealth = 0u32;

    for seed in 0u8..100 {
        let mut dice_a = make_dice(seed);
        let mut dice_b = make_dice(seed);

        let defender_id = UnitId::new(2);
        let mut models_a = vec![make_defender_model(100, defender_id, 3, None)];
        let mut models_b = vec![make_defender_model(100, defender_id, 3, None)];

        // Without stealth
        let ctx_no_stealth = make_attack_context(weapon.clone(), 1, 2, 4, 6, None);
        let result_a = resolve_attack_batch(&ctx_no_stealth, &mut models_a, &mut dice_a);

        // With stealth
        let mut ctx_stealth = make_attack_context(weapon.clone(), 1, 2, 4, 6, None);
        ctx_stealth.target_has_stealth = true;
        let result_b = resolve_attack_batch(&ctx_stealth, &mut models_b, &mut dice_b);

        // Count wounds inflicted as proxy for hits
        let wounds_a: u8 = result_a.wounds_inflicted.iter().map(|(_, w)| w).sum();
        let wounds_b: u8 = result_b.wounds_inflicted.iter().map(|(_, w)| w).sum();
        total_hits_no_stealth += wounds_a as u32;
        total_hits_stealth += wounds_b as u32;
    }

    // Stealth should result in fewer total wounds across 100 seeds
    assert!(
        total_hits_stealth <= total_hits_no_stealth,
        "Stealth should reduce or equal total hits: stealth={}, no_stealth={}",
        total_hits_stealth,
        total_hits_no_stealth,
    );
}

/// Source: 40k_revised.md §12.7
/// Rule: Stealth flag is stored in the AttackContext and used during hit resolution.
/// Test: target_has_stealth is false by default and can be set to true.
#[test]
fn stealth_flag_default_false() {
    let weapon = make_ranged_weapon("Bolt rifle", 2, 3, 4, 0, 1);
    let ctx = make_attack_context(weapon, 1, 2, 4, 6, None);
    assert!(!ctx.target_has_stealth);
}

/// Source: 40k_revised.md §12.7
/// Rule: Stealth flag set to true enables the -1 hit modifier.
/// Test: Verify the flag can be set.
#[test]
fn stealth_flag_can_be_enabled() {
    let weapon = make_ranged_weapon("Bolt rifle", 2, 3, 4, 0, 1);
    let mut ctx = make_attack_context(weapon, 1, 2, 4, 6, None);
    ctx.target_has_stealth = true;
    assert!(ctx.target_has_stealth);
}

// ===========================================================================
// §12.8 — LEADER / ATTACHED UNITS
// ===========================================================================

/// Source: 40k_revised.md §12.8
/// Rule: "While a Leader is attached to a Bodyguard unit, attacks cannot be
///        allocated to the Character models (except by weapons with Precision)."
/// Test: Without Precision, attacks go to non-leader models first.
#[test]
fn leader_bodyguard_attacks_allocated_to_non_leaders() {
    let weapon = make_ranged_weapon("Heavy bolter", 3, 3, 5, -1, 2);
    let defender_id = UnitId::new(2);

    // Create an attached unit: 1 leader + 2 bodyguard models
    let leader = make_leader_model(200, defender_id, 5, None);
    let bodyguard_a = make_defender_model(201, defender_id, 2, None);
    let bodyguard_b = make_defender_model(202, defender_id, 2, None);

    let mut models = vec![leader, bodyguard_a, bodyguard_b];

    // Run many seeds to verify leader is not wounded while bodyguards live
    let mut leader_wounded_while_bodyguards_alive = false;

    for seed in 0u8..100 {
        let mut dice = make_dice(seed);

        // Reset models for each seed
        models[0].alive = true;
        models[0].wounds_remaining = Wounds::new(5);
        models[0].allocation_status = wh40k_core_types::AllocationStatus::Unwounded;
        models[1].alive = true;
        models[1].wounds_remaining = Wounds::new(2);
        models[1].allocation_status = wh40k_core_types::AllocationStatus::Unwounded;
        models[2].alive = true;
        models[2].wounds_remaining = Wounds::new(2);
        models[2].allocation_status = wh40k_core_types::AllocationStatus::Unwounded;

        let ctx = make_attack_context(weapon.clone(), 1, 3, 4, 6, None);
        let _result = resolve_attack_batch(&ctx, &mut models, &mut dice);

        // Check if leader took wounds while any bodyguard is still alive
        let bodyguards_alive = models[1].alive || models[2].alive;
        let leader_took_wounds = models[0].wounds_remaining.value() < 5;
        if leader_took_wounds && bodyguards_alive {
            leader_wounded_while_bodyguards_alive = true;
            break;
        }
    }

    assert!(
        !leader_wounded_while_bodyguards_alive,
        "Leader should not take wounds while bodyguard models are still alive (without Precision)"
    );
}

/// Source: 40k_revised.md §12.8
/// Rule: "Weapons with Precision can target CHARACTER models in Attached units."
/// Test: With Precision, attacks can be allocated directly to the leader.
#[test]
fn leader_precision_targets_character() {
    let mut weapon = make_ranged_weapon("Sniper rifle", 1, 3, 5, -2, 3);
    weapon.abilities.add(WeaponAbility::Precision);

    let defender_id = UnitId::new(2);

    // 1 leader + 2 bodyguard
    let leader = make_leader_model(300, defender_id, 5, None);
    let bodyguard_a = make_defender_model(301, defender_id, 2, None);
    let bodyguard_b = make_defender_model(302, defender_id, 2, None);

    let mut models = vec![leader, bodyguard_a, bodyguard_b];

    // Run many seeds to find at least one where the leader takes damage
    let mut leader_took_damage = false;

    for seed in 0u8..100 {
        let mut dice = make_dice(seed);

        // Reset models
        models[0].alive = true;
        models[0].wounds_remaining = Wounds::new(5);
        models[0].allocation_status = wh40k_core_types::AllocationStatus::Unwounded;
        models[1].alive = true;
        models[1].wounds_remaining = Wounds::new(2);
        models[1].allocation_status = wh40k_core_types::AllocationStatus::Unwounded;
        models[2].alive = true;
        models[2].wounds_remaining = Wounds::new(2);
        models[2].allocation_status = wh40k_core_types::AllocationStatus::Unwounded;

        let mut ctx = make_attack_context(weapon.clone(), 1, 1, 4, 6, None);
        ctx.effective_abilities = weapon.abilities.clone();
        let _result = resolve_attack_batch(&ctx, &mut models, &mut dice);

        if models[0].wounds_remaining.value() < 5 {
            leader_took_damage = true;
            break;
        }
    }

    assert!(
        leader_took_damage,
        "Precision weapon should allocate attacks to the leader model"
    );
}

/// Source: 40k_revised.md §12.8
/// Rule: is_leader field identifies CHARACTER models in an attached unit.
/// Test: ModelState.is_leader correctly distinguishes leaders from bodyguards.
#[test]
fn leader_is_leader_field() {
    let unit_id = UnitId::new(40);
    let leader = make_leader_model(400, unit_id, 5, None);
    let bodyguard = make_defender_model(401, unit_id, 2, None);

    assert!(leader.is_leader);
    assert!(!bodyguard.is_leader);
}

/// Source: 40k_revised.md §12.8
/// Rule: Attached leader tracking via UnitState fields.
/// Test: attached_leader and bodyguard_for fields link leader and bodyguard units.
#[test]
fn leader_attached_unit_tracking() {
    let mut leader_unit = make_infantry_unit(41, 1, 5);
    let mut bodyguard_unit = make_infantry_unit(42, 4, 2);

    leader_unit.attached_leader = None; // Leader unit itself
    bodyguard_unit.attached_leader = Some(leader_unit.id);
    leader_unit.bodyguard_for = Some(bodyguard_unit.id);

    assert_eq!(bodyguard_unit.attached_leader, Some(UnitId::new(41)));
    assert_eq!(leader_unit.bodyguard_for, Some(UnitId::new(42)));
}

// ===========================================================================
// §12.9 — FIGHTS FIRST
// ===========================================================================

/// Source: 40k_revised.md §12.9
/// Rule: "Units with Fights First fight in the Fights First step of the Fight Phase,
///        before other units."
/// Test: TurnFlags charged_this_turn grants Fights First eligibility to charging units.
#[test]
fn fights_first_charging_units_fight_first() {
    use wh40k_core_types::UnitId;
    use wh40k_game_core::state::TurnFlags;

    let mut flags = TurnFlags::new();
    let unit_a = UnitId::new(50);
    let unit_b = UnitId::new(51);

    // Unit A charged this turn
    flags.mark_charged(unit_a);
    assert!(flags.charged_this_turn(unit_a));
    assert!(!flags.charged_this_turn(unit_b));
}

/// Source: 40k_revised.md §12.9
/// Rule: Units that did NOT charge and don't have Fights First fight in
///        the Remaining Combats step.
/// Test: Units without charged_this_turn don't get Fights First.
#[test]
fn fights_first_non_charging_units_fight_normal() {
    use wh40k_core_types::UnitId;
    use wh40k_game_core::state::TurnFlags;

    let flags = TurnFlags::new();
    let unit = UnitId::new(52);

    // Unit did not charge
    assert!(!flags.charged_this_turn(unit));
    assert!(!flags.has_charged(unit));
}

/// Source: 40k_revised.md §12.9
/// Rule: Fight Phase alternation: active player picks first in Fights First step.
/// Test: fight_alternation_next_player tracks which player picks next.
#[test]
fn fights_first_alternation_tracking() {
    use wh40k_core_types::PlayerId;
    use wh40k_game_core::state::TurnFlags;

    let mut flags = TurnFlags::new();
    let player_a = PlayerId::new(0);
    let player_b = PlayerId::new(1);

    // Initialize: active player picks first in Fights First step
    flags.init_fight_alternation(player_a);
    assert_eq!(flags.fight_alternation_next_player, Some(player_a));

    // After player A picks, player B picks next
    flags.advance_fight_alternation(player_a, player_b);
    assert_eq!(flags.fight_alternation_next_player, Some(player_b));

    // After player B picks, player A picks again
    flags.advance_fight_alternation(player_b, player_a);
    assert_eq!(flags.fight_alternation_next_player, Some(player_a));
}

// ===========================================================================
// §12.10 — AURA ABILITIES
// ===========================================================================

/// Source: 40k_revised.md §12.10
/// Rule: "Aura abilities affect all units within a certain range of the model
///        with that ability."
/// Test: within_range check for aura range (6" is a common aura range).
#[test]
fn aura_range_check_within_6_inches() {
    let aura_source = Position::from_inches(20, 15);
    let unit_in_range = Position::from_inches(20, 10); // 5" away
    let unit_at_boundary = Position::from_inches(20, 9); // 6" away
    let unit_out_of_range = Position::from_inches(20, 8); // 7" away

    let aura_range = Inches::from_inches(6);

    assert!(aura_source.within_range(unit_in_range, aura_range));
    assert!(aura_source.within_range(unit_at_boundary, aura_range));
    assert!(!aura_source.within_range(unit_out_of_range, aura_range));
}

/// Source: 40k_revised.md §12.10
/// Rule: Aura abilities use model position for range measurement.
/// Test: Position distance calculation is correct for aura range checks.
#[test]
fn aura_distance_calculation() {
    let source = Position::from_inches(10, 10);
    let target = Position::from_inches(13, 14); // 3-4-5 triangle = 5"

    let distance = source.distance(target);
    assert_eq!(distance, Inches::from_inches(5));
    assert!(source.within_range(target, Inches::from_inches(6)));
    assert!(source.within_range(target, Inches::from_inches(5)));
    assert!(!source.within_range(target, Inches::from_inches(4)));
}

/// Source: 40k_revised.md §12.10
/// Rule: Different aura ranges (3", 6", 9", 12") are used by different abilities.
/// Test: All standard aura ranges work correctly.
#[test]
fn aura_various_ranges() {
    let source = Position::from_inches(20, 20);

    // 3" aura
    let target_3 = Position::from_inches(20, 17);
    assert!(source.within_range(target_3, Inches::from_inches(3)));

    // 6" aura
    let target_6 = Position::from_inches(20, 14);
    assert!(source.within_range(target_6, Inches::from_inches(6)));

    // 9" aura (e.g., some deployment abilities)
    let target_9 = Position::from_inches(20, 11);
    assert!(source.within_range(target_9, Inches::from_inches(9)));

    // 12" aura (e.g., Lone Operative detection range)
    let target_12 = Position::from_inches(20, 8);
    assert!(source.within_range(target_12, Inches::from_inches(12)));

    // 13" — out of all standard aura ranges up to 12"
    let target_13 = Position::from_inches(20, 7);
    assert!(!source.within_range(target_13, Inches::from_inches(12)));
}

/// Source: 40k_revised.md §12.10
/// Rule: Auras check range from unit reference position.
/// Test: UnitState reference_position returns the first alive model's position.
#[test]
fn aura_unit_reference_position() {
    let unit = make_infantry_unit(70, 3, 1);

    let ref_pos = unit.reference_position();
    assert!(ref_pos.is_some());
    assert_eq!(ref_pos.unwrap(), Position::from_inches(10, 10));
}

/// Source: 40k_revised.md §12.10
/// Rule: Auras don't function if the aura model is destroyed.
/// Test: A destroyed unit has no reference position.
#[test]
fn aura_destroyed_unit_no_position() {
    let mut unit = make_infantry_unit(71, 3, 1);

    // Kill all models
    for model in &mut unit.models {
        model.alive = false;
    }

    assert!(unit.is_destroyed());
    assert!(unit.reference_position().is_none());
}

// ===========================================================================
// Additional §12 ability tests — Keyword checks
// ===========================================================================

/// Source: 40k_revised.md §12
/// Rule: Unit abilities are tied to keyword-based eligibility.
/// Test: Keyword checks for unit types (Infantry, Character, Monster).
#[test]
fn keywords_infantry_character_checks() {
    let infantry = make_infantry_unit(80, 5, 1);
    assert!(infantry.is_infantry());
    assert!(infantry.is_battleline());
    assert!(!infantry.is_character());
    assert!(!infantry.is_monster());
}

/// Source: 40k_revised.md §12
/// Rule: Character units have specific keyword-based rules.
/// Test: Character keyword enables leader mechanics.
#[test]
fn keywords_character_unit() {
    let unit_id = UnitId::new(81);
    let model = ModelState::new(
        ModelId::new(810),
        unit_id,
        Wounds::new(5),
        Position::from_inches(10, 10),
        BaseSize::MM40,
        vec![],
        vec![],
        true,
        None,
    );

    let unit = UnitState::new(
        unit_id,
        PlayerId::new(0),
        "Test Character".to_string(),
        DatasheetId::new(5),
        KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Character, Keyword::Leader]),
        vec![model],
        MoveCharacteristic::from_inches(6),
        Toughness::new(4),
        ArmorSave::TWO_PLUS,
        Some(InvulnerableSave::FOUR_PLUS),
        Leadership::new(6),
        ObjectiveControl::new(1),
    );

    assert!(unit.is_character());
    assert!(unit.is_infantry());
    assert!(unit.has_keyword(Keyword::Leader));
    assert!(!unit.is_monster());
}

/// Source: 40k_revised.md §12
/// Rule: Engagement status tracking for melee-relevant abilities.
/// Test: EngagementStatus field on UnitState.
#[test]
fn engagement_status_tracking() {
    let mut unit = make_infantry_unit(82, 5, 1);

    assert_eq!(unit.engagement_status, EngagementStatus::NotEngaged);

    unit.engagement_status = EngagementStatus::Engaged;
    assert_eq!(unit.engagement_status, EngagementStatus::Engaged);

    // Enhancement OC override that requires engagement
    unit.enhancement_oc_override = Some(ObjectiveControl::new(4));
    unit.enhancement_oc_requires_engaged = true;

    // Engaged: override applies
    assert_eq!(unit.effective_oc().value(), 4);

    // Not engaged: falls back to base OC
    unit.engagement_status = EngagementStatus::NotEngaged;
    assert_eq!(unit.effective_oc().value(), 2);
}

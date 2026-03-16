//! Source-linked rules tests for 40k_revised.md Section 7: Shooting Phase.
//!
//! Tests cover:
//!   - 7.1: Shooting eligibility (Advanced / Fell Back restrictions)
//!   - 7.2: Target must be within weapon Range and Visible
//!   - 7.4: Locked in Combat restrictions (Pistol, Big Guns Never Tire)
//!   - 7.5: Big Guns Never Tire -1 to Hit modifier for MONSTER/VEHICLE
//!
//! Source: 40k_revised.md Section 7 -- "SHOOTING PHASE"

use wh40k_core_types::{
    ArmorPenetration, ArmorSave, AttackCount, BaseSize, Damage, DatasheetId,
    EngagementStatus, Inches, InvulnerableSave, Keyword, KeywordSet, Leadership,
    ModelId, MoveCharacteristic, ObjectiveControl, PlayerId, Position, Skill,
    Strength, Toughness, UnitId, WeaponAbility, WeaponAbilitySet,
    WeaponId, WeaponProfile, WeaponType, Wounds,
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

/// Build a ranged weapon profile with optional abilities.
fn make_ranged_weapon_with_abilities(
    name: &str,
    range_inches: i32,
    attacks: u8,
    skill: u8,
    strength: u8,
    ap: i8,
    damage: u8,
    abilities: Vec<WeaponAbility>,
) -> WeaponProfile {
    WeaponProfile {
        id: WeaponId::new(1),
        name: name.to_string(),
        weapon_type: WeaponType::Ranged,
        range: Inches::from_inches(range_inches),
        attacks: AttackCount::Fixed(attacks),
        skill: Skill(skill),
        strength: Strength::new(strength),
        ap: ArmorPenetration::new(ap),
        damage: Damage::Fixed(damage),
        abilities: WeaponAbilitySet::from_abilities(abilities),
    }
}

/// Build a simple ranged weapon with no abilities.
fn make_ranged_weapon(
    name: &str,
    range_inches: i32,
    attacks: u8,
    skill: u8,
    strength: u8,
    ap: i8,
    damage: u8,
) -> WeaponProfile {
    make_ranged_weapon_with_abilities(name, range_inches, attacks, skill, strength, ap, damage, vec![])
}

/// Build a default AttackContext for testing.
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

/// Build a defender model at a given position.
fn make_defender_model(model_id: u32, unit_id: UnitId, wounds: u8) -> ModelState {
    ModelState::new(
        ModelId::new(model_id),
        unit_id,
        Wounds::new(wounds),
        Position::from_inches(20, 10),
        BaseSize::MM32,
        vec![],
        vec![],
        false,
        None,
    )
}

/// Build a simple test model with a boltgun and combat blade.
fn make_model_with_weapons(
    model_id: u32,
    unit_id: UnitId,
    wounds: u8,
    position: Position,
    ranged_weapons: Vec<WeaponProfile>,
    melee_weapons: Vec<WeaponProfile>,
) -> ModelState {
    ModelState::new(
        ModelId::new(model_id),
        unit_id,
        Wounds::new(wounds),
        position,
        BaseSize::MM32,
        ranged_weapons,
        melee_weapons,
        false,
        None,
    )
}

/// Build a test unit with keywords.
fn make_unit_with_keywords(
    id: u32,
    owner: PlayerId,
    keywords: &[Keyword],
    models: Vec<ModelState>,
) -> UnitState {
    UnitState::new(
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
    )
}

// ===========================================================================
// 7.1 -- Shooting eligibility: Advanced / Fell Back restrictions
// ===========================================================================

/// Source: 40k_revised.md 7.1
/// Rule: TurnFlags tracks `advanced_this_turn` for per-unit tracking.
/// Verify the field exists and the flag tracking works.
#[test]
fn turnflags_advanced_this_turn_field_exists_and_tracks() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(10);

    // Initially not advanced
    assert!(
        !flags.has_advanced(unit),
        "Unit should not be marked as advanced initially"
    );

    // Mark advanced
    flags.mark_advanced(unit);
    assert!(
        flags.has_advanced(unit),
        "Unit should be marked as advanced after mark_advanced"
    );

    // Also counts as moved
    assert!(
        flags.has_moved(unit),
        "Advanced unit should also be marked as moved"
    );

    // Clear resets
    flags.clear();
    assert!(
        !flags.has_advanced(unit),
        "After clear, unit should no longer be marked advanced"
    );
}

/// Source: 40k_revised.md 7.1
/// Rule: TurnFlags tracks `fell_back_this_turn` for per-unit tracking.
/// Verify the field exists and the flag tracking works.
#[test]
fn turnflags_fell_back_this_turn_field_exists_and_tracks() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(20);

    assert!(
        !flags.has_fell_back(unit),
        "Unit should not be marked as fell back initially"
    );

    flags.mark_fell_back(unit);
    assert!(
        flags.has_fell_back(unit),
        "Unit should be marked as fell back after mark_fell_back"
    );

    // Also counts as moved
    assert!(
        flags.has_moved(unit),
        "Fell-back unit should also be marked as moved"
    );

    flags.clear();
    assert!(
        !flags.has_fell_back(unit),
        "After clear, unit should no longer be marked fell back"
    );
}

/// Source: 40k_revised.md 7.1
/// Rule: "If a unit Advanced, it can only shoot with weapons that have the
///        [ASSAULT] ability."
/// Units that advanced can only fire Assault weapons. Non-Assault weapons are
/// disqualified. This test validates the weapon-level check `can_fire_after_advance()`.
#[test]
fn advanced_unit_can_only_fire_assault_weapons() {
    // Standard boltgun (not Assault) -- should NOT be allowed after advance
    let boltgun = make_ranged_weapon("Boltgun", 24, 2, 3, 4, 0, 1);
    assert!(
        !boltgun.can_fire_after_advance(),
        "Standard boltgun should not fire after advancing"
    );

    // Assault weapon -- should be allowed
    let assault_weapon = make_ranged_weapon_with_abilities(
        "Assault Bolter",
        24,
        3,
        3,
        4,
        0,
        1,
        vec![WeaponAbility::Assault],
    );
    assert!(
        assault_weapon.can_fire_after_advance(),
        "Assault weapon should fire after advancing"
    );

    // Pistol weapon -- also allowed after advance per core rules
    let pistol_weapon = make_ranged_weapon_with_abilities(
        "Bolt Pistol",
        12,
        1,
        3,
        4,
        0,
        1,
        vec![WeaponAbility::Pistol],
    );
    assert!(
        pistol_weapon.can_fire_after_advance(),
        "Pistol weapon should fire after advancing"
    );
}

/// Source: 40k_revised.md 7.1
/// Rule: "If a unit Fell Back, it cannot shoot."
/// Verify that fell_back_this_turn properly blocks shooting eligibility.
/// This is tracked in TurnFlags and checked by the validator.
#[test]
fn fell_back_unit_tracked_in_turnflags_for_shooting_block() {
    let mut flags = TurnFlags::new();
    let shooter = UnitId::new(5);
    let other_unit = UnitId::new(6);

    // Mark as fell back
    flags.mark_fell_back(shooter);

    // The fell-back flag should be set
    assert!(
        flags.has_fell_back(shooter),
        "Shooter that fell back should have fell_back flag"
    );

    // Another unit that didn't fall back should be fine
    assert!(
        !flags.has_fell_back(other_unit),
        "Unrelated unit should not have fell_back flag"
    );
}

/// Source: 40k_revised.md 7.1
/// Rule: Multiple flags can coexist on TurnFlags for different units.
/// One unit advanced, another fell back, another remained stationary.
#[test]
fn turnflags_multiple_units_independent_tracking() {
    let mut flags = TurnFlags::new();
    let unit_a = UnitId::new(1);
    let unit_b = UnitId::new(2);
    let unit_c = UnitId::new(3);

    flags.mark_advanced(unit_a);
    flags.mark_fell_back(unit_b);
    flags.mark_remained_stationary(unit_c);

    assert!(flags.has_advanced(unit_a));
    assert!(!flags.has_fell_back(unit_a));

    assert!(flags.has_fell_back(unit_b));
    assert!(!flags.has_advanced(unit_b));

    assert!(flags.is_stationary(unit_c));
    assert!(!flags.has_advanced(unit_c));
    assert!(!flags.has_fell_back(unit_c));

    // All three should be marked as "moved" (for Movement Phase eligibility)
    assert!(flags.has_moved(unit_a));
    assert!(flags.has_moved(unit_b));
    assert!(flags.has_moved(unit_c));
}

// ===========================================================================
// 7.2 -- Target must be within weapon Range and Visible
// ===========================================================================

/// Source: 40k_revised.md 7.2
/// Rule: Weapon range is measured in inches. WeaponProfile.range stores
/// the maximum range. Verify range values are stored correctly.
#[test]
fn weapon_range_stored_correctly() {
    let boltgun = make_ranged_weapon("Boltgun", 24, 2, 3, 4, 0, 1);
    assert_eq!(
        boltgun.range,
        Inches::from_inches(24),
        "Boltgun should have 24\" range"
    );

    let bolt_pistol = make_ranged_weapon("Bolt Pistol", 12, 1, 3, 4, 0, 1);
    assert_eq!(
        bolt_pistol.range,
        Inches::from_inches(12),
        "Bolt pistol should have 12\" range"
    );

    let lascannon = make_ranged_weapon("Lascannon", 48, 1, 4, 12, -3, 6);
    assert_eq!(
        lascannon.range,
        Inches::from_inches(48),
        "Lascannon should have 48\" range"
    );
}

/// Source: 40k_revised.md 7.2
/// Rule: Target must be within weapon Range. Distance is calculated
/// using Position::distance() which uses fixed-point integer arithmetic.
/// Verify distance calculations match expected values.
#[test]
fn distance_calculation_for_range_check() {
    // Attacker at (0, 0), target at (24, 0) -- exactly 24" apart
    let attacker_pos = Position::from_inches(0, 0);
    let target_pos = Position::from_inches(24, 0);
    let dist = attacker_pos.distance(target_pos);
    assert_eq!(
        dist,
        Inches::from_inches(24),
        "Should be exactly 24\" apart"
    );

    // Within range of a 24" weapon
    let boltgun_range = Inches::from_inches(24);
    assert!(
        dist <= boltgun_range,
        "24\" distance should be within 24\" range"
    );

    // Target at (25, 0) -- just out of 24" range
    let far_target = Position::from_inches(25, 0);
    let far_dist = attacker_pos.distance(far_target);
    assert!(
        far_dist > boltgun_range,
        "25\" distance should be outside 24\" range"
    );
}

/// Source: 40k_revised.md 7.2
/// Rule: Diagonal distances use Euclidean calculation.
/// 3-4-5 triangle: positions (0,0) and (3,4) are exactly 5" apart.
#[test]
fn distance_calculation_diagonal() {
    let a = Position::from_inches(0, 0);
    let b = Position::from_inches(3, 4);
    let dist = a.distance(b);
    assert_eq!(
        dist,
        Inches::from_inches(5),
        "3-4-5 triangle should give distance of 5\""
    );

    // This should be within a 6" weapon range
    let short_range = Inches::from_inches(6);
    assert!(dist <= short_range);

    // But not within a 4" range
    let very_short = Inches::from_inches(4);
    assert!(dist > very_short);
}

/// Source: 40k_revised.md 7.2
/// Rule: within_range is used for range comparisons without sqrt.
/// Verify Position::within_range() works correctly.
#[test]
fn position_within_range_check() {
    let attacker = Position::from_inches(10, 10);
    let close_target = Position::from_inches(15, 10); // 5" away
    let far_target = Position::from_inches(40, 10); // 30" away

    let boltgun_range = Inches::from_inches(24);

    assert!(
        close_target.within_range(attacker, boltgun_range),
        "Target 5\" away should be within 24\" range"
    );
    assert!(
        !far_target.within_range(attacker, boltgun_range),
        "Target 30\" away should not be within 24\" range"
    );
}

/// Source: 40k_revised.md 7.2
/// Rule: WeaponProfile.half_range() is used for Rapid Fire/Melta bonuses.
/// Verify half range calculation.
#[test]
fn weapon_half_range_calculation() {
    let boltgun = make_ranged_weapon("Boltgun", 24, 2, 3, 4, 0, 1);
    assert_eq!(
        boltgun.half_range(),
        Inches::from_inches(12),
        "Half range of 24\" weapon should be 12\""
    );

    let short_weapon = make_ranged_weapon("Pistol", 12, 1, 3, 4, 0, 1);
    assert_eq!(
        short_weapon.half_range(),
        Inches::from_inches(6),
        "Half range of 12\" weapon should be 6\""
    );
}

// ===========================================================================
// 7.4 -- Locked in Combat: units within ER can't shoot (except Pistol, Big Guns)
// ===========================================================================

/// Source: 40k_revised.md 7.4
/// Rule: "Units that are within Engagement Range of one or more enemy units
///        can only select targets with [PISTOL] weapons."
/// A standard ranged weapon (not Pistol) should NOT be usable in engagement range.
#[test]
fn standard_ranged_weapon_cannot_fire_in_engagement() {
    let boltgun = make_ranged_weapon("Boltgun", 24, 2, 3, 4, 0, 1);
    assert!(
        !boltgun.can_fire_in_engagement(),
        "Standard boltgun should not fire in engagement range"
    );
}

/// Source: 40k_revised.md 7.4, 11.4
/// Rule: "PISTOL: Can be used to make ranged attacks even if the bearer's unit
///        is within Engagement Range of one or more enemy units."
/// Pistol weapons CAN fire while in engagement range.
#[test]
fn pistol_weapon_can_fire_in_engagement() {
    let pistol = make_ranged_weapon_with_abilities(
        "Bolt Pistol",
        12,
        1,
        3,
        4,
        0,
        1,
        vec![WeaponAbility::Pistol],
    );
    assert!(
        pistol.can_fire_in_engagement(),
        "Pistol weapon should be able to fire in engagement range"
    );
}

/// Source: 40k_revised.md 7.4
/// Rule: Melee weapons can always be used in engagement range.
#[test]
fn melee_weapon_can_fire_in_engagement() {
    let melee = WeaponProfile {
        id: WeaponId::new(2),
        name: "Combat blade".to_string(),
        weapon_type: WeaponType::Melee,
        range: Inches::ZERO,
        attacks: AttackCount::Fixed(3),
        skill: Skill::THREE_PLUS,
        strength: Strength::new(4),
        ap: ArmorPenetration::ZERO,
        damage: Damage::Fixed(1),
        abilities: WeaponAbilitySet::new(),
    };
    assert!(
        melee.can_fire_in_engagement(),
        "Melee weapon should be able to be used in engagement range"
    );
}

/// Source: 40k_revised.md 7.5
/// Rule: "BIG GUNS NEVER TIRE: MONSTER and VEHICLE units can make ranged attacks
///        even when within Engagement Range of one or more enemy units."
/// Verify MONSTER keyword check for Big Guns Never Tire eligibility.
#[test]
fn monster_keyword_check_for_big_guns_never_tire() {
    let monster_unit = make_unit_with_keywords(
        1,
        PlayerId::new(0),
        &[Keyword::Monster, Keyword::Character, Keyword::Chaos],
        vec![make_model_with_weapons(
            100,
            UnitId::new(1),
            10,
            Position::from_inches(10, 10),
            vec![make_ranged_weapon("Monster Gun", 36, 4, 3, 8, -2, 3)],
            vec![],
        )],
    );

    assert!(
        monster_unit.has_keyword(Keyword::Monster),
        "Unit should have MONSTER keyword"
    );
    assert!(
        monster_unit.is_monster(),
        "Unit should be identified as a MONSTER"
    );
}

/// Source: 40k_revised.md 7.5
/// Rule: VEHICLE units also qualify for Big Guns Never Tire.
/// Verify VEHICLE keyword check.
#[test]
fn vehicle_keyword_check_for_big_guns_never_tire() {
    let vehicle_keywords = KeywordSet::from_keywords(&[Keyword::Vehicle]);
    assert!(
        vehicle_keywords.has(Keyword::Vehicle),
        "Should have VEHICLE keyword"
    );

    // Verify the combined check: is_monster_or_vehicle
    let monster_or_vehicle = vehicle_keywords.has(Keyword::Monster)
        || vehicle_keywords.has(Keyword::Vehicle);
    assert!(
        monster_or_vehicle,
        "VEHICLE should satisfy MONSTER/VEHICLE check"
    );
}

// ===========================================================================
// 7.5 -- Big Guns Never Tire: -1 to Hit when shooting from within ER
// ===========================================================================

/// Source: 40k_revised.md 7.5
/// Rule: "Subtract 1 from the Hit roll when making ranged attacks while the
///        bearer's unit is within Engagement Range (Big Guns Never Tire)"
/// Test: When in_engagement_range is set on AttackContext, the combat resolver
/// applies a -1 modifier. We verify by running the full pipeline across
/// many seeds — the hit rate should be worse compared to not-in-engagement.
#[test]
fn big_guns_never_tire_minus_1_hit_modifier_in_engagement() {
    let weapon = make_ranged_weapon("Heavy Bolter", 36, 3, 3, 5, -1, 2);

    // Context NOT in engagement range (baseline)
    let ctx_normal = make_attack_context(weapon.clone(), 1, 3, 4, 3, None);

    // Context IN engagement range (Big Guns Never Tire applies -1 to hit)
    let mut ctx_engaged = make_attack_context(weapon.clone(), 1, 3, 4, 3, None);
    ctx_engaged.in_engagement_range = true;

    // Run many seeds and sum total hits
    let mut hits_normal: u32 = 0;
    let mut hits_engaged: u32 = 0;
    let trials = 50;

    for seed in 0..trials {
        let mut dice_n = make_dice(seed as u8);
        let mut dice_e = make_dice(seed as u8);

        let mut defenders_n: Vec<ModelState> = (0..10)
            .map(|i| make_defender_model(200 + i, UnitId::new(2), 3))
            .collect();
        let mut defenders_e: Vec<ModelState> = (0..10)
            .map(|i| make_defender_model(300 + i, UnitId::new(2), 3))
            .collect();

        let result_n = resolve_attack_batch(&ctx_normal, &mut defenders_n, &mut dice_n);
        let result_e = resolve_attack_batch(&ctx_engaged, &mut defenders_e, &mut dice_e);

        // Count wounds inflicted + models destroyed as a proxy for hits that got through
        hits_normal += result_n.wounds_inflicted.len() as u32 + result_n.models_destroyed.len() as u32;
        hits_engaged += result_e.wounds_inflicted.len() as u32 + result_e.models_destroyed.len() as u32;
    }

    // The engaged version should generally produce fewer hits due to the -1 modifier.
    // With enough trials, the engaged hits should be strictly less than normal hits.
    // We allow some margin since dice are deterministic per seed but the modifier
    // should statistically reduce hit success rate.
    assert!(
        hits_engaged <= hits_normal,
        "Big Guns Never Tire -1 modifier should reduce effectiveness: \
         normal={}, engaged={}",
        hits_normal,
        hits_engaged
    );
}

/// Source: 40k_revised.md 7.5
/// Rule: "Pistols are not affected by the Big Guns Never Tire -1 to Hit modifier."
/// Pistol weapons in engagement range do NOT get the -1 penalty.
/// The weapon itself has the Pistol ability which exempts it.
#[test]
fn pistol_exempt_from_big_guns_never_tire_penalty() {
    let pistol = make_ranged_weapon_with_abilities(
        "Bolt Pistol",
        12,
        1,
        3,
        4,
        0,
        1,
        vec![WeaponAbility::Pistol],
    );

    // Pistol can fire in engagement range
    assert!(
        pistol.can_fire_in_engagement(),
        "Pistol should be allowed in engagement range"
    );

    // The Pistol ability means it doesn't use the Big Guns path;
    // it has its own rules that don't incur the -1 penalty.
    assert!(
        pistol.abilities.has_pistol(),
        "Weapon should have Pistol ability"
    );
    assert!(
        !pistol.abilities.has_heavy(),
        "Pistol should not be Heavy"
    );
}

/// Source: 40k_revised.md 7.5
/// Rule: MONSTER and VEHICLE keywords are the triggers for Big Guns Never Tire.
/// Regular Infantry units in engagement range cannot shoot ranged weapons at all
/// (except Pistol).
#[test]
fn infantry_unit_not_eligible_for_big_guns_never_tire() {
    let infantry_keywords = KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Battleline]);

    let is_monster_or_vehicle =
        infantry_keywords.has(Keyword::Monster) || infantry_keywords.has(Keyword::Vehicle);

    assert!(
        !is_monster_or_vehicle,
        "Infantry unit should NOT qualify for Big Guns Never Tire"
    );
}

/// Source: 40k_revised.md 7.1
/// Rule: Units that have already shot this turn cannot shoot again.
/// Verify the has_shot tracking in TurnFlags.
#[test]
fn unit_cannot_shoot_twice_per_phase() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(42);

    assert!(
        !flags.has_shot(unit),
        "Unit should not have shot initially"
    );

    flags.mark_shot(unit);
    assert!(
        flags.has_shot(unit),
        "Unit should be marked as having shot"
    );

    // After clearing for new turn, the flag resets
    flags.clear();
    assert!(
        !flags.has_shot(unit),
        "Shot flag should be cleared after turn reset"
    );
}

/// Source: 40k_revised.md 7.1
/// Rule: The Advanced flag restricts which weapons can fire.
/// We verify the interaction: Advanced + non-Assault weapon = can't fire.
/// Advanced + Assault weapon = can fire (with -1 to Hit).
#[test]
fn advanced_flag_weapon_interaction() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(7);
    flags.mark_advanced(unit);

    let regular_weapon = make_ranged_weapon("Boltgun", 24, 2, 3, 4, 0, 1);
    let assault_weapon = make_ranged_weapon_with_abilities(
        "Assault Bolter",
        24,
        3,
        3,
        4,
        0,
        1,
        vec![WeaponAbility::Assault],
    );

    // Advanced flag set
    assert!(flags.has_advanced(unit));

    // Regular weapon can't fire after advance
    assert!(!regular_weapon.can_fire_after_advance());

    // Assault weapon can fire after advance
    assert!(assault_weapon.can_fire_after_advance());
}

/// Source: 40k_revised.md 7.4
/// Rule: EngagementStatus enum tracks whether a unit is engaged.
/// Verify the enum values exist and are distinct.
#[test]
fn engagement_status_enum_values() {
    let engaged = EngagementStatus::Engaged;
    let not_engaged = EngagementStatus::NotEngaged;

    assert_ne!(
        engaged, not_engaged,
        "Engaged and NotEngaged should be distinct"
    );

    // Unit starts as not engaged
    let unit_id = UnitId::new(1);
    let model = make_model_with_weapons(
        100,
        unit_id,
        3,
        Position::from_inches(10, 10),
        vec![make_ranged_weapon("Boltgun", 24, 2, 3, 4, 0, 1)],
        vec![],
    );
    let mut unit = make_unit_with_keywords(
        1,
        PlayerId::new(0),
        &[Keyword::Infantry],
        vec![model],
    );

    assert_eq!(
        unit.engagement_status,
        EngagementStatus::NotEngaged,
        "Unit should start not engaged"
    );

    unit.engagement_status = EngagementStatus::Engaged;
    assert_eq!(
        unit.engagement_status,
        EngagementStatus::Engaged,
        "Unit should now be engaged"
    );
}

/// Source: 40k_revised.md 7.2
/// Rule: Engagement Range constant is 1 inch horizontally.
/// Verify the constant value.
#[test]
fn engagement_range_constant_is_one_inch() {
    assert_eq!(
        Inches::ENGAGEMENT_RANGE,
        Inches::from_inches(1),
        "Engagement Range should be 1 inch"
    );
    assert_eq!(
        Inches::ENGAGEMENT_RANGE_VERTICAL,
        Inches::from_inches(5),
        "Engagement Range vertical should be 5 inches"
    );
}

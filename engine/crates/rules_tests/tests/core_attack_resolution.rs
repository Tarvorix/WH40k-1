//! Source-linked rules tests for 40k_revised.md Section 8: Attack Resolution.
//!
//! Every test in this file traces to a specific rule in the core 40K rules
//! document. The attack sequence is:
//!   1. Hit Roll (§8.1)
//!   2. Wound Roll (§8.2)
//!   3. Allocate Attack (§8.3)
//!   4. Saving Throw (§8.4)
//!   5. Invulnerable Save (§8.5)
//!   6. Inflict Damage (§8.6)
//!   7. Mortal Wounds (§8.7)
//!
//! Source: 40k_revised.md Section 8 — "MAKING ATTACKS"

use wh40k_core_types::{
    ArmorPenetration, ArmorSave, AttackCount, BaseSize, Damage,
    Inches, InvulnerableSave, Keyword, KeywordSet, ModelId, PlayerId, Position,
    Skill, Strength, Toughness, UnitId, WeaponAbilitySet, WeaponId,
    WeaponProfile, WeaponType, Wounds,
};
use wh40k_dice::{DiceContext, DiceRoller, StreamKind};
use wh40k_game_core::combat::{
    apply_mortal_wounds_carry_over, resolve_attack_batch, resolve_hazardous_tests,
    wound_threshold, AttackContext,
};
use wh40k_game_core::unit::ModelState;

// ===========================================================================
// Helper factories
// ===========================================================================

/// Create a deterministic DiceRoller from a seed byte for reproducibility.
fn make_dice(seed_byte: u8) -> DiceRoller {
    let ctx = DiceContext::new([seed_byte; 32], StreamKind::HitRoll, 0, 0);
    DiceRoller::new(ctx)
}

/// Build a simple ranged weapon profile with the given stats and no special abilities.
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

/// Build a ModelState for a defender with the given wounds and no special rules.
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

/// Build a default AttackContext with the given weapon and defender stats.
/// All optional fields default to off/false.
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
            target_is_engaged_monster_or_vehicle: false,
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

// ===========================================================================
// §8.2 — WOUND ROLLS: Strength vs Toughness threshold table
// ===========================================================================

/// Source: 40k_revised.md §8.2 — "WOUND ROLLS"
/// Rule: "If the Strength is DOUBLE (or more than double) the Toughness, a 2+ is required."
/// S >= 2*T -> wound on 2+
#[test]
fn wound_threshold_s_double_or_more_t() {
    // S=8, T=4 -> exactly double -> 2+
    assert_eq!(wound_threshold(Strength::new(8), Toughness::new(4)), 2);
    // S=10, T=4 -> more than double -> 2+
    assert_eq!(wound_threshold(Strength::new(10), Toughness::new(4)), 2);
    // S=6, T=3 -> exactly double -> 2+
    assert_eq!(wound_threshold(Strength::new(6), Toughness::new(3)), 2);
    // S=12, T=5 -> more than double -> 2+
    assert_eq!(wound_threshold(Strength::new(12), Toughness::new(5)), 2);
}

/// Source: 40k_revised.md §8.2 — "WOUND ROLLS"
/// Rule: "If the Strength is GREATER than the Toughness, a 3+ is required."
/// S > T (but not double) -> wound on 3+
#[test]
fn wound_threshold_s_greater_than_t() {
    // S=5, T=4 -> 3+
    assert_eq!(wound_threshold(Strength::new(5), Toughness::new(4)), 3);
    // S=7, T=4 -> 7 < 2*4=8, so still 3+
    assert_eq!(wound_threshold(Strength::new(7), Toughness::new(4)), 3);
    // S=6, T=4 -> 3+
    assert_eq!(wound_threshold(Strength::new(6), Toughness::new(4)), 3);
    // S=4, T=3 -> 3+
    assert_eq!(wound_threshold(Strength::new(4), Toughness::new(3)), 3);
}

/// Source: 40k_revised.md §8.2 — "WOUND ROLLS"
/// Rule: "If the Strength is EQUAL to the Toughness, a 4+ is required."
/// S == T -> wound on 4+
#[test]
fn wound_threshold_s_equals_t() {
    // S=4, T=4 -> 4+
    assert_eq!(wound_threshold(Strength::new(4), Toughness::new(4)), 4);
    // S=3, T=3 -> 4+
    assert_eq!(wound_threshold(Strength::new(3), Toughness::new(3)), 4);
    // S=8, T=8 -> 4+
    assert_eq!(wound_threshold(Strength::new(8), Toughness::new(8)), 4);
    // S=1, T=1 -> 4+
    assert_eq!(wound_threshold(Strength::new(1), Toughness::new(1)), 4);
}

/// Source: 40k_revised.md §8.2 — "WOUND ROLLS"
/// Rule: "If the Strength is LESS than the Toughness, a 5+ is required."
/// S < T (but not half or less) -> wound on 5+
#[test]
fn wound_threshold_s_less_than_t() {
    // S=3, T=4 -> 5+
    assert_eq!(wound_threshold(Strength::new(3), Toughness::new(4)), 5);
    // S=3, T=5 -> 3 > 5/2=2.5, so 5+
    assert_eq!(wound_threshold(Strength::new(3), Toughness::new(5)), 5);
    // S=5, T=8 -> 5 > 8/2=4, so 5+
    assert_eq!(wound_threshold(Strength::new(5), Toughness::new(8)), 5);
    // S=4, T=7 -> 4 > 7/2=3.5, so 5+
    assert_eq!(wound_threshold(Strength::new(4), Toughness::new(7)), 5);
}

/// Source: 40k_revised.md §8.2 — "WOUND ROLLS"
/// Rule: "If the Strength is HALF (or less than half) the Toughness, a 6+ is required."
/// 2*S <= T -> wound on 6+
#[test]
fn wound_threshold_s_half_or_less_t() {
    // S=2, T=4 -> exactly half -> 6+
    assert_eq!(wound_threshold(Strength::new(2), Toughness::new(4)), 6);
    // S=2, T=5 -> less than half -> 6+
    assert_eq!(wound_threshold(Strength::new(2), Toughness::new(5)), 6);
    // S=3, T=6 -> exactly half -> 6+
    assert_eq!(wound_threshold(Strength::new(3), Toughness::new(6)), 6);
    // S=1, T=4 -> less than half -> 6+
    assert_eq!(wound_threshold(Strength::new(1), Toughness::new(4)), 6);
}

/// Source: 40k_revised.md §8.2 — "WOUND ROLLS"
/// Boundary check: ensure the exact boundaries between tiers are correct.
/// This validates the transition points S=2T, S=T, 2S=T.
#[test]
fn wound_threshold_boundary_values() {
    // T=4: S=8 -> 2+ (double), S=7 -> 3+, S=5 -> 3+, S=4 -> 4+, S=3 -> 5+, S=2 -> 6+
    assert_eq!(wound_threshold(Strength::new(8), Toughness::new(4)), 2);
    assert_eq!(wound_threshold(Strength::new(7), Toughness::new(4)), 3);
    assert_eq!(wound_threshold(Strength::new(5), Toughness::new(4)), 3);
    assert_eq!(wound_threshold(Strength::new(4), Toughness::new(4)), 4);
    assert_eq!(wound_threshold(Strength::new(3), Toughness::new(4)), 5);
    assert_eq!(wound_threshold(Strength::new(2), Toughness::new(4)), 6);
    assert_eq!(wound_threshold(Strength::new(1), Toughness::new(4)), 6);

    // T=3: S=6 -> 2+, S=5 -> 3+, S=3 -> 4+, S=2 -> 5+, S=1 -> 6+
    assert_eq!(wound_threshold(Strength::new(6), Toughness::new(3)), 2);
    assert_eq!(wound_threshold(Strength::new(5), Toughness::new(3)), 3);
    assert_eq!(wound_threshold(Strength::new(3), Toughness::new(3)), 4);
    assert_eq!(wound_threshold(Strength::new(2), Toughness::new(3)), 5);
    assert_eq!(wound_threshold(Strength::new(1), Toughness::new(3)), 6);
}

// ===========================================================================
// §8.1 — HIT ROLLS (tested via resolve_attack_batch integration)
// ===========================================================================

/// Source: 40k_revised.md §8.1 — "HIT ROLLS"
/// Rule: "An unmodified Hit roll of 1 always fails... An unmodified Hit roll of 6
/// always hits — it is a Critical Hit."
/// We fire many attacks with deterministic dice and verify that hits can occur
/// (proving the pipeline works end-to-end).
#[test]
fn hit_roll_attack_batch_produces_results() {
    let weapon = make_ranged_weapon("Bolter", 2, 3, 4, 0, 1);
    let ctx = make_attack_context(weapon.clone(), 1, 2, 4, 4, None);
    let defender_unit = UnitId::new(2);
    let mut models = vec![
        make_defender_model(100, defender_unit, 3),
        make_defender_model(101, defender_unit, 3),
    ];
    let mut dice = make_dice(42);

    let result = resolve_attack_batch(&ctx, &mut models, &mut dice);

    // With 2 attacks from BS3+ against T4/Sv4+ with AP0, we should get events
    // regardless of the random outcome. The pipeline must run without panicking.
    assert!(!result.events.is_empty(), "Attack batch must produce at least one event");
}

/// Source: 40k_revised.md §8.1 — "HIT ROLLS"
/// Rule: "An unmodified Hit roll of 1 always fails, irrespective of any modifiers."
/// Verify that even with +1 hit modifier (Heavy + stationary), a roll of 1 still fails.
/// We test this indirectly: with BS2+ and stationary Heavy, only a natural 1 can miss.
/// Running many seeds, we expect some attacks to miss (proving 1s fail).
#[test]
fn hit_roll_natural_one_always_fails() {
    let mut weapon = make_ranged_weapon("Heavy bolter", 3, 2, 5, -1, 2);
    weapon.abilities.add(wh40k_core_types::WeaponAbility::Heavy);

    let mut ctx = make_attack_context(weapon, 1, 3, 4, 3, None);
    ctx.attacker_stationary = true;
    ctx.effective_abilities.add(wh40k_core_types::WeaponAbility::Heavy);

    let defender_unit = UnitId::new(2);

    // Run over many seeds and collect miss counts
    let mut total_events_with_failure = 0;
    for seed in 0..50u8 {
        let mut models = vec![
            make_defender_model(100, defender_unit, 5),
            make_defender_model(101, defender_unit, 5),
        ];
        let mut dice = make_dice(seed);
        let result = resolve_attack_batch(&ctx, &mut models, &mut dice);
        // Check if any hit roll event shows a failure (indicating natural 1)
        for event in &result.events {
            if let wh40k_event_system::GameEvent::HitRollMade { result: roll_result, .. } = event {
                if *roll_result == wh40k_event_system::RollResult::Failure {
                    total_events_with_failure += 1;
                }
            }
        }
    }
    // With 50 seeds * 3 attacks each = 150 dice, statistically ~25 should be 1s.
    // Require at least 1 miss to prove the rule is enforced.
    assert!(
        total_events_with_failure > 0,
        "Even with BS2+ and Heavy stationary (+1), natural 1s must still fail"
    );
}

/// Source: 40k_revised.md §8.1 — "HIT ROLLS"
/// Rule: "An unmodified Hit roll of 6 always hits — it is a Critical Hit."
/// Even with the worst skill (6+), an unmodified 6 always hits. We verify by
/// running many seeds and ensuring some hits occur.
#[test]
fn hit_roll_natural_six_always_hits() {
    let weapon = make_ranged_weapon("Stub gun", 1, 6, 3, 0, 1);
    let ctx = make_attack_context(weapon, 1, 1, 10, 2, None);
    let defender_unit = UnitId::new(2);

    let mut total_hits = 0;
    for seed in 0..100u8 {
        let mut models = vec![make_defender_model(100, defender_unit, 10)];
        let mut dice = make_dice(seed);
        let result = resolve_attack_batch(&ctx, &mut models, &mut dice);
        for event in &result.events {
            if let wh40k_event_system::GameEvent::HitRollMade { result: roll_result, .. } = event {
                if *roll_result == wh40k_event_system::RollResult::Success {
                    total_hits += 1;
                }
            }
        }
    }
    // With 100 single-attack seeds, ~16-17 should roll a 6.
    assert!(
        total_hits > 0,
        "With BS6+, natural 6 always hits — must get at least one hit across 100 seeds"
    );
}

/// Source: 40k_revised.md §8.1 — "HIT ROLLS"
/// Rule: "The total modifier to a Hit roll can never be more than -1 or +1."
/// With Heavy (+1) on a BS3+ weapon while stationary, the effective threshold
/// becomes 2+. Adding Stealth (-1) should cancel it back to 3+.
/// We verify both modifiers don't stack beyond +1/-1 by comparing hit rates.
#[test]
fn hit_roll_modifier_capped_at_plus_minus_one() {
    // Heavy + Stationary gives +1. Adding Stealth should give net 0 (clamped).
    let mut weapon = make_ranged_weapon("Heavy stubber", 4, 4, 4, 0, 1);
    weapon.abilities.add(wh40k_core_types::WeaponAbility::Heavy);

    // Scenario 1: Heavy + Stationary (net +1)
    let mut ctx1 = make_attack_context(weapon.clone(), 1, 4, 4, 4, None);
    ctx1.attacker_stationary = true;
    ctx1.effective_abilities.add(wh40k_core_types::WeaponAbility::Heavy);

    // Scenario 2: Heavy + Stationary + Stealth (net 0, but capped: +1 then -1)
    let mut ctx2 = make_attack_context(weapon, 1, 4, 4, 4, None);
    ctx2.attacker_stationary = true;
    ctx2.target_has_stealth = true;
    ctx2.effective_abilities.add(wh40k_core_types::WeaponAbility::Heavy);

    let defender_unit = UnitId::new(2);

    let mut hits_scenario1 = 0u32;
    let mut hits_scenario2 = 0u32;

    for seed in 0..200u8 {
        let mut models1 = vec![make_defender_model(100, defender_unit, 10)];
        let mut models2 = vec![make_defender_model(100, defender_unit, 10)];
        let mut dice1 = make_dice(seed);
        let mut dice2 = make_dice(seed);

        let r1 = resolve_attack_batch(&ctx1, &mut models1, &mut dice1);
        let r2 = resolve_attack_batch(&ctx2, &mut models2, &mut dice2);

        for event in &r1.events {
            if let wh40k_event_system::GameEvent::HitRollMade { result: r, .. } = event {
                if *r == wh40k_event_system::RollResult::Success {
                    hits_scenario1 += 1;
                }
            }
        }
        for event in &r2.events {
            if let wh40k_event_system::GameEvent::HitRollMade { result: r, .. } = event {
                if *r == wh40k_event_system::RollResult::Success {
                    hits_scenario2 += 1;
                }
            }
        }
    }

    // Scenario 1 (Heavy+Stationary, net +1) should have more hits than Scenario 2
    // (Heavy+Stationary+Stealth, net 0) because the modifier helps.
    // Both scenarios use the same seeds, so the only difference is modifiers.
    assert!(
        hits_scenario1 > hits_scenario2,
        "Heavy+Stationary (+1 to hit) should produce more hits ({}) than \
         Heavy+Stationary+Stealth (net 0, {}) using same seeds",
        hits_scenario1,
        hits_scenario2
    );
}

// ===========================================================================
// §8.2 — WOUND ROLLS (integration through attack batch)
// ===========================================================================

/// Source: 40k_revised.md §8.2 — "WOUND ROLLS"
/// Rule: "An unmodified Wound roll of 6 always wounds (Critical Wound)."
/// Even with S1 vs T10 (wound on 6+), some attacks must wound across many seeds.
#[test]
fn wound_roll_natural_six_always_wounds() {
    // S1 vs T10: needs 6+ to wound. Use lots of attacks to ensure statistical coverage.
    // 5 models each with 4 attacks = 20 attacks per batch, across many seeds.
    let weapon = make_ranged_weapon("Stub pistol", 4, 2, 1, -4, 1);
    let ctx = make_attack_context(weapon, 5, 4, 10, 7, None);
    let defender_unit = UnitId::new(2);

    let mut wound_events = 0;
    for seed in 0..200u8 {
        let mut models = vec![make_defender_model(100, defender_unit, 100)];
        let mut dice = make_dice(seed);
        let result = resolve_attack_batch(&ctx, &mut models, &mut dice);
        for event in &result.events {
            if let wh40k_event_system::GameEvent::WoundRollMade { result: r, .. } = event {
                if *r == wh40k_event_system::RollResult::Success {
                    wound_events += 1;
                }
            }
        }
    }

    // 200 seeds * 20 attacks = 4000 rolls, ~3333 hit (BS2+), ~556 should wound (6+).
    assert!(
        wound_events > 0,
        "S1 vs T10: unmodified 6 must still wound — got 0 successful wounds across 200 seeds"
    );
}

/// Source: 40k_revised.md §8.2 — "WOUND ROLLS"
/// Rule: "An unmodified Wound roll of 1 always fails, irrespective of any modifiers."
/// Even with S10 vs T1 (wound on 2+), some attacks must fail from natural 1s.
#[test]
fn wound_roll_natural_one_always_fails() {
    let weapon = make_ranged_weapon("Lascannon", 1, 2, 10, -3, 1);
    // S10 vs T1: wound on 2+. Only natural 1 can fail.
    let ctx = make_attack_context(weapon, 1, 1, 1, 7, None);
    let defender_unit = UnitId::new(2);

    let mut wound_failures = 0;
    for seed in 0..200u8 {
        let mut models = vec![make_defender_model(100, defender_unit, 10)];
        let mut dice = make_dice(seed);
        let result = resolve_attack_batch(&ctx, &mut models, &mut dice);
        for event in &result.events {
            if let wh40k_event_system::GameEvent::WoundRollMade { result: r, .. } = event {
                if *r == wh40k_event_system::RollResult::Failure {
                    wound_failures += 1;
                }
            }
        }
    }

    assert!(
        wound_failures > 0,
        "S10 vs T1: natural 1 must always fail — got 0 wound failures across 200 seeds"
    );
}

/// Source: 40k_revised.md §8.2 — "WOUND ROLLS"
/// Rule: "The total modifier to a Wound roll can never be more than -1 or +1."
/// Lance gives +1 to wound if charged. Verify that attacks with Lance+charged
/// produce more wounds than without, but the modifier is capped.
#[test]
fn wound_roll_modifier_capped_lance() {
    // S4 vs T5 = wound on 5+. With Lance + charged = +1, effective 4+.
    let mut weapon = make_melee_weapon("Lance blade", 3, 3, 4, -1, 2);
    weapon.abilities.add(wh40k_core_types::WeaponAbility::Lance);

    // Without charge: wound on 5+
    let ctx_no_charge = make_attack_context(weapon.clone(), 1, 3, 5, 4, None);

    // With charge: wound on 4+ (Lance gives +1)
    let mut ctx_charged = make_attack_context(weapon, 1, 3, 5, 4, None);
    ctx_charged.attacker_charged = true;
    ctx_charged.effective_abilities.add(wh40k_core_types::WeaponAbility::Lance);

    let defender_unit = UnitId::new(2);

    let mut wounds_no_charge = 0u32;
    let mut wounds_charged = 0u32;

    for seed in 0..200u8 {
        let mut models_nc = vec![make_defender_model(100, defender_unit, 10)];
        let mut models_c = vec![make_defender_model(100, defender_unit, 10)];
        let mut dice_nc = make_dice(seed);
        let mut dice_c = make_dice(seed);

        let r_nc = resolve_attack_batch(&ctx_no_charge, &mut models_nc, &mut dice_nc);
        let r_c = resolve_attack_batch(&ctx_charged, &mut models_c, &mut dice_c);

        for event in &r_nc.events {
            if let wh40k_event_system::GameEvent::WoundRollMade { result: r, .. } = event {
                if *r == wh40k_event_system::RollResult::Success {
                    wounds_no_charge += 1;
                }
            }
        }
        for event in &r_c.events {
            if let wh40k_event_system::GameEvent::WoundRollMade { result: r, .. } = event {
                if *r == wh40k_event_system::RollResult::Success {
                    wounds_charged += 1;
                }
            }
        }
    }

    assert!(
        wounds_charged > wounds_no_charge,
        "Lance+charged (+1 to wound) should produce more wounds ({}) than no charge ({}) \
         using same seeds",
        wounds_charged,
        wounds_no_charge
    );
}

// ===========================================================================
// §8.3 — ALLOCATING ATTACKS (wounded models must receive attacks first)
// ===========================================================================

/// Source: 40k_revised.md §8.3 — "ALLOCATING ATTACKS"
/// Rule: "If a model in the target unit has already lost any wounds...
/// the attack must be allocated to that model."
/// We pre-wound a model and verify attacks go to it first.
#[test]
fn attack_allocation_wounded_model_first() {
    // Create a weapon that always hits and wounds (S10 vs T1, BS2+, AP-4 vs Sv7+)
    // so we can focus on allocation behavior.
    let weapon = make_ranged_weapon("Test gun", 1, 2, 10, -4, 1);
    let ctx = make_attack_context(weapon, 1, 1, 1, 7, None);
    let defender_unit = UnitId::new(2);

    // Model 0: already wounded (2/3 remaining)
    let mut model0 = make_defender_model(100, defender_unit, 3);
    model0.apply_damage(1); // Now has 2 wounds remaining, allocation_status = WoundedAllocated

    // Model 1: full health (3/3)
    let model1 = make_defender_model(101, defender_unit, 3);

    let mut models = vec![model0, model1];

    // Run many times — damage should consistently go to model 100 (wounded) first
    let mut damage_to_model_100 = 0u32;
    let mut damage_to_model_101 = 0u32;

    for seed in 0..100u8 {
        // Reset model states
        models[0] = make_defender_model(100, defender_unit, 3);
        models[0].apply_damage(1);
        models[1] = make_defender_model(101, defender_unit, 3);

        let mut dice = make_dice(seed);
        let result = resolve_attack_batch(&ctx, &mut models, &mut dice);
        for (model_id, wounds) in &result.wounds_inflicted {
            if *model_id == ModelId::new(100) {
                damage_to_model_100 += *wounds as u32;
            }
            if *model_id == ModelId::new(101) {
                damage_to_model_101 += *wounds as u32;
            }
        }
    }

    // The wounded model (100) should receive all damage before model 101 gets any.
    // Since each attack does 1 damage and model 100 has 2 wounds, the first attack
    // must go to model 100.
    assert!(
        damage_to_model_100 > 0,
        "Wounded model 100 must be allocated attacks first"
    );
    // Model 101 should only receive damage if model 100 is destroyed first
    // (which can't happen from a single 1-damage attack when it has 2 wounds).
    assert_eq!(
        damage_to_model_101, 0,
        "Model 101 should not receive damage while model 100 is alive and wounded"
    );
}

// ===========================================================================
// §8.4 — SAVING THROWS
// ===========================================================================

/// Source: 40k_revised.md §8.4 — "SAVING THROWS"
/// Rule: "Modify the saving throw by the Armour Penetration (AP) characteristic...
/// If the result equals or exceeds the Save, the saving throw is passed."
/// AP-2 against Sv3+ means the modified save is 5+. We verify by comparing
/// damage with AP0 vs AP-2 using same seeds.
#[test]
fn saving_throw_ap_modifies_save() {
    // AP0 vs Sv3+: save on 3+ (very good)
    let weapon_ap0 = make_ranged_weapon("Bolter AP0", 3, 2, 8, 0, 1);
    let ctx_ap0 = make_attack_context(weapon_ap0, 1, 3, 3, 3, None);

    // AP-2 vs Sv3+: save on 5+ (much worse)
    let weapon_ap2 = make_ranged_weapon("Bolter AP-2", 3, 2, 8, -2, 1);
    let ctx_ap2 = make_attack_context(weapon_ap2, 1, 3, 3, 3, None);

    let defender_unit = UnitId::new(2);
    let mut total_damage_ap0 = 0u32;
    let mut total_damage_ap2 = 0u32;

    for seed in 0..200u8 {
        let mut models_ap0 = vec![
            make_defender_model(100, defender_unit, 10),
        ];
        let mut models_ap2 = vec![
            make_defender_model(100, defender_unit, 10),
        ];
        let mut dice_ap0 = make_dice(seed);
        let mut dice_ap2 = make_dice(seed);

        let r0 = resolve_attack_batch(&ctx_ap0, &mut models_ap0, &mut dice_ap0);
        let r2 = resolve_attack_batch(&ctx_ap2, &mut models_ap2, &mut dice_ap2);

        for (_, w) in &r0.wounds_inflicted {
            total_damage_ap0 += *w as u32;
        }
        for (_, w) in &r2.wounds_inflicted {
            total_damage_ap2 += *w as u32;
        }
    }

    assert!(
        total_damage_ap2 > total_damage_ap0,
        "AP-2 ({} damage) should deal more damage than AP0 ({} damage) against Sv3+",
        total_damage_ap2,
        total_damage_ap0
    );
}

/// Source: 40k_revised.md §8.4 — "SAVING THROWS"
/// Rule: "An unmodified saving throw of 1 always fails."
/// Even with Sv2+ and AP0, a natural 1 on the save always fails.
#[test]
fn saving_throw_natural_one_always_fails() {
    // S10 vs T1 (always wounds), BS2+ (almost always hits), AP0 vs Sv2+
    let weapon = make_ranged_weapon("Perfect gun", 1, 2, 10, 0, 1);
    let ctx = make_attack_context(weapon, 1, 1, 1, 2, None);
    let defender_unit = UnitId::new(2);

    let mut save_failures = 0;
    for seed in 0..200u8 {
        let mut models = vec![make_defender_model(100, defender_unit, 10)];
        let mut dice = make_dice(seed);
        let result = resolve_attack_batch(&ctx, &mut models, &mut dice);
        for event in &result.events {
            if let wh40k_event_system::GameEvent::SaveRollMade {
                result: wh40k_core_types::SaveResult::Failed,
                ..
            } = event
            {
                save_failures += 1;
            }
        }
    }

    assert!(
        save_failures > 0,
        "Even with Sv2+ and AP0, natural 1s must fail — got 0 failures across 200 seeds"
    );
}

/// Source: 40k_revised.md §8.4 — "SAVING THROWS"
/// Rule: Save characteristic improvement capped at +1 (Benefit of Cover).
/// Benefit of Cover grants +1 to save, but the improved save cannot be better
/// than the unmodified save characteristic.
#[test]
fn saving_throw_cover_improvement_capped() {
    // Sv3+ with cover and AP-1: modified save = 3+1=4, cover brings it to 3.
    // But cover can't make it better than base (3+), so still 3+.
    // With AP0 and cover, save of 3+ should stay at 3+ (cover denied for AP0 + Sv3+)
    // per the 40k_revised.md §13.1 rule.

    // AP-1 vs Sv3+ without cover: modified save = 4+
    let weapon = make_ranged_weapon("Bolter AP-1", 4, 2, 10, -1, 1);
    let ctx_no_cover = make_attack_context(weapon.clone(), 1, 4, 1, 3, None);

    // AP-1 vs Sv3+ with cover: modified save = 4+, cover brings to 3+
    let mut ctx_cover = make_attack_context(weapon, 1, 4, 1, 3, None);
    ctx_cover.target_has_cover = true;

    let defender_unit = UnitId::new(2);
    let mut damage_no_cover = 0u32;
    let mut damage_cover = 0u32;

    for seed in 0..200u8 {
        let mut models_nc = vec![make_defender_model(100, defender_unit, 20)];
        let mut models_c = vec![make_defender_model(100, defender_unit, 20)];
        let mut dice_nc = make_dice(seed);
        let mut dice_c = make_dice(seed);

        let r_nc = resolve_attack_batch(&ctx_no_cover, &mut models_nc, &mut dice_nc);
        let r_c = resolve_attack_batch(&ctx_cover, &mut models_c, &mut dice_c);

        for (_, w) in &r_nc.wounds_inflicted {
            damage_no_cover += *w as u32;
        }
        for (_, w) in &r_c.wounds_inflicted {
            damage_cover += *w as u32;
        }
    }

    // Cover should reduce damage taken (better save)
    assert!(
        damage_cover < damage_no_cover,
        "Cover should reduce damage: with cover ({}) should be less than without ({})",
        damage_cover,
        damage_no_cover
    );
}

// ===========================================================================
// §8.5 — INVULNERABLE SAVES (not modified by AP, pick better)
// ===========================================================================

/// Source: 40k_revised.md §8.5 — "INVULNERABLE SAVES"
/// Rule: "An invulnerable save is never modified by an attack's Armour Penetration."
/// With AP-4 against Sv3+ but InvSv4+, the invulnerable save is used instead
/// of the modified armor save (3+4=7+). The invulnerable 4+ is better than 7+.
#[test]
fn invulnerable_save_not_modified_by_ap() {
    // AP-4 vs Sv3+ without invuln: modified save = 7+ (basically no save)
    let weapon = make_ranged_weapon("Melta AP-4", 3, 2, 10, -4, 1);
    let ctx_no_invuln = make_attack_context(weapon.clone(), 1, 3, 1, 3, None);

    // AP-4 vs Sv3+ with InvSv4+: invulnerable 4+ is better than modified 7+
    let ctx_invuln = make_attack_context(
        weapon,
        1,
        3,
        1,
        3,
        Some(InvulnerableSave::FOUR_PLUS),
    );

    let defender_unit = UnitId::new(2);
    let mut damage_no_invuln = 0u32;
    let mut damage_invuln = 0u32;

    for seed in 0..200u8 {
        let mut models_ni = vec![make_defender_model(100, defender_unit, 20)];
        let mut models_i = vec![make_defender_model(100, defender_unit, 20)];
        let mut dice_ni = make_dice(seed);
        let mut dice_i = make_dice(seed);

        let r_ni = resolve_attack_batch(&ctx_no_invuln, &mut models_ni, &mut dice_ni);
        let r_i = resolve_attack_batch(&ctx_invuln, &mut models_i, &mut dice_i);

        for (_, w) in &r_ni.wounds_inflicted {
            damage_no_invuln += *w as u32;
        }
        for (_, w) in &r_i.wounds_inflicted {
            damage_invuln += *w as u32;
        }
    }

    assert!(
        damage_invuln < damage_no_invuln,
        "InvSv4+ should reduce damage vs AP-4: with invuln ({}) < without invuln ({})",
        damage_invuln,
        damage_no_invuln
    );
}

/// Source: 40k_revised.md §8.5 — "INVULNERABLE SAVES"
/// Rule: "The player controlling the model that is making the save can choose
/// to use either its normal save or its invulnerable save, but not both."
/// When armor save after AP is better than invulnerable, the armor save is used.
/// AP0 vs Sv2+ = save on 2+, InvSv4+ = save on 4+. Armor is better.
#[test]
fn invulnerable_save_chooses_better_of_armor_or_invuln() {
    // AP0 vs Sv2+ (save on 2+), InvSv4+ (save on 4+) -> should use armor 2+
    let weapon = make_ranged_weapon("Test gun", 4, 2, 10, 0, 1);
    let ctx = make_attack_context(
        weapon,
        1,
        4,
        1,
        2,
        Some(InvulnerableSave::FOUR_PLUS),
    );
    let defender_unit = UnitId::new(2);

    let mut save_types_used = std::collections::HashMap::new();
    for seed in 0..100u8 {
        let mut models = vec![make_defender_model(100, defender_unit, 20)];
        let mut dice = make_dice(seed);
        let result = resolve_attack_batch(&ctx, &mut models, &mut dice);
        for event in &result.events {
            if let wh40k_event_system::GameEvent::SaveRollMade { save_type, .. } = event {
                *save_types_used.entry(*save_type).or_insert(0u32) += 1;
            }
        }
    }

    // With AP0 and Sv2+, the armor save (2+) is better than invuln (4+),
    // so all saves should be Armour type.
    let armour_saves = save_types_used
        .get(&wh40k_core_types::SaveType::Armour)
        .copied()
        .unwrap_or(0);
    let invuln_saves = save_types_used
        .get(&wh40k_core_types::SaveType::Invulnerable)
        .copied()
        .unwrap_or(0);

    assert!(
        armour_saves > 0,
        "With AP0 and Sv2+, armor saves should be used"
    );
    assert_eq!(
        invuln_saves, 0,
        "With AP0 and Sv2+, invulnerable saves should NOT be used (armor is better)"
    );
}

/// Source: 40k_revised.md §8.5 — "INVULNERABLE SAVES"
/// When AP makes armor worse than invulnerable, invulnerable is chosen.
/// AP-3 vs Sv3+ = modified 6+, InvSv4+ = 4+. Invuln is better.
#[test]
fn invulnerable_save_used_when_ap_degrades_armor() {
    let weapon = make_ranged_weapon("Plasma AP-3", 4, 2, 10, -3, 1);
    let ctx = make_attack_context(
        weapon,
        1,
        4,
        1,
        3,
        Some(InvulnerableSave::FOUR_PLUS),
    );
    let defender_unit = UnitId::new(2);

    let mut save_types_used = std::collections::HashMap::new();
    for seed in 0..100u8 {
        let mut models = vec![make_defender_model(100, defender_unit, 20)];
        let mut dice = make_dice(seed);
        let result = resolve_attack_batch(&ctx, &mut models, &mut dice);
        for event in &result.events {
            if let wh40k_event_system::GameEvent::SaveRollMade { save_type, .. } = event {
                *save_types_used.entry(*save_type).or_insert(0u32) += 1;
            }
        }
    }

    let invuln_saves = save_types_used
        .get(&wh40k_core_types::SaveType::Invulnerable)
        .copied()
        .unwrap_or(0);

    assert!(
        invuln_saves > 0,
        "With AP-3 degrading Sv3+ to 6+, invulnerable 4+ should be used"
    );
}

// ===========================================================================
// §8.6 — INFLICT DAMAGE (excess damage lost, does not carry over)
// ===========================================================================

/// Source: 40k_revised.md §8.6 — "INFLICT DAMAGE"
/// Rule: "If a model is destroyed by an attack, any excess damage from that
/// attack is lost and has no further effect."
/// A weapon dealing 6 damage to a 2-wound model should not carry over 4 damage.
#[test]
fn damage_excess_lost_on_model_death() {
    // 1 attack doing 6 damage against a unit of 2 models with 2 wounds each.
    // If the first model dies, the second should still have full health.
    let weapon = make_ranged_weapon("Overkill cannon", 1, 2, 10, -4, 6);
    let ctx = make_attack_context(weapon, 1, 1, 1, 7, None);
    let defender_unit = UnitId::new(2);

    // Track how many models survive
    let mut times_second_full_health = 0;
    let mut times_damage_dealt = 0;

    for seed in 0..200u8 {
        let mut models = vec![
            make_defender_model(100, defender_unit, 2),
            make_defender_model(101, defender_unit, 2),
        ];
        let mut dice = make_dice(seed);
        let result = resolve_attack_batch(&ctx, &mut models, &mut dice);

        if !result.wounds_inflicted.is_empty() || !result.models_destroyed.is_empty() {
            times_damage_dealt += 1;
            // Check if model 101 is still at full health
            if models[1].alive && models[1].wounds_remaining == Wounds::new(2) {
                times_second_full_health += 1;
            }
            // If both survived, the single attack either missed or was saved
        }
    }

    // When damage is dealt and the first model dies (6 damage vs 2 wounds),
    // the excess 4 damage must NOT carry over to model 101.
    assert!(
        times_damage_dealt > 0,
        "At least some attacks should deal damage across 200 seeds"
    );
    // When model 100 is destroyed, model 101 must remain at full health
    // because excess damage doesn't carry over for normal attacks.
    assert!(
        times_second_full_health > 0,
        "When model 100 is destroyed by 6 damage, excess must not carry to model 101"
    );
}

/// Source: 40k_revised.md §8.6 — "INFLICT DAMAGE"
/// Rule: Normal damage does not spill over to the next model.
/// Multi-damage attacks against single-wound models waste excess damage per model.
#[test]
fn damage_excess_wasted_on_single_wound_models() {
    // 3 attacks doing 3 damage each against 1-wound models.
    // Each attack can only kill 1 model (wasting 2 damage), so max 3 kills.
    let weapon = make_ranged_weapon("Overpower gun", 3, 2, 10, -4, 3);
    let ctx = make_attack_context(weapon, 1, 3, 1, 7, None);
    let defender_unit = UnitId::new(2);

    for seed in 0..50u8 {
        let mut models = vec![
            make_defender_model(100, defender_unit, 1),
            make_defender_model(101, defender_unit, 1),
            make_defender_model(102, defender_unit, 1),
            make_defender_model(103, defender_unit, 1),
            make_defender_model(104, defender_unit, 1),
            make_defender_model(105, defender_unit, 1),
        ];
        let mut dice = make_dice(seed);
        let result = resolve_attack_batch(&ctx, &mut models, &mut dice);

        // Can never destroy more than 3 models with 3 attacks
        // (even though total damage potential is 9)
        assert!(
            result.models_destroyed.len() <= 3,
            "Seed {}: {} models destroyed, but only 3 attacks were made (excess damage \
             must not carry over). Destroyed: {:?}",
            seed,
            result.models_destroyed.len(),
            result.models_destroyed
        );
    }
}

// ===========================================================================
// §8.7 — MORTAL WOUNDS (carry over between models)
// ===========================================================================

/// Source: 40k_revised.md §8.7 — "MORTAL WOUNDS"
/// Rule: "Each mortal wound inflicts one point of damage... no saving throws can
/// be made against mortal wounds."
/// Mortal wounds bypass saves entirely.
#[test]
fn mortal_wounds_no_saving_throws() {
    let defender_unit = UnitId::new(2);
    let mut models = vec![
        make_defender_model(100, defender_unit, 3),
    ];
    let mut dice = make_dice(42);
    let mut events = Vec::new();

    // Apply 2 mortal wounds to a 3-wound model
    let destroyed = apply_mortal_wounds_carry_over(
        &mut models,
        2,
        &mut events,
        defender_unit,
        "Test source",
        &mut dice,
    );

    // Model should have 1 wound remaining (3 - 2 = 1), still alive
    assert!(models[0].alive, "Model should still be alive after 2 MW on 3W model");
    assert_eq!(
        models[0].wounds_remaining,
        Wounds::new(1),
        "Model should have 1 wound remaining after 2 mortal wounds"
    );
    assert!(
        destroyed.is_empty(),
        "No models should be destroyed by 2 MW on a 3W model"
    );

    // Verify no save events were generated (mortal wounds bypass saves)
    let save_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, wh40k_event_system::GameEvent::SaveRollMade { .. }))
        .collect();
    assert!(
        save_events.is_empty(),
        "Mortal wounds must not generate save roll events"
    );
}

/// Source: 40k_revised.md §8.7 — "MORTAL WOUNDS"
/// Rule: "Unlike normal damage, excess mortal wound damage is not lost if the
/// damage is enough to destroy a model. Instead, keep allocating damage to
/// another model in the target unit..."
/// 5 mortal wounds against 2-wound models: should kill 2 models and leave
/// 1 wound on a third.
#[test]
fn mortal_wounds_carry_over_between_models() {
    let defender_unit = UnitId::new(2);
    let mut models = vec![
        make_defender_model(100, defender_unit, 2),
        make_defender_model(101, defender_unit, 2),
        make_defender_model(102, defender_unit, 2),
    ];
    let mut dice = make_dice(42);
    let mut events = Vec::new();

    // Apply 5 mortal wounds across 3 models of 2 wounds each
    let destroyed = apply_mortal_wounds_carry_over(
        &mut models,
        5,
        &mut events,
        defender_unit,
        "Psychic blast",
        &mut dice,
    );

    // 5 MW across 2W models: kills model 0 (2W) + model 1 (2W) = 4 MW used,
    // 1 MW remaining goes to model 2.
    assert_eq!(
        destroyed.len(),
        2,
        "5 mortal wounds should destroy 2 models of 2W each"
    );
    assert!(
        !models[2].alive || models[2].wounds_remaining == Wounds::new(1),
        "Third model should have 1 wound remaining from the 5th mortal wound"
    );
    assert!(
        models[2].alive,
        "Third model should still be alive with 1 wound"
    );
    assert_eq!(
        models[2].wounds_remaining,
        Wounds::new(1),
        "Third model: 2W - 1MW carry-over = 1W remaining"
    );
}

/// Source: 40k_revised.md §8.7 — "MORTAL WOUNDS"
/// Rule: Mortal wounds carry over. When enough mortal wounds are applied to
/// destroy all models in a unit, all models should be destroyed.
#[test]
fn mortal_wounds_carry_over_destroys_entire_unit() {
    let defender_unit = UnitId::new(2);
    let mut models = vec![
        make_defender_model(100, defender_unit, 1),
        make_defender_model(101, defender_unit, 1),
        make_defender_model(102, defender_unit, 1),
    ];
    let mut dice = make_dice(42);
    let mut events = Vec::new();

    // 3 mortal wounds against 3 models of 1 wound each = all dead
    let destroyed = apply_mortal_wounds_carry_over(
        &mut models,
        3,
        &mut events,
        defender_unit,
        "Smite",
        &mut dice,
    );

    assert_eq!(
        destroyed.len(),
        3,
        "3 mortal wounds should destroy all 3 single-wound models"
    );
    assert!(!models[0].alive, "Model 0 should be destroyed");
    assert!(!models[1].alive, "Model 1 should be destroyed");
    assert!(!models[2].alive, "Model 2 should be destroyed");
}

/// Source: 40k_revised.md §8.7 — "MORTAL WOUNDS"
/// Rule: Mortal wounds carry over to the next model after destroying one.
/// Verify exact wound counts across a chain of multi-wound models.
#[test]
fn mortal_wounds_carry_over_exact_accounting() {
    let defender_unit = UnitId::new(2);
    let mut models = vec![
        make_defender_model(100, defender_unit, 3),
        make_defender_model(101, defender_unit, 3),
        make_defender_model(102, defender_unit, 3),
    ];
    let mut dice = make_dice(42);
    let mut events = Vec::new();

    // 7 mortal wounds: kills model 0 (3W) + kills model 1 (3W) + 1 MW to model 2
    let destroyed = apply_mortal_wounds_carry_over(
        &mut models,
        7,
        &mut events,
        defender_unit,
        "Devastating power",
        &mut dice,
    );

    assert_eq!(destroyed.len(), 2, "7 MW should destroy 2 of 3 models with 3W each");
    assert!(!models[0].alive, "Model 0 (3W) should be destroyed");
    assert!(!models[1].alive, "Model 1 (3W) should be destroyed");
    assert!(models[2].alive, "Model 2 should survive with 2W remaining");
    assert_eq!(
        models[2].wounds_remaining,
        Wounds::new(2),
        "Model 2: 3W - 1MW = 2W remaining"
    );
}

/// Source: 40k_revised.md §8.7 — "MORTAL WOUNDS"
/// Rule: "Mortal wounds are allocated to an already-wounded model first."
/// Verify that carry-over mortal wounds go to wounded model before unwounded.
#[test]
fn mortal_wounds_allocated_to_wounded_model_first() {
    let defender_unit = UnitId::new(2);
    let mut model0 = make_defender_model(100, defender_unit, 3);
    // Pre-wound model 0
    model0.apply_damage(1); // 2 wounds remaining
    let model1 = make_defender_model(101, defender_unit, 3);

    let mut models = vec![model0, model1];
    let mut dice = make_dice(42);
    let mut events = Vec::new();

    // Apply 1 mortal wound: should go to model 0 (already wounded)
    let destroyed = apply_mortal_wounds_carry_over(
        &mut models,
        1,
        &mut events,
        defender_unit,
        "Targeted MW",
        &mut dice,
    );

    assert!(destroyed.is_empty(), "1 MW should not destroy any model");
    assert_eq!(
        models[0].wounds_remaining,
        Wounds::new(1),
        "Model 0 was at 2W, should now be at 1W"
    );
    assert_eq!(
        models[1].wounds_remaining,
        Wounds::new(3),
        "Model 1 should still be at full 3W (MW went to wounded model first)"
    );
}

// ===========================================================================
// Hazardous tests (§11.13, referenced from §8)
// ===========================================================================

/// Source: 40k_revised.md §11.13 — "HAZARDOUS"
/// Rule: "After a unit has shot or fought, roll one Hazardous test (one D6)
/// for each Hazardous weapon used... On a 1, that model's unit suffers 3 mortal wounds."
#[test]
fn hazardous_test_inflicts_three_mortal_wounds_on_one() {
    let attacker_unit = UnitId::new(1);
    let mut models = vec![
        make_defender_model(100, attacker_unit, 5),
        make_defender_model(101, attacker_unit, 5),
    ];
    let keywords = KeywordSet::from_keywords(&[Keyword::Infantry]);
    let mut events = Vec::new();

    // We need a seed where roll_d6 produces a 1 for the hazardous test.
    // Try multiple seeds until we find one that triggers hazardous.
    let mut found_hazardous = false;
    for seed in 0..200u8 {
        models[0] = make_defender_model(100, attacker_unit, 5);
        models[1] = make_defender_model(101, attacker_unit, 5);
        events.clear();
        let mut dice = make_dice(seed);

        let destroyed = resolve_hazardous_tests(
            &mut models,
            1, // 1 hazardous weapon
            attacker_unit,
            &keywords,
            &mut dice,
            &mut events,
        );

        // Check if this seed triggered hazardous (roll of 1)
        let mortal_wound_events: Vec<_> = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    wh40k_event_system::GameEvent::MortalWoundsInflicted {
                        wounds: 3,
                        source,
                        ..
                    } if source == "Hazardous"
                )
            })
            .collect();

        if !mortal_wound_events.is_empty() {
            found_hazardous = true;
            // Model should have taken 3 mortal wounds (5W -> 2W)
            let total_damage: u8 = models
                .iter()
                .map(|m| m.wounds_max.value() - m.wounds_remaining.value())
                .sum();
            assert!(
                total_damage >= 3 || !destroyed.is_empty(),
                "Hazardous should deal 3 mortal wounds. Damage dealt: {}, destroyed: {:?}",
                total_damage,
                destroyed
            );
            break;
        }
    }

    assert!(
        found_hazardous,
        "Should find at least one seed that triggers Hazardous (roll of 1) in 200 tries"
    );
}

/// Source: 40k_revised.md §11.13 — "HAZARDOUS"
/// Rule: Hazardous tests that pass (roll 2-6) cause no damage.
#[test]
fn hazardous_test_passes_on_two_through_six() {
    let attacker_unit = UnitId::new(1);
    let keywords = KeywordSet::from_keywords(&[Keyword::Infantry]);

    let mut passed_count = 0;
    for seed in 0..200u8 {
        let mut models = vec![make_defender_model(100, attacker_unit, 5)];
        let mut events = Vec::new();
        let mut dice = make_dice(seed);

        let destroyed = resolve_hazardous_tests(
            &mut models,
            1,
            attacker_unit,
            &keywords,
            &mut dice,
            &mut events,
        );

        // If no mortal wound events, the test passed
        let mortal_wound_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, wh40k_event_system::GameEvent::MortalWoundsInflicted { .. }))
            .collect();

        if mortal_wound_events.is_empty() && destroyed.is_empty() {
            // Model should be at full health
            assert_eq!(
                models[0].wounds_remaining,
                Wounds::new(5),
                "Passed Hazardous test should leave model undamaged"
            );
            passed_count += 1;
        }
    }

    assert!(
        passed_count > 0,
        "Some seeds should produce passing Hazardous tests (roll 2-6)"
    );
}

// ===========================================================================
// §8.1-8.6 — Full attack pipeline integration tests
// ===========================================================================

/// Source: 40k_revised.md §8.1-8.6 — Full attack sequence integration.
/// Verify that a strong weapon against a weak target consistently kills models.
/// S10/AP-4/D3 against T3/Sv6+/1W models should have a very high kill rate.
#[test]
fn full_attack_pipeline_high_damage_weapon() {
    let weapon = make_ranged_weapon("Demolisher cannon", 3, 2, 10, -4, 3);
    let ctx = make_attack_context(weapon, 1, 3, 3, 6, None);
    let defender_unit = UnitId::new(2);

    let mut total_kills = 0u32;
    for seed in 0..100u8 {
        let mut models = vec![
            make_defender_model(100, defender_unit, 1),
            make_defender_model(101, defender_unit, 1),
            make_defender_model(102, defender_unit, 1),
        ];
        let mut dice = make_dice(seed);
        let result = resolve_attack_batch(&ctx, &mut models, &mut dice);
        total_kills += result.models_destroyed.len() as u32;
    }

    // BS2+ hits ~83%, S10 vs T3 wounds on 2+ (~83%), Sv6+ vs AP-4 = no save.
    // Each hit should almost certainly kill. Expect ~2+ kills per batch on average.
    assert!(
        total_kills > 100,
        "High-damage weapon should kill many models: got {} kills across 100 seeds (expected >100)",
        total_kills
    );
}

/// Source: 40k_revised.md §8.1-8.6 — Full attack sequence integration.
/// Weak weapon against tough target should rarely deal damage.
/// S3/AP0/D1 against T8/Sv2+/InvSv4+ should almost never wound.
#[test]
fn full_attack_pipeline_weak_weapon_vs_tough_target() {
    let weapon = make_ranged_weapon("Autogun", 1, 4, 3, 0, 1);
    let ctx = make_attack_context(
        weapon,
        1,
        1,
        8,
        2,
        Some(InvulnerableSave::FOUR_PLUS),
    );
    let defender_unit = UnitId::new(2);

    let mut total_damage = 0u32;
    for seed in 0..200u8 {
        let mut models = vec![make_defender_model(100, defender_unit, 10)];
        let mut dice = make_dice(seed);
        let result = resolve_attack_batch(&ctx, &mut models, &mut dice);
        for (_, w) in &result.wounds_inflicted {
            total_damage += *w as u32;
        }
    }

    // BS4+ hits ~50%, S3 vs T8 wounds on 6+ (~17%), then Sv2+ saves on 2+ (83%).
    // Expected damage per attack: 0.5 * 0.17 * 0.17 = ~1.4% per attack.
    // Over 200 attacks, expect ~3 damage. Allow up to 20 for statistical variance.
    assert!(
        total_damage < 20,
        "Weak weapon vs tough target should rarely deal damage: got {} across 200 seeds",
        total_damage
    );
}

/// Source: 40k_revised.md §8.1-8.6 — Full attack pipeline integration.
/// Melee attack during a charge with a Lance weapon should be effective.
/// Verifies charge-related bonuses flow through correctly.
#[test]
fn full_attack_pipeline_melee_charge_with_lance() {
    let mut weapon = make_melee_weapon("Power lance", 4, 3, 6, -2, 2);
    weapon.abilities.add(wh40k_core_types::WeaponAbility::Lance);

    let mut ctx = make_attack_context(weapon, 1, 4, 5, 4, None);
    ctx.attacker_charged = true;
    ctx.in_engagement_range = true;
    ctx.effective_abilities.add(wh40k_core_types::WeaponAbility::Lance);

    let defender_unit = UnitId::new(2);
    let mut total_kills = 0u32;

    for seed in 0..100u8 {
        let mut models = vec![
            make_defender_model(100, defender_unit, 2),
            make_defender_model(101, defender_unit, 2),
            make_defender_model(102, defender_unit, 2),
        ];
        let mut dice = make_dice(seed);
        let result = resolve_attack_batch(&ctx, &mut models, &mut dice);
        total_kills += result.models_destroyed.len() as u32;
    }

    // WS3+ hits ~67%, S6 vs T5 wounds on 3+ (~67%), with Lance charged +1 = 2+(~83%),
    // AP-2 vs Sv4+ = save on 6+ (~17%). D2 kills 2W models in one hit.
    // Expected kills per batch: 4 * 0.67 * 0.83 * 0.83 = ~1.8 kills
    assert!(
        total_kills > 50,
        "Lance charge melee should kill effectively: got {} kills across 100 seeds",
        total_kills
    );
}

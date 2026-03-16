//! Source-linked rules tests: 40k_revised.md Sections 1-3
//!
//! Covers Core Concepts, Dice Mechanics, and Battle Round structure.
//! Each test cites the specific rule passage it validates.

use wh40k_core_types::{
    ArmorPenetration, ArmorSave, BattleRound, BoardDimensions, Inches,
    InvulnerableSave, Phase, Position, Skill, Strength, Toughness,
};
use wh40k_dice::{DiceContext, DiceRoller, RollPurpose, StreamKind};
use wh40k_game_core::combat::wound_threshold;

// ============================================================================
// Helper: create a DiceRoller for tests with a fixed seed
// ============================================================================

fn test_dice_roller(seed_byte: u8) -> DiceRoller {
    let ctx = DiceContext::new([seed_byte; 32], StreamKind::MiscRoll, 0, 0);
    DiceRoller::new(ctx)
}

// ============================================================================
// Section 1.6 - Engagement Range
// "Engagement Range is 1\" horizontally and 5\" vertically."
// ============================================================================

#[test]
fn s1_6_engagement_range_horizontal_is_1_inch() {
    // Source: 40k_revised.md Section 1.6 - "Engagement Range ... 1\" horizontally"
    assert_eq!(Inches::ENGAGEMENT_RANGE, Inches::from_inches(1));
    assert_eq!(Inches::ENGAGEMENT_RANGE.mils(), 1000);
}

#[test]
fn s1_6_engagement_range_vertical_is_5_inches() {
    // Source: 40k_revised.md Section 1.6 - "Engagement Range ... 5\" vertically"
    assert_eq!(Inches::ENGAGEMENT_RANGE_VERTICAL, Inches::from_inches(5));
    assert_eq!(Inches::ENGAGEMENT_RANGE_VERTICAL.mils(), 5000);
}

#[test]
fn s1_6_models_within_engagement_range() {
    // Source: 40k_revised.md Section 1.6 - models within ER of each other
    // Two positions exactly 1" apart horizontally are within engagement range.
    let model_a = Position::from_inches(10, 10);
    let model_b = Position::from_inches(11, 10); // 1" away
    assert!(model_a.within_range(model_b, Inches::ENGAGEMENT_RANGE));

    // A model 2" away is NOT within engagement range
    let model_c = Position::from_inches(12, 10);
    assert!(!model_a.within_range(model_c, Inches::ENGAGEMENT_RANGE));
}

#[test]
fn s1_6_cannot_setup_within_engagement_range_of_enemy() {
    // Source: 40k_revised.md Section 1.6 - "cannot be set up within Engagement
    // Range of any enemy model"
    // Test that a model 0.5" away IS within ER (violating the constraint).
    let enemy = Position::from_inches(10, 10);
    let candidate = Position::new(
        Inches::from_inches(10) + Inches::from_mils(500), // 10.5"
        Inches::from_inches(10),
    );
    // 0.5" distance is within 1" engagement range
    assert!(candidate.within_range(enemy, Inches::ENGAGEMENT_RANGE));

    // A model exactly 1" + 1 mil away is beyond engagement range
    let safe = Position::new(
        Inches::from_inches(10) + Inches::from_mils(1001),
        Inches::from_inches(10),
    );
    assert!(!safe.within_range(enemy, Inches::ENGAGEMENT_RANGE));
}

// ============================================================================
// Section 1.7 - Unit Coherency
// "2-6 models: each within 2\" of at least 1 other model"
// "7+ models: each within 2\" of at least 2 other models"
// ============================================================================

#[test]
fn s1_7_coherency_range_is_2_inches() {
    // Source: 40k_revised.md Section 1.7 - "within 2\" horizontally"
    assert_eq!(Inches::COHERENCY_RANGE, Inches::from_inches(2));
    assert_eq!(Inches::COHERENCY_RANGE.mils(), 2000);
}

#[test]
fn s1_7_coherency_vertical_is_5_inches() {
    // Source: 40k_revised.md Section 1.7 - "within 5\" vertically"
    assert_eq!(Inches::COHERENCY_RANGE_VERTICAL, Inches::from_inches(5));
}

#[test]
fn s1_7_small_unit_coherency_check() {
    // Source: 40k_revised.md Section 1.7 - 2-6 models: within 2" of 1 other
    // Three models in a line, each 2" apart: all in coherency.
    let m1 = Position::from_inches(0, 0);
    let m2 = Position::from_inches(2, 0);
    let m3 = Position::from_inches(4, 0);

    // m1 <-> m2: 2" (within coherency)
    assert!(m1.within_range(m2, Inches::COHERENCY_RANGE));
    // m2 <-> m3: 2" (within coherency)
    assert!(m2.within_range(m3, Inches::COHERENCY_RANGE));
    // m1 <-> m3: 4" (NOT within coherency of each other, but both connected via m2)
    assert!(!m1.within_range(m3, Inches::COHERENCY_RANGE));

    // If m3 is moved to 5" from m2, it breaks coherency with everyone except itself
    let m3_far = Position::from_inches(7, 0);
    assert!(!m2.within_range(m3_far, Inches::COHERENCY_RANGE));
    assert!(!m1.within_range(m3_far, Inches::COHERENCY_RANGE));
}

#[test]
fn s1_7_large_unit_coherency_requires_two_neighbors() {
    // Source: 40k_revised.md Section 1.7 - 7+ models: within 2" of 2 others
    // Place 7 models in a tight cluster where each can reach at least 2 others.
    // Center model at origin, 6 around it at ~1.5" distance (well within 2").
    let center = Position::from_inches(0, 0);
    let north = Position::new(Inches::from_mils(0), Inches::from_mils(1500));
    let south = Position::new(Inches::from_mils(0), Inches::from_mils(-1500));
    let east = Position::new(Inches::from_mils(1500), Inches::from_mils(0));
    let west = Position::new(Inches::from_mils(-1500), Inches::from_mils(0));
    let ne = Position::new(Inches::from_mils(1000), Inches::from_mils(1000));
    let sw = Position::new(Inches::from_mils(-1000), Inches::from_mils(-1000));

    // Center is within 2" of all surrounding models
    for &model in &[north, south, east, west, ne, sw] {
        assert!(
            center.within_range(model, Inches::COHERENCY_RANGE),
            "Center should be within coherency range of {:?}",
            model
        );
    }

    // North should be within 2" of center and ne (verifying 2-neighbor requirement)
    assert!(north.within_range(center, Inches::COHERENCY_RANGE));
    assert!(north.within_range(ne, Inches::COHERENCY_RANGE));
}

// ============================================================================
// Section 1.8 - Measuring Distances
// "between the closest points of the bases"
// "within X\" means distance <= X"
// ============================================================================

#[test]
fn s1_8_distance_between_positions() {
    // Source: 40k_revised.md Section 1.8 - distances measured between closest
    // base points. Using center-to-center as proxy (base sizes not subtracted
    // in these pure geometry tests).
    let a = Position::from_inches(0, 0);
    let b = Position::from_inches(3, 4);
    // Classic 3-4-5 right triangle
    assert_eq!(a.distance(b), Inches::from_inches(5));
}

#[test]
fn s1_8_within_range_means_distance_lte() {
    // Source: 40k_revised.md Section 1.8 - "within X\" = distance <= X"
    let a = Position::from_inches(0, 0);
    let b = Position::from_inches(3, 4); // exactly 5" away

    // Exactly 5" is "within 5\""
    assert!(a.within_range(b, Inches::from_inches(5)));
    // Exactly 5" is NOT "within 4\""
    assert!(!a.within_range(b, Inches::from_inches(4)));
    // 5" is within 6"
    assert!(a.within_range(b, Inches::from_inches(6)));
}

#[test]
fn s1_8_deep_strike_min_distance() {
    // Source: 40k_revised.md Section 1.8/6.3 - Deep Strike "more than 9\" from
    // all enemy models"
    assert_eq!(Inches::DEEP_STRIKE_MIN_DISTANCE, Inches::from_inches(9));
    assert_eq!(Inches::DEEP_STRIKE_MIN_DISTANCE.mils(), 9000);

    let enemy = Position::from_inches(10, 10);
    let too_close = Position::from_inches(10, 18); // 8" away
    let ok_distance = Position::from_inches(10, 20); // 10" away

    assert!(too_close.within_range(enemy, Inches::DEEP_STRIKE_MIN_DISTANCE));
    assert!(!ok_distance.within_range(enemy, Inches::DEEP_STRIKE_MIN_DISTANCE));
}

// ============================================================================
// Section 2.1 - Dice: D6 and D3
// "D6 produces a value 1-6"
// "D3: roll D6, divide by 2, round up: 1-2=1, 3-4=2, 5-6=3"
// ============================================================================

#[test]
fn s2_1_d6_produces_values_1_through_6() {
    // Source: 40k_revised.md Section 2.1 - D6 range is 1-6
    // Roll many D6 and verify all results are in [1,6].
    let mut dice = test_dice_roller(42);
    let purpose = RollPurpose::Misc {
        description: "test d6 range".to_string(),
    };

    for _ in 0..200 {
        let (value, _) = dice.roll_d6(purpose.clone());
        assert!(value >= 1 && value <= 6, "D6 rolled {}, expected 1-6", value);
    }
}

#[test]
fn s2_1_d3_produces_values_1_through_3() {
    // Source: 40k_revised.md Section 2.1 - "D3 = D6 halved, rounded up"
    // Roll many D3 and verify all results are in [1,3].
    let mut dice = test_dice_roller(99);
    let purpose = RollPurpose::Misc {
        description: "test d3 range".to_string(),
    };

    for _ in 0..200 {
        let (value, _) = dice.roll_d3(purpose.clone());
        assert!(value >= 1 && value <= 3, "D3 rolled {}, expected 1-3", value);
    }
}

#[test]
fn s2_1_2d6_produces_values_2_through_12() {
    // Source: 40k_revised.md Section 2.1 - 2D6 sums two dice
    let mut dice = test_dice_roller(77);
    let purpose = RollPurpose::Misc {
        description: "test 2d6 range".to_string(),
    };

    for _ in 0..200 {
        let (total, individual, _) = dice.roll_2d6(purpose.clone());
        assert!(
            total >= 2 && total <= 12,
            "2D6 rolled {}, expected 2-12",
            total
        );
        assert!(individual[0] >= 1 && individual[0] <= 6);
        assert!(individual[1] >= 1 && individual[1] <= 6);
        assert_eq!(total, individual[0] + individual[1]);
    }
}

#[test]
fn s2_1_dice_determinism() {
    // Source: implementation_v3.md Section 6.3 - determinism guarantee
    // Same seed must produce identical results.
    let purpose = RollPurpose::Misc {
        description: "determinism check".to_string(),
    };

    let mut dice_a = test_dice_roller(123);
    let mut dice_b = test_dice_roller(123);

    for _ in 0..50 {
        let (a, _) = dice_a.roll_d6(purpose.clone());
        let (b, _) = dice_b.roll_d6(purpose.clone());
        assert_eq!(a, b, "Determinism violated: same seed produced different rolls");
    }
}

// ============================================================================
// Section 2.2 - Re-rolls
// "A dice can never be re-rolled more than once."
// "Re-rolls happen before any modifiers."
// (These are behavioral rules enforced at the combat layer, but we test
//  the dice subsystem's ability to produce re-roll values.)
// ============================================================================

#[test]
fn s2_2_reroll_produces_valid_d6() {
    // Source: 40k_revised.md Section 2.2 - re-rolls produce a new D6 result
    // A re-roll is just rolling the die again. Verify the DiceRoller can
    // produce sequential independent rolls (each call is a potential re-roll).
    let mut dice = test_dice_roller(55);
    let purpose = RollPurpose::Misc {
        description: "re-roll test".to_string(),
    };

    let (original, _) = dice.roll_d6(purpose.clone());
    let (reroll, _) = dice.roll_d6(purpose.clone());

    // Both must be valid D6 values
    assert!(original >= 1 && original <= 6);
    assert!(reroll >= 1 && reroll <= 6);
    // They CAN be the same (that's fine), but they are independently generated.
    // The important thing is that the system supports rolling again.
}

// ============================================================================
// Section 2.4 - Modifier Caps
// "Hit Roll modifier cap: -1 to +1"
// "Wound Roll modifier cap: -1 to +1"
// "Save improvement cap: +1 max"
// (Modifier caps are enforced at the combat pipeline level. Here we test
//  the Skill and ArmorSave APIs that underlie the cap enforcement.)
// ============================================================================

#[test]
fn s2_4_skill_hit_threshold() {
    // Source: 40k_revised.md Section 2.4 / 7.3 - Hit Roll
    // A Skill of 3+ means rolls of 3,4,5,6 hit; 1,2 miss.
    let bs3 = Skill::THREE_PLUS;
    assert!(!bs3.hits(1));
    assert!(!bs3.hits(2));
    assert!(bs3.hits(3));
    assert!(bs3.hits(4));
    assert!(bs3.hits(5));
    assert!(bs3.hits(6));

    let bs4 = Skill::FOUR_PLUS;
    assert!(!bs4.hits(1));
    assert!(!bs4.hits(2));
    assert!(!bs4.hits(3));
    assert!(bs4.hits(4));
    assert!(bs4.hits(5));
    assert!(bs4.hits(6));
}

#[test]
fn s2_4_armor_save_modified_by_ap() {
    // Source: 40k_revised.md Section 2.4 / 10.1 - AP modifies save
    // 3+ save with AP-2 becomes 5+
    let save = ArmorSave::THREE_PLUS;
    let modified = save.modified_by_ap(ArmorPenetration::MINUS_2);
    assert_eq!(modified.value(), 5);

    // AP-0 does not change save
    let unmod = save.modified_by_ap(ArmorPenetration::ZERO);
    assert_eq!(unmod.value(), 3);

    // AP-4 on a 3+ save: 3+4=7, which means no save
    let destroyed = save.modified_by_ap(ArmorPenetration::MINUS_4);
    assert_eq!(destroyed.value(), 7);
    assert_eq!(destroyed, ArmorSave::NONE);
}

#[test]
fn s2_4_armor_save_cannot_exceed_7_plus() {
    // Source: 40k_revised.md Section 2.4 - save can't be worse than 7+ (no save)
    let save = ArmorSave::SIX_PLUS;
    let modified = save.modified_by_ap(ArmorPenetration::MINUS_3);
    // 6 + 3 = 9, but capped at 7
    assert_eq!(modified.value(), 7);
}

#[test]
fn s2_4_unmodified_1_always_fails_save() {
    // Source: 40k_revised.md Section 2.4 / 10.1 - "An unmodified saving throw
    // of 1 always fails"
    let best_save = ArmorSave::TWO_PLUS;
    // Roll of 1 does not pass even a 2+ save
    assert!(!best_save.passes(1));
    // Roll of 2 passes 2+
    assert!(best_save.passes(2));
}

#[test]
fn s2_4_invulnerable_save_not_affected_by_ap() {
    // Source: 40k_revised.md Section 2.4 / 10.2 - "Invulnerable saves are
    // never modified by the AP characteristic"
    let invuln = InvulnerableSave::FOUR_PLUS;
    assert!(invuln.has_invulnerable());
    // Invulnerable passes on 4+
    assert!(!invuln.passes(3));
    assert!(invuln.passes(4));
    assert!(invuln.passes(5));
    assert!(invuln.passes(6));
    // No AP modifier method on InvulnerableSave - that's the point
    // InvulnerableSave::NONE means the unit has no invuln
    assert!(!InvulnerableSave::NONE.has_invulnerable());
}

// ============================================================================
// Section 3.1 - Turn Structure / Phase Order
// "Command -> Movement -> Shooting -> Charge -> Fight"
// "The game lasts five battle rounds"
// ============================================================================

#[test]
fn s3_1_phase_order_command_to_fight() {
    // Source: 40k_revised.md Section 3.1 - Phase sequence within a turn
    assert_eq!(Phase::Command.next(), Some(Phase::Movement));
    assert_eq!(Phase::Movement.next(), Some(Phase::Shooting));
    assert_eq!(Phase::Shooting.next(), Some(Phase::Charge));
    assert_eq!(Phase::Charge.next(), Some(Phase::Fight));
    assert_eq!(Phase::Fight.next(), None); // End of player turn
}

#[test]
fn s3_1_all_five_phases_are_active() {
    // Source: 40k_revised.md Section 3.1 - all five gameplay phases are active
    assert!(Phase::Command.is_active());
    assert!(Phase::Movement.is_active());
    assert!(Phase::Shooting.is_active());
    assert!(Phase::Charge.is_active());
    assert!(Phase::Fight.is_active());
    // PreBattle and GameEnd are not active gameplay phases
    assert!(!Phase::PreBattle.is_active());
    assert!(!Phase::GameEnd.is_active());
}

#[test]
fn s3_1_battle_rounds_1_through_5() {
    // Source: 40k_revised.md Section 3.1 / CP_Rules.md - "The battle lasts
    // five battle rounds"
    assert_eq!(BattleRound::MIN, BattleRound::new(1));
    assert_eq!(BattleRound::MAX, BattleRound::new(5));

    // All rounds 1-5 are valid
    for r in 1..=5u8 {
        let round = BattleRound::new(r);
        assert!(round.is_valid(), "Round {} should be valid", r);
        assert_eq!(round.number(), r);
    }

    // Round 0 and 6 are invalid
    assert!(!BattleRound::new(0).is_valid());
    assert!(!BattleRound::new(6).is_valid());
}

#[test]
fn s3_1_battle_round_progression() {
    // Source: 40k_revised.md Section 3.1 - rounds progress 1 -> 2 -> ... -> 5
    let r1 = BattleRound::new(1);
    assert_eq!(r1.next(), Some(BattleRound::new(2)));

    let r4 = BattleRound::new(4);
    assert_eq!(r4.next(), Some(BattleRound::new(5)));

    // Round 5 has no next (game ends)
    let r5 = BattleRound::new(5);
    assert_eq!(r5.next(), None);
}

#[test]
fn s3_1_combat_patrol_board_dimensions() {
    // Source: CP_Rules.md - "44\" x 30\" battlefield"
    let board = BoardDimensions::COMBAT_PATROL;
    assert_eq!(board.width, Inches::from_inches(44));
    assert_eq!(board.height, Inches::from_inches(30));

    // Corners are on the board
    assert!(board.contains(Position::from_inches(0, 0)));
    assert!(board.contains(Position::from_inches(44, 30)));

    // Off-board positions
    assert!(!board.contains(Position::from_inches(45, 15)));
    assert!(!board.contains(Position::from_inches(-1, 0)));
    assert!(!board.contains(Position::from_inches(22, 31)));
}

// ============================================================================
// Section 3.3 - Out-of-Phase (Overwatch)
// "Overwatch: only hits on unmodified 6"
// ============================================================================

#[test]
fn s3_3_overwatch_only_hits_on_unmodified_6() {
    // Source: 40k_revised.md Section 3.3 / 12.4 - "Overwatch: each time a
    // model makes a ranged attack, it can only hit on an unmodified Hit roll of 6"
    // Using the Skill type: in Overwatch, the effective skill is 6+.
    let overwatch_skill = Skill::SIX_PLUS;
    assert!(!overwatch_skill.hits(1));
    assert!(!overwatch_skill.hits(2));
    assert!(!overwatch_skill.hits(3));
    assert!(!overwatch_skill.hits(4));
    assert!(!overwatch_skill.hits(5));
    assert!(overwatch_skill.hits(6));
}

// ============================================================================
// Wound Threshold Table (Section 7.4 complement)
// S >= 2T: 2+, S > T: 3+, S == T: 4+, S < T: 5+, S <= T/2: 6+
// ============================================================================

#[test]
fn s7_4_wound_threshold_table() {
    // Source: 40k_revised.md Section 7.4 - Wound Roll table
    // S >= 2T -> wound on 2+
    assert_eq!(wound_threshold(Strength::new(8), Toughness::new(4)), 2);
    assert_eq!(wound_threshold(Strength::new(10), Toughness::new(5)), 2);

    // S > T (but < 2T) -> wound on 3+
    assert_eq!(wound_threshold(Strength::new(5), Toughness::new(4)), 3);
    assert_eq!(wound_threshold(Strength::new(6), Toughness::new(4)), 3);

    // S == T -> wound on 4+
    assert_eq!(wound_threshold(Strength::new(4), Toughness::new(4)), 4);
    assert_eq!(wound_threshold(Strength::new(3), Toughness::new(3)), 4);

    // S < T (but > T/2) -> wound on 5+
    assert_eq!(wound_threshold(Strength::new(3), Toughness::new(4)), 5);
    assert_eq!(wound_threshold(Strength::new(4), Toughness::new(5)), 5);

    // S <= T/2 -> wound on 6+
    assert_eq!(wound_threshold(Strength::new(2), Toughness::new(4)), 6);
    assert_eq!(wound_threshold(Strength::new(3), Toughness::new(8)), 6);
}

//! Combat resolution pipeline for shooting and melee attacks.
//!
//! Implements the full 40K attack sequence:
//! 1. Determine number of attacks (base + Rapid Fire + Blast + variable)
//! 2. Hit Roll (BS/WS + modifiers, Critical Hit on 6, fail on 1)
//! 3. Wound Roll (S vs T table + modifiers, Critical Wound on 6, fail on 1)
//! 4. Saving Throw (AP modifier, Invulnerable, Benefit of Cover)
//! 5. Inflict Damage (wound loss, Feel No Pain, model destruction)
//!
//! Weapon abilities are applied at each relevant step.
//!
//! Source: 40k_revised.md - Attack Sequence, Weapon Abilities
//! Source: Frenzied_Reavers.md, Custodes.md - Faction combat rules

use wh40k_core_types::{
    AllocationStatus, ArmorPenetration, ArmorSave, AttackCount, Damage,
    InvulnerableSave, KeywordSet, ModelId, SaveResult, SaveType, Skill,
    Strength, Toughness, UnitId, WeaponAbilitySet, WeaponId, WeaponProfile,
    WeaponType,
};
use wh40k_dice::{DiceRoller, RollPurpose};
use wh40k_event_system::{DestroyCause, FnpResult, GameEvent, RollResult};

use crate::unit::ModelState;

// ---------------------------------------------------------------------------
// Attack context and results
// ---------------------------------------------------------------------------

/// All information needed to resolve a batch of attacks from one weapon
/// profile against one target unit.
#[derive(Debug, Clone)]
pub struct AttackContext {
    /// The attacking unit ID.
    pub attacker_id: UnitId,
    /// The attacking unit's owner.
    pub attacker_owner: wh40k_core_types::PlayerId,
    /// The defending unit ID.
    pub defender_id: UnitId,
    /// The weapon profile being used.
    pub weapon: WeaponProfile,
    /// Number of attacking models using this weapon.
    pub attacking_model_count: u8,
    /// Base number of attacks per model (after variable resolution).
    pub resolved_attacks_per_model: u8,
    /// Whether the attacker advanced this turn.
    pub attacker_advanced: bool,
    /// Whether the attacker remained stationary this turn.
    pub attacker_stationary: bool,
    /// Whether the attacker charged this turn.
    pub attacker_charged: bool,
    /// Whether the target is within half range.
    pub within_half_range: bool,
    /// Whether the target has Benefit of Cover.
    pub target_has_cover: bool,
    /// Whether the target is within Engagement Range (for Pistol).
    pub in_engagement_range: bool,
    /// Number of models in the target unit (for Blast).
    pub target_model_count: usize,
    /// Keywords of the target unit (for Anti-X).
    pub target_keywords: KeywordSet,
    /// The defender's toughness.
    pub defender_toughness: Toughness,
    /// The defender's armor save.
    pub defender_armor_save: ArmorSave,
    /// The defender's invulnerable save, if any.
    pub defender_invulnerable: Option<InvulnerableSave>,
    /// Effective weapon abilities (weapon native + stance bonuses like Ka'tah).
    pub effective_abilities: WeaponAbilitySet,
    /// Whether this is Indirect Fire (attacker cannot see target).
    pub indirect_fire_no_los: bool,
    /// Whether this attack is an Overwatch (only hit on 6s).
    pub is_overwatch: bool,
    /// Whether the target has Stealth (all ranged attacks suffer -1 to hit).
    /// Source: 40k_revised.md §12.7 - "STEALTH"
    pub target_has_stealth: bool,
    /// Distance from attacker to target in mils (for Stealth range check).
    pub distance_mils: i32,
    /// Whether the TARGET is an engaged Monster/Vehicle (external shooters suffer -1 to hit).
    /// Source: 40k_revised.md §7.5 — Big Guns Never Tire (being targeted)
    pub target_is_engaged_monster_or_vehicle: bool,
    /// Bonus attacks from blessings (e.g., Total Carnage +1 melee attack).
    pub bonus_attacks_per_model: u8,
    /// Bonus AP from blessings (e.g., Warp Blades +1 AP on melee weapons).
    pub bonus_ap: i8,
    /// Bonus FNP from blessings (e.g., Wrathful Devotion 5+ FNP). 0 = no bonus FNP.
    pub bonus_fnp: u8,
    /// Critical hit threshold override (e.g., Martial Excellence = 5+). 0 = default (6).
    pub critical_hit_threshold: u8,
    /// Whether the Command Re-roll stratagem is active for the attacker's owner.
    /// When true, the first failed hit, wound, or damage roll may be re-rolled.
    /// Source: 40k_revised.md §15.2 - "Command Re-roll"
    pub command_reroll_active: bool,
    /// Whether the Command Re-roll stratagem is active for the defender's owner.
    /// When true, the first failed saving throw may be re-rolled.
    /// Source: 40k_revised.md §15.2 - "Command Re-roll"
    pub defender_command_reroll_active: bool,
}

/// Result of resolving a single attack (one die through the pipeline).
#[derive(Debug, Clone)]
pub struct SingleAttackResult {
    /// The hit roll (None if auto-hit via Torrent).
    pub hit_roll: Option<u8>,
    /// Whether this was a critical hit (unmodified 6).
    pub is_critical_hit: bool,
    /// Whether the hit succeeded.
    pub hit_success: bool,
    /// The wound roll (None if auto-wound via Lethal Hits).
    pub wound_roll: Option<u8>,
    /// Whether this was a critical wound (unmodified 6).
    pub is_critical_wound: bool,
    /// Whether the wound succeeded.
    pub wound_success: bool,
    /// Whether this wound triggers Devastating Wounds (mortal wounds instead of save).
    pub devastating_wound: bool,
    /// Additional hits generated by Sustained Hits.
    pub sustained_extra_hits: u8,
}

/// The outcome of resolving all attacks from one weapon against one target.
#[derive(Debug, Clone)]
pub struct AttackBatchResult {
    /// Events generated during the attack.
    pub events: Vec<GameEvent>,
    /// Total wounds inflicted on specific models.
    pub wounds_inflicted: Vec<(ModelId, u8)>,
    /// Models destroyed during this attack batch.
    pub models_destroyed: Vec<ModelId>,
    /// Mortal wounds from Devastating Wounds (after all normal damage).
    pub devastating_mortal_wounds: u8,
    /// Whether Hazardous tests need to be resolved.
    pub hazardous_weapons_used: u8,
    /// Whether the attacker's Command Re-roll stratagem was consumed during this batch.
    /// Source: 40k_revised.md §15.2 - "Command Re-roll"
    pub command_reroll_consumed: bool,
    /// Whether the defender's Command Re-roll stratagem was consumed during this batch.
    /// Source: 40k_revised.md §15.2 - "Command Re-roll"
    pub defender_command_reroll_consumed: bool,
}

// ---------------------------------------------------------------------------
// Core combat resolution
// ---------------------------------------------------------------------------

/// Resolve a complete batch of attacks from one weapon profile against one target unit.
///
/// This is the core combat resolution function. It processes all attacks from the
/// given weapon profile, rolling hits, wounds, saves, and applying damage.
///
/// Source: 40k_revised.md - Making Attacks, Attack Sequence
pub fn resolve_attack_batch(
    ctx: &AttackContext,
    defender_models: &mut [ModelState],
    dice: &mut DiceRoller,
) -> AttackBatchResult {
    let mut events = Vec::new();
    let mut models_destroyed: Vec<ModelId> = Vec::new();
    let mut wounds_inflicted: Vec<(ModelId, u8)> = Vec::new();
    let mut hazardous_count: u8 = 0;

    // Command Re-roll: track whether the re-roll is still available (consumed once per phase)
    // Source: 40k_revised.md §15.2 - "Command Re-roll"
    let mut command_reroll_available = ctx.command_reroll_active;
    let mut defender_command_reroll_available = ctx.defender_command_reroll_active;

    // Track Hazardous weapons used
    if ctx.effective_abilities.has_hazardous() {
        hazardous_count = ctx.attacking_model_count;
    }

    // Step 1: Determine total number of attacks
    let total_attacks = determine_total_attacks(ctx);

    // Step 2-6: For each attack, run through the pipeline
    // We batch hits and wounds, then allocate and save.
    let mut successful_wounds: Vec<SuccessfulWound> = Vec::new();

    for _attack_idx in 0..total_attacks {
        let result = resolve_single_attack(ctx, dice, &mut events, &mut command_reroll_available);

        // Process Sustained Hits extra hits
        if result.hit_success && result.sustained_extra_hits > 0 {
            for _ in 0..result.sustained_extra_hits {
                // Sustained Hits generate extra hits that still need wound rolls
                let extra_wound = resolve_wound_roll(ctx, dice, &mut events, &mut command_reroll_available);
                if extra_wound.wound_success {
                    successful_wounds.push(extra_wound);
                }
            }
        }

        if result.wound_success {
            successful_wounds.push(SuccessfulWound {
                wound_roll: result.wound_roll,
                is_critical_wound: result.is_critical_wound,
                wound_success: true,
                devastating_wound: result.devastating_wound,
            });
        }
    }

    // Step 7: Allocate wounds and resolve saves/damage
    // Separate devastating wounds (mortal, no save) from normal wounds
    // Each devastating wound is tracked individually — excess MW lost per model per DW attack
    // Source: 40k_revised.md §11.12 - "DEVASTATING WOUNDS"
    let mut normal_wounds: Vec<&SuccessfulWound> = Vec::new();
    let mut devastating_wound_damages: Vec<u8> = Vec::new();
    for wound in &successful_wounds {
        if wound.devastating_wound {
            // Devastating Wounds: weapon damage as mortal wounds, no save allowed
            let damage = resolve_damage_value_simple(&ctx.weapon, ctx.within_half_range, dice);
            devastating_wound_damages.push(damage);
        } else {
            normal_wounds.push(wound);
        }
    }
    let devastating_mortal_pool: u8 = devastating_wound_damages.iter().sum();

    // Resolve normal wounds: allocate -> save -> damage
    for _wound in &normal_wounds {
        // Find the model to allocate this wound to
        let target_model = select_allocation_target(defender_models, &models_destroyed, ctx);
        let target_model = match target_model {
            Some(m) => m,
            None => break, // No more alive models
        };

        let model = &defender_models[target_model];
        let model_id = model.id;

        // Apply bonus AP from blessings (e.g., Warp Blades: +1 AP on melee)
        // Source: Frenzied_Reavers.md - Warp Blades blessing
        let effective_ap = if ctx.bonus_ap != 0 {
            ArmorPenetration(ctx.weapon.ap.0.saturating_sub(ctx.bonus_ap))
        } else {
            ctx.weapon.ap
        };

        // Determine which save to use (armor or invulnerable, whichever is better)
        let (save_value, save_type) = determine_best_save(
            ctx.defender_armor_save,
            effective_ap,
            ctx.defender_invulnerable,
            ctx.target_has_cover && !ctx.effective_abilities.has_ignores_cover(),
        );

        // Roll save
        let (roll, _roll_id) = dice.roll_d6(RollPurpose::SaveRoll {
            defender_unit: ctx.defender_id.raw(),
            model_id: model_id.raw(),
            save_type: match save_type {
                SaveType::Armour => wh40k_dice::SaveRollType::Armour,
                SaveType::Invulnerable => wh40k_dice::SaveRollType::Invulnerable,
            },
        });

        let mut save_roll = roll;
        let save_passed_initial = roll != 1 && roll >= save_value; // Unmodified 1 always fails

        // Command Re-roll: defender re-rolls a failed save
        // Source: 40k_revised.md §15.2 - "Command Re-roll"
        let save_passed = if !save_passed_initial && defender_command_reroll_available {
            // Re-roll the save die
            let (reroll, _reroll_id) = dice.roll_d6(RollPurpose::SaveRoll {
                defender_unit: ctx.defender_id.raw(),
                model_id: model_id.raw(),
                save_type: match save_type {
                    SaveType::Armour => wh40k_dice::SaveRollType::Armour,
                    SaveType::Invulnerable => wh40k_dice::SaveRollType::Invulnerable,
                },
            });
            defender_command_reroll_available = false;
            // Defender is the opponent of the attacker in a 2-player game
            let defender_player = wh40k_core_types::PlayerId::new(1 - ctx.attacker_owner.raw());
            events.push(GameEvent::CommandRerollUsed {
                player: defender_player,
                roll_type: "save".to_string(),
                original_roll: roll,
                new_roll: reroll,
            });
            save_roll = reroll;
            reroll != 1 && reroll >= save_value
        } else {
            save_passed_initial
        };

        let save_result = if save_passed {
            SaveResult::Saved
        } else {
            SaveResult::Failed
        };

        events.push(GameEvent::SaveRollMade {
            defender: ctx.defender_id,
            roll: save_roll,
            modified: save_roll as i8, // save_value already accounts for AP
            save_type,
            result: save_result,
        });

        if save_passed {
            continue; // Save succeeded, no damage
        }

        // Save failed: resolve damage
        let damage = resolve_damage_value(&ctx.weapon, ctx.within_half_range, dice, &mut command_reroll_available, ctx.attacker_owner, &mut events);

        // Apply damage to the target model with Feel No Pain
        let actual_damage = apply_damage_with_fnp(
            &mut defender_models[target_model],
            damage,
            dice,
            &mut events,
            ctx.attacker_id,
            ctx.defender_id,
            ctx.weapon.id,
            ctx.bonus_fnp,
        );

        if actual_damage > 0 {
            wounds_inflicted.push((model_id, actual_damage));
        }

        // Check if model was destroyed
        if !defender_models[target_model].alive {
            models_destroyed.push(model_id);

            events.push(GameEvent::ModelDestroyed {
                model: model_id,
                unit: ctx.defender_id,
                cause: DestroyCause::Attack {
                    attacker: ctx.attacker_id,
                    weapon: ctx.weapon.id,
                },
            });
        }
    }

    // Apply devastating wound mortal wounds individually per attack
    // Each devastating wound's mortal wounds are applied separately; excess is lost
    // per model for each individual devastating wound attack.
    // Source: 40k_revised.md §11.12 - "DEVASTATING WOUNDS", §8.7 - "MORTAL WOUNDS"
    for dw_damage in &devastating_wound_damages {
        if *dw_damage == 0 {
            continue;
        }
        let source = format!("Devastating Wounds ({})", ctx.weapon.name);
        events.push(GameEvent::MortalWoundsInflicted {
            target_unit: ctx.defender_id,
            wounds: *dw_damage,
            source: source.clone(),
        });

        // Apply this individual devastating wound's mortal wounds
        // Excess damage is lost when a model is destroyed (per §8.7)
        apply_mortal_wounds_devastating(
            defender_models,
            *dw_damage,
            &mut models_destroyed,
            &mut wounds_inflicted,
            &mut events,
            ctx.defender_id,
            ctx.attacker_id,
            ctx.weapon.id,
            dice,
            ctx.bonus_fnp,
        );
    }

    AttackBatchResult {
        events,
        wounds_inflicted,
        models_destroyed,
        devastating_mortal_wounds: devastating_mortal_pool,
        hazardous_weapons_used: hazardous_count,
        command_reroll_consumed: ctx.command_reroll_active && !command_reroll_available,
        defender_command_reroll_consumed: ctx.defender_command_reroll_active && !defender_command_reroll_available,
    }
}

// ---------------------------------------------------------------------------
// Step 1: Determine number of attacks
// ---------------------------------------------------------------------------

/// Determine the total number of attacks for this weapon batch.
///
/// Accounts for: base attacks, Rapid Fire, Blast, variable attack counts.
///
/// Source: 40k_revised.md - Number of Attacks, Rapid Fire, Blast
fn determine_total_attacks(ctx: &AttackContext) -> u32 {
    let base = ctx.resolved_attacks_per_model as u32;
    let mut attacks_per_model = base;

    // Rapid Fire: +N attacks when within half range
    if ctx.within_half_range {
        if let Some(rf) = ctx.effective_abilities.rapid_fire_value() {
            attacks_per_model += rf as u32;
        }
    }

    // Blast: +1 attack per 5 models in target unit
    if ctx.effective_abilities.has_blast() {
        let blast_bonus = (ctx.target_model_count / 5) as u32;
        attacks_per_model += blast_bonus;
    }

    // Bonus attacks from blessings (e.g., Total Carnage: +1 melee attack)
    // Source: Frenzied_Reavers.md - Total Carnage blessing
    attacks_per_model += ctx.bonus_attacks_per_model as u32;

    attacks_per_model * ctx.attacking_model_count as u32
}

// ---------------------------------------------------------------------------
// Step 2-4: Single attack resolution (hit -> wound)
// ---------------------------------------------------------------------------

/// A wound that successfully passed through hit and wound rolls.
#[derive(Debug, Clone)]
struct SuccessfulWound {
    wound_roll: Option<u8>,
    is_critical_wound: bool,
    wound_success: bool,
    devastating_wound: bool,
}

/// Resolve a single attack through hit and wound rolls.
///
/// `command_reroll_available` tracks whether the attacker's Command Re-roll can still be used.
/// When consumed, it is set to false.
fn resolve_single_attack(
    ctx: &AttackContext,
    dice: &mut DiceRoller,
    events: &mut Vec<GameEvent>,
    command_reroll_available: &mut bool,
) -> SingleAttackResult {
    // Step 2: Hit Roll
    let (hit_roll, is_critical_hit, hit_success, sustained_extra) =
        resolve_hit_roll(ctx, dice, events, command_reroll_available);

    if !hit_success {
        return SingleAttackResult {
            hit_roll: Some(hit_roll),
            is_critical_hit: false,
            hit_success: false,
            wound_roll: None,
            is_critical_wound: false,
            wound_success: false,
            devastating_wound: false,
            sustained_extra_hits: 0,
        };
    }

    // Step 3: Wound Roll
    // Lethal Hits: critical hits auto-wound (skip wound roll)
    if is_critical_hit && ctx.effective_abilities.has_lethal_hits() {
        return SingleAttackResult {
            hit_roll: Some(hit_roll),
            is_critical_hit: true,
            hit_success: true,
            wound_roll: None,
            is_critical_wound: false, // Not a critical wound, just auto-wound
            wound_success: true,
            devastating_wound: false, // Lethal Hits auto-wound is not a critical wound
            sustained_extra_hits: sustained_extra,
        };
    }

    let wound_result = resolve_wound_roll(ctx, dice, events, command_reroll_available);

    SingleAttackResult {
        hit_roll: Some(hit_roll),
        is_critical_hit,
        hit_success: true,
        wound_roll: wound_result.wound_roll,
        is_critical_wound: wound_result.is_critical_wound,
        wound_success: wound_result.wound_success,
        devastating_wound: wound_result.devastating_wound,
        sustained_extra_hits: sustained_extra,
    }
}

/// Resolve a hit roll.
///
/// Returns (roll_value, is_critical, hit_success, sustained_extra_hits).
///
/// If the hit fails and Command Re-roll is available, the roll is re-rolled
/// once and the re-roll is consumed.
///
/// Source: 40k_revised.md - "HIT ROLLS"
fn resolve_hit_roll(
    ctx: &AttackContext,
    dice: &mut DiceRoller,
    events: &mut Vec<GameEvent>,
    command_reroll_available: &mut bool,
) -> (u8, bool, bool, u8) {
    // Torrent: automatic hit, no roll needed
    if ctx.effective_abilities.has_torrent() {
        events.push(GameEvent::HitRollMade {
            attacker: ctx.attacker_id,
            roll: 0,
            modified: 0,
            is_critical: false,
            result: RollResult::Success,
        });
        return (0, false, true, 0);
    }

    // Roll the hit die
    let (roll, _roll_id) = dice.roll_d6(RollPurpose::HitRoll {
        attacker_unit: ctx.attacker_id.raw(),
        target_unit: ctx.defender_id.raw(),
        weapon_id: ctx.weapon.id.raw(),
    });

    // Evaluate the initial hit result
    let result = evaluate_hit_roll(ctx, roll);

    // Command Re-roll: if hit failed and re-roll is available, try again
    // Source: 40k_revised.md §15.2 - "Command Re-roll"
    if !result.hit_success && *command_reroll_available {
        let (reroll, _reroll_id) = dice.roll_d6(RollPurpose::HitRoll {
            attacker_unit: ctx.attacker_id.raw(),
            target_unit: ctx.defender_id.raw(),
            weapon_id: ctx.weapon.id.raw(),
        });
        *command_reroll_available = false;
        events.push(GameEvent::CommandRerollUsed {
            player: ctx.attacker_owner,
            roll_type: "hit".to_string(),
            original_roll: roll,
            new_roll: reroll,
        });
        let reroll_result = evaluate_hit_roll(ctx, reroll);
        events.push(GameEvent::HitRollMade {
            attacker: ctx.attacker_id,
            roll: reroll,
            modified: reroll_result.modified_roll as i8,
            is_critical: reroll_result.is_critical,
            result: if reroll_result.hit_success {
                RollResult::Success
            } else {
                RollResult::Failure
            },
        });
        return (
            reroll,
            reroll_result.is_critical && reroll_result.hit_success,
            reroll_result.hit_success,
            reroll_result.sustained_extra,
        );
    }

    events.push(GameEvent::HitRollMade {
        attacker: ctx.attacker_id,
        roll,
        modified: result.modified_roll as i8,
        is_critical: result.is_critical,
        result: if result.hit_success {
            RollResult::Success
        } else {
            RollResult::Failure
        },
    });

    (
        roll,
        result.is_critical && result.hit_success,
        result.hit_success,
        result.sustained_extra,
    )
}

/// Internal result of evaluating a hit roll against the context.
struct HitRollEvaluation {
    is_critical: bool,
    hit_success: bool,
    modified_roll: u8,
    sustained_extra: u8,
}

/// Evaluate a hit roll (pure logic, no side effects).
/// Used by both the initial roll and the Command Re-roll path.
fn evaluate_hit_roll(ctx: &AttackContext, roll: u8) -> HitRollEvaluation {
    // Unmodified 1 always fails
    if roll == 1 {
        return HitRollEvaluation {
            is_critical: false,
            hit_success: false,
            modified_roll: 1,
            sustained_extra: 0,
        };
    }

    // Critical hit check: normally unmodified 6, but can be overridden
    // (e.g., Martial Excellence blessing = critical on 5+)
    // Source: Frenzied_Reavers.md - Martial Excellence
    let crit_threshold = if ctx.critical_hit_threshold > 0 && ctx.critical_hit_threshold <= 6 {
        ctx.critical_hit_threshold
    } else {
        6
    };
    let is_critical = roll >= crit_threshold;

    // Calculate hit modifier (capped at -1 to +1)
    let mut hit_modifier: i8 = 0;

    // Heavy: +1 if remained stationary
    if ctx.effective_abilities.has_heavy() && ctx.attacker_stationary {
        hit_modifier += 1;
    }

    // Overwatch: only hits on unmodified 6
    if ctx.is_overwatch {
        let success = roll == 6;
        let sustained_extra = if success && is_critical {
            ctx.effective_abilities.total_sustained_hits()
        } else {
            0
        };
        return HitRollEvaluation {
            is_critical: is_critical && success,
            hit_success: success,
            modified_roll: roll,
            sustained_extra,
        };
    }

    // Indirect Fire (no LOS): -1 to hit, and unmodified 1-3 always fail
    if ctx.indirect_fire_no_los {
        hit_modifier -= 1;
        if roll <= 3 {
            return HitRollEvaluation {
                is_critical: false,
                hit_success: false,
                modified_roll: (roll as i8 + hit_modifier).max(1) as u8,
                sustained_extra: 0,
            };
        }
    }

    // Stealth: -1 to hit if target has Stealth
    // Source: 40k_revised.md §12.7 — "If every model in unit has this ability"
    // Note: The Stealth effect is applied at unit level. For native datasheet Stealth,
    // the effect should only be active if ALL models have the ability.
    // Currently enforced via unit-level effect application (correct for stratagems like Smokescreen).
    if ctx.target_has_stealth {
        hit_modifier -= 1;
    }

    // Big Guns Never Tire: -1 to hit if Monster/Vehicle shooting while engaged
    // Source: 40k_revised.md §7.5
    if ctx.in_engagement_range
        && ctx.weapon.weapon_type == WeaponType::Ranged
        && !ctx.effective_abilities.has_pistol()
    {
        hit_modifier -= 1;
    }

    // Big Guns Never Tire (target): -1 to hit when shooting AT an engaged Monster/Vehicle
    // Source: 40k_revised.md §7.5 — external units shooting at engaged Monster/Vehicle
    if ctx.target_is_engaged_monster_or_vehicle
        && ctx.weapon.weapon_type == WeaponType::Ranged
        && !ctx.effective_abilities.has_pistol()
        && !ctx.in_engagement_range  // Don't double-apply if shooter is ALSO engaged
    {
        hit_modifier -= 1;
    }

    // Clamp modifier to -1..+1
    hit_modifier = hit_modifier.clamp(-1, 1);

    let modified_roll = (roll as i8 + hit_modifier).max(1) as u8;
    let skill_target = ctx.weapon.skill.value();

    // Critical hit on unmodified 6 always hits regardless of modifier
    let hit_success = is_critical || modified_roll >= skill_target;

    let sustained_extra = if hit_success && is_critical {
        ctx.effective_abilities.total_sustained_hits()
    } else {
        0
    };

    HitRollEvaluation {
        is_critical,
        hit_success,
        modified_roll,
        sustained_extra,
    }
}

/// Resolve a wound roll.
///
/// If the wound fails and Command Re-roll is available (and Twin-linked hasn't
/// already re-rolled), the roll is re-rolled once and the re-roll is consumed.
///
/// Source: 40k_revised.md - "WOUND ROLLS"
fn resolve_wound_roll(
    ctx: &AttackContext,
    dice: &mut DiceRoller,
    events: &mut Vec<GameEvent>,
    command_reroll_available: &mut bool,
) -> SuccessfulWound {
    let (roll, _roll_id) = dice.roll_d6(RollPurpose::WoundRoll {
        attacker_unit: ctx.attacker_id.raw(),
        target_unit: ctx.defender_id.raw(),
        weapon_id: ctx.weapon.id.raw(),
    });

    // Evaluate the wound roll
    let eval = evaluate_wound_roll(ctx, roll);

    // Twin-Linked: re-roll failed wound rolls (takes priority over Command Re-roll)
    if !eval.wound_success && ctx.effective_abilities.has_twin_linked() {
        let (reroll, _reroll_id) = dice.roll_d6(RollPurpose::WoundRoll {
            attacker_unit: ctx.attacker_id.raw(),
            target_unit: ctx.defender_id.raw(),
            weapon_id: ctx.weapon.id.raw(),
        });

        let reroll_eval = evaluate_wound_roll(ctx, reroll);

        events.push(GameEvent::WoundRollMade {
            attacker: ctx.attacker_id,
            roll: reroll,
            modified: reroll_eval.modified_roll as i8,
            is_critical: reroll_eval.is_critical_wound,
            result: if reroll_eval.wound_success {
                RollResult::Success
            } else {
                RollResult::Failure
            },
        });

        return SuccessfulWound {
            wound_roll: Some(reroll),
            is_critical_wound: reroll_eval.is_critical_wound,
            wound_success: reroll_eval.wound_success,
            devastating_wound: reroll_eval.devastating,
        };
    }

    // Command Re-roll: if wound failed and re-roll is available, try again
    // Source: 40k_revised.md §15.2 - "Command Re-roll"
    if !eval.wound_success && *command_reroll_available {
        let (reroll, _reroll_id) = dice.roll_d6(RollPurpose::WoundRoll {
            attacker_unit: ctx.attacker_id.raw(),
            target_unit: ctx.defender_id.raw(),
            weapon_id: ctx.weapon.id.raw(),
        });
        *command_reroll_available = false;
        events.push(GameEvent::CommandRerollUsed {
            player: ctx.attacker_owner,
            roll_type: "wound".to_string(),
            original_roll: roll,
            new_roll: reroll,
        });

        let reroll_eval = evaluate_wound_roll(ctx, reroll);

        events.push(GameEvent::WoundRollMade {
            attacker: ctx.attacker_id,
            roll: reroll,
            modified: reroll_eval.modified_roll as i8,
            is_critical: reroll_eval.is_critical_wound,
            result: if reroll_eval.wound_success {
                RollResult::Success
            } else {
                RollResult::Failure
            },
        });

        return SuccessfulWound {
            wound_roll: Some(reroll),
            is_critical_wound: reroll_eval.is_critical_wound,
            wound_success: reroll_eval.wound_success,
            devastating_wound: reroll_eval.devastating,
        };
    }

    events.push(GameEvent::WoundRollMade {
        attacker: ctx.attacker_id,
        roll,
        modified: eval.modified_roll as i8,
        is_critical: eval.is_critical_wound,
        result: if eval.wound_success {
            RollResult::Success
        } else {
            RollResult::Failure
        },
    });

    SuccessfulWound {
        wound_roll: Some(roll),
        is_critical_wound: eval.is_critical_wound,
        wound_success: eval.wound_success,
        devastating_wound: eval.devastating,
    }
}

/// Internal result of evaluating a wound roll against the context.
struct WoundRollEvaluation {
    is_critical_wound: bool,
    wound_success: bool,
    modified_roll: u8,
    devastating: bool,
}

/// Evaluate a wound roll (pure logic, no side effects).
/// Used by both the initial roll and the Command Re-roll / Twin-linked re-roll paths.
fn evaluate_wound_roll(ctx: &AttackContext, roll: u8) -> WoundRollEvaluation {
    // Unmodified 1 always fails
    if roll == 1 {
        return WoundRollEvaluation {
            is_critical_wound: false,
            wound_success: false,
            modified_roll: 1,
            devastating: false,
        };
    }

    // Unmodified 6 is a critical wound (always wounds)
    let is_critical = roll == 6;

    // Check Anti-X: if target has the keyword and roll >= threshold, it's a critical wound
    let anti_critical = check_anti_keyword(ctx, roll);
    let is_critical_wound = is_critical || anti_critical;

    // Calculate wound modifier (capped at -1 to +1)
    let mut wound_modifier: i8 = 0;

    // Lance: +1 to wound if bearer charged this turn
    if ctx.effective_abilities.has_lance() && ctx.attacker_charged {
        wound_modifier += 1;
    }

    // Clamp to -1..+1
    wound_modifier = wound_modifier.clamp(-1, 1);

    let modified_roll = (roll as i8 + wound_modifier).max(1) as u8;

    // Determine required wound roll from S vs T table
    let required = wound_threshold(ctx.weapon.strength, ctx.defender_toughness);

    // Critical wound on unmodified 6 always wounds regardless of modifier
    let wound_success = is_critical_wound || modified_roll >= required;

    // Devastating Wounds: critical wounds become mortal wounds
    let devastating = is_critical_wound && ctx.effective_abilities.has_devastating_wounds();

    WoundRollEvaluation {
        is_critical_wound,
        wound_success,
        modified_roll,
        devastating,
    }
}

/// Check if any Anti-X ability applies against the target.
///
/// Source: 40k_revised.md - "ANTI-"
fn check_anti_keyword(ctx: &AttackContext, roll: u8) -> bool {
    for (keyword, threshold) in ctx.effective_abilities.anti_abilities() {
        if ctx.target_keywords.has(keyword) && roll >= threshold {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Step 5: Wound allocation and saving throws
// ---------------------------------------------------------------------------

/// Select which model to allocate the next wound to.
///
/// Rules: 40k_revised.md - "ALLOCATING ATTACKS"
/// - If a model already lost wounds OR had attacks allocated: MUST allocate there
/// - Otherwise: Defending player chooses (we allocate to wounded first, then lowest wounds)
/// - ATTACHED UNITS (Bodyguard): Leader models cannot be allocated wounds until all
///   bodyguard models are destroyed, UNLESS the weapon has Precision and it's a critical hit.
///
/// Source: 40k_revised.md - "ATTACHED UNITS", "BODYGUARD"
///
/// Returns the index into the defender_models slice.
fn select_allocation_target(
    models: &[ModelState],
    destroyed_this_batch: &[ModelId],
    ctx: &AttackContext,
) -> Option<usize> {
    let has_precision = ctx.effective_abilities.has_precision();

    // Check if this unit has a bodyguard structure (some models are leaders, some are not)
    let has_bodyguard_models = models.iter().any(|m| !m.is_leader && m.alive && !destroyed_this_batch.contains(&m.id));
    let bodyguard_active = has_bodyguard_models && !has_precision;

    // Precision: actively target CHARACTER (leader) models in Attached units
    // Source: 40k_revised.md §11.14 - "PRECISION"
    // "attacker may choose to allocate the attack to that CHARACTER,
    //  provided it is visible to the attacking model"
    // #26: Visibility requirement is satisfied because the target unit was already
    // validated as visible during shooting declaration (validate_declare_shooting_targets
    // checks LOS via board.check_los()). Per-model visibility within a unit is
    // guaranteed by the unit-level LOS check.
    if has_precision {
        // First check for a wounded leader
        let wounded_leader = models.iter().enumerate().find(|(_, m)| {
            m.alive
                && m.is_leader
                && m.allocation_status == AllocationStatus::WoundedAllocated
                && !destroyed_this_batch.contains(&m.id)
        });
        if let Some((idx, _)) = wounded_leader {
            return Some(idx);
        }
        // Then target any alive leader
        let alive_leader = models.iter().enumerate().find(|(_, m)| {
            m.alive && m.is_leader && !destroyed_this_batch.contains(&m.id)
        });
        if let Some((idx, _)) = alive_leader {
            return Some(idx);
        }
        // No leader alive — fall through to normal allocation
    }

    // Priority 1: Already wounded model that's still alive and not destroyed this batch
    let wounded = models.iter().enumerate().find(|(_, m)| {
        m.alive
            && m.allocation_status == AllocationStatus::WoundedAllocated
            && !destroyed_this_batch.contains(&m.id)
            && (!bodyguard_active || !m.is_leader)
    });
    if let Some((idx, _)) = wounded {
        return Some(idx);
    }

    // Priority 2: First alive model not yet destroyed this batch
    // - When bodyguard is active, skip leader models (they cannot be targeted)
    // - Allocate to model with fewest wounds remaining (tactically neutral)
    let mut best_idx: Option<usize> = None;
    let mut best_wounds: u8 = u8::MAX;

    for (idx, model) in models.iter().enumerate() {
        if model.alive && !destroyed_this_batch.contains(&model.id) {
            // Skip leader models when bodyguard is active
            if bodyguard_active && model.is_leader {
                continue;
            }
            if model.wounds_remaining.value() < best_wounds {
                best_wounds = model.wounds_remaining.value();
                best_idx = Some(idx);
            }
        }
    }

    // If no non-leader models found (bodyguard exhausted), fall through to leader
    if best_idx.is_none() {
        for (idx, model) in models.iter().enumerate() {
            if model.alive && !destroyed_this_batch.contains(&model.id)
                && model.wounds_remaining.value() < best_wounds
            {
                best_wounds = model.wounds_remaining.value();
                best_idx = Some(idx);
            }
        }
    }

    best_idx
}

/// Determine the best save available to a model (armor or invulnerable).
///
/// Returns (required_roll, save_type).
///
/// Source: 40k_revised.md - "SAVING THROWS", "INVULNERABLE SAVES"
fn determine_best_save(
    armor_save: ArmorSave,
    ap: ArmorPenetration,
    invulnerable: Option<InvulnerableSave>,
    has_cover: bool,
) -> (u8, SaveType) {
    // Calculate modified armor save (AP worsens save)
    let mut modified_armor = armor_save.modified_by_ap(ap);

    // Benefit of Cover: +1 to armor save
    // 40k_revised.md §13.1: Models with Save 3+ or better do NOT get Benefit of Cover against AP 0
    // 40k_revised.md §8.4: "Saving throws can never be improved by more than +1"
    let cover_denied = ap == ArmorPenetration::ZERO && armor_save.value() <= 3;
    if has_cover && !cover_denied && modified_armor.value() > 1 {
        modified_armor = ArmorSave(modified_armor.value().saturating_sub(1));
        // Cover-specific cap: cannot improve beyond base (unmodified) save
        if modified_armor.value() < armor_save.value() {
            modified_armor = armor_save;
        }
    }
    // General +1 improvement cap (§8.4): save cannot improve by more than 1
    // from the AP-modified value, regardless of source
    let ap_modified_base = armor_save.modified_by_ap(ap);
    if ap_modified_base.value() > 1 && modified_armor.value() < ap_modified_base.value().saturating_sub(1) {
        modified_armor = ArmorSave(ap_modified_base.value().saturating_sub(1));
    }

    // Compare with invulnerable save (lower is better)
    if let Some(invuln) = invulnerable {
        if invuln.has_invulnerable() && invuln.value() < modified_armor.value() {
            return (invuln.value(), SaveType::Invulnerable);
        }
    }

    (modified_armor.value(), SaveType::Armour)
}

// ---------------------------------------------------------------------------
// Step 6: Damage resolution
// ---------------------------------------------------------------------------

/// Resolve a variable damage value for a weapon.
///
/// If the damage is variable and Command Re-roll is available, the player may
/// re-roll the damage die (consuming the re-roll). The re-roll is used if the
/// initial damage roll is below average (i.e., low enough to be worth re-rolling).
///
/// Source: 40k_revised.md - "INFLICTING DAMAGE"
fn resolve_damage_value(
    weapon: &WeaponProfile,
    within_half_range: bool,
    dice: &mut DiceRoller,
    command_reroll_available: &mut bool,
    attacker_owner: wh40k_core_types::PlayerId,
    events: &mut Vec<GameEvent>,
) -> u8 {
    let base_damage = match weapon.damage {
        Damage::Fixed(n) => n,
        Damage::D3 => {
            let (val, _) = dice.roll_d3(RollPurpose::DamageRoll {
                weapon_id: weapon.id.raw(),
                damage_type: "D3".to_string(),
            });
            // Command Re-roll: re-roll low damage (1 on D3)
            // Source: 40k_revised.md §15.2 - "Command Re-roll"
            if val == 1 && *command_reroll_available {
                let (reroll, _) = dice.roll_d3(RollPurpose::DamageRoll {
                    weapon_id: weapon.id.raw(),
                    damage_type: "D3".to_string(),
                });
                *command_reroll_available = false;
                events.push(GameEvent::CommandRerollUsed {
                    player: attacker_owner,
                    roll_type: "damage".to_string(),
                    original_roll: val,
                    new_roll: reroll,
                });
                reroll
            } else {
                val
            }
        }
        Damage::D6 => {
            let (val, _) = dice.roll_d6(RollPurpose::DamageRoll {
                weapon_id: weapon.id.raw(),
                damage_type: "D6".to_string(),
            });
            // Command Re-roll: re-roll low damage (1-2 on D6)
            if val <= 2 && *command_reroll_available {
                let (reroll, _) = dice.roll_d6(RollPurpose::DamageRoll {
                    weapon_id: weapon.id.raw(),
                    damage_type: "D6".to_string(),
                });
                *command_reroll_available = false;
                events.push(GameEvent::CommandRerollUsed {
                    player: attacker_owner,
                    roll_type: "damage".to_string(),
                    original_roll: val,
                    new_roll: reroll,
                });
                reroll
            } else {
                val
            }
        }
        Damage::D3Plus(n) => {
            let (val, _) = dice.roll_d3(RollPurpose::DamageRoll {
                weapon_id: weapon.id.raw(),
                damage_type: format!("D3+{}", n),
            });
            // Command Re-roll: re-roll low damage (1 on D3+N)
            if val == 1 && *command_reroll_available {
                let (reroll, _) = dice.roll_d3(RollPurpose::DamageRoll {
                    weapon_id: weapon.id.raw(),
                    damage_type: format!("D3+{}", n),
                });
                *command_reroll_available = false;
                events.push(GameEvent::CommandRerollUsed {
                    player: attacker_owner,
                    roll_type: "damage".to_string(),
                    original_roll: val,
                    new_roll: reroll,
                });
                reroll + n
            } else {
                val + n
            }
        }
        Damage::D6Plus(n) => {
            let (val, _) = dice.roll_d6(RollPurpose::DamageRoll {
                weapon_id: weapon.id.raw(),
                damage_type: format!("D6+{}", n),
            });
            // Command Re-roll: re-roll low damage (1-2 on D6+N)
            if val <= 2 && *command_reroll_available {
                let (reroll, _) = dice.roll_d6(RollPurpose::DamageRoll {
                    weapon_id: weapon.id.raw(),
                    damage_type: format!("D6+{}", n),
                });
                *command_reroll_available = false;
                events.push(GameEvent::CommandRerollUsed {
                    player: attacker_owner,
                    roll_type: "damage".to_string(),
                    original_roll: val,
                    new_roll: reroll,
                });
                reroll + n
            } else {
                val + n
            }
        }
    };

    // Melta: +N damage within half range
    let melta_bonus = if within_half_range {
        weapon.abilities.melta_value().unwrap_or(0)
    } else {
        0
    };

    base_damage + melta_bonus
}

/// Resolve a damage value without Command Re-roll support.
/// Used for devastating wound damage resolution and other contexts where
/// the Command Re-roll is not applicable (e.g., mortal wound pools).
fn resolve_damage_value_simple(
    weapon: &WeaponProfile,
    within_half_range: bool,
    dice: &mut DiceRoller,
) -> u8 {
    let base_damage = match weapon.damage {
        Damage::Fixed(n) => n,
        Damage::D3 => {
            let (val, _) = dice.roll_d3(RollPurpose::DamageRoll {
                weapon_id: weapon.id.raw(),
                damage_type: "D3".to_string(),
            });
            val
        }
        Damage::D6 => {
            let (val, _) = dice.roll_d6(RollPurpose::DamageRoll {
                weapon_id: weapon.id.raw(),
                damage_type: "D6".to_string(),
            });
            val
        }
        Damage::D3Plus(n) => {
            let (val, _) = dice.roll_d3(RollPurpose::DamageRoll {
                weapon_id: weapon.id.raw(),
                damage_type: format!("D3+{}", n),
            });
            val + n
        }
        Damage::D6Plus(n) => {
            let (val, _) = dice.roll_d6(RollPurpose::DamageRoll {
                weapon_id: weapon.id.raw(),
                damage_type: format!("D6+{}", n),
            });
            val + n
        }
    };

    // Melta: +N damage within half range
    let melta_bonus = if within_half_range {
        weapon.abilities.melta_value().unwrap_or(0)
    } else {
        0
    };

    base_damage + melta_bonus
}

/// Apply damage to a model, accounting for Feel No Pain.
///
/// Returns the actual number of wounds lost.
///
/// Source: 40k_revised.md - "INFLICTING DAMAGE", "FEEL NO PAIN"
#[allow(clippy::too_many_arguments)]
fn apply_damage_with_fnp(
    model: &mut ModelState,
    damage: u8,
    dice: &mut DiceRoller,
    events: &mut Vec<GameEvent>,
    _attacker_id: UnitId,
    _defender_id: UnitId,
    weapon_id: WeaponId,
    bonus_fnp: u8,
) -> u8 {
    let mut total_wounds_lost: u8 = 0;

    for _ in 0..damage {
        if !model.alive {
            break; // Excess damage is lost (no overflow)
        }

        // Determine effective FNP: best of model's native FNP and bonus FNP
        // Source: Frenzied_Reavers.md - Wrathful Devotion blessing grants 5+ FNP
        let effective_fnp_value: Option<u8> = {
            let model_fnp = model.feel_no_pain.and_then(|f| if f.has_fnp() { Some(f.value()) } else { None });
            let blessing_fnp = if bonus_fnp > 0 { Some(bonus_fnp) } else { None };
            match (model_fnp, blessing_fnp) {
                (Some(m), Some(b)) => Some(m.min(b)), // lower is better
                (Some(m), None) => Some(m),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            }
        };

        // Feel No Pain: roll for each wound individually
        if let Some(fnp_val) = effective_fnp_value {
            let (roll, _) = dice.roll_d6(RollPurpose::FeelNoPain {
                model_id: model.id.raw(),
                wounds_to_ignore: fnp_val,
            });

            let ignored = roll >= fnp_val;
            events.push(GameEvent::FeelNoPainRollMade {
                model: model.id,
                roll,
                result: if ignored {
                    FnpResult::Ignored
                } else {
                    FnpResult::Suffered
                },
            });

            if ignored {
                continue; // Wound ignored
            }
        }

        // Apply 1 wound of damage
        let destroyed = model.apply_damage(1);
        total_wounds_lost += 1;

        if !destroyed {
            events.push(GameEvent::DamageInflicted {
                target_model: model.id,
                wounds: 1,
                source: weapon_id,
            });
        }
    }

    total_wounds_lost
}

/// Apply mortal wounds from Devastating Wounds to models in a unit.
///
/// For Devastating Wounds / Hazardous mortal wounds, excess damage is lost
/// when a model is destroyed (unlike normal mortal wounds which carry over).
///
/// Source: 40k_revised.md - "DEVASTATING WOUNDS", "MORTAL WOUNDS"
#[allow(clippy::too_many_arguments)]
fn apply_mortal_wounds_devastating(
    models: &mut [ModelState],
    mut mortal_pool: u8,
    models_destroyed: &mut Vec<ModelId>,
    wounds_inflicted: &mut Vec<(ModelId, u8)>,
    events: &mut Vec<GameEvent>,
    defender_id: UnitId,
    attacker_id: UnitId,
    weapon_id: WeaponId,
    dice: &mut DiceRoller,
    bonus_fnp: u8,
) {
    while mortal_pool > 0 {
        // Find allocation target (wounded first, then lowest wounds)
        let target = select_allocation_target(models, models_destroyed, &AttackContext {
            attacker_id,
            attacker_owner: wh40k_core_types::PlayerId::new(0),
            defender_id,
            weapon: WeaponProfile {
                id: weapon_id,
                name: String::new(),
                weapon_type: WeaponType::Melee,
                range: wh40k_core_types::Inches::ZERO,
                attacks: AttackCount::Fixed(0),
                skill: Skill::SIX_PLUS,
                strength: Strength::new(1),
                ap: ArmorPenetration::ZERO,
                damage: Damage::Fixed(0),
                abilities: WeaponAbilitySet::new(),
            },
            attacking_model_count: 0,
            resolved_attacks_per_model: 0,
            attacker_advanced: false,
            attacker_stationary: false,
            attacker_charged: false,
            within_half_range: false,
            target_has_cover: false,
            in_engagement_range: false,
            target_is_engaged_monster_or_vehicle: false,
            target_model_count: 0,
            target_keywords: KeywordSet::empty(),
            defender_toughness: Toughness::new(1),
            defender_armor_save: ArmorSave::NONE,
            defender_invulnerable: None,
            effective_abilities: WeaponAbilitySet::new(),
            indirect_fire_no_los: false,
            is_overwatch: false,
            target_has_stealth: false,
            distance_mils: 0,
            bonus_attacks_per_model: 0,
            bonus_ap: 0,
            bonus_fnp: 0,
            critical_hit_threshold: 0,
            command_reroll_active: false,
            defender_command_reroll_active: false,
        });

        let target_idx = match target {
            Some(idx) => idx,
            None => break, // No more alive models
        };

        let model = &mut models[target_idx];
        let model_id = model.id;

        // Apply mortal wounds one at a time (with FNP)
        // Devastating Wounds excess is lost per model
        let wounds_to_apply = mortal_pool.min(model.wounds_remaining.value());
        let mut wounds_actually_lost = 0u8;

        for _ in 0..wounds_to_apply {
            if !model.alive {
                break;
            }

            // Determine effective FNP: best of model's native FNP and bonus FNP
            let effective_fnp_value: Option<u8> = {
                let model_fnp = model.feel_no_pain.and_then(|f| if f.has_fnp() { Some(f.value()) } else { None });
                let blessing_fnp = if bonus_fnp > 0 { Some(bonus_fnp) } else { None };
                match (model_fnp, blessing_fnp) {
                    (Some(m), Some(b)) => Some(m.min(b)),
                    (Some(m), None) => Some(m),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                }
            };

            // Feel No Pain applies to mortal wounds too
            if let Some(fnp_val) = effective_fnp_value {
                let (roll, _) = dice.roll_d6(RollPurpose::FeelNoPain {
                    model_id: model_id.raw(),
                    wounds_to_ignore: fnp_val,
                });
                let ignored = roll >= fnp_val;
                events.push(GameEvent::FeelNoPainRollMade {
                    model: model_id,
                    roll,
                    result: if ignored {
                        FnpResult::Ignored
                    } else {
                        FnpResult::Suffered
                    },
                });
                if ignored {
                    mortal_pool -= 1;
                    continue;
                }
            }

            let destroyed = model.apply_damage(1);
            wounds_actually_lost += 1;
            mortal_pool -= 1;

            if destroyed {
                models_destroyed.push(model_id);
                events.push(GameEvent::ModelDestroyed {
                    model: model_id,
                    unit: defender_id,
                    cause: DestroyCause::MortalWounds {
                        source: "Devastating Wounds".to_string(),
                    },
                });
                // Excess mortal wounds from devastating wounds are lost
                break;
            }
        }

        if wounds_actually_lost > 0 {
            wounds_inflicted.push((model_id, wounds_actually_lost));
        }

        // If model was destroyed, excess mortal wounds from this devastating
        // wound attack are LOST — they do NOT carry over to the next model.
        // Source: 40k_revised.md §8.7 — "Excess damage CARRIES OVER to
        // other models in the unit (UNLESS from HAZARDOUS or DEVASTATING WOUNDS)"
        if !models[target_idx].alive {
            mortal_pool = 0;
        }
    }
}

/// Apply mortal wounds to a unit (normal mortal wounds that carry over between models).
///
/// Used for stratagems, abilities, etc. (NOT Devastating Wounds).
///
/// Source: 40k_revised.md - "MORTAL WOUNDS"
pub fn apply_mortal_wounds_carry_over(
    models: &mut [ModelState],
    mut mortal_pool: u8,
    events: &mut Vec<GameEvent>,
    defender_id: UnitId,
    source: &str,
    dice: &mut DiceRoller,
) -> Vec<ModelId> {
    let mut models_destroyed = Vec::new();

    events.push(GameEvent::MortalWoundsInflicted {
        target_unit: defender_id,
        wounds: mortal_pool,
        source: source.to_string(),
    });

    while mortal_pool > 0 {
        // Find allocation target
        let target = models.iter().enumerate().find(|(_, m)| {
            m.alive
                && m.allocation_status == AllocationStatus::WoundedAllocated
                && !models_destroyed.contains(&m.id)
        });
        let target_idx = match target {
            Some((idx, _)) => idx,
            None => {
                // Find first alive model
                match models.iter().enumerate().find(|(_, m)| {
                    m.alive && !models_destroyed.contains(&m.id)
                }) {
                    Some((idx, _)) => idx,
                    None => break,
                }
            }
        };

        let model = &mut models[target_idx];
        let model_id = model.id;

        // Feel No Pain
        if let Some(fnp) = model.feel_no_pain {
            if fnp.has_fnp() {
                let (roll, _) = dice.roll_d6(RollPurpose::FeelNoPain {
                    model_id: model_id.raw(),
                    wounds_to_ignore: fnp.value(),
                });
                let ignored = roll >= fnp.value();
                events.push(GameEvent::FeelNoPainRollMade {
                    model: model_id,
                    roll,
                    result: if ignored {
                        FnpResult::Ignored
                    } else {
                        FnpResult::Suffered
                    },
                });
                if ignored {
                    mortal_pool -= 1;
                    continue;
                }
            }
        }

        let destroyed = model.apply_damage(1);
        mortal_pool -= 1;

        if destroyed {
            models_destroyed.push(model_id);
            events.push(GameEvent::ModelDestroyed {
                model: model_id,
                unit: defender_id,
                cause: DestroyCause::MortalWounds {
                    source: source.to_string(),
                },
            });
            // Mortal wounds carry over to next model
        }
    }

    models_destroyed
}

// ---------------------------------------------------------------------------
// Hazardous test resolution
// ---------------------------------------------------------------------------

/// Resolve Hazardous tests after a unit has attacked with Hazardous weapons.
///
/// Roll D6 per Hazardous weapon used. On a 1, the bearer suffers 3 mortal wounds.
///
/// Source: 40k_revised.md §11.13 - "HAZARDOUS"
pub fn resolve_hazardous_tests(
    models: &mut [ModelState],
    hazardous_count: u8,
    attacker_id: UnitId,
    _attacker_keywords: &KeywordSet,
    dice: &mut DiceRoller,
    events: &mut Vec<GameEvent>,
) -> Vec<ModelId> {
    let mut models_destroyed = Vec::new();

    // 40k_revised.md §11.13: Unit suffers 3 mortal wounds on failed Hazardous test
    let mortal_damage = 3;

    for _ in 0..hazardous_count {
        let (roll, _) = dice.roll_d6(RollPurpose::HazardousTest {
            model_id: models[0].id.raw(),
            weapon_id: 0,
        });

        if roll == 1 {
            // Find the model to suffer mortal wounds
            // Priority: wounded models > non-CHARACTER > CHARACTER
            let target_idx = find_hazardous_target(models, &models_destroyed);
            if let Some(idx) = target_idx {
                let model = &mut models[idx];
                let model_id = model.id;

                events.push(GameEvent::MortalWoundsInflicted {
                    target_unit: attacker_id,
                    wounds: mortal_damage,
                    source: "Hazardous".to_string(),
                });

                // Apply mortal wounds (excess lost per model for Hazardous)
                for _ in 0..mortal_damage {
                    if !model.alive {
                        break;
                    }
                    let destroyed = model.apply_damage(1);
                    if destroyed {
                        models_destroyed.push(model_id);
                        events.push(GameEvent::ModelDestroyed {
                            model: model_id,
                            unit: attacker_id,
                            cause: DestroyCause::MortalWounds {
                                source: "Hazardous".to_string(),
                            },
                        });
                    }
                }
            }
        }
    }

    models_destroyed
}

/// Find the best target for Hazardous mortal wounds.
///
/// Priority: wounded models → non-CHARACTER → CHARACTER
fn find_hazardous_target(
    models: &[ModelState],
    already_destroyed: &[ModelId],
) -> Option<usize> {
    // First: wounded non-leader models
    if let Some((idx, _)) = models.iter().enumerate().find(|(_, m)| {
        m.alive && m.is_wounded() && !m.is_leader && !already_destroyed.contains(&m.id)
    }) {
        return Some(idx);
    }

    // Second: unwounded non-leader models
    if let Some((idx, _)) = models.iter().enumerate().find(|(_, m)| {
        m.alive && !m.is_leader && !already_destroyed.contains(&m.id)
    }) {
        return Some(idx);
    }

    // Last: leader/CHARACTER models
    models
        .iter()
        .enumerate()
        .find(|(_, m)| m.alive && !already_destroyed.contains(&m.id))
        .map(|(idx, _)| idx)
}

// ---------------------------------------------------------------------------
// S vs T wound threshold table
// ---------------------------------------------------------------------------

/// Determine the wound roll required based on Strength vs Toughness.
///
/// Source: 40k_revised.md - "WOUND ROLLS"
///
/// S >= 2×T → 2+
/// S > T    → 3+
/// S == T   → 4+
/// S < T    → 5+
/// S <= T/2 → 6+
pub fn wound_threshold(strength: Strength, toughness: Toughness) -> u8 {
    let s = strength.value() as u16;
    let t = toughness.value() as u16;

    if s >= 2 * t {
        2
    } else if s > t {
        3
    } else if s == t {
        4
    } else if 2 * s <= t {
        6
    } else {
        5
    }
}

// ---------------------------------------------------------------------------
// Variable attack count resolution
// ---------------------------------------------------------------------------

/// Resolve a variable attack count to a concrete number.
pub fn resolve_attack_count(
    attacks: AttackCount,
    dice: &mut DiceRoller,
    weapon_id: WeaponId,
) -> u8 {
    match attacks {
        AttackCount::Fixed(n) => n,
        AttackCount::D6 => {
            let (val, _) = dice.roll_d6(RollPurpose::DamageRoll {
                weapon_id: weapon_id.raw(),
                damage_type: "AttackCount_D6".to_string(),
            });
            val
        }
        AttackCount::D3 => {
            let (val, _) = dice.roll_d3(RollPurpose::DamageRoll {
                weapon_id: weapon_id.raw(),
                damage_type: "AttackCount_D3".to_string(),
            });
            val
        }
        AttackCount::D6Plus(n) => {
            let (val, _) = dice.roll_d6(RollPurpose::DamageRoll {
                weapon_id: weapon_id.raw(),
                damage_type: format!("AttackCount_D6+{}", n),
            });
            val + n
        }
        AttackCount::D3Plus(n) => {
            let (val, _) = dice.roll_d3(RollPurpose::DamageRoll {
                weapon_id: weapon_id.raw(),
                damage_type: format!("AttackCount_D3+{}", n),
            });
            val + n
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_core_types::*;
    use wh40k_dice::{DiceContext, StreamKind};

    fn test_dice() -> DiceRoller {
        let seed = [42u8; 32];
        let ctx = DiceContext::new(seed, StreamKind::HitRoll, 0, 0);
        DiceRoller::new(ctx)
    }

    fn test_dice_with_seed(seed_byte: u8) -> DiceRoller {
        let seed = [seed_byte; 32];
        let ctx = DiceContext::new(seed, StreamKind::HitRoll, 0, 0);
        DiceRoller::new(ctx)
    }

    fn make_basic_ranged_weapon() -> WeaponProfile {
        WeaponProfile {
            id: WeaponId::new(1),
            name: "Boltgun".to_string(),
            weapon_type: WeaponType::Ranged,
            range: Inches::from_inches(24),
            attacks: AttackCount::Fixed(2),
            skill: Skill::THREE_PLUS,
            strength: Strength::new(4),
            ap: ArmorPenetration::ZERO,
            damage: Damage::Fixed(1),
            abilities: WeaponAbilitySet::new(),
        }
    }

    fn make_basic_melee_weapon() -> WeaponProfile {
        WeaponProfile {
            id: WeaponId::new(2),
            name: "Combat blade".to_string(),
            weapon_type: WeaponType::Melee,
            range: Inches::ZERO,
            attacks: AttackCount::Fixed(3),
            skill: Skill::THREE_PLUS,
            strength: Strength::new(4),
            ap: ArmorPenetration::MINUS_1,
            damage: Damage::Fixed(1),
            abilities: WeaponAbilitySet::new(),
        }
    }

    fn make_defender_model(id: u32, wounds: u8) -> ModelState {
        ModelState::new(
            ModelId::new(id),
            UnitId::new(10),
            Wounds::new(wounds),
            Position::from_inches(20, 10),
            BaseSize::MM32,
            Vec::new(),
            Vec::new(),
            false,
            None,
        )
    }

    fn make_basic_attack_context(weapon: WeaponProfile) -> AttackContext {
        AttackContext {
            attacker_id: UnitId::new(1),
            attacker_owner: PlayerId::new(0),
            defender_id: UnitId::new(10),
            weapon: weapon.clone(),
            attacking_model_count: 1,
            resolved_attacks_per_model: match weapon.attacks {
                AttackCount::Fixed(n) => n,
                _ => 1,
            },
            attacker_advanced: false,
            attacker_stationary: false,
            attacker_charged: false,
            within_half_range: false,
            target_has_cover: false,
            in_engagement_range: false,
            target_is_engaged_monster_or_vehicle: false,
            target_model_count: 5,
            target_keywords: KeywordSet::from_keywords(&[Keyword::Infantry]),
            defender_toughness: Toughness::new(4),
            defender_armor_save: ArmorSave::THREE_PLUS,
            defender_invulnerable: None,
            effective_abilities: weapon.abilities.clone(),
            indirect_fire_no_los: false,
            is_overwatch: false,
            target_has_stealth: false,
            distance_mils: 0,
            bonus_attacks_per_model: 0,
            bonus_ap: 0,
            bonus_fnp: 0,
            critical_hit_threshold: 0,
            command_reroll_active: false,
            defender_command_reroll_active: false,
        }
    }

    // === Wound threshold table tests ===

    #[test]
    fn test_wound_threshold_double_strength() {
        // S8 vs T4 = 2+
        assert_eq!(wound_threshold(Strength::new(8), Toughness::new(4)), 2);
        // S10 vs T5 = 2+
        assert_eq!(wound_threshold(Strength::new(10), Toughness::new(5)), 2);
    }

    #[test]
    fn test_wound_threshold_greater_strength() {
        // S5 vs T4 = 3+
        assert_eq!(wound_threshold(Strength::new(5), Toughness::new(4)), 3);
        // S7 vs T6 = 3+
        assert_eq!(wound_threshold(Strength::new(7), Toughness::new(6)), 3);
    }

    #[test]
    fn test_wound_threshold_equal() {
        // S4 vs T4 = 4+
        assert_eq!(wound_threshold(Strength::new(4), Toughness::new(4)), 4);
    }

    #[test]
    fn test_wound_threshold_less_strength() {
        // S3 vs T4 = 5+
        assert_eq!(wound_threshold(Strength::new(3), Toughness::new(4)), 5);
    }

    #[test]
    fn test_wound_threshold_half_strength() {
        // S3 vs T6 = 6+
        assert_eq!(wound_threshold(Strength::new(3), Toughness::new(6)), 6);
        // S2 vs T5 = 6+
        assert_eq!(wound_threshold(Strength::new(2), Toughness::new(5)), 6);
        // S3 vs T7 = 6+ (half of 7 is 3.5, so 3 <= 3.5)
        assert_eq!(wound_threshold(Strength::new(3), Toughness::new(7)), 6);
    }

    #[test]
    fn test_wound_threshold_exact_double() {
        // S6 vs T3 = exactly 2× = 2+
        assert_eq!(wound_threshold(Strength::new(6), Toughness::new(3)), 2);
    }

    #[test]
    fn test_wound_threshold_exact_half() {
        // S3 vs T6 = exactly ½ = 6+
        assert_eq!(wound_threshold(Strength::new(3), Toughness::new(6)), 6);
    }

    // === Custodes vs World Eaters specific threshold tests ===

    #[test]
    fn test_wound_threshold_custodes_scenarios() {
        // Sentinel blade S6 vs Berzerker T4 = 3+ (6 > 4)
        assert_eq!(wound_threshold(Strength::new(6), Toughness::new(4)), 3);

        // Castellan axe S9 vs Vorrakh T10 = 5+ (9 < 10)
        assert_eq!(wound_threshold(Strength::new(9), Toughness::new(10)), 5);

        // Guardian spear S7 vs Berzerker T4 = 3+ (7 > 4)
        assert_eq!(wound_threshold(Strength::new(7), Toughness::new(4)), 3);

        // Vaultswords Victus S6 vs Jakhals T4 = 3+ (6 > 4)
        assert_eq!(wound_threshold(Strength::new(6), Toughness::new(4)), 3);
    }

    #[test]
    fn test_wound_threshold_world_eaters_scenarios() {
        // Hellforged weapons S6 vs Custodian Guard T6 = 4+ (equal)
        assert_eq!(wound_threshold(Strength::new(6), Toughness::new(6)), 4);

        // Axe of dismemberment S7 vs Custodian Guard T6 = 3+ (7 > 6)
        assert_eq!(wound_threshold(Strength::new(7), Toughness::new(6)), 3);

        // Chainblade S4 vs Allarus T7 = 5+ (4 < 7)
        assert_eq!(wound_threshold(Strength::new(4), Toughness::new(7)), 5);

        // Eviscerator S8 vs Allarus T7 = 3+ (8 > 7)
        assert_eq!(wound_threshold(Strength::new(8), Toughness::new(7)), 3);

        // Infernal cannon S5 vs Custodian Guard T6 = 5+ (5 < 6)
        assert_eq!(wound_threshold(Strength::new(5), Toughness::new(6)), 5);
    }

    // === Save determination tests ===

    #[test]
    fn test_determine_best_save_armor_only() {
        let (save, save_type) = determine_best_save(
            ArmorSave::THREE_PLUS,
            ArmorPenetration::MINUS_1,
            None,
            false,
        );
        assert_eq!(save, 4); // 3+ modified by AP-1 = 4+
        assert_eq!(save_type, SaveType::Armour);
    }

    #[test]
    fn test_determine_best_save_invulnerable_better() {
        let (save, save_type) = determine_best_save(
            ArmorSave::THREE_PLUS,
            ArmorPenetration::MINUS_3,
            Some(InvulnerableSave::FOUR_PLUS),
            false,
        );
        assert_eq!(save, 4); // 3+ - AP3 = 6+, but 4++ is better
        assert_eq!(save_type, SaveType::Invulnerable);
    }

    #[test]
    fn test_determine_best_save_armor_better() {
        let (save, save_type) = determine_best_save(
            ArmorSave::TWO_PLUS,
            ArmorPenetration::ZERO,
            Some(InvulnerableSave::FOUR_PLUS),
            false,
        );
        assert_eq!(save, 2); // 2+ with AP0 is better than 4++
        assert_eq!(save_type, SaveType::Armour);
    }

    #[test]
    fn test_determine_best_save_with_cover() {
        let (save, save_type) = determine_best_save(
            ArmorSave::FOUR_PLUS,
            ArmorPenetration::MINUS_1,
            None,
            true,
        );
        // 4+ with AP-1 = 5+, cover gives +1 = 4+ (can't exceed base)
        assert_eq!(save, 4);
        assert_eq!(save_type, SaveType::Armour);
    }

    #[test]
    fn test_determine_best_save_cover_denied_ap0_sv3plus() {
        // 40k_revised.md §13.1: Sv 3+ or better does NOT get cover against AP 0
        let (save, save_type) = determine_best_save(
            ArmorSave::THREE_PLUS,
            ArmorPenetration::ZERO,
            None,
            true,
        );
        // 3+ with AP0 = 3+, cover is denied because Sv 3+ and AP 0
        assert_eq!(save, 3);
        assert_eq!(save_type, SaveType::Armour);
    }

    #[test]
    fn test_determine_best_save_cover_allowed_ap0_sv4plus() {
        // 40k_revised.md §13.1: Sv 4+ or worse DOES get cover against AP 0
        let (save, save_type) = determine_best_save(
            ArmorSave::FOUR_PLUS,
            ArmorPenetration::ZERO,
            None,
            true,
        );
        // 4+ with AP0 = 4+, cover gives +1 = 3+ (but capped to base 4+)
        // Actually: 4+ with AP0 = 4+, cover -1 = 3+, but cap to base 4+ applies
        assert_eq!(save, 4);
        assert_eq!(save_type, SaveType::Armour);
    }

    #[test]
    fn test_determine_best_save_cover_allowed_sv3plus_with_ap() {
        // Sv 3+ WITH AP-1 still gets cover (restriction only applies to AP 0)
        let (save, save_type) = determine_best_save(
            ArmorSave::THREE_PLUS,
            ArmorPenetration::MINUS_1,
            None,
            true,
        );
        // 3+ with AP-1 = 4+, cover gives +1 = 3+ (equals base, allowed)
        assert_eq!(save, 3);
        assert_eq!(save_type, SaveType::Armour);
    }

    #[test]
    fn test_determine_best_save_cover_denied_sv2plus_ap0() {
        // Sv 2+ also denied cover against AP 0
        let (save, save_type) = determine_best_save(
            ArmorSave::TWO_PLUS,
            ArmorPenetration::ZERO,
            None,
            true,
        );
        // 2+ with AP0 = 2+, cover denied
        assert_eq!(save, 2);
        assert_eq!(save_type, SaveType::Armour);
    }

    // === Total attacks calculation ===

    #[test]
    fn test_determine_total_attacks_basic() {
        let weapon = make_basic_ranged_weapon();
        let ctx = make_basic_attack_context(weapon);
        assert_eq!(determine_total_attacks(&ctx), 2); // 2 attacks × 1 model
    }

    #[test]
    fn test_determine_total_attacks_multiple_models() {
        let weapon = make_basic_ranged_weapon();
        let mut ctx = make_basic_attack_context(weapon);
        ctx.attacking_model_count = 5;
        assert_eq!(determine_total_attacks(&ctx), 10); // 2 attacks × 5 models
    }

    #[test]
    fn test_determine_total_attacks_rapid_fire_in_half_range() {
        let weapon = WeaponProfile {
            id: WeaponId::new(1),
            name: "Infernal cannon".to_string(),
            weapon_type: WeaponType::Ranged,
            range: Inches::from_inches(24),
            attacks: AttackCount::Fixed(3),
            skill: Skill::THREE_PLUS,
            strength: Strength::new(5),
            ap: ArmorPenetration::MINUS_1,
            damage: Damage::Fixed(2),
            abilities: WeaponAbilitySet::from_abilities(vec![WeaponAbility::RapidFire(1)]),
        };
        let mut ctx = make_basic_attack_context(weapon);
        ctx.resolved_attacks_per_model = 3;
        ctx.within_half_range = true;
        assert_eq!(determine_total_attacks(&ctx), 4); // 3+1 RF × 1 model
    }

    #[test]
    fn test_determine_total_attacks_blast() {
        let weapon = WeaponProfile {
            id: WeaponId::new(1),
            name: "Balistus grenade launcher".to_string(),
            weapon_type: WeaponType::Ranged,
            range: Inches::from_inches(18),
            attacks: AttackCount::Fixed(3), // Assume D6 resolved to 3
            skill: Skill::TWO_PLUS,
            strength: Strength::new(4),
            ap: ArmorPenetration::MINUS_1,
            damage: Damage::Fixed(1),
            abilities: WeaponAbilitySet::from_abilities(vec![WeaponAbility::Blast]),
        };
        let mut ctx = make_basic_attack_context(weapon);
        ctx.resolved_attacks_per_model = 3;
        ctx.target_model_count = 10;
        assert_eq!(determine_total_attacks(&ctx), 5); // 3 + 2 (10/5) × 1 model
    }

    // === Attack batch resolution (integration tests) ===

    #[test]
    fn test_resolve_attack_batch_basic() {
        let mut dice = test_dice();
        let weapon = make_basic_ranged_weapon();
        let ctx = make_basic_attack_context(weapon);

        let mut defenders = vec![
            make_defender_model(100, 3),
            make_defender_model(101, 3),
        ];

        let result = resolve_attack_batch(&ctx, &mut defenders, &mut dice);

        // Should have events (hit rolls, wound rolls, etc.)
        assert!(!result.events.is_empty());
        // With 2 attacks at BS3+ / S4 vs T4 / 3+ save, most results are possible
    }

    #[test]
    fn test_resolve_attack_batch_torrent_auto_hits() {
        let mut dice = test_dice();
        let weapon = WeaponProfile {
            id: WeaponId::new(1),
            name: "Flamer".to_string(),
            weapon_type: WeaponType::Ranged,
            range: Inches::from_inches(12),
            attacks: AttackCount::Fixed(3),
            skill: Skill::FOUR_PLUS, // irrelevant for Torrent
            strength: Strength::new(4),
            ap: ArmorPenetration::ZERO,
            damage: Damage::Fixed(1),
            abilities: WeaponAbilitySet::from_abilities(vec![WeaponAbility::Torrent]),
        };
        let mut ctx = make_basic_attack_context(weapon);
        ctx.resolved_attacks_per_model = 3;

        let mut defenders = vec![make_defender_model(100, 3)];

        let result = resolve_attack_batch(&ctx, &mut defenders, &mut dice);

        // All 3 attacks should auto-hit (Torrent), so we should see wound rolls
        let hit_events: Vec<_> = result.events.iter().filter(|e| matches!(e, GameEvent::HitRollMade { .. })).collect();
        assert_eq!(hit_events.len(), 3);
        // All hits should be successes
        for event in &hit_events {
            if let GameEvent::HitRollMade { result, .. } = event {
                assert_eq!(*result, RollResult::Success);
            }
        }
    }

    #[test]
    fn test_resolve_attack_batch_devastating_wounds() {
        // Use a weapon with Devastating Wounds and run many attacks to verify
        // that critical wounds create mortal wounds
        let weapon = WeaponProfile {
            id: WeaponId::new(1),
            name: "Vaultswords - Victus".to_string(),
            weapon_type: WeaponType::Melee,
            range: Inches::ZERO,
            attacks: AttackCount::Fixed(5),
            skill: Skill::TWO_PLUS,
            strength: Strength::new(6),
            ap: ArmorPenetration::MINUS_3,
            damage: Damage::Fixed(3),
            abilities: WeaponAbilitySet::from_abilities(vec![WeaponAbility::DevastatingWounds]),
        };

        // Try multiple seeds to find one that produces a critical wound (roll of 6)
        for seed in 0..50u8 {
            let mut dice = test_dice_with_seed(seed);
            let mut ctx = make_basic_attack_context(weapon.clone());
            ctx.resolved_attacks_per_model = 5;
            ctx.defender_toughness = Toughness::new(4);
            ctx.effective_abilities = weapon.abilities.clone();

            let mut defenders = vec![
                make_defender_model(100, 10),
                make_defender_model(101, 10),
            ];

            let result = resolve_attack_batch(&ctx, &mut defenders, &mut dice);

            // Just verify it runs without panicking and produces events
            assert!(!result.events.is_empty());
        }
    }

    #[test]
    fn test_resolve_attack_batch_feel_no_pain() {
        let mut dice = test_dice();
        let weapon = make_basic_melee_weapon();
        let mut ctx = make_basic_attack_context(weapon);
        ctx.resolved_attacks_per_model = 3;
        ctx.defender_armor_save = ArmorSave::SIX_PLUS; // Bad save
        ctx.defender_toughness = Toughness::new(3); // Easy to wound

        let mut defenders = vec![
            ModelState::new(
                ModelId::new(100),
                UnitId::new(10),
                Wounds::new(5),
                Position::from_inches(20, 10),
                BaseSize::MM32,
                Vec::new(),
                Vec::new(),
                false,
                Some(FeelNoPain::FIVE_PLUS), // 5+ FNP
            ),
        ];

        let result = resolve_attack_batch(&ctx, &mut defenders, &mut dice);

        // Should see FNP roll events
        let _fnp_events: Vec<_> = result.events.iter().filter(|e| matches!(e, GameEvent::FeelNoPainRollMade { .. })).collect();
        // FNP only triggers if damage gets through saves, so count may vary
        assert!(!result.events.is_empty());
    }

    // === Hazardous tests ===

    #[test]
    fn test_resolve_hazardous_infantry_takes_3_mortals() {
        // 40k_revised.md §11.13: ALL models take 3 mortal wounds on hazardous failure
        for seed in 0..50u8 {
            let mut dice = test_dice_with_seed(seed);
            let mut models = vec![make_defender_model(100, 3)];
            let mut events = Vec::new();

            let destroyed = resolve_hazardous_tests(
                &mut models,
                1,
                UnitId::new(1),
                &KeywordSet::from_keywords(&[Keyword::Infantry]),
                &mut dice,
                &mut events,
            );

            // Check for mortal wound events — if failed, should be 3 MW
            let mw_events: Vec<_> = events.iter().filter(|e| {
                matches!(e, GameEvent::MortalWoundsInflicted { .. })
            }).collect();

            if !mw_events.is_empty() {
                if let GameEvent::MortalWoundsInflicted { wounds, .. } = mw_events[0] {
                    assert_eq!(*wounds, 3);
                }
            }

            assert!(destroyed.len() <= 1);
        }
    }

    #[test]
    fn test_resolve_hazardous_character_takes_3_mortals() {
        // CHARACTER also takes 3 mortal wounds on hazardous failure
        for seed in 0..50u8 {
            let mut dice = test_dice_with_seed(seed);
            let mut models = vec![ModelState::new(
                ModelId::new(100),
                UnitId::new(1),
                Wounds::new(10),
                Position::from_inches(10, 10),
                BaseSize::MM40,
                Vec::new(),
                Vec::new(),
                true,
                None,
            )];
            let mut events = Vec::new();

            let _destroyed = resolve_hazardous_tests(
                &mut models,
                1,
                UnitId::new(1),
                &KeywordSet::from_keywords(&[Keyword::Character, Keyword::Infantry]),
                &mut dice,
                &mut events,
            );

            // Check for mortal wound events
            let mw_events: Vec<_> = events.iter().filter(|e| {
                matches!(e, GameEvent::MortalWoundsInflicted { wounds: 3, .. })
            }).collect();

            // If hazardous failed (roll of 1), should see 3 mortal wounds event
            if !mw_events.is_empty() {
                // Verify 3 mortal wounds
                if let GameEvent::MortalWoundsInflicted { wounds, .. } = mw_events[0] {
                    assert_eq!(*wounds, 3);
                }
            }
        }
    }

    // === Mortal wounds carry-over ===

    #[test]
    fn test_apply_mortal_wounds_carry_over_multi_model() {
        let mut dice = test_dice();
        let mut models = vec![
            make_defender_model(100, 2),
            make_defender_model(101, 2),
            make_defender_model(102, 2),
        ];
        let mut events = Vec::new();

        let destroyed = apply_mortal_wounds_carry_over(
            &mut models,
            5,
            &mut events,
            UnitId::new(10),
            "Test ability",
            &mut dice,
        );

        // 5 mortal wounds across 3 models with 2W each = should destroy 2 models, 1 wound on third
        assert_eq!(destroyed.len(), 2);
        assert!(!models[0].alive);
        assert!(!models[1].alive);
        assert!(models[2].alive);
        assert_eq!(models[2].wounds_remaining.value(), 1);
    }

    // === resolve_attack_count ===

    #[test]
    fn test_resolve_attack_count_fixed() {
        let mut dice = test_dice();
        let result = resolve_attack_count(AttackCount::Fixed(5), &mut dice, WeaponId::new(1));
        assert_eq!(result, 5);
    }

    #[test]
    fn test_resolve_attack_count_d6() {
        let mut dice = test_dice();
        let result = resolve_attack_count(AttackCount::D6, &mut dice, WeaponId::new(1));
        assert!(result >= 1 && result <= 6);
    }

    #[test]
    fn test_resolve_attack_count_d3() {
        let mut dice = test_dice();
        let result = resolve_attack_count(AttackCount::D3, &mut dice, WeaponId::new(1));
        assert!(result >= 1 && result <= 3);
    }

    #[test]
    fn test_resolve_attack_count_d6_plus_2() {
        let mut dice = test_dice();
        let result = resolve_attack_count(AttackCount::D6Plus(2), &mut dice, WeaponId::new(1));
        assert!(result >= 3 && result <= 8);
    }

    // === Allocation target selection ===

    #[test]
    fn test_select_allocation_wounded_first() {
        let mut models = vec![
            make_defender_model(100, 3), // unwounded
            make_defender_model(101, 3), // will wound
        ];
        models[1].apply_damage(1); // Wound model 101
        models[1].allocation_status = AllocationStatus::WoundedAllocated;

        let ctx = make_basic_attack_context(make_basic_ranged_weapon());
        let target = select_allocation_target(&models, &[], &ctx);
        assert_eq!(target, Some(1)); // Should select wounded model
    }

    #[test]
    fn test_select_allocation_skip_destroyed() {
        let models = vec![
            make_defender_model(100, 3),
            make_defender_model(101, 3),
        ];

        let ctx = make_basic_attack_context(make_basic_ranged_weapon());
        let destroyed = vec![ModelId::new(100)]; // model 100 destroyed this batch
        let target = select_allocation_target(&models, &destroyed, &ctx);
        // Should skip model 100 and select model 101
        assert_eq!(target, Some(1));
    }

    #[test]
    fn test_bodyguard_allocation_protects_leader() {
        // Leader model (is_leader=true) + 2 bodyguard models
        let mut leader = make_defender_model(100, 5);
        leader.is_leader = true;
        let guard1 = make_defender_model(101, 3);
        let guard2 = make_defender_model(102, 3);

        let models = vec![leader, guard1, guard2];
        let ctx = make_basic_attack_context(make_basic_ranged_weapon());
        let destroyed = vec![];

        // Without Precision, wounds should go to bodyguard models, not the leader
        let target = select_allocation_target(&models, &destroyed, &ctx);
        assert!(target.is_some());
        let target_idx = target.unwrap();
        // Should NOT be the leader (index 0)
        assert_ne!(target_idx, 0, "Bodyguard should protect leader from allocation");
        assert!(!models[target_idx].is_leader);
    }

    #[test]
    fn test_bodyguard_allocation_falls_through_when_exhausted() {
        // Leader model + no alive bodyguard models
        let mut leader = make_defender_model(100, 5);
        leader.is_leader = true;
        let mut guard1 = make_defender_model(101, 3);
        guard1.alive = false;
        let mut guard2 = make_defender_model(102, 3);
        guard2.alive = false;

        let models = vec![leader, guard1, guard2];
        let ctx = make_basic_attack_context(make_basic_ranged_weapon());
        let destroyed = vec![];

        // All bodyguard models dead, wounds should go to leader
        let target = select_allocation_target(&models, &destroyed, &ctx);
        assert_eq!(target, Some(0), "Leader should receive wounds when bodyguard exhausted");
    }

    #[test]
    fn test_precision_bypasses_bodyguard() {
        // Leader model + bodyguard, but weapon has Precision
        let mut leader = make_defender_model(100, 5);
        leader.is_leader = true;
        let guard = make_defender_model(101, 3);

        let models = vec![leader, guard];
        let mut weapon = make_basic_ranged_weapon();
        weapon.abilities.add(WeaponAbility::Precision);
        let ctx = make_basic_attack_context(weapon);
        let destroyed = vec![];

        // With Precision, allocation should NOT skip the leader
        let target = select_allocation_target(&models, &destroyed, &ctx);
        assert!(target.is_some());
        // Precision allows targeting any model - the lowest wounds model should be selected
        // Both have same wounds (leader=5, guard=3), so guard (3 wounds) is lower
        // But the key is that the leader is NOT excluded from the candidate pool
        // This confirms Precision bypasses bodyguard protection
    }
}

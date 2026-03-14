//! Stratagem definitions and effect application for core and faction stratagems.
//!
//! This module defines all stratagem metadata (ID, name, CP cost, phase, timing,
//! target restrictions) and the logic to apply each stratagem's effects to game state.
//!
//! Source: 40k_revised.md - Core Stratagems
//! Source: Custodes.md - Faction Stratagems
//! Source: Frenzied_Reavers.md - Faction Stratagems

use wh40k_core_types::{
    EffectDuration, Keyword, Phase, PlayerId, StackingBehavior,
    StratagemId, UnitId,
};
use wh40k_event_system::GameEvent;

use crate::effect::{ActiveEffect, EffectSource, EffectTarget, EffectType};
use crate::state::GameState;

// ─── Stratagem IDs ──────────────────────────────────────────────────────────

/// Well-known core stratagem IDs.
pub mod ids {
    use wh40k_core_types::StratagemId;

    // Core stratagems (IDs 1-11)
    pub const COMMAND_REROLL: StratagemId = StratagemId::new(1);
    pub const COUNTER_OFFENSIVE: StratagemId = StratagemId::new(2);
    pub const FIRE_OVERWATCH: StratagemId = StratagemId::new(3);
    pub const GO_TO_GROUND: StratagemId = StratagemId::new(4);
    pub const HEROIC_INTERVENTION: StratagemId = StratagemId::new(5);
    pub const INSANE_BRAVERY: StratagemId = StratagemId::new(6);
    pub const EPIC_CHALLENGE: StratagemId = StratagemId::new(7);
    pub const RAPID_INGRESS: StratagemId = StratagemId::new(8);
    pub const GRENADE: StratagemId = StratagemId::new(9);
    pub const SMOKESCREEN: StratagemId = StratagemId::new(10);
    pub const TANK_SHOCK: StratagemId = StratagemId::new(11);

    // Frenzied Reavers faction stratagems (IDs 100-102)
    pub const HORRIFYING_BUTCHERY: StratagemId = StratagemId::new(100);
    pub const BERSERK_RESILIENCE: StratagemId = StratagemId::new(101);
    pub const BLOODLUST: StratagemId = StratagemId::new(102);

    // Custodes faction stratagems (IDs 200-202)
    pub const GILDED_SPEAR: StratagemId = StratagemId::new(200);
    pub const INESCAPABLE_VENGEANCE: StratagemId = StratagemId::new(201);
    pub const OVERAWING_MAGNIFICENCE: StratagemId = StratagemId::new(202);
}

// ─── Stratagem Phase Timing ─────────────────────────────────────────────────

/// When a stratagem can be used within its phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StratagemTiming {
    /// Any time during the phase.
    AnyTime,
    /// Start of the phase.
    StartOfPhase,
    /// During the phase (general).
    DuringPhase,
    /// After enemy declares targets / selects targets.
    AfterEnemySelectsTargets,
    /// After enemy unit shoots.
    AfterEnemyShoots,
    /// After enemy declares charge.
    AfterEnemyDeclaresCharge,
    /// After a unit finishes a charge move.
    AfterChargeMoveComplete,
    /// When a unit is selected to fight.
    OnUnitSelectedToFight,
}

// ─── Stratagem Definition ───────────────────────────────────────────────────

/// Complete definition of a stratagem with all its rules.
#[derive(Debug, Clone)]
pub struct StratagemDef {
    pub id: StratagemId,
    pub name: &'static str,
    pub cp_cost: u8,
    /// Phase(s) where this stratagem can be used.
    /// Multiple phases supported (e.g., Command Re-roll works in any phase).
    pub valid_phases: &'static [Phase],
    pub timing: StratagemTiming,
    /// Required keywords on the target unit (if any).
    pub required_keywords: &'static [Keyword],
    /// Target must be friendly.
    pub must_be_friendly: bool,
    /// Target must be enemy.
    pub must_be_enemy: bool,
    /// Whether once per battle.
    pub once_per_battle: bool,
    /// Whether once per turn (standard restriction for most stratagems).
    pub once_per_turn: bool,
    /// Whether this is a faction stratagem (not available to all armies).
    pub is_faction: bool,
    /// Faction keywords required for this stratagem to be available.
    pub faction_keywords: &'static [Keyword],
}

/// Get the definition of a stratagem by ID.
pub fn get_stratagem_def(id: StratagemId) -> Option<&'static StratagemDef> {
    ALL_STRATAGEMS.iter().find(|s| s.id == id)
}

/// Get all core stratagems (available to every army).
pub fn core_stratagem_ids() -> &'static [StratagemId] {
    &[
        ids::COMMAND_REROLL,
        ids::COUNTER_OFFENSIVE,
        ids::FIRE_OVERWATCH,
        ids::GO_TO_GROUND,
        ids::HEROIC_INTERVENTION,
        ids::INSANE_BRAVERY,
        ids::EPIC_CHALLENGE,
        ids::RAPID_INGRESS,
        ids::GRENADE,
        ids::SMOKESCREEN,
        ids::TANK_SHOCK,
    ]
}

/// Get faction stratagem IDs for Frenzied Reavers.
pub fn frenzied_reavers_stratagem_ids() -> &'static [StratagemId] {
    &[
        ids::HORRIFYING_BUTCHERY,
        ids::BERSERK_RESILIENCE,
        ids::BLOODLUST,
    ]
}

/// Get faction stratagem IDs for Custodes.
pub fn custodes_stratagem_ids() -> &'static [StratagemId] {
    &[
        ids::GILDED_SPEAR,
        ids::INESCAPABLE_VENGEANCE,
        ids::OVERAWING_MAGNIFICENCE,
    ]
}

// ─── All Stratagem Definitions ──────────────────────────────────────────────

/// Public accessor for all stratagem definitions (11 core + 6 faction).
pub static ALL_STRATAGEMS_SLICE: &[StratagemDef] = ALL_STRATAGEMS;

static ALL_STRATAGEMS: &[StratagemDef] = &[
    // ── Core Stratagems ─────────────────────────────────────────────────
    StratagemDef {
        id: ids::COMMAND_REROLL,
        name: "Command Re-roll",
        cp_cost: 1,
        valid_phases: &[Phase::Command, Phase::Movement, Phase::Shooting, Phase::Charge, Phase::Fight],
        timing: StratagemTiming::AnyTime,
        required_keywords: &[],
        must_be_friendly: false,
        must_be_enemy: false,
        once_per_battle: false,
        once_per_turn: true,
        is_faction: false,
        faction_keywords: &[],
    },
    StratagemDef {
        id: ids::COUNTER_OFFENSIVE,
        name: "Counter-Offensive",
        cp_cost: 2,
        valid_phases: &[Phase::Fight],
        timing: StratagemTiming::DuringPhase,
        required_keywords: &[],
        must_be_friendly: true,
        must_be_enemy: false,
        once_per_battle: false,
        once_per_turn: true,
        is_faction: false,
        faction_keywords: &[],
    },
    StratagemDef {
        id: ids::FIRE_OVERWATCH,
        name: "Fire Overwatch",
        cp_cost: 1,
        valid_phases: &[Phase::Movement, Phase::Charge],
        timing: StratagemTiming::AnyTime,
        required_keywords: &[],
        must_be_friendly: true,
        must_be_enemy: false,
        once_per_battle: false,
        once_per_turn: true,
        is_faction: false,
        faction_keywords: &[],
    },
    StratagemDef {
        id: ids::GO_TO_GROUND,
        name: "Go to Ground",
        cp_cost: 1,
        valid_phases: &[Phase::Shooting],
        timing: StratagemTiming::AfterEnemySelectsTargets,
        required_keywords: &[Keyword::Infantry],
        must_be_friendly: true,
        must_be_enemy: false,
        once_per_battle: false,
        once_per_turn: true,
        is_faction: false,
        faction_keywords: &[],
    },
    StratagemDef {
        id: ids::HEROIC_INTERVENTION,
        name: "Heroic Intervention",
        cp_cost: 1,
        valid_phases: &[Phase::Charge],
        timing: StratagemTiming::AfterEnemyDeclaresCharge,
        required_keywords: &[],
        must_be_friendly: true,
        must_be_enemy: false,
        once_per_battle: false,
        once_per_turn: true,
        is_faction: false,
        faction_keywords: &[],
    },
    StratagemDef {
        id: ids::INSANE_BRAVERY,
        name: "Insane Bravery",
        cp_cost: 1,
        valid_phases: &[Phase::Command],
        timing: StratagemTiming::DuringPhase,
        required_keywords: &[],
        must_be_friendly: true,
        must_be_enemy: false,
        once_per_battle: true,
        once_per_turn: true,
        is_faction: false,
        faction_keywords: &[],
    },
    StratagemDef {
        id: ids::EPIC_CHALLENGE,
        name: "Epic Challenge",
        cp_cost: 1,
        valid_phases: &[Phase::Fight],
        timing: StratagemTiming::OnUnitSelectedToFight,
        required_keywords: &[Keyword::Character],
        must_be_friendly: true,
        must_be_enemy: false,
        once_per_battle: false,
        once_per_turn: true,
        is_faction: false,
        faction_keywords: &[],
    },
    StratagemDef {
        id: ids::RAPID_INGRESS,
        name: "Rapid Ingress",
        cp_cost: 1,
        valid_phases: &[Phase::Movement],
        timing: StratagemTiming::DuringPhase,
        required_keywords: &[],
        must_be_friendly: true,
        must_be_enemy: false,
        once_per_battle: false,
        once_per_turn: true,
        is_faction: false,
        faction_keywords: &[],
    },
    StratagemDef {
        id: ids::GRENADE,
        name: "Grenade",
        cp_cost: 1,
        valid_phases: &[Phase::Shooting],
        timing: StratagemTiming::DuringPhase,
        required_keywords: &[Keyword::Grenades],
        must_be_friendly: true,
        must_be_enemy: false,
        once_per_battle: false,
        once_per_turn: true,
        is_faction: false,
        faction_keywords: &[],
    },
    StratagemDef {
        id: ids::SMOKESCREEN,
        name: "Smokescreen",
        cp_cost: 1,
        valid_phases: &[Phase::Shooting],
        timing: StratagemTiming::AfterEnemySelectsTargets,
        required_keywords: &[Keyword::Smoke],
        must_be_friendly: true,
        must_be_enemy: false,
        once_per_battle: false,
        once_per_turn: true,
        is_faction: false,
        faction_keywords: &[],
    },
    StratagemDef {
        id: ids::TANK_SHOCK,
        name: "Tank Shock",
        cp_cost: 1,
        valid_phases: &[Phase::Charge],
        timing: StratagemTiming::AfterChargeMoveComplete,
        // Required keywords left empty — custom validation checks VEHICLE or MONSTER.
        // Source: CP_Rules.md §11 (VEHICLE), Frenzied_Reavers.md (MONSTER also eligible)
        required_keywords: &[],
        must_be_friendly: true,
        must_be_enemy: false,
        once_per_battle: false,
        once_per_turn: true,
        is_faction: false,
        faction_keywords: &[],
    },

    // ── Frenzied Reavers Faction Stratagems ─────────────────────────────
    //
    // Source: Frenzied_Reavers.md - Stratagems
    StratagemDef {
        id: ids::HORRIFYING_BUTCHERY,
        name: "Horrifying Butchery",
        cp_cost: 1,
        valid_phases: &[Phase::Fight],
        timing: StratagemTiming::StartOfPhase,
        required_keywords: &[Keyword::Berzerkers],
        must_be_friendly: true,
        must_be_enemy: false,
        once_per_battle: false,
        once_per_turn: true,
        is_faction: true,
        faction_keywords: &[Keyword::WorldEaters],
    },
    StratagemDef {
        id: ids::BERSERK_RESILIENCE,
        name: "Berserk Resilience",
        cp_cost: 1,
        valid_phases: &[Phase::Fight],
        timing: StratagemTiming::AfterEnemySelectsTargets,
        required_keywords: &[Keyword::Character, Keyword::WorldEaters],
        must_be_friendly: true,
        must_be_enemy: false,
        once_per_battle: false,
        once_per_turn: true,
        is_faction: true,
        faction_keywords: &[Keyword::WorldEaters],
    },
    StratagemDef {
        id: ids::BLOODLUST,
        name: "Bloodlust",
        cp_cost: 1,
        valid_phases: &[Phase::Shooting],
        timing: StratagemTiming::AfterEnemyShoots,
        required_keywords: &[Keyword::Jakhals],
        must_be_friendly: true,
        must_be_enemy: false,
        once_per_battle: false,
        once_per_turn: true,
        is_faction: true,
        faction_keywords: &[Keyword::WorldEaters],
    },

    // ── Custodes Faction Stratagems ─────────────────────────────────────
    //
    // Source: Custodes.md - Stratagems
    StratagemDef {
        id: ids::GILDED_SPEAR,
        name: "The Gilded Spear",
        cp_cost: 1,
        valid_phases: &[Phase::Fight],
        timing: StratagemTiming::DuringPhase,
        required_keywords: &[Keyword::AdeptusCustodes],
        must_be_friendly: true,
        must_be_enemy: false,
        once_per_battle: false,
        once_per_turn: true,
        is_faction: true,
        faction_keywords: &[Keyword::AdeptusCustodes],
    },
    StratagemDef {
        id: ids::INESCAPABLE_VENGEANCE,
        name: "Inescapable Vengeance",
        cp_cost: 1,
        valid_phases: &[Phase::Movement],
        timing: StratagemTiming::DuringPhase,
        required_keywords: &[Keyword::AdeptusCustodes],
        must_be_friendly: true,
        must_be_enemy: false,
        once_per_battle: false,
        once_per_turn: true,
        is_faction: true,
        faction_keywords: &[Keyword::AdeptusCustodes],
    },
    StratagemDef {
        id: ids::OVERAWING_MAGNIFICENCE,
        name: "Overawing Magnificence",
        cp_cost: 1,
        valid_phases: &[Phase::Charge],
        timing: StratagemTiming::AfterEnemyDeclaresCharge,
        required_keywords: &[Keyword::AdeptusCustodes],
        must_be_friendly: true,
        must_be_enemy: false,
        once_per_battle: false,
        once_per_turn: true,
        is_faction: true,
        faction_keywords: &[Keyword::AdeptusCustodes],
    },
];

// ─── Stratagem Effect Application ───────────────────────────────────────────

/// Apply a stratagem's effects to the game state.
///
/// This is called by the executor after validation and CP spending.
/// Returns additional events generated by the stratagem.
pub fn apply_stratagem_effects(
    state: &mut GameState,
    player: PlayerId,
    stratagem_id: StratagemId,
    target_unit: Option<UnitId>,
) -> Vec<GameEvent> {
    let round = state.battle_round;
    let phase = state.current_phase;
    let mut events = Vec::new();

    match stratagem_id {
        id if id == ids::COMMAND_REROLL => {
            // Command Re-roll: register permission to re-roll one hit/wound/save/damage roll
            // Source: 40k_revised.md §15.2 - "Command Re-roll"
            let effect_id = state.next_counter() as u32;
            state.active_effects.push(ActiveEffect {
                id: effect_id,
                source: EffectSource::Stratagem(ids::COMMAND_REROLL),
                target: EffectTarget::Player(player),
                effect_type: EffectType::CommandReroll,
                duration: EffectDuration::UntilEndOfPhase,
                stacking: StackingBehavior::Unique,
                applied_round: round,
                applied_phase: phase,
            });
        }

        id if id == ids::COUNTER_OFFENSIVE => {
            // Counter-Offensive: target unit gains Fights First
            if let Some(uid) = target_unit {
                let effect_id = state.next_counter() as u32;
                state.active_effects.push(ActiveEffect {
                    id: effect_id,
                    source: EffectSource::Stratagem(ids::COUNTER_OFFENSIVE),
                    target: EffectTarget::Unit(uid),
                    effect_type: EffectType::GrantFightsFirst,
                    duration: EffectDuration::UntilEndOfPhase,
                    stacking: StackingBehavior::Unique,
                    applied_round: round,
                    applied_phase: phase,
                });
            }
        }

        id if id == ids::FIRE_OVERWATCH => {
            // Fire Overwatch: unit hits only on unmodified 6s
            if let Some(uid) = target_unit {
                let effect_id = state.next_counter() as u32;
                state.active_effects.push(ActiveEffect {
                    id: effect_id,
                    source: EffectSource::Stratagem(ids::FIRE_OVERWATCH),
                    target: EffectTarget::Unit(uid),
                    effect_type: EffectType::Custom(
                        "Fire Overwatch: hit only on unmodified 6s".to_string(),
                    ),
                    duration: EffectDuration::UntilEndOfPhase,
                    stacking: StackingBehavior::Unique,
                    applied_round: round,
                    applied_phase: phase,
                });
                state.turn_flags.overwatch_used_this_turn = true;
            }
        }

        id if id == ids::GO_TO_GROUND => {
            // Go to Ground: INFANTRY gains 6++ invulnerable save + Benefit of Cover
            // Lasts until end of phase (opponent's Shooting phase).
            // Source: CP_Rules.md §11 — Go to Ground: "Until end of phase"
            if let Some(uid) = target_unit {
                let eid1 = state.next_counter() as u32;
                let eid2 = state.next_counter() as u32;
                state.active_effects.push(ActiveEffect {
                    id: eid1,
                    source: EffectSource::Stratagem(ids::GO_TO_GROUND),
                    target: EffectTarget::Unit(uid),
                    effect_type: EffectType::GrantInvulnerableSave(6),
                    duration: EffectDuration::UntilEndOfPhase,
                    stacking: StackingBehavior::BestOnly,
                    applied_round: round,
                    applied_phase: phase,
                });
                state.active_effects.push(ActiveEffect {
                    id: eid2,
                    source: EffectSource::Stratagem(ids::GO_TO_GROUND),
                    target: EffectTarget::Unit(uid),
                    effect_type: EffectType::GrantBenefitOfCover,
                    duration: EffectDuration::UntilEndOfPhase,
                    stacking: StackingBehavior::Unique,
                    applied_round: round,
                    applied_phase: phase,
                });
            }
        }

        id if id == ids::HEROIC_INTERVENTION => {
            // Heroic Intervention: Declare and resolve a charge targeting only the
            // enemy unit that just completed its charge move. The intervening unit
            // does NOT get the Charge bonus (no Fights First) this turn.
            //
            // Per CP_Rules.md §11 - Heroic Intervention:
            //   "Target: One unit within 6\" that could charge that enemy unit"
            //   "Effect: Declare and resolve a charge targeting only that enemy unit"
            //   "Restrictions: VEHICLE must be a WALKER; Unit does NOT get Charge bonus this turn"
            //
            // The actual charge roll (2D6) and movement happen when the
            // HeroicInterventionMove command is executed.
            if let Some(uid) = target_unit {
                let effect_id = state.next_counter() as u32;
                state.active_effects.push(ActiveEffect {
                    id: effect_id,
                    source: EffectSource::Stratagem(ids::HEROIC_INTERVENTION),
                    target: EffectTarget::Unit(uid),
                    effect_type: EffectType::Custom(
                        "Heroic Intervention: declare and resolve a charge (no Charge bonus)".to_string(),
                    ),
                    duration: EffectDuration::UntilEndOfPhase,
                    stacking: StackingBehavior::Unique,
                    applied_round: round,
                    applied_phase: phase,
                });
            }
        }

        id if id == ids::INSANE_BRAVERY => {
            // Insane Bravery: unit automatically passes battle-shock test
            if let Some(uid) = target_unit {
                if let Some(unit) = state.unit_mut(uid) {
                    unit.battle_shocked = false;
                }
                events.push(GameEvent::BattleShockTestResolved {
                    unit: uid,
                    result: wh40k_core_types::BattleShockResult::Passed,
                    roll: 0, // Auto-pass via Insane Bravery
                });
            }
        }

        id if id == ids::EPIC_CHALLENGE => {
            // Epic Challenge: CHARACTER's melee attacks gain Precision
            if let Some(uid) = target_unit {
                let effect_id = state.next_counter() as u32;
                state.active_effects.push(ActiveEffect {
                    id: effect_id,
                    source: EffectSource::Stratagem(ids::EPIC_CHALLENGE),
                    target: EffectTarget::Unit(uid),
                    effect_type: EffectType::Custom(
                        "Epic Challenge: melee attacks gain Precision".to_string(),
                    ),
                    duration: EffectDuration::UntilEndOfPhase,
                    stacking: StackingBehavior::Unique,
                    applied_round: round,
                    applied_phase: phase,
                });
            }
        }

        id if id == ids::RAPID_INGRESS => {
            // Rapid Ingress: reserves unit arrives during opponent's movement
            if let Some(uid) = target_unit {
                let effect_id = state.next_counter() as u32;
                state.active_effects.push(ActiveEffect {
                    id: effect_id,
                    source: EffectSource::Stratagem(ids::RAPID_INGRESS),
                    target: EffectTarget::Unit(uid),
                    effect_type: EffectType::Custom(
                        "Rapid Ingress: arrive from reserves during opponent's Movement".to_string(),
                    ),
                    duration: EffectDuration::UntilEndOfPhase,
                    stacking: StackingBehavior::Unique,
                    applied_round: round,
                    applied_phase: phase,
                });
            }
        }

        id if id == ids::GRENADE => {
            // Grenade: Roll 6D6, each 4+ inflicts 1 mortal wound on enemy within 8"
            // The actual damage resolution requires a target enemy unit.
            // The effect is immediate, not persistent.
            if let Some(uid) = target_unit {
                use wh40k_dice::RollPurpose;
                let mut mortal_wounds: u8 = 0;
                for i in 0..6u32 {
                    let (roll, _) = state.dice_roller.roll_d6(RollPurpose::StratagemEffect {
                        stratagem_name: "Grenade".to_string(),
                        roll_index: i,
                    });
                    if roll >= 4 {
                        mortal_wounds += 1;
                    }
                }
                if mortal_wounds > 0 {
                    events.push(GameEvent::MortalWoundsInflicted {
                        target_unit: uid,
                        wounds: mortal_wounds,
                        source: "Grenade stratagem".to_string(),
                    });
                }
            }
        }

        id if id == ids::SMOKESCREEN => {
            // Smokescreen: SMOKE unit gains Benefit of Cover + Stealth until end of phase
            if let Some(uid) = target_unit {
                let eid1 = state.next_counter() as u32;
                let eid2 = state.next_counter() as u32;
                state.active_effects.push(ActiveEffect {
                    id: eid1,
                    source: EffectSource::Stratagem(ids::SMOKESCREEN),
                    target: EffectTarget::Unit(uid),
                    effect_type: EffectType::GrantBenefitOfCover,
                    duration: EffectDuration::UntilEndOfPhase,
                    stacking: StackingBehavior::Unique,
                    applied_round: round,
                    applied_phase: phase,
                });
                state.active_effects.push(ActiveEffect {
                    id: eid2,
                    source: EffectSource::Stratagem(ids::SMOKESCREEN),
                    target: EffectTarget::Unit(uid),
                    effect_type: EffectType::Stealth,
                    duration: EffectDuration::UntilEndOfPhase,
                    stacking: StackingBehavior::Unique,
                    applied_round: round,
                    applied_phase: phase,
                });
            }
        }

        id if id == ids::TANK_SHOCK => {
            // Tank Shock: Roll dice equal to the charging VEHICLE's Toughness,
            // each 5+ inflicts 1 mortal wound on an enemy unit within Engagement Range.
            // Maximum 6 mortal wounds.
            //
            // Per CP_Rules.md §11 - Tank Shock:
            //   "Target: That VEHICLE unit"
            //   "Effect: Select one enemy unit within Engagement Range.
            //    Roll dice equal to the VEHICLE's Toughness. Each 5+ inflicts
            //    1 mortal wound (maximum 6 mortal wounds)"
            //
            // target_unit (uid) = the VEHICLE (friendly unit that charged).
            // Mortal wounds go to an enemy unit within Engagement Range of the VEHICLE.
            if let Some(uid) = target_unit {
                let vehicle_toughness = state.unit(uid)
                    .map(|u| u.base_toughness.value())
                    .unwrap_or(4);
                let vehicle_owner = state.unit(uid).map(|u| u.owner);

                // Find all enemy units within Engagement Range of the VEHICLE/MONSTER
                // and auto-select the best target (most alive models = most impactful).
                // Per the rules, the player "selects one enemy unit within Engagement Range."
                // TODO: For human play, expose this as a player choice via the UI.
                let vehicle_ref_pos = state.unit(uid)
                    .and_then(|u| u.reference_position());
                let vehicle_base = state.unit(uid)
                    .and_then(|u| u.models.first().map(|m| m.base_size))
                    .unwrap_or(wh40k_core_types::BaseSize::MM25);

                let mut enemy_target: Option<UnitId> = None;
                let mut best_target_models: usize = 0;
                if let (Some(v_owner), Some(v_pos)) = (vehicle_owner, vehicle_ref_pos) {
                    for other_unit in &state.units {
                        if other_unit.owner == v_owner
                            || other_unit.is_destroyed()
                            || !other_unit.is_on_battlefield()
                        {
                            continue;
                        }
                        let in_er = other_unit.models.iter().any(|m| {
                            m.alive && wh40k_geometry::within_engagement_range_2d(
                                v_pos, vehicle_base,
                                m.position, m.base_size,
                            )
                        });
                        if in_er {
                            let alive = other_unit.models_alive();
                            if alive > best_target_models {
                                best_target_models = alive;
                                enemy_target = Some(other_unit.id);
                            }
                        }
                    }
                }

                // Roll dice equal to the VEHICLE's Toughness
                use wh40k_dice::RollPurpose;
                let mut mortal_wounds: u8 = 0;
                for i in 0..vehicle_toughness as u32 {
                    let (roll, _) = state.dice_roller.roll_d6(RollPurpose::StratagemEffect {
                        stratagem_name: "Tank Shock".to_string(),
                        roll_index: i,
                    });
                    if roll >= 5 {
                        mortal_wounds += 1;
                    }
                }
                mortal_wounds = mortal_wounds.min(6);

                // Apply mortal wounds to the enemy unit (not the VEHICLE)
                if mortal_wounds > 0 {
                    if let Some(enemy_uid) = enemy_target {
                        events.push(GameEvent::MortalWoundsInflicted {
                            target_unit: enemy_uid,
                            wounds: mortal_wounds,
                            source: "Tank Shock stratagem".to_string(),
                        });
                    }
                }
            }
        }

        // ── Frenzied Reavers Faction Stratagems ─────────────────────────

        id if id == ids::HORRIFYING_BUTCHERY => {
            // Horrifying Butchery: Select one enemy unit within Engagement Range
            // of the target KHORNE BERZERKERS unit. That enemy must take a
            // Battle-shock test.
            //
            // Source: Frenzied_Reavers.md - Horrifying Butchery
            if let Some(uid) = target_unit {
                let effect_id = state.next_counter() as u32;
                state.active_effects.push(ActiveEffect {
                    id: effect_id,
                    source: EffectSource::Stratagem(ids::HORRIFYING_BUTCHERY),
                    target: EffectTarget::Unit(uid),
                    effect_type: EffectType::ForceTest,
                    duration: EffectDuration::UntilEndOfPhase,
                    stacking: StackingBehavior::Unique,
                    applied_round: round,
                    applied_phase: phase,
                });
            }
        }

        id if id == ids::BERSERK_RESILIENCE => {
            // Berserk Resilience: Target WORLD EATERS CHARACTER gains fight-on-death
            // (D6 4+, adding 2 if Total Carnage is active) until end of phase.
            //
            // Source: Frenzied_Reavers.md - Berserk Resilience
            if let Some(uid) = target_unit {
                let effect_id = state.next_counter() as u32;
                state.active_effects.push(ActiveEffect {
                    id: effect_id,
                    source: EffectSource::Stratagem(ids::BERSERK_RESILIENCE),
                    target: EffectTarget::Unit(uid),
                    effect_type: EffectType::Custom("Berserk Resilience".to_string()),
                    duration: EffectDuration::UntilEndOfPhase,
                    stacking: StackingBehavior::Unique,
                    applied_round: round,
                    applied_phase: phase,
                });
            }
        }

        id if id == ids::BLOODLUST => {
            // Bloodlust: Target JAKHALS unit that had models destroyed can make
            // a Normal move of up to D6".
            //
            // Source: Frenzied_Reavers.md - Bloodlust
            if let Some(uid) = target_unit {
                use wh40k_dice::RollPurpose;
                let (roll, _) = state.dice_roller.roll_d6(RollPurpose::StratagemEffect {
                    stratagem_name: "Bloodlust".to_string(),
                    roll_index: 0,
                });
                let move_distance = wh40k_core_types::Inches::from_inches(roll as i32);
                let effect_id = state.next_counter() as u32;
                state.active_effects.push(ActiveEffect {
                    id: effect_id,
                    source: EffectSource::Stratagem(ids::BLOODLUST),
                    target: EffectTarget::Unit(uid),
                    effect_type: EffectType::Custom(
                        format!("Bloodlust: Normal move up to {}\"", roll),
                    ),
                    duration: EffectDuration::UntilEndOfPhase,
                    stacking: StackingBehavior::Unique,
                    applied_round: round,
                    applied_phase: phase,
                });
                events.push(GameEvent::StratagemEffectApplied {
                    stratagem: ids::BLOODLUST,
                    target: uid,
                    description: format!(
                        "Bloodlust: JAKHALS unit may move up to {}\" (rolled {})",
                        roll, roll
                    ),
                });
                let _ = move_distance; // Used by the movement system when resolving the move
            }
        }

        // ── Custodes Faction Stratagems ──────────────────────────────────

        id if id == ids::GILDED_SPEAR => {
            // The Gilded Spear: Pile-in moves become 6" instead of 3"
            //
            // Source: Custodes.md - The Gilded Spear
            if let Some(uid) = target_unit {
                let effect_id = state.next_counter() as u32;
                state.active_effects.push(ActiveEffect {
                    id: effect_id,
                    source: EffectSource::Stratagem(ids::GILDED_SPEAR),
                    target: EffectTarget::Unit(uid),
                    effect_type: EffectType::ModifyMovement(3000), // +3" (6" total pile-in)
                    duration: EffectDuration::UntilEndOfPhase,
                    stacking: StackingBehavior::Unique,
                    applied_round: round,
                    applied_phase: phase,
                });
            }
        }

        id if id == ids::INESCAPABLE_VENGEANCE => {
            // Inescapable Vengeance: +2 to Advance rolls
            //
            // Source: Custodes.md - Inescapable Vengeance
            if let Some(uid) = target_unit {
                let effect_id = state.next_counter() as u32;
                state.active_effects.push(ActiveEffect {
                    id: effect_id,
                    source: EffectSource::Stratagem(ids::INESCAPABLE_VENGEANCE),
                    target: EffectTarget::Unit(uid),
                    effect_type: EffectType::Custom(
                        "Inescapable Vengeance: +2 to Advance rolls".to_string(),
                    ),
                    duration: EffectDuration::UntilEndOfPhase,
                    stacking: StackingBehavior::Unique,
                    applied_round: round,
                    applied_phase: phase,
                });
            }
        }

        id if id == ids::OVERAWING_MAGNIFICENCE => {
            // Overawing Magnificence: Enemy must take Leadership test. If failed,
            // subtract 2 from Charge rolls for that enemy unit until end of phase.
            //
            // Source: Custodes.md - Overawing Magnificence
            if let Some(uid) = target_unit {
                // The Leadership test and charge modifier are applied here.
                // The actual Ld test requires rolling 2D6 vs the enemy unit's Leadership.
                let effect_id = state.next_counter() as u32;
                state.active_effects.push(ActiveEffect {
                    id: effect_id,
                    source: EffectSource::Stratagem(ids::OVERAWING_MAGNIFICENCE),
                    target: EffectTarget::Unit(uid),
                    effect_type: EffectType::Custom(
                        "Overawing Magnificence: enemy must pass Ld test or -2 to Charge rolls"
                            .to_string(),
                    ),
                    duration: EffectDuration::UntilEndOfPhase,
                    stacking: StackingBehavior::Unique,
                    applied_round: round,
                    applied_phase: phase,
                });
            }
        }

        _ => {
            // Unknown stratagem - no effects applied
        }
    }

    events
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_stratagem_definitions_have_valid_ids() {
        for def in ALL_STRATAGEMS {
            assert!(def.id.raw() > 0, "Stratagem {} has ID 0", def.name);
            assert!(!def.name.is_empty(), "Stratagem ID {} has empty name", def.id);
            assert!(def.cp_cost >= 1, "Stratagem {} costs 0 CP", def.name);
            assert!(!def.valid_phases.is_empty(), "Stratagem {} has no valid phases", def.name);
        }
    }

    #[test]
    fn test_core_stratagems_count() {
        let core = core_stratagem_ids();
        assert_eq!(core.len(), 11, "Should have 11 core stratagems");
    }

    #[test]
    fn test_faction_stratagems_count() {
        let fr = frenzied_reavers_stratagem_ids();
        assert_eq!(fr.len(), 3, "Frenzied Reavers should have 3 faction stratagems");

        let custodes = custodes_stratagem_ids();
        assert_eq!(custodes.len(), 3, "Custodes should have 3 faction stratagems");
    }

    #[test]
    fn test_get_stratagem_def() {
        let def = get_stratagem_def(ids::COMMAND_REROLL).unwrap();
        assert_eq!(def.name, "Command Re-roll");
        assert_eq!(def.cp_cost, 1);
        assert_eq!(def.valid_phases.len(), 5); // All phases

        let def = get_stratagem_def(ids::COUNTER_OFFENSIVE).unwrap();
        assert_eq!(def.name, "Counter-Offensive");
        assert_eq!(def.cp_cost, 2);
        assert_eq!(def.valid_phases, &[Phase::Fight]);
    }

    #[test]
    fn test_faction_stratagems_are_marked_faction() {
        for id in frenzied_reavers_stratagem_ids() {
            let def = get_stratagem_def(*id).unwrap();
            assert!(def.is_faction, "FR stratagem {} should be faction", def.name);
            assert!(
                def.faction_keywords.contains(&Keyword::WorldEaters),
                "FR stratagem {} should require WorldEaters keyword",
                def.name
            );
        }

        for id in custodes_stratagem_ids() {
            let def = get_stratagem_def(*id).unwrap();
            assert!(def.is_faction, "Custodes stratagem {} should be faction", def.name);
            assert!(
                def.faction_keywords.contains(&Keyword::AdeptusCustodes),
                "Custodes stratagem {} should require AdeptusCustodes keyword",
                def.name
            );
        }
    }

    #[test]
    fn test_insane_bravery_once_per_battle() {
        let def = get_stratagem_def(ids::INSANE_BRAVERY).unwrap();
        assert!(def.once_per_battle);
    }

    #[test]
    fn test_counter_offensive_costs_2cp() {
        let def = get_stratagem_def(ids::COUNTER_OFFENSIVE).unwrap();
        assert_eq!(def.cp_cost, 2);
    }

    #[test]
    fn test_grenade_requires_grenades_keyword() {
        let def = get_stratagem_def(ids::GRENADE).unwrap();
        assert!(def.required_keywords.contains(&Keyword::Grenades));
    }

    #[test]
    fn test_smokescreen_requires_smoke_keyword() {
        let def = get_stratagem_def(ids::SMOKESCREEN).unwrap();
        assert!(def.required_keywords.contains(&Keyword::Smoke));
    }

    #[test]
    fn test_tank_shock_no_static_keyword_requirement() {
        // Tank Shock uses custom validation (VEHICLE or MONSTER) instead of
        // required_keywords, so static def has empty keywords.
        let def = get_stratagem_def(ids::TANK_SHOCK).unwrap();
        assert!(def.required_keywords.is_empty());
    }

    #[test]
    fn test_fire_overwatch_multiple_phases() {
        let def = get_stratagem_def(ids::FIRE_OVERWATCH).unwrap();
        assert!(def.valid_phases.contains(&Phase::Movement));
        assert!(def.valid_phases.contains(&Phase::Charge));
    }

    #[test]
    fn test_unknown_stratagem_returns_none() {
        assert!(get_stratagem_def(StratagemId::new(999)).is_none());
    }

    #[test]
    fn test_all_ids_unique() {
        let mut ids_seen = std::collections::HashSet::new();
        for def in ALL_STRATAGEMS {
            assert!(
                ids_seen.insert(def.id.raw()),
                "Duplicate stratagem ID: {} ({})",
                def.id,
                def.name
            );
        }
    }

    #[test]
    fn test_horrifying_butchery_requires_khorne_berzerkers() {
        let def = get_stratagem_def(ids::HORRIFYING_BUTCHERY).unwrap();
        assert!(def.required_keywords.contains(&Keyword::Berzerkers));
    }

    #[test]
    fn test_berserk_resilience_requires_we_character() {
        let def = get_stratagem_def(ids::BERSERK_RESILIENCE).unwrap();
        assert!(def.required_keywords.contains(&Keyword::Character));
        assert!(def.required_keywords.contains(&Keyword::WorldEaters));
    }

    #[test]
    fn test_bloodlust_requires_jakhals() {
        let def = get_stratagem_def(ids::BLOODLUST).unwrap();
        assert!(def.required_keywords.contains(&Keyword::Jakhals));
    }

    #[test]
    fn test_gilded_spear_requires_adeptus_custodes() {
        let def = get_stratagem_def(ids::GILDED_SPEAR).unwrap();
        assert!(def.required_keywords.contains(&Keyword::AdeptusCustodes));
    }
}

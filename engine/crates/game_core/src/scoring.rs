//! Scoring module for secondary objectives and mission-specific VP scoring.
//!
//! Handles:
//! - Enhancement application (persistent effects on warlords)
//! - Secondary objective evaluation and VP awarding
//! - Primary mission scoring per round
//! - Mission-specific scoring rules for all 6 CP missions
//! - End-of-game scoring (Battle Ready bonus)
//!
//! Source: 40k_revised.md - Scoring, Objectives
//! Source: Custodes.md - Enhancements, Secondary Objectives
//! Source: Frenzied_Reavers.md - Enhancements, Secondary Objectives
//! Source: CP_Rules.md - Mission Scoring Rules

use wh40k_core_types::{
    EffectDuration, Keyword, ObjectiveId, Phase, PlayerId,
    StackingBehavior, UnitId, VictoryPoints,
};
use wh40k_event_system::{
    DestroyCause, EffectId, EffectTarget as EventEffectTarget, GameEvent, VpSource,
};

use crate::effect::{ActiveEffect, EffectSource, EffectTarget, EffectType};
use crate::state::GameState;

// ─── Enhancement Definitions ────────────────────────────────────────────────

/// Enhancement IDs for all Combat Patrol enhancements.
pub mod enhancement_ids {
    use wh40k_core_types::EnhancementId;

    // Frenzied Reavers enhancements
    pub const FEARSOME_PRESENCE: EnhancementId = EnhancementId::new(1);
    pub const BANE_OF_THE_CRAVEN: EnhancementId = EnhancementId::new(2);

    // Custodes enhancements
    pub const WATCHMAN_OF_TERRA: EnhancementId = EnhancementId::new(3);
    pub const WARRIOR_EXEMPLAR: EnhancementId = EnhancementId::new(4);
}

/// Apply a selected enhancement to the warlord unit as persistent effects.
///
/// Source: Custodes.md - Enhancements
/// Source: Frenzied_Reavers.md - Enhancements
pub fn apply_enhancement(
    state: &mut GameState,
    player: PlayerId,
    enhancement_id: wh40k_core_types::EnhancementId,
    warlord_unit_id: UnitId,
) -> Vec<GameEvent> {
    let round = state.battle_round;
    let phase = state.current_phase;
    let mut events = Vec::new();
    let _ = player; // Used for future faction validation

    match enhancement_id {
        id if id == enhancement_ids::FEARSOME_PRESENCE => {
            // Fearsome Presence: OC 5 while not Battle-shocked
            // Source: Frenzied_Reavers.md - Fearsome Presence
            let effect_id = state.next_counter() as u32;
            state.active_effects.push(ActiveEffect {
                id: effect_id,
                source: EffectSource::Enhancement(enhancement_id),
                target: EffectTarget::Unit(warlord_unit_id),
                effect_type: EffectType::ModifyOC(5),
                duration: EffectDuration::Persistent,
                stacking: StackingBehavior::Unique,
                applied_round: round,
                applied_phase: phase,
            });

            // Set the OC override on the unit directly
            if let Some(unit) = state.unit_mut(warlord_unit_id) {
                unit.enhancement_oc_override = Some(wh40k_core_types::ObjectiveControl::new(5));
                unit.enhancement_oc_requires_engaged = false;
            }

            events.push(GameEvent::EffectApplied {
                effect_id: EffectId::new(effect_id),
                target: EventEffectTarget::Unit(warlord_unit_id),
                duration: EffectDuration::Persistent,
            });
        }

        id if id == enhancement_ids::BANE_OF_THE_CRAVEN => {
            // Bane of the Craven: enemies falling back from bearer must take
            // Desperate Escape tests. -1 if Battle-shocked.
            // Source: Frenzied_Reavers.md - Bane of the Craven
            let effect_id = state.next_counter() as u32;
            state.active_effects.push(ActiveEffect {
                id: effect_id,
                source: EffectSource::Enhancement(enhancement_id),
                target: EffectTarget::Unit(warlord_unit_id),
                effect_type: EffectType::Custom(
                    "Bane of the Craven: enemies must Desperate Escape on Fall Back".to_string(),
                ),
                duration: EffectDuration::Persistent,
                stacking: StackingBehavior::Unique,
                applied_round: round,
                applied_phase: phase,
            });
            events.push(GameEvent::EffectApplied {
                effect_id: EffectId::new(effect_id),
                target: EventEffectTarget::Unit(warlord_unit_id),
                duration: EffectDuration::Persistent,
            });
        }

        id if id == enhancement_ids::WATCHMAN_OF_TERRA => {
            // Watchman of Terra: OC 4 while in Engagement Range of enemies
            // Source: Custodes.md - Watchman of Terra
            let effect_id = state.next_counter() as u32;
            state.active_effects.push(ActiveEffect {
                id: effect_id,
                source: EffectSource::Enhancement(enhancement_id),
                target: EffectTarget::Unit(warlord_unit_id),
                effect_type: EffectType::ModifyOC(4),
                duration: EffectDuration::Persistent,
                stacking: StackingBehavior::Unique,
                applied_round: round,
                applied_phase: phase,
            });

            // Set the OC override on the unit (requires engagement range)
            if let Some(unit) = state.unit_mut(warlord_unit_id) {
                unit.enhancement_oc_override = Some(wh40k_core_types::ObjectiveControl::new(4));
                unit.enhancement_oc_requires_engaged = true;
            }

            events.push(GameEvent::EffectApplied {
                effect_id: EffectId::new(effect_id),
                target: EventEffectTarget::Unit(warlord_unit_id),
                duration: EffectDuration::Persistent,
            });
        }

        id if id == enhancement_ids::WARRIOR_EXEMPLAR => {
            // Warrior Exemplar: D6 3+ = 1CP when destroying enemy unit
            // Source: Custodes.md - Warrior Exemplar
            let effect_id = state.next_counter() as u32;
            state.active_effects.push(ActiveEffect {
                id: effect_id,
                source: EffectSource::Enhancement(enhancement_id),
                target: EffectTarget::Unit(warlord_unit_id),
                effect_type: EffectType::Custom("Warrior Exemplar".to_string()),
                duration: EffectDuration::Persistent,
                stacking: StackingBehavior::Unique,
                applied_round: round,
                applied_phase: phase,
            });
            events.push(GameEvent::EffectApplied {
                effect_id: EffectId::new(effect_id),
                target: EventEffectTarget::Unit(warlord_unit_id),
                duration: EffectDuration::Persistent,
            });
        }

        _ => {
            // Unknown enhancement
        }
    }

    events
}

// ─── Secondary Objective Definitions ────────────────────────────────────────

/// Secondary objective IDs.
pub mod secondary_ids {
    use wh40k_core_types::SecondaryObjectiveId;

    // Frenzied Reavers secondaries
    pub const CHAMPIONS_OF_KHORNE: SecondaryObjectiveId = SecondaryObjectiveId::new(1);
    pub const SKULL_TAKERS: SecondaryObjectiveId = SecondaryObjectiveId::new(2);

    // Custodes secondaries
    pub const RAISE_THE_VEXILLAS: SecondaryObjectiveId = SecondaryObjectiveId::new(3);
    pub const CONSECRATED_GROUND: SecondaryObjectiveId = SecondaryObjectiveId::new(4);
}

/// Score secondary objectives for a player at the appropriate timing.
///
/// Called at the end of the relevant phase. Returns VP scored and events.
///
/// Source: Custodes.md - Secondary Objectives
/// Source: Frenzied_Reavers.md - Secondary Objectives
pub fn score_secondary_objectives(
    state: &GameState,
    player: PlayerId,
) -> Vec<(VictoryPoints, GameEvent)> {
    let player_state = state.player(player);
    let secondary_id = match player_state.secondary_choice {
        Some(id) => id,
        None => return Vec::new(),
    };

    let mut results = Vec::new();
    let max_secondary_vp: i16 = 12;

    match secondary_id {
        id if id == secondary_ids::CHAMPIONS_OF_KHORNE => {
            // Champions of Khorne: From BR2+, at end of Command Phase:
            // If 1 NML objective controlled with CHARACTER within range: 2VP
            // If 2 NML objectives controlled with CHARACTER within range: 3VP instead
            // Max 12VP total
            //
            // Source: Frenzied_Reavers.md - Champions of Khorne
            if state.current_phase == Phase::Command
                && state.battle_round.number() >= 2
            {
                let current_secondary = state.player(player).mission_progress.secondary_vp.value();
                if current_secondary >= max_secondary_vp {
                    return results;
                }

                let mut nml_objectives_with_character = 0u8;

                // Check objectives in No Man's Land controlled by this player
                // with a non-Battle-shocked CHARACTER within range
                for objective in &state.board.objectives {
                    if !is_objective_in_no_mans_land(objective) {
                        continue;
                    }

                    // Check if player controls this objective
                    let controlling_player = calculate_objective_controller(state, objective.id);
                    if controlling_player != Some(player) {
                        continue;
                    }

                    // Check if a non-Battle-shocked CHARACTER is within range
                    let has_character = state.units.iter().any(|u| {
                        u.owner == player
                            && !u.is_destroyed()
                            && u.is_on_battlefield()
                            && u.has_keyword(Keyword::Character)
                            && !u.battle_shocked
                            && is_unit_within_objective_range(state, u, objective.id)
                    });

                    if has_character {
                        nml_objectives_with_character += 1;
                    }
                }

                let scored: i16 = match nml_objectives_with_character {
                    0 => 0,
                    1 => 2,
                    _ => 3,
                };

                if scored > 0 {
                    let remaining = max_secondary_vp - current_secondary;
                    let vp_val = scored.min(remaining);
                    let vp = VictoryPoints::new(vp_val);
                    results.push((
                        vp,
                        GameEvent::VictoryPointsScored {
                            player,
                            amount: vp_val as u16,
                            source: VpSource::SecondaryObjective(format!(
                                "Champions of Khorne: {} NML objective(s) with CHARACTER",
                                nml_objectives_with_character
                            )),
                        },
                    ));
                }
            }
        }

        id if id == secondary_ids::SKULL_TAKERS => {
            // Skull Takers: At end of Fight Phase, for each CHARACTER from your army
            // that destroyed one or more enemy units this phase: 3VP.
            // Max 12VP total.
            //
            // Source: Frenzied_Reavers.md - Skull Takers
            if state.current_phase == Phase::Fight {
                let current_secondary = state.player(player).mission_progress.secondary_vp.value();
                if current_secondary >= max_secondary_vp {
                    return results;
                }

                // Query event log for UnitDestroyed events in this round's Fight phase
                // to find which CHARACTER units from this player destroyed enemy units.
                let fight_events = state.event_bus.events_in_round_phase(
                    state.battle_round,
                    Phase::Fight,
                );

                let mut character_destroyers = std::collections::HashSet::new();
                for entry in &fight_events {
                    if let GameEvent::UnitDestroyed { cause: DestroyCause::Attack { attacker, .. }, .. } = &entry.event {
                        // Check if the attacker is a CHARACTER owned by this player
                        if let Some(attacker_unit) = state.unit(*attacker) {
                            if attacker_unit.owner == player
                                && attacker_unit.has_keyword(Keyword::Character)
                            {
                                character_destroyers.insert(*attacker);
                            }
                        }
                    }
                }

                let characters_with_kills = character_destroyers.len() as i16;
                let scored = characters_with_kills * 3;

                if scored > 0 {
                    let remaining = max_secondary_vp - current_secondary;
                    let vp_val = scored.min(remaining);
                    let vp = VictoryPoints::new(vp_val);
                    results.push((
                        vp,
                        GameEvent::VictoryPointsScored {
                            player,
                            amount: vp_val as u16,
                            source: VpSource::SecondaryObjective(format!(
                                "Skull Takers: {} CHARACTER(s) destroyed enemy units",
                                characters_with_kills
                            )),
                        },
                    ));
                }
            }
        }

        id if id == secondary_ids::RAISE_THE_VEXILLAS => {
            // Raise the Vexillas: From BR3+, at end of your turn:
            // Score 3VP if you control BOTH the objective closest to your
            // battlefield edge AND the objective closest to opponent's edge.
            // Max 9VP.
            //
            // Source: Custodes.md - Raise the Vexillas
            let max_vexilla_vp: i16 = 9;
            if state.battle_round.number() >= 3 {
                let current_secondary = state.player(player).mission_progress.secondary_vp.value();
                if current_secondary >= max_vexilla_vp {
                    return results;
                }

                let (closest_own, closest_enemy) = find_edge_objectives(state, player);

                let controls_own = closest_own
                    .and_then(|obj_id| calculate_objective_controller(state, obj_id))
                    == Some(player);
                let controls_enemy = closest_enemy
                    .and_then(|obj_id| calculate_objective_controller(state, obj_id))
                    == Some(player);

                if controls_own && controls_enemy {
                    let remaining = max_vexilla_vp - current_secondary;
                    let vp_val = 3i16.min(remaining);
                    let vp = VictoryPoints::new(vp_val);
                    results.push((
                        vp,
                        GameEvent::VictoryPointsScored {
                            player,
                            amount: vp_val as u16,
                            source: VpSource::SecondaryObjective(
                                "Raise the Vexillas: control both edge objectives".to_string(),
                            ),
                        },
                    ));
                }
            }
        }

        id if id == secondary_ids::CONSECRATED_GROUND => {
            // Consecrated Ground: Ongoing tracking
            // +3VP each time an enemy unit is destroyed
            // -1VP each time an ADEPTUS CUSTODES model is destroyed (min 0 total)
            //
            // This is handled incrementally via events, not at end of phase.
            // The scoring module tracks it via the mission progress.
            //
            // Source: Custodes.md - Consecrated Ground
            // Note: Actual VP calculation is done incrementally when units/models are destroyed
        }

        _ => {
            // Unknown secondary
        }
    }

    results
}

/// Score Consecrated Ground when an enemy unit is destroyed.
/// Returns VP gained (3) or None if not applicable.
///
/// Source: Custodes.md - Consecrated Ground
pub fn score_consecrated_ground_kill(
    state: &GameState,
    player: PlayerId,
) -> Option<(VictoryPoints, GameEvent)> {
    let player_state = state.player(player);
    if player_state.secondary_choice != Some(secondary_ids::CONSECRATED_GROUND) {
        return None;
    }

    Some((
        VictoryPoints::new(3),
        GameEvent::VictoryPointsScored {
            player,
            amount: 3,
            source: VpSource::SecondaryObjective(
                "Consecrated Ground: enemy unit destroyed (+3VP)".to_string(),
            ),
        },
    ))
}

/// Score Consecrated Ground penalty when a Custodes model is destroyed.
/// Returns VP penalty (1) or None if not applicable.
///
/// Source: Custodes.md - Consecrated Ground
pub fn score_consecrated_ground_loss(
    state: &GameState,
    player: PlayerId,
) -> Option<VictoryPoints> {
    let player_state = state.player(player);
    if player_state.secondary_choice != Some(secondary_ids::CONSECRATED_GROUND) {
        return None;
    }

    // Only deduct if current secondary VP > 0
    if state.player(player).mission_progress.secondary_vp.value() > 0 {
        Some(VictoryPoints::new(1))
    } else {
        None
    }
}

// ─── Mission Scoring ────────────────────────────────────────────────────────

/// Mission IDs for the 6 Combat Patrol missions.
pub mod mission_ids {
    use wh40k_core_types::MissionId;

    pub const CLASH_OF_PATROLS: MissionId = MissionId::new(1);
    pub const ARCHEOTECH_RECOVERY: MissionId = MissionId::new(2);
    pub const FORWARD_OUTPOST: MissionId = MissionId::new(3);
    pub const SCORCHED_EARTH: MissionId = MissionId::new(4);
    pub const SWEEPING_RAID: MissionId = MissionId::new(5);
    pub const DISPLAY_OF_MIGHT: MissionId = MissionId::new(6);
}

/// Score primary objectives for the current round based on mission rules.
///
/// Returns VP scored for each player and events.
///
/// Source: CP_Rules.md - Mission Scoring
pub fn score_primary_objectives(
    state: &GameState,
    mission_id: Option<wh40k_core_types::MissionId>,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    let round = state.battle_round.number();

    match mission_id {
        Some(id) if id == mission_ids::CLASH_OF_PATROLS => {
            score_clash_of_patrols(state, round)
        }
        Some(id) if id == mission_ids::ARCHEOTECH_RECOVERY => {
            score_archeotech_recovery(state, round)
        }
        Some(id) if id == mission_ids::FORWARD_OUTPOST => {
            score_forward_outpost(state, round)
        }
        Some(id) if id == mission_ids::SCORCHED_EARTH => {
            score_scorched_earth(state, round)
        }
        Some(id) if id == mission_ids::SWEEPING_RAID => {
            score_sweeping_raid(state, round)
        }
        Some(id) if id == mission_ids::DISPLAY_OF_MIGHT => {
            score_display_of_might(state, round)
        }
        _ => {
            // Default: 5VP per objective controlled, rounds 2-5
            score_default_primary(state, round)
        }
    }
}

/// Default primary scoring: 5VP per objective controlled, rounds 2-5.
fn score_default_primary(
    state: &GameState,
    round: u8,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    let mut results = Vec::new();

    if round < 2 {
        return results;
    }

    for player_idx in 0..2u32 {
        let player = PlayerId::new(player_idx);
        let mut objectives_controlled = 0u16;

        for objective in &state.board.objectives {
            if calculate_objective_controller(state, objective.id) == Some(player) {
                objectives_controlled += 1;
            }
        }

        if objectives_controlled > 0 {
            let vp_val = objectives_controlled * 5;
            let vp = VictoryPoints::new(vp_val as i16);
            results.push((
                player,
                vp,
                GameEvent::VictoryPointsScored {
                    player,
                    amount: vp_val,
                    source: VpSource::PrimaryObjective(ObjectiveId::new(0)),
                },
            ));
        }
    }

    results
}

/// Mission 1 - Clash of Patrols: 5VP per objective controlled, rounds 2-5.
///
/// Source: CP_Rules.md - Mission 1: Clash of Patrols
fn score_clash_of_patrols(
    state: &GameState,
    round: u8,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    // Same as default: 5VP per objective per round, BR2-5
    score_default_primary(state, round)
}

/// Mission 2 - Archeotech Recovery: Objectives removed over time, +10VP end game
/// for controlling the Archeotech (center) objective.
///
/// BR2-3: 5VP per objective controlled
/// BR3 end: Remove the 2 objectives closest to battlefield edges
/// BR4-5: 5VP per remaining objective
/// End of game: 10VP if controlling the center (Archeotech) objective
///
/// Source: CP_Rules.md - Mission 2: Archeotech Recovery
fn score_archeotech_recovery(
    state: &GameState,
    round: u8,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    let mut results = Vec::new();

    if round < 2 {
        return results;
    }

    // Score for each player based on objectives they control
    for player_idx in 0..2u32 {
        let player = PlayerId::new(player_idx);
        let mut objectives_controlled = 0u16;

        for objective in &state.board.objectives {
            // In BR4-5, edge objectives should have been removed from the board
            // by the phase progression system. We just score what's on the board.
            if calculate_objective_controller(state, objective.id) == Some(player) {
                objectives_controlled += 1;
            }
        }

        if objectives_controlled > 0 {
            let vp_val = objectives_controlled * 5;
            let vp = VictoryPoints::new(vp_val as i16);
            results.push((
                player,
                vp,
                GameEvent::VictoryPointsScored {
                    player,
                    amount: vp_val,
                    source: VpSource::MissionRule(format!(
                        "Archeotech Recovery: {} objective(s) controlled (round {})",
                        objectives_controlled, round
                    )),
                },
            ));
        }
    }

    results
}

/// Score Archeotech Recovery end-of-game bonus.
/// +10VP for controlling the center (Archeotech) objective.
///
/// Source: CP_Rules.md - Mission 2: Archeotech Recovery
pub fn score_archeotech_endgame(
    state: &GameState,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    let mut results = Vec::new();

    // Find center objective (closest to board center: 22", 15")
    let center = find_center_objective(state);
    if let Some(center_id) = center {
        for player_idx in 0..2u32 {
            let player = PlayerId::new(player_idx);
            if calculate_objective_controller(state, center_id) == Some(player) {
                let vp = VictoryPoints::new(10);
                results.push((
                    player,
                    vp,
                    GameEvent::VictoryPointsScored {
                        player,
                        amount: 10,
                        source: VpSource::MissionRule(
                            "Archeotech Recovery: control Archeotech objective (+10VP)".to_string(),
                        ),
                    },
                ));
            }
        }
    }

    results
}

/// Mission 3 - Forward Outpost: 10VP for controlling the objective in the
/// enemy's deployment zone. Enemy loses 1CP if you control their DZ objective.
///
/// BR2-5: 5VP per objective controlled
/// Bonus: 10VP if controlling objective in enemy DZ
/// Enemy penalty: -1CP if you control their DZ objective
///
/// Source: CP_Rules.md - Mission 3: Forward Outpost
fn score_forward_outpost(
    state: &GameState,
    round: u8,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    let mut results = Vec::new();

    if round < 2 {
        return results;
    }

    for player_idx in 0..2u32 {
        let player = PlayerId::new(player_idx);
        let mut objectives_controlled = 0u16;
        let mut controls_enemy_dz = false;

        for objective in &state.board.objectives {
            if calculate_objective_controller(state, objective.id) == Some(player) {
                objectives_controlled += 1;

                // Check if this objective is in the enemy's deployment zone
                if is_objective_in_enemy_dz(objective, player) {
                    controls_enemy_dz = true;
                }
            }
        }

        if objectives_controlled > 0 {
            let mut vp_val = objectives_controlled * 5;
            if controls_enemy_dz {
                vp_val += 10;
            }
            let vp = VictoryPoints::new(vp_val as i16);
            results.push((
                player,
                vp,
                GameEvent::VictoryPointsScored {
                    player,
                    amount: vp_val,
                    source: VpSource::MissionRule(format!(
                        "Forward Outpost: {} obj controlled{} (round {})",
                        objectives_controlled,
                        if controls_enemy_dz { " + enemy DZ bonus" } else { "" },
                        round
                    )),
                },
            ));
        }
    }

    results
}

/// Mission 4 - Scorched Earth: Players can "raze" objectives they control
/// in enemy territory for 10VP. Razed objectives are removed.
///
/// BR2-5: 5VP per objective controlled
/// Raze action: 10VP for razing an objective in enemy territory
///
/// Source: CP_Rules.md - Mission 4: Scorched Earth
fn score_scorched_earth(
    state: &GameState,
    round: u8,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    // Base scoring is same as default (5VP per objective)
    // Razing is handled as a separate action/command, not in per-round scoring
    score_default_primary(state, round)
}

/// Award VP for razing an objective in Scorched Earth mission.
///
/// Source: CP_Rules.md - Mission 4: Scorched Earth
pub fn score_raze_objective(
    player: PlayerId,
    objective_id: ObjectiveId,
) -> (VictoryPoints, GameEvent) {
    let vp = VictoryPoints::new(10);
    (
        vp,
        GameEvent::VictoryPointsScored {
            player,
            amount: 10,
            source: VpSource::MissionRule(format!(
                "Scorched Earth: razed objective {}",
                objective_id
            )),
        },
    )
}

/// Mission 5 - Sweeping Raid: Supply Lines mechanic (+1CP on 4+),
/// asymmetric end-game scoring based on objectives in own/enemy territory.
///
/// BR2-5: 5VP per objective controlled
/// Supply Lines: At start of Command Phase, if controlling an objective
/// in enemy territory, roll D6 - on 4+, gain 1CP.
/// End game: More VP for objectives deeper in enemy territory.
///
/// Source: CP_Rules.md - Mission 5: Sweeping Raid
fn score_sweeping_raid(
    state: &GameState,
    round: u8,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    // Base scoring same as default
    // Supply Lines CP gain is handled in Command Phase execution
    score_default_primary(state, round)
}

/// Score Sweeping Raid end-of-game bonus.
/// +5VP for each objective in enemy territory you control.
///
/// Source: CP_Rules.md - Mission 5: Sweeping Raid
pub fn score_sweeping_raid_endgame(
    state: &GameState,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    let mut results = Vec::new();

    for player_idx in 0..2u32 {
        let player = PlayerId::new(player_idx);
        let mut enemy_objectives = 0u16;

        for objective in &state.board.objectives {
            if calculate_objective_controller(state, objective.id) == Some(player)
                && is_objective_in_enemy_dz(objective, player)
            {
                enemy_objectives += 1;
            }
        }

        if enemy_objectives > 0 {
            let vp_val = enemy_objectives * 5;
            let vp = VictoryPoints::new(vp_val as i16);
            results.push((
                player,
                vp,
                GameEvent::VictoryPointsScored {
                    player,
                    amount: vp_val,
                    source: VpSource::MissionRule(format!(
                        "Sweeping Raid: {} enemy territory objective(s) bonus",
                        enemy_objectives
                    )),
                },
            ));
        }
    }

    results
}

/// Mission 6 - Display of Might: CHARACTER units can "claim" objectives.
/// Consecutive turns of claiming give bonus VP.
///
/// BR2-5: 5VP per objective controlled
/// CHARACTER claiming: If a CHARACTER controls an objective, it becomes
/// "claimed". If the same CHARACTER claims it for consecutive turns, +5VP.
///
/// Source: CP_Rules.md - Mission 6: Display of Might
fn score_display_of_might(
    state: &GameState,
    round: u8,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    let mut results = Vec::new();

    if round < 2 {
        return results;
    }

    for player_idx in 0..2u32 {
        let player = PlayerId::new(player_idx);
        let mut objectives_controlled = 0u16;
        let mut character_claimed = 0u16;

        for objective in &state.board.objectives {
            if calculate_objective_controller(state, objective.id) == Some(player) {
                objectives_controlled += 1;

                // Check if a CHARACTER is within range of this objective
                let has_character = state.units.iter().any(|u| {
                    u.owner == player
                        && !u.is_destroyed()
                        && u.is_on_battlefield()
                        && u.has_keyword(Keyword::Character)
                        && !u.battle_shocked
                        && is_unit_within_objective_range(state, u, objective.id)
                });

                if has_character {
                    character_claimed += 1;
                }
            }
        }

        if objectives_controlled > 0 {
            let mut vp_val = objectives_controlled * 5;
            // Bonus for CHARACTER-claimed objectives
            vp_val += character_claimed * 5;
            let vp = VictoryPoints::new(vp_val as i16);
            results.push((
                player,
                vp,
                GameEvent::VictoryPointsScored {
                    player,
                    amount: vp_val,
                    source: VpSource::MissionRule(format!(
                        "Display of Might: {} controlled, {} CHARACTER claimed (round {})",
                        objectives_controlled, character_claimed, round
                    )),
                },
            ));
        }
    }

    results
}

/// Award Battle Ready bonus VP (10VP for painted army).
///
/// Source: CP_Rules.md - Battle Ready Bonus
pub fn apply_battle_ready_bonus(player: PlayerId) -> (VictoryPoints, GameEvent) {
    let vp = VictoryPoints::new(10);
    (
        vp,
        GameEvent::VictoryPointsScored {
            player,
            amount: 10,
            source: VpSource::Custom("Battle Ready: fully painted army bonus".to_string()),
        },
    )
}

/// Calculate end-of-game scoring for all applicable systems.
///
/// Source: CP_Rules.md - End of Game Scoring
pub fn calculate_end_of_game_score(
    state: &GameState,
    mission_id: Option<wh40k_core_types::MissionId>,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    let mut results = Vec::new();

    // Mission-specific end-of-game bonuses
    if let Some(mid) = mission_id {
        if mid == mission_ids::ARCHEOTECH_RECOVERY {
            results.extend(score_archeotech_endgame(state));
        } else if mid == mission_ids::SWEEPING_RAID {
            results.extend(score_sweeping_raid_endgame(state));
        }
    }

    // Battle Ready bonus for both players
    for player_idx in 0..2u32 {
        let player = PlayerId::new(player_idx);
        let (vp, event) = apply_battle_ready_bonus(player);
        results.push((player, vp, event));
    }

    results
}

/// Determine the winner based on final VP totals.
///
/// Source: CP_Rules.md - Determining the Winner
pub fn determine_winner(state: &GameState) -> wh40k_core_types::GameOutcome {
    let p0_vp = state.players[0].vp.value();
    let p1_vp = state.players[1].vp.value();

    if p0_vp > p1_vp {
        wh40k_core_types::GameOutcome::Victory(PlayerId::new(0))
    } else if p1_vp > p0_vp {
        wh40k_core_types::GameOutcome::Victory(PlayerId::new(1))
    } else {
        wh40k_core_types::GameOutcome::Draw
    }
}

// ─── Helper Functions ───────────────────────────────────────────────────────

/// Calculate which player controls an objective based on OC sums within range.
///
/// Source: 40k_revised.md - Objective Control
pub fn calculate_objective_controller(
    state: &GameState,
    objective_id: ObjectiveId,
) -> Option<PlayerId> {
    let _objective = state.board.objectives.iter().find(|o| o.id == objective_id)?;

    let mut player_oc = [0i32; 2];

    for unit in &state.units {
        if unit.is_destroyed() || !unit.is_on_battlefield() {
            continue;
        }

        // Check if unit is within objective range (3" horizontal)
        if is_unit_within_objective_range(state, unit, objective_id) {
            let oc = unit.effective_oc().value() as i32;
            let player_idx = unit.owner.raw() as usize;
            if player_idx < 2 {
                player_oc[player_idx] += oc;
            }
        }
    }

    if player_oc[0] > player_oc[1] {
        Some(PlayerId::new(0))
    } else if player_oc[1] > player_oc[0] {
        Some(PlayerId::new(1))
    } else {
        None // Contested / no one controls
    }
}

/// Check if a unit has models within objective control range (3").
fn is_unit_within_objective_range(
    state: &GameState,
    unit: &crate::unit::UnitState,
    objective_id: ObjectiveId,
) -> bool {
    let objective = match state.board.objectives.iter().find(|o| o.id == objective_id) {
        Some(o) => o,
        None => return false,
    };

    let obj_range = wh40k_core_types::Inches::from_inches(3);

    unit.alive_models().iter().any(|model| {
        let dist = wh40k_geometry::distance(model.position, objective.position);
        dist <= obj_range
    })
}

/// Check if an objective is in No Man's Land (y=9" to y=21").
/// Standard CP deployment: Player 0 zone is y=0 to y=9", Player 1 zone is y=21" to y=30".
/// NML is the 12" gap in the center.
///
/// Source: CP_Rules.md - Standard deployment zones
fn is_objective_in_no_mans_land(
    objective: &wh40k_geometry::ObjectiveMarker,
) -> bool {
    let y_mils = objective.position.y.mils();
    // NML is y=9" to y=21" (9000 to 21000 mils)
    y_mils > 9000 && y_mils < 21000
}

/// Check if an objective is in the enemy's deployment zone relative to a player.
/// Player 0's DZ is y=0 to y=9", Player 1's DZ is y=21" to y=30".
fn is_objective_in_enemy_dz(
    objective: &wh40k_geometry::ObjectiveMarker,
    player: PlayerId,
) -> bool {
    let y_mils = objective.position.y.mils();
    if player.raw() == 0 {
        // Player 0's enemy is Player 1, whose DZ is y=21" to y=30"
        y_mils >= 21000
    } else {
        // Player 1's enemy is Player 0, whose DZ is y=0 to y=9"
        y_mils <= 9000
    }
}

/// Find the objectives closest to each player's battlefield edge.
/// Returns (closest_to_own_edge, closest_to_enemy_edge).
fn find_edge_objectives(
    state: &GameState,
    player: PlayerId,
) -> (Option<ObjectiveId>, Option<ObjectiveId>) {
    if state.board.objectives.is_empty() {
        return (None, None);
    }

    // Player 0's edge is y=0, Player 1's edge is y=30"
    let own_edge_y_mils: i32 = if player.raw() == 0 { 0 } else { 30_000 };
    let enemy_edge_y_mils: i32 = if player.raw() == 0 { 30_000 } else { 0 };

    let mut closest_own: Option<(ObjectiveId, i32)> = None;
    let mut closest_enemy: Option<(ObjectiveId, i32)> = None;

    for objective in &state.board.objectives {
        let obj_y = objective.position.y.mils();

        let dist_own = (obj_y - own_edge_y_mils).abs();
        let dist_enemy = (obj_y - enemy_edge_y_mils).abs();

        if closest_own.is_none_or(|(_, d)| dist_own < d) {
            closest_own = Some((objective.id, dist_own));
        }
        if closest_enemy.is_none_or(|(_, d)| dist_enemy < d) {
            closest_enemy = Some((objective.id, dist_enemy));
        }
    }

    (closest_own.map(|(id, _)| id), closest_enemy.map(|(id, _)| id))
}

/// Find the objective closest to board center (22", 15").
fn find_center_objective(state: &GameState) -> Option<ObjectiveId> {
    let center_x = 22_000i32; // 22" in mils
    let center_y = 15_000i32; // 15" in mils

    state.board.objectives.iter().min_by_key(|obj| {
        let dx = obj.position.x.mils() - center_x;
        let dy = obj.position.y.mils() - center_y;
        dx * dx + dy * dy // Squared distance is fine for comparison
    }).map(|obj| obj.id)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_core_types::BattleRound;

    #[test]
    fn test_enhancement_ids_unique() {
        let ids = [
            enhancement_ids::FEARSOME_PRESENCE,
            enhancement_ids::BANE_OF_THE_CRAVEN,
            enhancement_ids::WATCHMAN_OF_TERRA,
            enhancement_ids::WARRIOR_EXEMPLAR,
        ];
        let mut set = std::collections::HashSet::new();
        for id in &ids {
            assert!(set.insert(id.raw()), "Duplicate enhancement ID: {}", id);
        }
    }

    #[test]
    fn test_secondary_ids_unique() {
        let ids = [
            secondary_ids::CHAMPIONS_OF_KHORNE,
            secondary_ids::SKULL_TAKERS,
            secondary_ids::RAISE_THE_VEXILLAS,
            secondary_ids::CONSECRATED_GROUND,
        ];
        let mut set = std::collections::HashSet::new();
        for id in &ids {
            assert!(set.insert(id.raw()), "Duplicate secondary ID: {}", id);
        }
    }

    #[test]
    fn test_mission_ids_unique() {
        let ids = [
            mission_ids::CLASH_OF_PATROLS,
            mission_ids::ARCHEOTECH_RECOVERY,
            mission_ids::FORWARD_OUTPOST,
            mission_ids::SCORCHED_EARTH,
            mission_ids::SWEEPING_RAID,
            mission_ids::DISPLAY_OF_MIGHT,
        ];
        let mut set = std::collections::HashSet::new();
        for id in &ids {
            assert!(set.insert(id.raw()), "Duplicate mission ID: {}", id);
        }
    }

    #[test]
    fn test_battle_ready_bonus() {
        let (vp, event) = apply_battle_ready_bonus(PlayerId::new(0));
        assert_eq!(vp.value(), 10);
        if let GameEvent::VictoryPointsScored { player, amount, .. } = event {
            assert_eq!(player, PlayerId::new(0));
            assert_eq!(amount, 10);
        } else {
            panic!("Expected VictoryPointsScored event");
        }
    }

    #[test]
    fn test_fr_enhancements_count() {
        // Frenzied Reavers has 2 enhancements
        assert_ne!(enhancement_ids::FEARSOME_PRESENCE, enhancement_ids::BANE_OF_THE_CRAVEN);
    }

    #[test]
    fn test_custodes_enhancements_count() {
        // Custodes has 2 enhancements
        assert_ne!(enhancement_ids::WATCHMAN_OF_TERRA, enhancement_ids::WARRIOR_EXEMPLAR);
    }

    #[test]
    fn test_fr_secondaries_count() {
        // Frenzied Reavers has 2 secondaries
        assert_ne!(secondary_ids::CHAMPIONS_OF_KHORNE, secondary_ids::SKULL_TAKERS);
    }

    #[test]
    fn test_custodes_secondaries_count() {
        // Custodes has 2 secondaries
        assert_ne!(secondary_ids::RAISE_THE_VEXILLAS, secondary_ids::CONSECRATED_GROUND);
    }

    #[test]
    fn test_no_mans_land_detection() {
        use wh40k_core_types::{Inches, Position};

        // Objective at y=15" (center) should be in NML
        let obj_center = wh40k_geometry::ObjectiveMarker {
            id: ObjectiveId::new(1),
            position: Position::from_inches(22, 15),
            height: Inches::ZERO,
            control_status: wh40k_geometry::ObjectiveControlStatus::Uncontrolled,
            label: "Center".to_string(),
        };
        assert!(is_objective_in_no_mans_land(&obj_center));

        // Objective at y=5" should NOT be in NML (Player 0 DZ)
        let obj_p0 = wh40k_geometry::ObjectiveMarker {
            id: ObjectiveId::new(2),
            position: Position::from_inches(22, 5),
            height: Inches::ZERO,
            control_status: wh40k_geometry::ObjectiveControlStatus::Uncontrolled,
            label: "P0 DZ".to_string(),
        };
        assert!(!is_objective_in_no_mans_land(&obj_p0));

        // Objective at y=25" should NOT be in NML (Player 1 DZ)
        let obj_p1 = wh40k_geometry::ObjectiveMarker {
            id: ObjectiveId::new(3),
            position: Position::from_inches(22, 25),
            height: Inches::ZERO,
            control_status: wh40k_geometry::ObjectiveControlStatus::Uncontrolled,
            label: "P1 DZ".to_string(),
        };
        assert!(!is_objective_in_no_mans_land(&obj_p1));
    }

    #[test]
    fn test_enemy_dz_detection() {
        use wh40k_core_types::{Inches, Position};

        // Objective at y=25" (Player 1 DZ)
        let obj = wh40k_geometry::ObjectiveMarker {
            id: ObjectiveId::new(1),
            position: Position::from_inches(22, 25),
            height: Inches::ZERO,
            control_status: wh40k_geometry::ObjectiveControlStatus::Uncontrolled,
            label: "P1 DZ".to_string(),
        };

        // For Player 0, y=25" is enemy DZ
        assert!(is_objective_in_enemy_dz(&obj, PlayerId::new(0)));
        // For Player 1, y=25" is own DZ
        assert!(!is_objective_in_enemy_dz(&obj, PlayerId::new(1)));

        // Objective at y=5" (Player 0 DZ)
        let obj2 = wh40k_geometry::ObjectiveMarker {
            id: ObjectiveId::new(2),
            position: Position::from_inches(22, 5),
            height: Inches::ZERO,
            control_status: wh40k_geometry::ObjectiveControlStatus::Uncontrolled,
            label: "P0 DZ".to_string(),
        };

        // For Player 1, y=5" is enemy DZ
        assert!(is_objective_in_enemy_dz(&obj2, PlayerId::new(1)));
        // For Player 0, y=5" is own DZ
        assert!(!is_objective_in_enemy_dz(&obj2, PlayerId::new(0)));
    }

    #[test]
    fn test_consecrated_ground_kill_scoring() {
        use wh40k_dice::{DiceContext, DiceRoller, StreamKind};

        let seed = [0u8; 32];
        let ctx = DiceContext::new(seed, StreamKind::BattleShockTest, 0, 0);
        let mut state = GameState {
            content_version: "test".to_string(),
            scenario_id: None,
            battle_round: BattleRound::new(1),
            active_player: PlayerId::new(0),
            current_phase: Phase::PreBattle,
            current_subphase: wh40k_core_types::SubPhase::DetermineAttackerDefender,
            decision_owner: PlayerId::new(0),
            players: [
                crate::state::PlayerState::new(PlayerId::new(0), "P0".to_string()),
                crate::state::PlayerState::new(PlayerId::new(1), "P1".to_string()),
            ],
            units: Vec::new(),
            board: wh40k_geometry::Board::combat_patrol(),
            event_bus: wh40k_event_system::EventBus::new(),
            command_history: wh40k_command_system::CommandHistory::new(),
            dice_roller: DiceRoller::new(ctx),
            active_effects: Vec::new(),
            reaction_windows: Vec::new(),
            turn_flags: crate::state::TurnFlags::new(),
            game_outcome: wh40k_core_types::GameOutcome::InProgress,
            deterministic_counter: 0,
        };

        // Without Consecrated Ground selected, should return None
        assert!(score_consecrated_ground_kill(&state, PlayerId::new(0)).is_none());

        // Select Consecrated Ground for player 0
        state.players[0].secondary_choice = Some(secondary_ids::CONSECRATED_GROUND);

        // Should return Some(3VP)
        let result = score_consecrated_ground_kill(&state, PlayerId::new(0));
        assert!(result.is_some());
        let (vp, event) = result.unwrap();
        assert_eq!(vp.value(), 3);
        if let GameEvent::VictoryPointsScored { amount, .. } = event {
            assert_eq!(amount, 3);
        } else {
            panic!("Expected VictoryPointsScored event");
        }
    }

    #[test]
    fn test_consecrated_ground_loss_no_vp() {
        use wh40k_dice::{DiceContext, DiceRoller, StreamKind};

        let seed = [0u8; 32];
        let ctx = DiceContext::new(seed, StreamKind::BattleShockTest, 0, 0);
        let mut state = GameState {
            content_version: "test".to_string(),
            scenario_id: None,
            battle_round: BattleRound::new(1),
            active_player: PlayerId::new(0),
            current_phase: Phase::PreBattle,
            current_subphase: wh40k_core_types::SubPhase::DetermineAttackerDefender,
            decision_owner: PlayerId::new(0),
            players: [
                crate::state::PlayerState::new(PlayerId::new(0), "P0".to_string()),
                crate::state::PlayerState::new(PlayerId::new(1), "P1".to_string()),
            ],
            units: Vec::new(),
            board: wh40k_geometry::Board::combat_patrol(),
            event_bus: wh40k_event_system::EventBus::new(),
            command_history: wh40k_command_system::CommandHistory::new(),
            dice_roller: DiceRoller::new(ctx),
            active_effects: Vec::new(),
            reaction_windows: Vec::new(),
            turn_flags: crate::state::TurnFlags::new(),
            game_outcome: wh40k_core_types::GameOutcome::InProgress,
            deterministic_counter: 0,
        };

        state.players[0].secondary_choice = Some(secondary_ids::CONSECRATED_GROUND);

        // With 0 secondary VP, loss should return None (can't go below 0)
        assert!(score_consecrated_ground_loss(&state, PlayerId::new(0)).is_none());

        // Give some secondary VP first
        state.players[0].mission_progress.secondary_vp = VictoryPoints::new(5);
        let result = score_consecrated_ground_loss(&state, PlayerId::new(0));
        assert!(result.is_some());
        assert_eq!(result.unwrap().value(), 1);
    }

    #[test]
    fn test_determine_winner() {
        use wh40k_dice::{DiceContext, DiceRoller, StreamKind};

        let seed = [0u8; 32];
        let ctx = DiceContext::new(seed, StreamKind::BattleShockTest, 0, 0);
        let mut state = GameState {
            content_version: "test".to_string(),
            scenario_id: None,
            battle_round: BattleRound::new(5),
            active_player: PlayerId::new(0),
            current_phase: Phase::PreBattle,
            current_subphase: wh40k_core_types::SubPhase::DetermineAttackerDefender,
            decision_owner: PlayerId::new(0),
            players: [
                crate::state::PlayerState::new(PlayerId::new(0), "P0".to_string()),
                crate::state::PlayerState::new(PlayerId::new(1), "P1".to_string()),
            ],
            units: Vec::new(),
            board: wh40k_geometry::Board::combat_patrol(),
            event_bus: wh40k_event_system::EventBus::new(),
            command_history: wh40k_command_system::CommandHistory::new(),
            dice_roller: DiceRoller::new(ctx),
            active_effects: Vec::new(),
            reaction_windows: Vec::new(),
            turn_flags: crate::state::TurnFlags::new(),
            game_outcome: wh40k_core_types::GameOutcome::InProgress,
            deterministic_counter: 0,
        };

        // Draw at 0-0
        assert!(matches!(determine_winner(&state), wh40k_core_types::GameOutcome::Draw));

        // Player 0 wins
        state.players[0].vp = VictoryPoints::new(30);
        state.players[1].vp = VictoryPoints::new(20);
        assert!(matches!(
            determine_winner(&state),
            wh40k_core_types::GameOutcome::Victory(p) if p == PlayerId::new(0)
        ));

        // Player 1 wins
        state.players[0].vp = VictoryPoints::new(10);
        state.players[1].vp = VictoryPoints::new(25);
        assert!(matches!(
            determine_winner(&state),
            wh40k_core_types::GameOutcome::Victory(p) if p == PlayerId::new(1)
        ));
    }

    #[test]
    fn test_score_raze_objective() {
        let (vp, event) = score_raze_objective(PlayerId::new(0), ObjectiveId::new(3));
        assert_eq!(vp.value(), 10);
        if let GameEvent::VictoryPointsScored { player, amount, .. } = event {
            assert_eq!(player, PlayerId::new(0));
            assert_eq!(amount, 10);
        } else {
            panic!("Expected VictoryPointsScored event");
        }
    }
}

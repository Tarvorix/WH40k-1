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
            // Raise the Vexillas scores at end of turn, not here.
            // Handled by score_secondary_objectives_end_of_turn().
            // Source: Custodes.md - "at the end of YOUR turn"
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

/// Score secondary objectives that trigger at end of the player's turn.
///
/// Called from end_player_turn in the phase system.
///
/// Source: Custodes.md - Raise the Vexillas: "at the end of YOUR turn"
pub fn score_secondary_objectives_end_of_turn(
    state: &GameState,
    player: PlayerId,
) -> Vec<(VictoryPoints, GameEvent)> {
    let player_state = state.player(player);
    let secondary_id = match player_state.secondary_choice {
        Some(id) => id,
        None => return Vec::new(),
    };

    let mut results = Vec::new();

    if secondary_id == secondary_ids::RAISE_THE_VEXILLAS {
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

/// When during the turn the scoring function is being called.
///
/// Missions with BR5 split timing (M1, M3, M4, M6) need to know whether
/// scoring is happening at end of Command Phase or end of turn, because
/// the 2nd player scores at end of turn in BR5 instead of end of Command Phase.
///
/// Source: CP_Rules.md - Missions 1, 3, 4, 6 BR5 scoring timing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoringTiming {
    /// Scoring at end of Command Phase (default for BR2-4, and BR5 1st player).
    EndOfCommandPhase,
    /// Scoring at end of turn (BR5 2nd player only, for missions with split timing).
    EndOfTurn,
}

/// Determine which players should score given the round, timing, and whether
/// this mission uses BR5 split timing.
///
/// Per CP_Rules.md & 40k_revised.md:
/// Each player scores at the end of THEIR OWN Command Phase (one per player per round).
/// Since end_current_phase is called once per player turn, we return only the
/// active player to avoid double-scoring.
///
/// - BR2-4 EndOfCommandPhase: active player only (each player scores once per round)
/// - BR5 with split timing:
///   - EndOfCommandPhase: only the 1st player scores
///   - EndOfTurn: only the 2nd player scores
/// - BR5 without split timing: active player at EndOfCommandPhase only
fn players_to_score(state: &GameState, round: u8, timing: ScoringTiming, has_br5_split: bool) -> Vec<PlayerId> {
    match timing {
        ScoringTiming::EndOfCommandPhase => {
            if round == 5 && has_br5_split {
                // BR5 with split: only the 1st player scores at end of command phase
                let first_player = if state.players[0].first_turn {
                    PlayerId::new(0)
                } else {
                    PlayerId::new(1)
                };
                if state.active_player == first_player {
                    vec![first_player]
                } else {
                    // 2nd player's Command Phase in BR5 with split: don't score here
                    // (they score at EndOfTurn instead)
                    vec![]
                }
            } else {
                // BR2-4 or BR5 without split: active player scores at end of their Command Phase
                vec![state.active_player]
            }
        }
        ScoringTiming::EndOfTurn => {
            if round == 5 && has_br5_split {
                // BR5 with split: only the 2nd player scores at end of turn
                let second_player = if state.players[0].first_turn {
                    PlayerId::new(1)
                } else {
                    PlayerId::new(0)
                };
                if state.active_player == second_player {
                    vec![second_player]
                } else {
                    vec![]
                }
            } else {
                // BR2-4 or BR5 without split: no scoring at end of turn
                vec![]
            }
        }
    }
}

/// Score primary objectives for the current round based on mission rules.
///
/// The `timing` parameter indicates when during the turn this is being called,
/// which matters for BR5 split timing in missions 1, 3, 4, and 6.
///
/// Returns VP scored for each player and events.
///
/// Source: CP_Rules.md - Mission Scoring
pub fn score_primary_objectives(
    state: &GameState,
    mission_id: Option<wh40k_core_types::MissionId>,
    timing: ScoringTiming,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    let round = state.battle_round.number();

    match mission_id {
        Some(id) if id == mission_ids::CLASH_OF_PATROLS => {
            score_clash_of_patrols(state, round, timing)
        }
        Some(id) if id == mission_ids::ARCHEOTECH_RECOVERY => {
            score_archeotech_recovery(state, round)
        }
        Some(id) if id == mission_ids::FORWARD_OUTPOST => {
            score_forward_outpost(state, round, timing)
        }
        Some(id) if id == mission_ids::SCORCHED_EARTH => {
            score_scorched_earth(state, round, timing)
        }
        Some(id) if id == mission_ids::SWEEPING_RAID => {
            score_sweeping_raid(state, round)
        }
        Some(id) if id == mission_ids::DISPLAY_OF_MIGHT => {
            score_display_of_might(state, round, timing)
        }
        _ => {
            // Default: 5VP per objective controlled, rounds 2-5
            score_default_primary(state, round)
        }
    }
}

/// Default primary scoring: 5VP per objective controlled, rounds 2-5.
/// Only scores the active player at end of their Command Phase.
fn score_default_primary(
    state: &GameState,
    round: u8,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    let mut results = Vec::new();

    if round < 2 {
        return results;
    }

    let player = state.active_player;
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

    results
}

/// Mission 1 - Clash of Patrols: Take and Hold primary objective.
///
/// BR2-4: End of Command Phase, 5VP per objective controlled (max 15VP).
/// BR5 1st player: as above (end of Command Phase).
/// BR5 2nd player: same scoring but at end of turn.
///
/// Source: CP_Rules.md - Mission 1: Clash of Patrols
fn score_clash_of_patrols(
    state: &GameState,
    round: u8,
    timing: ScoringTiming,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    let mut results = Vec::new();

    if round < 2 {
        return results;
    }

    // BR5 has split timing: 1st player at end of command phase, 2nd at end of turn
    let scoring_players = players_to_score(state, round, timing, true);

    for player in scoring_players {
        let mut objectives_controlled = 0u16;

        for objective in &state.board.objectives {
            if calculate_objective_controller(state, objective.id) == Some(player) {
                objectives_controlled += 1;
            }
        }

        if objectives_controlled > 0 {
            // 5VP per objective controlled, max 15VP per CP_Rules.md
            let vp_val = (objectives_controlled * 5).min(15);
            let vp = VictoryPoints::new(vp_val as i16);
            results.push((
                player,
                vp,
                GameEvent::VictoryPointsScored {
                    player,
                    amount: vp_val,
                    source: VpSource::MissionRule(format!(
                        "Clash of Patrols: {} objective(s) controlled (round {}, max 15VP)",
                        objectives_controlled, round
                    )),
                },
            ));
        }
    }

    results
}

/// Evaluate Retrieve Intelligence mechanic for Clash of Patrols (Mission 1).
///
/// From battle round 2 onwards, in your Command Phase:
/// - Select one objective marker you control
/// - If your WARLORD is on the battlefield (or embarked in TRANSPORT): gain 1CP
/// - Each objective marker can only be selected once (by either player)
///
/// Returns the selected objective (if any) and events generated.
///
/// Source: CP_Rules.md - Mission 1: Clash of Patrols, Retrieve Intelligence
pub fn evaluate_retrieve_intelligence(
    state: &mut GameState,
    player: PlayerId,
    selected_objective: ObjectiveId,
) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let round = state.battle_round.number();

    // Only from BR2 onwards
    if round < 2 {
        return events;
    }

    // Check the objective hasn't already been selected by either player
    let already_selected_p0 = state.players[0]
        .mission_progress
        .retrieved_intelligence_objectives
        .contains(&selected_objective);
    let already_selected_p1 = state.players[1]
        .mission_progress
        .retrieved_intelligence_objectives
        .contains(&selected_objective);
    if already_selected_p0 || already_selected_p1 {
        return events; // Each objective can only be selected once
    }

    // Player must control the objective
    if calculate_objective_controller(state, selected_objective) != Some(player) {
        return events;
    }

    // Record that this objective has been selected
    state
        .player_mut(player)
        .mission_progress
        .retrieved_intelligence_objectives
        .push(selected_objective);

    // Check if WARLORD is on the battlefield (or embarked in TRANSPORT).
    // Warlord is identified as a CHARACTER unit whose player has an enhancement choice.
    let has_enhancement = state.player(player).enhancement_choice.is_some();
    let warlord_available = has_enhancement && state.units.iter().any(|unit| {
        unit.owner == player
            && !unit.is_destroyed()
            && unit.is_on_battlefield()
            && unit.is_character()
    });

    if warlord_available {
        let gained = state.player_mut(player).gain_extra_cp(1);
        if gained > 0 {
            events.push(GameEvent::CommandPointsGained {
                player,
                amount: 1,
                reason: wh40k_event_system::CpReason::Custom(
                    format!(
                        "Retrieve Intelligence: WARLORD on battlefield, selected objective {} (round {})",
                        selected_objective, round
                    ),
                ),
            });
        }
    }

    events
}

/// Mission 2 - Archeotech Recovery: Recover Archeotech primary objective.
///
/// BR2-5: End of Command Phase, 5VP per objective controlled (max 15VP).
/// End of game: +10VP if you control the last No Man's Land objective.
///
/// Note: Irradiated Power Cells objective removal is handled by the phase system.
/// We just score whatever objectives remain on the board.
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

    // No BR5 split timing — active player scores at end of their Command Phase
    let player = state.active_player;
    let mut objectives_controlled = 0u16;

    for objective in &state.board.objectives {
        if calculate_objective_controller(state, objective.id) == Some(player) {
            objectives_controlled += 1;
        }
    }

    if objectives_controlled > 0 {
        // 5VP per objective controlled, max 15VP per CP_Rules.md
        let vp_val = (objectives_controlled * 5).min(15);
        let vp = VictoryPoints::new(vp_val as i16);
        results.push((
            player,
            vp,
            GameEvent::VictoryPointsScored {
                player,
                amount: vp_val,
                source: VpSource::MissionRule(format!(
                    "Archeotech Recovery: {} objective(s) controlled (round {}, max 15VP)",
                    objectives_controlled, round
                )),
            },
        ));
    }

    results
}

/// Score Archeotech Recovery end-of-game bonus.
///
/// Per CP_Rules.md: "If you control the last No Man's Land objective: +10 VP"
/// After Irradiated Power Cells removes Gamma (BR4) and Beta (BR5) objectives,
/// the remaining NML objective(s) are what's scored. If a player controls
/// a remaining NML objective, they get +10VP.
///
/// Source: CP_Rules.md - Mission 2: Archeotech Recovery
pub fn score_archeotech_endgame(
    state: &GameState,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    let mut results = Vec::new();

    // Find remaining No Man's Land objectives on the board
    let nml_objectives: Vec<ObjectiveId> = state
        .board
        .objectives
        .iter()
        .filter(|obj| is_objective_in_no_mans_land(obj))
        .map(|obj| obj.id)
        .collect();

    // If there are remaining NML objectives, check who controls them
    // The rules say "the last No Man's Land objective" — award +10VP for controlling it
    for nml_obj_id in &nml_objectives {
        for player_idx in 0..2u32 {
            let player = PlayerId::new(player_idx);
            if calculate_objective_controller(state, *nml_obj_id) == Some(player) {
                let vp = VictoryPoints::new(10);
                results.push((
                    player,
                    vp,
                    GameEvent::VictoryPointsScored {
                        player,
                        amount: 10,
                        source: VpSource::MissionRule(format!(
                            "Archeotech Recovery: control last NML objective {} (+10VP)",
                            nml_obj_id
                        )),
                    },
                ));
            }
        }
    }

    results
}

/// Mission 3 - Forward Outpost: Vital Ground primary objective.
///
/// BR2-4: End of Command Phase, 5VP per NML objective controlled +
///        10VP for controlling enemy DZ objective (max 15VP).
/// BR5 1st player: as above (end of Command Phase).
/// BR5 2nd player: same scoring but at end of turn.
///
/// IMPORTANT: Only NML objectives give 5VP each. DZ objectives give 0VP for
/// per-objective scoring. The enemy DZ objective gives a flat 10VP bonus.
/// Own DZ objectives give nothing.
///
/// Source: CP_Rules.md - Mission 3: Forward Outpost
fn score_forward_outpost(
    state: &GameState,
    round: u8,
    timing: ScoringTiming,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    let mut results = Vec::new();

    if round < 2 {
        return results;
    }

    // BR5 has split timing: 1st player at end of command phase, 2nd at end of turn
    let scoring_players = players_to_score(state, round, timing, true);

    for player in scoring_players {
        let mut nml_objectives_controlled = 0u16;
        let mut controls_enemy_dz = false;

        for objective in &state.board.objectives {
            if calculate_objective_controller(state, objective.id) == Some(player) {
                // Only NML objectives count for per-objective VP
                if is_objective_in_no_mans_land(objective) {
                    nml_objectives_controlled += 1;
                }
                // Check if this is the enemy DZ objective
                if is_objective_in_enemy_dz(objective, player) {
                    controls_enemy_dz = true;
                }
            }
        }

        // 5VP per NML objective + 10VP for enemy DZ objective, max 15VP
        let mut vp_val = nml_objectives_controlled * 5;
        if controls_enemy_dz {
            vp_val += 10;
        }
        // Apply max 15VP cap per CP_Rules.md
        vp_val = vp_val.min(15);

        if vp_val > 0 {
            let vp = VictoryPoints::new(vp_val as i16);
            results.push((
                player,
                vp,
                GameEvent::VictoryPointsScored {
                    player,
                    amount: vp_val,
                    source: VpSource::MissionRule(format!(
                        "Forward Outpost: {} NML obj{} (round {}, max 15VP)",
                        nml_objectives_controlled,
                        if controls_enemy_dz { " + enemy DZ (+10VP)" } else { "" },
                        round
                    )),
                },
            ));
        }
    }

    results
}

/// Evaluate Sabotage Enemy Comms mechanic for Forward Outpost (Mission 3).
///
/// At end of your turn: if you control the objective in opponent's deployment zone,
/// until end of battle the opponent cannot use the Command Re-roll Stratagem.
///
/// This should be called at end of each player's turn during Mission 3.
///
/// Source: CP_Rules.md - Mission 3: Forward Outpost, Sabotage Enemy Comms
pub fn evaluate_sabotage_enemy_comms(
    state: &mut GameState,
    active_player: PlayerId,
) -> Vec<GameEvent> {
    let mut events = Vec::new();

    // Check if the active player controls an objective in the opponent's DZ
    let opponent = if active_player.raw() == 0 {
        PlayerId::new(1)
    } else {
        PlayerId::new(0)
    };

    let controls_opponent_dz = state.board.objectives.iter().any(|objective| {
        is_objective_in_enemy_dz(objective, active_player)
            && calculate_objective_controller(state, objective.id) == Some(active_player)
    });

    if controls_opponent_dz {
        // Block opponent's Command Re-roll until end of battle
        if !state.player(opponent).mission_progress.command_reroll_blocked {
            state
                .player_mut(opponent)
                .mission_progress
                .command_reroll_blocked = true;

            events.push(GameEvent::VictoryPointsScored {
                player: active_player,
                amount: 0,
                source: VpSource::MissionRule(
                    "Sabotage Enemy Comms: opponent's Command Re-roll blocked until end of battle"
                        .to_string(),
                ),
            });
        }
    }

    events
}

/// Mission 4 - Scorched Earth: Raze and Ruin primary objective.
///
/// BR2-4: End of Command Phase, threshold-based scoring:
///   - 5VP if you control 1+ objectives
///   - 5VP if you control more objectives than opponent
///   - 10VP if you razed an objective this turn
/// BR5 1st player: as above (end of Command Phase).
/// BR5 2nd player: same scoring but at end of turn.
///
/// Source: CP_Rules.md - Mission 4: Scorched Earth
fn score_scorched_earth(
    state: &GameState,
    round: u8,
    timing: ScoringTiming,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    let mut results = Vec::new();

    if round < 2 {
        return results;
    }

    // BR5 has split timing: 1st player at end of command phase, 2nd at end of turn
    let scoring_players = players_to_score(state, round, timing, true);

    // Pre-compute objective control counts for both players
    let mut control_count = [0u16; 2];
    for objective in &state.board.objectives {
        for player_idx in 0..2u32 {
            let player = PlayerId::new(player_idx);
            if calculate_objective_controller(state, objective.id) == Some(player) {
                control_count[player_idx as usize] += 1;
            }
        }
    }

    for player in scoring_players {
        let idx = player.raw() as usize;
        let opponent_idx = 1 - idx;
        let mut total_vp = 0u16;
        let mut categories_met = Vec::new();

        // Category 1: 5VP if control 1+ objectives
        if control_count[idx] >= 1 {
            total_vp += 5;
            categories_met.push("control 1+ obj");
        }

        // Category 2: 5VP if control more objectives than opponent
        if control_count[idx] > control_count[opponent_idx] {
            total_vp += 5;
            categories_met.push("control more than opponent");
        }

        // Category 3: 10VP if razed an objective this turn
        if state.player(player).mission_progress.razed_this_turn {
            total_vp += 10;
            categories_met.push("razed objective (+10VP)");
        }

        if total_vp > 0 {
            let vp = VictoryPoints::new(total_vp as i16);
            results.push((
                player,
                vp,
                GameEvent::VictoryPointsScored {
                    player,
                    amount: total_vp,
                    source: VpSource::MissionRule(format!(
                        "Scorched Earth: {} (round {})",
                        categories_met.join(", "), round
                    )),
                },
            ));
        }
    }

    results
}

/// Validate whether a raze action is legal in Scorched Earth mission.
///
/// Restrictions per CP_Rules.md - Mission 4: Scorched Earth:
/// - From battle round 2 onwards
/// - Must have 2+ objectives remaining on the board
/// - Player must control the objective
/// - No enemy units within 3" of the objective
/// - Attacker (Player 0) cannot raze objective marker A
/// - Defender (Player 1) cannot raze objective marker B
///
/// Returns Ok(()) if valid, Err(reason) if invalid.
///
/// Source: CP_Rules.md - Mission 4: Scorched Earth, Raze and Ruin
pub fn validate_raze_objective(
    state: &GameState,
    player: PlayerId,
    objective_id: ObjectiveId,
) -> Result<(), String> {
    let round = state.battle_round.number();

    // Must be BR2+
    if round < 2 {
        return Err("Raze is only available from battle round 2 onwards".to_string());
    }

    // Must have 2+ objectives remaining
    if state.board.objectives.len() < 2 {
        return Err("Cannot raze: fewer than 2 objectives remain".to_string());
    }

    // Player must control the objective
    if calculate_objective_controller(state, objective_id) != Some(player) {
        return Err("Cannot raze: you do not control this objective".to_string());
    }

    // Find the objective to check restrictions
    let objective = state
        .board
        .objectives
        .iter()
        .find(|o| o.id == objective_id)
        .ok_or_else(|| "Objective not found on board".to_string())?;

    // Attacker (Player 0) cannot raze objective A
    if player.raw() == 0 && objective.label == "A" {
        return Err("Attacker cannot raze objective marker A".to_string());
    }

    // Defender (Player 1) cannot raze objective B
    if player.raw() == 1 && objective.label == "B" {
        return Err("Defender cannot raze objective marker B".to_string());
    }

    // No enemy units within 3" of the objective
    let raze_exclusion_range = wh40k_core_types::Inches::from_inches(3);
    let has_enemy_nearby = state.units.iter().any(|unit| {
        unit.owner != player
            && !unit.is_destroyed()
            && unit.is_on_battlefield()
            && unit.alive_models().iter().any(|model| {
                wh40k_geometry::distance(model.position, objective.position) <= raze_exclusion_range
            })
    });

    if has_enemy_nearby {
        return Err("Cannot raze: enemy units within 3\" of objective".to_string());
    }

    Ok(())
}

/// Execute a raze action and award VP for Scorched Earth mission.
///
/// This should be called after validate_raze_objective succeeds.
/// Sets the razed_this_turn flag and awards 10VP (incorporated into
/// threshold scoring via score_scorched_earth).
///
/// Note: The actual objective removal from the board must be handled
/// by the calling code (phase system).
///
/// Source: CP_Rules.md - Mission 4: Scorched Earth
pub fn score_raze_objective(
    player: PlayerId,
    objective_id: ObjectiveId,
) -> (VictoryPoints, GameEvent) {
    // The 10VP from razing is part of the threshold scoring (score_scorched_earth),
    // not a separate award. This function returns a 0VP event for tracking purposes.
    // The razed_this_turn flag triggers the 10VP in score_scorched_earth.
    let vp = VictoryPoints::new(0);
    (
        vp,
        GameEvent::VictoryPointsScored {
            player,
            amount: 0,
            source: VpSource::MissionRule(format!(
                "Scorched Earth: razed objective {} (10VP awarded in round scoring)",
                objective_id
            )),
        },
    )
}

/// Mission 5 - Sweeping Raid: Priority Targets scoring.
///
/// BR2-4 ONLY (not BR5): End of Command phase, 5VP per objective controlled (max 15VP).
/// Supply Lines CP gain is handled separately in Command Phase execution.
/// End-of-battle bonuses handled in score_sweeping_raid_endgame().
///
/// Source: CP_Rules.md - Mission 5: Sweeping Raid
fn score_sweeping_raid(
    state: &GameState,
    round: u8,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    let mut results = Vec::new();

    // Only score BR 2-4 per CP_Rules.md
    if round < 2 || round > 4 {
        return results;
    }

    // Active player scores at end of their Command Phase
    let player = state.active_player;
    let mut objectives_controlled = 0u16;

    for objective in &state.board.objectives {
        if calculate_objective_controller(state, objective.id) == Some(player) {
            objectives_controlled += 1;
        }
    }

    if objectives_controlled > 0 {
        // Max 15VP per scoring (3 objectives × 5VP)
        let vp_val = (objectives_controlled * 5).min(15);
        let vp = VictoryPoints::new(vp_val as i16);
        results.push((
            player,
            vp,
            GameEvent::VictoryPointsScored {
                player,
                amount: vp_val,
                source: VpSource::MissionRule(format!(
                        "Sweeping Raid: {} objective(s) controlled (round {}, max 15VP)",
                        objectives_controlled, round
                    )),
                },
            ));
    }

    results
}

/// Score Sweeping Raid end-of-battle bonus.
///
/// Per CP_Rules.md - Mission 5: Sweeping Raid:
/// - Attacker (Player 0): 5VP for objective C, 10VP for objective D
/// - Defender (Player 1): 5VP for objective B, 10VP for objective A
///
/// Objectives are identified by label (A, B, C, D).
pub fn score_sweeping_raid_endgame(
    state: &GameState,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    let mut results = Vec::new();

    // Build a map of objective label → id for lookup
    let label_to_id: Vec<(&str, ObjectiveId)> = state
        .board
        .objectives
        .iter()
        .map(|o| (o.label.as_str(), o.id))
        .collect();

    let find_obj = |label: &str| -> Option<ObjectiveId> {
        label_to_id.iter().find(|(l, _)| *l == label).map(|(_, id)| *id)
    };

    // Attacker (Player 0): 5VP for C, 10VP for D
    let attacker = PlayerId::new(0);
    let mut attacker_vp = 0u16;
    let mut attacker_details = Vec::new();

    if let Some(obj_c) = find_obj("C") {
        if calculate_objective_controller(state, obj_c) == Some(attacker) {
            attacker_vp += 5;
            attacker_details.push("obj C (+5VP)");
        }
    }
    if let Some(obj_d) = find_obj("D") {
        if calculate_objective_controller(state, obj_d) == Some(attacker) {
            attacker_vp += 10;
            attacker_details.push("obj D (+10VP)");
        }
    }

    if attacker_vp > 0 {
        let vp = VictoryPoints::new(attacker_vp as i16);
        results.push((
            attacker,
            vp,
            GameEvent::VictoryPointsScored {
                player: attacker,
                amount: attacker_vp,
                source: VpSource::MissionRule(format!(
                    "Sweeping Raid end-of-battle: Attacker {}",
                    attacker_details.join(", ")
                )),
            },
        ));
    }

    // Defender (Player 1): 5VP for B, 10VP for A
    let defender = PlayerId::new(1);
    let mut defender_vp = 0u16;
    let mut defender_details = Vec::new();

    if let Some(obj_b) = find_obj("B") {
        if calculate_objective_controller(state, obj_b) == Some(defender) {
            defender_vp += 5;
            defender_details.push("obj B (+5VP)");
        }
    }
    if let Some(obj_a) = find_obj("A") {
        if calculate_objective_controller(state, obj_a) == Some(defender) {
            defender_vp += 10;
            defender_details.push("obj A (+10VP)");
        }
    }

    if defender_vp > 0 {
        let vp = VictoryPoints::new(defender_vp as i16);
        results.push((
            defender,
            vp,
            GameEvent::VictoryPointsScored {
                player: defender,
                amount: defender_vp,
                source: VpSource::MissionRule(format!(
                    "Sweeping Raid end-of-battle: Defender {}",
                    defender_details.join(", ")
                )),
            },
        ));
    }

    results
}

/// Evaluate Supply Lines mechanic for Sweeping Raid (Mission 5).
///
/// At start of Command Phase: if the active player controls their own
/// deployment zone objective, roll D6. On 4+, gain 1CP.
///
/// Source: CP_Rules.md - Mission 5: Sweeping Raid, Supply Lines
pub fn evaluate_supply_lines(
    state: &mut GameState,
    player: PlayerId,
) -> Vec<GameEvent> {
    let mut events = Vec::new();

    // Find an objective in the player's own deployment zone that they control
    let controls_own_dz_obj = state.board.objectives.iter().any(|objective| {
        is_objective_in_own_dz(objective, player)
            && calculate_objective_controller(state, objective.id) == Some(player)
    });

    if !controls_own_dz_obj {
        return events;
    }

    // Roll D6 for Supply Lines
    let (roll, _) = state.dice_roller.roll_d6(wh40k_dice::RollPurpose::Misc {
        description: "Supply Lines".to_string(),
    });

    if roll >= 4 {
        let gained = state.player_mut(player).gain_extra_cp(1);
        if gained > 0 {
            events.push(GameEvent::CommandPointsGained {
                player,
                amount: 1,
                reason: wh40k_event_system::CpReason::Custom(
                    format!("Supply Lines: controlled DZ objective, rolled {} (4+)", roll),
                ),
            });
        }
    }

    events
}

/// Mission 6 - Display of Might: Symbolic Sites scoring.
///
/// BR2-4: End of Command Phase, 5VP each for:
///   1) Control 1+ objectives
///   2) Control 2+ objectives
///   3) 1+ symbolic sites (NML objectives) claimed by your CHARACTER
///   4) Same CHARACTER has claimed same site for 2+ consecutive turns
/// Max 20VP per scoring.
/// BR5 1st player: as above (end of Command Phase).
/// BR5 2nd player: same scoring but at end of turn.
///
/// Claim Sites rule: NML objectives are symbolic sites. If you control one
/// AND have 1+ non-Battle-shocked CHARACTER models within range, that site
/// is claimed by those models.
///
/// Source: CP_Rules.md - Mission 6: Display of Might
fn score_display_of_might(
    state: &GameState,
    round: u8,
    timing: ScoringTiming,
) -> Vec<(PlayerId, VictoryPoints, GameEvent)> {
    let mut results = Vec::new();

    if round < 2 {
        return results;
    }

    // BR5 has split timing: 1st player at end of command phase, 2nd at end of turn
    let scoring_players = players_to_score(state, round, timing, true);

    for player in scoring_players {
        let mut total_vp = 0u16;
        let mut categories_met = Vec::new();

        // Count ALL objectives controlled (not just symbolic)
        let mut objectives_controlled = 0u16;
        for objective in &state.board.objectives {
            if calculate_objective_controller(state, objective.id) == Some(player) {
                objectives_controlled += 1;
            }
        }

        // Category 1: Control 1+ objectives → 5VP
        if objectives_controlled >= 1 {
            total_vp += 5;
            categories_met.push("control 1+ obj");
        }

        // Category 2: Control 2+ objectives → 5VP
        if objectives_controlled >= 2 {
            total_vp += 5;
            categories_met.push("control 2+ obj");
        }

        // Evaluate symbolic site claims:
        // NML objectives controlled by player with a CHARACTER within range
        let mut current_claims: Vec<(ObjectiveId, UnitId)> = Vec::new();
        for objective in &state.board.objectives {
            // Only NML objectives are symbolic sites
            if !is_objective_in_no_mans_land(objective) {
                continue;
            }
            // Player must control the objective
            if calculate_objective_controller(state, objective.id) != Some(player) {
                continue;
            }
            // Find CHARACTER models within range (alive, not battle-shocked)
            for unit in &state.units {
                if unit.owner == player
                    && !unit.is_destroyed()
                    && unit.is_on_battlefield()
                    && unit.has_keyword(Keyword::Character)
                    && !unit.battle_shocked
                    && is_unit_within_objective_range(state, unit, objective.id)
                {
                    current_claims.push((objective.id, unit.id));
                }
            }
        }

        // Category 3: 1+ symbolic sites claimed by your CHARACTER → 5VP
        let sites_claimed: std::collections::HashSet<ObjectiveId> =
            current_claims.iter().map(|(obj, _)| *obj).collect();
        if !sites_claimed.is_empty() {
            total_vp += 5;
            categories_met.push("1+ symbolic claimed");
        }

        // Category 4: Same CHARACTER has claimed same site for 2+ consecutive turns → 5VP
        // Check if any current claim (obj, character) also existed in the previous round
        let previous_round = round.saturating_sub(1);
        let has_consecutive = round >= 3 && current_claims.iter().any(|(obj_id, unit_id)| {
            state.player(player).mission_progress.symbolic_site_claims.iter().any(|(r, o, u)| {
                *r == previous_round && *o == *obj_id && *u == *unit_id
            })
        });
        if has_consecutive {
            total_vp += 5;
            categories_met.push("consecutive claim");
        }

        if total_vp > 0 {
            let vp = VictoryPoints::new(total_vp as i16);
            results.push((
                player,
                vp,
                GameEvent::VictoryPointsScored {
                    player,
                    amount: total_vp,
                    source: VpSource::MissionRule(format!(
                        "Display of Might: {} (round {})",
                        categories_met.join(", "), round
                    )),
                },
            ));
        }
    }

    results
}

/// Record symbolic site claims for Display of Might (Mission 6).
///
/// Must be called alongside scoring so that consecutive-turn claims can
/// be checked in future rounds. Records which CHARACTER models are currently
/// claiming which symbolic sites (NML objectives they control and are within range of).
///
/// Source: CP_Rules.md - Mission 6: Display of Might, Claim Sites rule
pub fn record_display_of_might_claims(
    state: &mut GameState,
    round: u8,
) {
    for player_idx in 0..2u32 {
        let player = PlayerId::new(player_idx);
        let mut claims = Vec::new();

        for objective in &state.board.objectives {
            // Only NML objectives are symbolic sites
            if !is_objective_in_no_mans_land(objective) {
                continue;
            }
            // Player must control the objective
            if calculate_objective_controller(state, objective.id) != Some(player) {
                continue;
            }
            // Find CHARACTER models within range (alive, not battle-shocked)
            for unit in &state.units {
                if unit.owner == player
                    && !unit.is_destroyed()
                    && unit.is_on_battlefield()
                    && unit.has_keyword(Keyword::Character)
                    && !unit.battle_shocked
                    && is_unit_within_objective_range(state, unit, objective.id)
                {
                    claims.push((round, objective.id, unit.id));
                }
            }
        }

        state
            .player_mut(player)
            .mission_progress
            .symbolic_site_claims
            .extend(claims);
    }
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
pub fn is_unit_within_objective_range(
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
pub fn is_objective_in_no_mans_land(
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

/// Check if an objective is in the player's own deployment zone.
/// Player 0's DZ is y=0 to y=9", Player 1's DZ is y=21" to y=30".
fn is_objective_in_own_dz(
    objective: &wh40k_geometry::ObjectiveMarker,
    player: PlayerId,
) -> bool {
    let y_mils = objective.position.y.mils();
    if player.raw() == 0 {
        // Player 0's DZ is y=0 to y=9"
        y_mils <= 9000
    } else {
        // Player 1's DZ is y=21" to y=30"
        y_mils >= 21000
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


// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_core_types::{BattleRound, GameMode};

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
            secured_by: None,
        };
        assert!(is_objective_in_no_mans_land(&obj_center));

        // Objective at y=5" should NOT be in NML (Player 0 DZ)
        let obj_p0 = wh40k_geometry::ObjectiveMarker {
            id: ObjectiveId::new(2),
            position: Position::from_inches(22, 5),
            height: Inches::ZERO,
            control_status: wh40k_geometry::ObjectiveControlStatus::Uncontrolled,
            label: "P0 DZ".to_string(),
            secured_by: None,
        };
        assert!(!is_objective_in_no_mans_land(&obj_p0));

        // Objective at y=25" should NOT be in NML (Player 1 DZ)
        let obj_p1 = wh40k_geometry::ObjectiveMarker {
            id: ObjectiveId::new(3),
            position: Position::from_inches(22, 25),
            height: Inches::ZERO,
            control_status: wh40k_geometry::ObjectiveControlStatus::Uncontrolled,
            label: "P1 DZ".to_string(),
            secured_by: None,
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
            secured_by: None,
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
            secured_by: None,
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
            deployment_config: None,
            game_mode: GameMode::CombatPatrol,
            mode_state: None,
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
            deployment_config: None,
            game_mode: GameMode::CombatPatrol,
            mode_state: None,
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
            deployment_config: None,
            game_mode: GameMode::CombatPatrol,
            mode_state: None,
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
    fn test_score_raze_objective_tracking() {
        // score_raze_objective returns 0VP because the 10VP from razing
        // is incorporated into threshold scoring via score_scorched_earth.
        // This function just returns a tracking event.
        let (vp, event) = score_raze_objective(PlayerId::new(0), ObjectiveId::new(3));
        assert_eq!(vp.value(), 0);
        if let GameEvent::VictoryPointsScored { player, amount, .. } = event {
            assert_eq!(player, PlayerId::new(0));
            assert_eq!(amount, 0);
        } else {
            panic!("Expected VictoryPointsScored event");
        }
    }

    // ─── Helper to create a test GameState with objectives ────────────────

    fn make_scoring_test_state(round: u8) -> GameState {
        use wh40k_dice::{DiceContext, DiceRoller, StreamKind};
        use wh40k_core_types::{Inches, Position};

        let seed = [0u8; 32];
        let ctx = DiceContext::new(seed, StreamKind::BattleShockTest, 0, 0);
        let mut state = GameState {
            content_version: "test".to_string(),
            scenario_id: None,
            battle_round: BattleRound::new(round),
            active_player: PlayerId::new(0),
            current_phase: Phase::PreBattle,
            current_subphase: wh40k_core_types::SubPhase::DetermineAttackerDefender,
            decision_owner: PlayerId::new(0),
            players: [
                crate::state::PlayerState::new(PlayerId::new(0), "Attacker".to_string()),
                crate::state::PlayerState::new(PlayerId::new(1), "Defender".to_string()),
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
            deployment_config: None,
            game_mode: GameMode::CombatPatrol,
            mode_state: None,
        };

        // Set up objectives:
        // A (y=5") = P0 DZ, B (y=25") = P1 DZ
        // C (y=12") = NML, D (y=18") = NML
        state.board.objectives = vec![
            wh40k_geometry::ObjectiveMarker {
                id: ObjectiveId::new(1),
                position: Position::from_inches(22, 5),
                height: Inches::ZERO,
                control_status: wh40k_geometry::ObjectiveControlStatus::Uncontrolled,
                label: "A".to_string(),
                secured_by: None,
            },
            wh40k_geometry::ObjectiveMarker {
                id: ObjectiveId::new(2),
                position: Position::from_inches(22, 25),
                height: Inches::ZERO,
                control_status: wh40k_geometry::ObjectiveControlStatus::Uncontrolled,
                label: "B".to_string(),
                secured_by: None,
            },
            wh40k_geometry::ObjectiveMarker {
                id: ObjectiveId::new(3),
                position: Position::from_inches(15, 12),
                height: Inches::ZERO,
                control_status: wh40k_geometry::ObjectiveControlStatus::Uncontrolled,
                label: "C".to_string(),
                secured_by: None,
            },
            wh40k_geometry::ObjectiveMarker {
                id: ObjectiveId::new(4),
                position: Position::from_inches(29, 18),
                height: Inches::ZERO,
                control_status: wh40k_geometry::ObjectiveControlStatus::Uncontrolled,
                label: "D".to_string(),
                secured_by: None,
            },
        ];

        // Player 0 goes first by default
        state.players[0].first_turn = true;
        state.players[1].first_turn = false;

        state
    }

    /// Add a unit with specified OC near an objective to control it.
    fn add_controlling_unit(
        state: &mut GameState,
        player: PlayerId,
        unit_id: u32,
        objective_position: wh40k_core_types::Position,
        oc: u8,
    ) {
        use wh40k_core_types::*;

        let model = crate::unit::ModelState::new(
            ModelId::new(unit_id * 10 + 1),
            UnitId::new(unit_id),
            Wounds::new(2),
            objective_position,
            BaseSize::MM32,
            Vec::new(), // ranged_weapons
            Vec::new(), // melee_weapons
            false,      // is_leader
            None,       // feel_no_pain
        );

        let mut unit = crate::unit::UnitState::new(
            UnitId::new(unit_id),
            player,
            format!("Test Unit {}", unit_id),
            DatasheetId::new(1),
            KeywordSet::empty(),
            vec![model],
            MoveCharacteristic::from_inches(6),
            Toughness::new(4),
            ArmorSave::THREE_PLUS,
            None,                       // invulnerable_save
            Leadership::new(7),
            ObjectiveControl::new(oc),
        );
        // Must set to OnBattlefield since new() defaults to Undeployed
        unit.status = UnitStatus::OnBattlefield;

        state.units.push(unit);
    }

    // ─── Mission 1: Clash of Patrols Tests ────────────────────────────────

    #[test]
    fn test_m1_clash_of_patrols_15vp_cap() {
        let mut state = make_scoring_test_state(3);
        let p0 = PlayerId::new(0);

        // P0 controls all 4 objectives (would be 20VP without cap)
        let positions: Vec<_> = state.board.objectives.iter().map(|o| o.position).collect();
        for (i, pos) in positions.iter().enumerate() {
            add_controlling_unit(&mut state, p0, (i + 1) as u32, *pos, 2);
        }

        let results = score_clash_of_patrols(&state, 3, ScoringTiming::EndOfCommandPhase);
        assert_eq!(results.len(), 1);
        // Max 15VP cap applied
        assert_eq!(results[0].1.value(), 15);
    }

    #[test]
    fn test_m1_clash_of_patrols_normal_scoring() {
        let mut state = make_scoring_test_state(2);
        let p0 = PlayerId::new(0);

        // P0 controls 2 objectives = 10VP
        let pos_a = state.board.objectives[0].position;
        let pos_c = state.board.objectives[2].position;
        add_controlling_unit(&mut state, p0, 1, pos_a, 2);
        add_controlling_unit(&mut state, p0, 2, pos_c, 2);

        let results = score_clash_of_patrols(&state, 2, ScoringTiming::EndOfCommandPhase);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.value(), 10);
    }

    #[test]
    fn test_m1_clash_of_patrols_br1_no_scoring() {
        let state = make_scoring_test_state(1);
        let results = score_clash_of_patrols(&state, 1, ScoringTiming::EndOfCommandPhase);
        assert!(results.is_empty());
    }

    #[test]
    fn test_m1_clash_of_patrols_br5_split_timing() {
        let mut state = make_scoring_test_state(5);
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);
        state.players[0].first_turn = true;
        state.players[1].first_turn = false;

        // P0 controls obj A, P1 controls obj B
        let pos_a = state.board.objectives[0].position;
        let pos_b = state.board.objectives[1].position;
        add_controlling_unit(&mut state, p0, 1, pos_a, 2);
        add_controlling_unit(&mut state, p1, 2, pos_b, 2);

        // At end of command phase in BR5: only 1st player (P0) scores
        state.active_player = p0; // P0's command phase
        let results_cmd = score_clash_of_patrols(&state, 5, ScoringTiming::EndOfCommandPhase);
        assert_eq!(results_cmd.len(), 1);
        assert_eq!(results_cmd[0].0, p0);
        assert_eq!(results_cmd[0].1.value(), 5);

        // At end of turn in BR5: only 2nd player (P1) scores
        state.active_player = p1; // P1's turn ending
        let results_eot = score_clash_of_patrols(&state, 5, ScoringTiming::EndOfTurn);
        assert_eq!(results_eot.len(), 1);
        assert_eq!(results_eot[0].0, p1);
        assert_eq!(results_eot[0].1.value(), 5);
    }

    // ─── Mission 2: Archeotech Recovery Tests ─────────────────────────────

    #[test]
    fn test_m2_archeotech_recovery_15vp_cap() {
        let mut state = make_scoring_test_state(3);
        let p0 = PlayerId::new(0);

        // P0 controls all 4 objectives
        let positions: Vec<_> = state.board.objectives.iter().map(|o| o.position).collect();
        for (i, pos) in positions.iter().enumerate() {
            add_controlling_unit(&mut state, p0, (i + 1) as u32, *pos, 2);
        }

        let results = score_archeotech_recovery(&state, 3);
        assert_eq!(results.len(), 1);
        // Max 15VP cap applied
        assert_eq!(results[0].1.value(), 15);
    }

    #[test]
    fn test_m2_archeotech_endgame_nml_objective() {
        let mut state = make_scoring_test_state(5);
        let p0 = PlayerId::new(0);

        // Remove all DZ objectives, keep only NML objective C
        state.board.objectives.retain(|obj| obj.label == "C");
        let pos_c = state.board.objectives[0].position;
        add_controlling_unit(&mut state, p0, 1, pos_c, 2);

        let results = score_archeotech_endgame(&state);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, p0);
        assert_eq!(results[0].1.value(), 10);
    }

    #[test]
    fn test_m2_archeotech_endgame_no_nml_objectives() {
        let mut state = make_scoring_test_state(5);

        // Remove all NML objectives (only DZ objectives remain)
        state.board.objectives.retain(|obj| !is_objective_in_no_mans_land(obj));

        let results = score_archeotech_endgame(&state);
        assert!(results.is_empty());
    }

    // ─── Mission 3: Forward Outpost Tests ─────────────────────────────────

    #[test]
    fn test_m3_forward_outpost_nml_only_scoring() {
        let mut state = make_scoring_test_state(2);
        let p0 = PlayerId::new(0);

        // P0 controls own DZ objective A (should NOT count for VP)
        let pos_a = state.board.objectives[0].position;
        add_controlling_unit(&mut state, p0, 1, pos_a, 2);

        let results = score_forward_outpost(&state, 2, ScoringTiming::EndOfCommandPhase);
        // Own DZ objective gives 0VP in Forward Outpost
        assert!(results.is_empty());
    }

    #[test]
    fn test_m3_forward_outpost_nml_plus_enemy_dz() {
        let mut state = make_scoring_test_state(3);
        let p0 = PlayerId::new(0);

        // P0 controls NML objective C (5VP) + enemy DZ objective B (10VP) = 15VP
        let pos_c = state.board.objectives[2].position;
        let pos_b = state.board.objectives[1].position;
        add_controlling_unit(&mut state, p0, 1, pos_c, 2);
        add_controlling_unit(&mut state, p0, 2, pos_b, 2);

        let results = score_forward_outpost(&state, 3, ScoringTiming::EndOfCommandPhase);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, p0);
        // 5VP (1 NML) + 10VP (enemy DZ) = 15VP (at cap)
        assert_eq!(results[0].1.value(), 15);
    }

    #[test]
    fn test_m3_forward_outpost_15vp_cap() {
        let mut state = make_scoring_test_state(3);
        let p0 = PlayerId::new(0);

        // P0 controls 2 NML objectives (10VP) + enemy DZ (10VP) = 20VP before cap
        let pos_c = state.board.objectives[2].position;
        let pos_d = state.board.objectives[3].position;
        let pos_b = state.board.objectives[1].position;
        add_controlling_unit(&mut state, p0, 1, pos_c, 2);
        add_controlling_unit(&mut state, p0, 2, pos_d, 2);
        add_controlling_unit(&mut state, p0, 3, pos_b, 2);

        let results = score_forward_outpost(&state, 3, ScoringTiming::EndOfCommandPhase);
        assert_eq!(results.len(), 1);
        // Capped at 15VP
        assert_eq!(results[0].1.value(), 15);
    }

    #[test]
    fn test_m3_forward_outpost_br5_split() {
        let mut state = make_scoring_test_state(5);
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);

        // P0 controls NML C, P1 controls NML D
        let pos_c = state.board.objectives[2].position;
        let pos_d = state.board.objectives[3].position;
        add_controlling_unit(&mut state, p0, 1, pos_c, 2);
        add_controlling_unit(&mut state, p1, 2, pos_d, 2);

        // BR5 end of command phase: only P0 (1st player) scores
        state.active_player = p0;
        let results_cmd = score_forward_outpost(&state, 5, ScoringTiming::EndOfCommandPhase);
        assert_eq!(results_cmd.len(), 1);
        assert_eq!(results_cmd[0].0, p0);

        // BR5 end of turn: only P1 (2nd player) scores
        state.active_player = p1;
        let results_eot = score_forward_outpost(&state, 5, ScoringTiming::EndOfTurn);
        assert_eq!(results_eot.len(), 1);
        assert_eq!(results_eot[0].0, p1);
    }

    // ─── Mission 4: Scorched Earth Tests ──────────────────────────────────

    #[test]
    fn test_m4_scorched_earth_threshold_control_1plus() {
        let mut state = make_scoring_test_state(2);
        let p0 = PlayerId::new(0);

        // P0 controls 1 objective, P1 controls 0
        // → 5VP (control 1+) + 5VP (control more than opponent) = 10VP
        let pos_a = state.board.objectives[0].position;
        add_controlling_unit(&mut state, p0, 1, pos_a, 2);

        let results = score_scorched_earth(&state, 2, ScoringTiming::EndOfCommandPhase);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, p0);
        assert_eq!(results[0].1.value(), 10);
    }

    #[test]
    fn test_m4_scorched_earth_threshold_control_more() {
        let mut state = make_scoring_test_state(3);
        let p0 = PlayerId::new(0);

        // P0 controls 2 objectives, P1 controls 1
        // Only active player (P0) scores at end of their Command Phase
        let pos_a = state.board.objectives[0].position;
        let pos_c = state.board.objectives[2].position;
        add_controlling_unit(&mut state, p0, 1, pos_a, 2);
        add_controlling_unit(&mut state, p0, 2, pos_c, 2);

        state.active_player = p0;
        let results = score_scorched_earth(&state, 3, ScoringTiming::EndOfCommandPhase);
        // P0: 5VP (control 1+) + 5VP (control more than opponent) = 10VP
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, p0);
        assert_eq!(results[0].1.value(), 10);
    }

    #[test]
    fn test_m4_scorched_earth_threshold_with_raze() {
        let mut state = make_scoring_test_state(3);
        let p0 = PlayerId::new(0);

        // P0 controls 1 objective (P1 controls 0) AND razed one this turn
        // → 5VP (control 1+) + 5VP (control more) + 10VP (razed) = 20VP
        let pos_a = state.board.objectives[0].position;
        add_controlling_unit(&mut state, p0, 1, pos_a, 2);
        state.players[0].mission_progress.razed_this_turn = true;

        let results = score_scorched_earth(&state, 3, ScoringTiming::EndOfCommandPhase);
        let p0_result = results.iter().find(|(p, _, _)| *p == p0).unwrap();
        // 5VP (control 1+) + 5VP (control more) + 10VP (razed) = 20VP
        assert_eq!(p0_result.1.value(), 20);
    }

    #[test]
    fn test_m4_scorched_earth_max_score() {
        let mut state = make_scoring_test_state(2);
        let p0 = PlayerId::new(0);

        // P0 controls 2 objectives (more than P1's 0), razed one
        let pos_a = state.board.objectives[0].position;
        let pos_c = state.board.objectives[2].position;
        add_controlling_unit(&mut state, p0, 1, pos_a, 2);
        add_controlling_unit(&mut state, p0, 2, pos_c, 2);
        state.players[0].mission_progress.razed_this_turn = true;

        let results = score_scorched_earth(&state, 2, ScoringTiming::EndOfCommandPhase);
        let p0_result = results.iter().find(|(p, _, _)| *p == p0).unwrap();
        // 5VP (1+) + 5VP (more) + 10VP (razed) = 20VP
        assert_eq!(p0_result.1.value(), 20);
    }

    #[test]
    fn test_m4_scorched_earth_br5_split() {
        let mut state = make_scoring_test_state(5);
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);

        let pos_a = state.board.objectives[0].position;
        let pos_b = state.board.objectives[1].position;
        add_controlling_unit(&mut state, p0, 1, pos_a, 2);
        add_controlling_unit(&mut state, p1, 2, pos_b, 2);

        // BR5 end of command phase: only P0 (1st player)
        state.active_player = p0;
        let results_cmd = score_scorched_earth(&state, 5, ScoringTiming::EndOfCommandPhase);
        assert_eq!(results_cmd.len(), 1);
        assert_eq!(results_cmd[0].0, p0);

        // BR5 end of turn: only P1 (2nd player)
        state.active_player = p1;
        let results_eot = score_scorched_earth(&state, 5, ScoringTiming::EndOfTurn);
        assert_eq!(results_eot.len(), 1);
        assert_eq!(results_eot[0].0, p1);
    }

    // ─── Mission 4: Raze Validation Tests ─────────────────────────────────

    #[test]
    fn test_m4_validate_raze_br1_denied() {
        let state = make_scoring_test_state(1);
        let result = validate_raze_objective(&state, PlayerId::new(0), ObjectiveId::new(1));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("battle round 2"));
    }

    #[test]
    fn test_m4_validate_raze_attacker_cannot_raze_a() {
        let mut state = make_scoring_test_state(2);
        let p0 = PlayerId::new(0);
        let pos_a = state.board.objectives[0].position;
        add_controlling_unit(&mut state, p0, 1, pos_a, 2);

        let result = validate_raze_objective(&state, p0, ObjectiveId::new(1));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Attacker cannot raze objective marker A"));
    }

    #[test]
    fn test_m4_validate_raze_defender_cannot_raze_b() {
        let mut state = make_scoring_test_state(2);
        let p1 = PlayerId::new(1);
        let pos_b = state.board.objectives[1].position;
        add_controlling_unit(&mut state, p1, 1, pos_b, 2);

        let result = validate_raze_objective(&state, p1, ObjectiveId::new(2));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Defender cannot raze objective marker B"));
    }

    #[test]
    fn test_m4_validate_raze_must_control() {
        let mut state = make_scoring_test_state(2);
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);

        // P1 controls objective C, P0 tries to raze it
        let pos_c = state.board.objectives[2].position;
        add_controlling_unit(&mut state, p1, 1, pos_c, 2);

        let result = validate_raze_objective(&state, p0, ObjectiveId::new(3));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("do not control"));
    }

    #[test]
    fn test_m4_validate_raze_too_few_objectives() {
        let mut state = make_scoring_test_state(2);
        let p0 = PlayerId::new(0);

        // Remove all but 1 objective
        state.board.objectives.retain(|obj| obj.label == "C");
        let pos_c = state.board.objectives[0].position;
        add_controlling_unit(&mut state, p0, 1, pos_c, 2);

        let result = validate_raze_objective(&state, p0, ObjectiveId::new(3));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("fewer than 2"));
    }

    #[test]
    fn test_m4_validate_raze_enemy_nearby() {
        let mut state = make_scoring_test_state(2);
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);

        // P0 controls objective C
        let pos_c = state.board.objectives[2].position;
        add_controlling_unit(&mut state, p0, 1, pos_c, 2);

        // P1 has a unit right on top of objective C (within 3")
        add_controlling_unit(&mut state, p1, 2, pos_c, 0);

        let result = validate_raze_objective(&state, p0, ObjectiveId::new(3));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("enemy units within 3"));
    }

    #[test]
    fn test_m4_validate_raze_legal() {
        let mut state = make_scoring_test_state(2);
        let p0 = PlayerId::new(0);

        // P0 controls NML objective C, no enemies nearby, 4 objectives exist
        let pos_c = state.board.objectives[2].position;
        add_controlling_unit(&mut state, p0, 1, pos_c, 2);

        let result = validate_raze_objective(&state, p0, ObjectiveId::new(3));
        assert!(result.is_ok());
    }

    // ─── Mission 3: Sabotage Enemy Comms Test ─────────────────────────────

    #[test]
    fn test_m3_sabotage_enemy_comms() {
        let mut state = make_scoring_test_state(3);
        let p0 = PlayerId::new(0);

        // P0 controls enemy DZ objective B
        let pos_b = state.board.objectives[1].position;
        add_controlling_unit(&mut state, p0, 1, pos_b, 2);

        assert!(!state.players[1].mission_progress.command_reroll_blocked);

        let events = evaluate_sabotage_enemy_comms(&mut state, p0);
        assert!(!events.is_empty());
        assert!(state.players[1].mission_progress.command_reroll_blocked);
    }

    #[test]
    fn test_m3_sabotage_enemy_comms_not_triggered() {
        let mut state = make_scoring_test_state(3);
        let p0 = PlayerId::new(0);

        // P0 controls own DZ objective A (not enemy DZ)
        let pos_a = state.board.objectives[0].position;
        add_controlling_unit(&mut state, p0, 1, pos_a, 2);

        let events = evaluate_sabotage_enemy_comms(&mut state, p0);
        assert!(events.is_empty());
        assert!(!state.players[1].mission_progress.command_reroll_blocked);
    }

    // ─── Mission 6: Display of Might BR5 Split Test ───────────────────────

    #[test]
    fn test_m6_display_of_might_br5_split() {
        let mut state = make_scoring_test_state(5);
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);

        // Both players control 1 objective each
        let pos_a = state.board.objectives[0].position;
        let pos_b = state.board.objectives[1].position;
        add_controlling_unit(&mut state, p0, 1, pos_a, 2);
        add_controlling_unit(&mut state, p1, 2, pos_b, 2);

        // BR5 end of command phase: only P0 (1st player) scores
        state.active_player = p0;
        let results_cmd = score_display_of_might(&state, 5, ScoringTiming::EndOfCommandPhase);
        assert_eq!(results_cmd.len(), 1);
        assert_eq!(results_cmd[0].0, p0);
        assert_eq!(results_cmd[0].1.value(), 5); // control 1+ = 5VP

        // BR5 end of turn: only P1 (2nd player) scores
        state.active_player = p1;
        let results_eot = score_display_of_might(&state, 5, ScoringTiming::EndOfTurn);
        assert_eq!(results_eot.len(), 1);
        assert_eq!(results_eot[0].0, p1);
        assert_eq!(results_eot[0].1.value(), 5); // control 1+ = 5VP
    }

    // ─── ScoringTiming and players_to_score Tests ─────────────────────────

    #[test]
    fn test_players_to_score_br2_4_active_player_only() {
        // BR2-4: only the active player scores at end of their Command Phase
        let state = make_scoring_test_state(3);
        // active_player is P0 in test state
        let players = players_to_score(&state, 3, ScoringTiming::EndOfCommandPhase, true);
        assert_eq!(players.len(), 1);
        assert_eq!(players[0], PlayerId::new(0));
    }

    #[test]
    fn test_players_to_score_br5_no_split() {
        let state = make_scoring_test_state(5);
        // Without BR5 split, active player scores at end of command phase
        let players = players_to_score(&state, 5, ScoringTiming::EndOfCommandPhase, false);
        assert_eq!(players.len(), 1);
        assert_eq!(players[0], PlayerId::new(0));
    }

    #[test]
    fn test_players_to_score_br5_split_cmd_phase() {
        let mut state = make_scoring_test_state(5);
        state.players[0].first_turn = true;
        state.players[1].first_turn = false;
        state.active_player = PlayerId::new(0); // 1st player's turn

        // At end of command phase in BR5 with split: only 1st player
        let players = players_to_score(&state, 5, ScoringTiming::EndOfCommandPhase, true);
        assert_eq!(players.len(), 1);
        assert_eq!(players[0], PlayerId::new(0));
    }

    #[test]
    fn test_players_to_score_br5_split_end_of_turn() {
        let mut state = make_scoring_test_state(5);
        state.players[0].first_turn = true;
        state.players[1].first_turn = false;
        state.active_player = PlayerId::new(1); // 2nd player's turn ending

        // At end of turn in BR5 with split: only 2nd player
        let players = players_to_score(&state, 5, ScoringTiming::EndOfTurn, true);
        assert_eq!(players.len(), 1);
        assert_eq!(players[0], PlayerId::new(1));
    }

    // ─── score_primary_objectives Dispatch Test ───────────────────────────

    #[test]
    fn test_score_primary_objectives_dispatch_m1() {
        let mut state = make_scoring_test_state(2);
        let p0 = PlayerId::new(0);
        let pos_a = state.board.objectives[0].position;
        add_controlling_unit(&mut state, p0, 1, pos_a, 2);

        let results = score_primary_objectives(
            &state,
            Some(mission_ids::CLASH_OF_PATROLS),
            ScoringTiming::EndOfCommandPhase,
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, p0);
        assert_eq!(results[0].1.value(), 5);
    }

    #[test]
    fn test_score_primary_objectives_dispatch_m4() {
        let mut state = make_scoring_test_state(2);
        let p0 = PlayerId::new(0);
        let pos_a = state.board.objectives[0].position;
        add_controlling_unit(&mut state, p0, 1, pos_a, 2);

        let results = score_primary_objectives(
            &state,
            Some(mission_ids::SCORCHED_EARTH),
            ScoringTiming::EndOfCommandPhase,
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, p0);
        // Threshold: control 1+ = 5VP, control more (1 > 0) = 5VP = 10VP
        assert_eq!(results[0].1.value(), 10);
    }
}

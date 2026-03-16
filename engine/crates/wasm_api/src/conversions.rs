//! Conversion functions from internal engine types to JSON-friendly view models.
//!
//! Each function projects engine-internal types into the flat, string-friendly
//! view model structs defined in `view_models.rs`, suitable for crossing the
//! WASM boundary as JSON strings.

use crate::view_models::*;

use wh40k_core_types::{
    AllocationStatus, ArmorPenetration, ArmorSave, AttackCount, Damage, EngagementStatus,
    FeelNoPain, GameMode, GameOutcome, HatchwayState, InvulnerableSave, Phase, PlayerId,
    SubPhase, UnitStatus, WeaponAbility, WeaponProfile, WeaponType,
};
use wh40k_command_system::Command;
use wh40k_event_system::EventLogEntry;
use wh40k_game_core::{
    GameState, ModelState, PlayerState, TurnFlags, UnitState, WargearAbilityState,
};
use wh40k_geometry::{Board, ObjectiveControlStatus, ObjectiveMarker, TerrainPiece};
use wh40k_replay::{ReplayFile, ReplayPlayer, ReplayPlayerInfo};
use wh40k_search_abstraction::{MacroAction, TacticalIntent};
use wh40k_search_core::SearchResult;

// ===========================================================================
// Top-level game state conversion
// ===========================================================================

/// Convert a full GameState into a GameView projection for the UI.
pub fn game_state_to_view(state: &GameState) -> GameView {
    let players = [
        player_to_view(&state.players[0]),
        player_to_view(&state.players[1]),
    ];

    let units: Vec<UnitView> = state
        .units
        .iter()
        .map(|u| unit_to_view(u, &state.turn_flags))
        .collect();

    let ba_hatchway_states = state.boarding_state().map(|ba| &ba.hatchway_states);
    let board = board_to_view_with_deployment(&state.board, state.deployment_config.as_ref(), ba_hatchway_states);

    // Take the last 50 events from the event bus log.
    let log = state.event_bus.log();
    let start = if log.len() > 50 { log.len() - 50 } else { 0 };
    let events: Vec<EventView> = log[start..].iter().map(event_to_view).collect();

    let outcome = game_outcome_to_view(&state.game_outcome, &state.players);

    let in_progress = matches!(state.game_outcome, GameOutcome::InProgress);

    // Game mode string
    let game_mode = match state.game_mode {
        GameMode::CombatPatrol => "CombatPatrol".to_string(),
        GameMode::BoardingActions => "BoardingActions".to_string(),
    };

    // Boarding Actions specific view data
    let (hatchway_states, secured_objectives) = if let Some(ba) = state.boarding_state() {
        let hatches: Vec<HatchwayStateView> = ba.hatchway_states.iter().map(|(id, st)| {
            HatchwayStateView {
                id: id.0,
                state: match st {
                    HatchwayState::Open => "Open".to_string(),
                    HatchwayState::Closed => "Closed".to_string(),
                    HatchwayState::Locked => "Locked".to_string(),
                    HatchwayState::OneWayOpened => "OneWayOpened".to_string(),
                },
            }
        }).collect();

        let secured: Vec<SecuredObjectiveView> = ba.secured_objectives.iter().map(|(obj_id, player_id)| {
            SecuredObjectiveView {
                objective_id: obj_id.0,
                player: player_id.0,
            }
        }).collect();

        (Some(hatches), Some(secured))
    } else {
        (None, None)
    };

    GameView {
        battle_round: state.battle_round.number(),
        phase: phase_to_string(&state.current_phase),
        subphase: subphase_to_string(&state.current_subphase),
        active_player: state.active_player.0,
        decision_owner: state.decision_owner.0,
        players,
        units,
        board,
        events,
        outcome,
        in_progress,
        content_version: state.content_version.clone(),
        scenario_id: state.scenario_id.map(|m| m.0),
        game_mode,
        hatchway_states,
        secured_objectives,
    }
}

// ===========================================================================
// Player conversion
// ===========================================================================

/// Convert a PlayerState into a PlayerView.
pub fn player_to_view(player: &PlayerState) -> PlayerView {
    PlayerView {
        id: player.id.0,
        name: player.name.clone(),
        faction_id: player.faction_id.map(|f| f.0),
        cp: player.cp.value(),
        vp: player.vp.value(),
        primary_vp: player.mission_progress.primary_vp.value(),
        secondary_vp: player.mission_progress.secondary_vp.value(),
        enhancement_choice: player.enhancement_choice.map(|e| e.0),
        secondary_choice: player.secondary_choice.map(|s| s.0),
        patrol_squad_choice: player.patrol_squad_choice.map(|s| s as u32),
        first_turn: player.first_turn,
        active_blessings: player.faction_round_flags.active_blessings.clone(),
        blessing_dice: player.faction_round_flags.blessing_dice.clone(),
    }
}

// ===========================================================================
// Unit conversion
// ===========================================================================

/// Convert a UnitState into a UnitView, including turn flag status.
pub fn unit_to_view(unit: &UnitState, turn_flags: &TurnFlags) -> UnitView {
    let keywords: Vec<String> = unit.keywords.bit_indices().map(keyword_bit_to_name).map(String::from).collect();

    let models: Vec<ModelView> = unit.models.iter().map(model_to_view).collect();

    let armor_save = format_armor_save(&unit.base_armor_save);

    let invulnerable_save = unit.invulnerable_save.as_ref().map(format_invuln);

    let status_str = match unit.status {
        UnitStatus::OnBattlefield => "OnBattlefield".to_string(),
        UnitStatus::InStrategicReserves => "InStrategicReserves".to_string(),
        UnitStatus::Destroyed => "Destroyed".to_string(),
        UnitStatus::Undeployed => "Undeployed".to_string(),
    };

    let engagement_str = match unit.engagement_status {
        EngagementStatus::NotEngaged => "NotEngaged".to_string(),
        EngagementStatus::Engaged => "Engaged".to_string(),
    };

    let reserve_type_str = unit.reserve_type.map(|rt| match rt {
        wh40k_game_core::ReserveType::StrategicReserves => "StrategicReserves".to_string(),
        wh40k_game_core::ReserveType::DeepStrike => "DeepStrike".to_string(),
    });

    let position = unit.reference_position().map(|p| PositionView {
        x: p.x.as_f64(),
        y: p.y.as_f64(),
    });

    let wargear_abilities: Vec<WargearAbilityView> = unit
        .wargear_abilities
        .iter()
        .map(wargear_ability_to_view)
        .collect();

    let unit_id = unit.id;
    let turn_flags_view = UnitTurnFlagsView {
        has_moved: turn_flags.has_moved(unit_id),
        has_shot: turn_flags.has_shot(unit_id),
        has_charged: turn_flags.has_charged(unit_id),
        has_fought: turn_flags.has_fought(unit_id),
        has_advanced: turn_flags.has_advanced(unit_id),
        has_fell_back: turn_flags.has_fell_back(unit_id),
        is_stationary: turn_flags.is_stationary(unit_id),
        charged_this_turn: turn_flags.charged_this_turn(unit_id),
    };

    UnitView {
        id: unit.id.0,
        owner: unit.owner.0,
        name: unit.name.clone(),
        datasheet_id: unit.datasheet_id.0,
        keywords,
        models,
        movement: unit.base_movement.distance().as_f64(),
        toughness: unit.base_toughness.0,
        armor_save,
        invulnerable_save,
        leadership: unit.base_leadership.0,
        oc: unit.base_oc.0,
        status: status_str,
        battle_shocked: unit.battle_shocked,
        below_half_strength: unit.below_half_strength,
        engagement_status: engagement_str,
        attached_leader: unit.attached_leader.map(|u| u.0),
        bodyguard_for: unit.bodyguard_for.map(|u| u.0),
        reserve_type: reserve_type_str,
        models_alive: unit.models_alive(),
        starting_model_count: unit.starting_model_count(),
        total_wounds_remaining: unit.total_wounds_remaining(),
        wargear_abilities,
        is_character: unit.is_character(),
        is_infantry: unit.is_infantry(),
        position,
        turn_flags: turn_flags_view,
    }
}

/// Convert a WargearAbilityState into a WargearAbilityView.
fn wargear_ability_to_view(ability: &WargearAbilityState) -> WargearAbilityView {
    WargearAbilityView {
        name: ability.name.clone(),
        description: ability.description.clone(),
        active: ability.active,
    }
}

// ===========================================================================
// Model conversion
// ===========================================================================

/// Convert a ModelState into a ModelView.
pub fn model_to_view(model: &ModelState) -> ModelView {
    let ranged_weapons: Vec<WeaponView> = model.ranged_weapons.iter().map(weapon_to_view).collect();
    let melee_weapons: Vec<WeaponView> = model.melee_weapons.iter().map(weapon_to_view).collect();

    let feel_no_pain = model.feel_no_pain.as_ref().map(format_fnp);

    let allocation_status_str = match model.allocation_status {
        AllocationStatus::Unwounded => "Unwounded".to_string(),
        AllocationStatus::WoundedAllocated => "WoundedAllocated".to_string(),
    };

    ModelView {
        id: model.id.0,
        unit_id: model.unit_id.0,
        alive: model.alive,
        wounds_max: model.wounds_max.0,
        wounds_remaining: model.wounds_remaining.0,
        position: PositionView {
            x: model.position.x.as_f64(),
            y: model.position.y.as_f64(),
        },
        base_size_mm: model.base_size.0,
        ranged_weapons,
        melee_weapons,
        is_leader: model.is_leader,
        feel_no_pain,
        allocation_status: allocation_status_str,
    }
}

// ===========================================================================
// Weapon conversion
// ===========================================================================

/// Convert a WeaponProfile into a WeaponView.
pub fn weapon_to_view(weapon: &WeaponProfile) -> WeaponView {
    let weapon_type_str = match weapon.weapon_type {
        WeaponType::Ranged => "Ranged".to_string(),
        WeaponType::Melee => "Melee".to_string(),
    };

    let abilities: Vec<String> = weapon
        .abilities
        .abilities
        .iter()
        .map(weapon_ability_to_string)
        .collect();

    // For Torrent weapons, the skill display is "N/A"
    let skill_str = if weapon.abilities.has_torrent() {
        "N/A".to_string()
    } else {
        format!("{}+", weapon.skill.value())
    };

    WeaponView {
        id: weapon.id.0,
        name: weapon.name.clone(),
        weapon_type: weapon_type_str,
        range: weapon.range.as_f64(),
        attacks: format_attacks(&weapon.attacks),
        skill: skill_str,
        strength: weapon.strength.0,
        ap: format_ap(&weapon.ap),
        damage: format_damage(&weapon.damage),
        abilities,
    }
}

// ===========================================================================
// Board / Terrain / Objective conversion
// ===========================================================================

/// Convert a Board into a BoardView.
pub fn board_to_view_with_deployment(
    board: &Board,
    deployment_config: Option<&wh40k_geometry::DeploymentConfig>,
    ba_hatchway_states: Option<&std::collections::HashMap<wh40k_core_types::HatchwayId, wh40k_core_types::HatchwayState>>,
) -> BoardView {
    let terrain: Vec<TerrainView> = board
        .terrain
        .iter()
        .enumerate()
        .map(|(i, t)| terrain_to_view(i, t))
        .collect();

    let mut objectives: Vec<ObjectiveView> = board.objectives.iter().map(objective_to_view).collect();

    // Populate deployment zones from the deployment config
    let deployment_zones: Vec<DeploymentZoneView> = match deployment_config {
        Some(config) => {
            vec![
                deployment_zone_to_view(&config.attacker_zone),
                deployment_zone_to_view(&config.defender_zone),
            ]
        }
        None => Vec::new(),
    };

    // Boarding Actions map geometry
    let (walls, compartments, hatchways, entry_zones) = if let Some(bmap) = &board.boarding_map {
        let walls_view: Vec<WallView> = bmap.walls.iter().map(|w| WallView {
            id: w.id,
            start: PositionView { x: w.start.x.as_f64(), y: w.start.y.as_f64() },
            end: PositionView { x: w.end.x.as_f64(), y: w.end.y.as_f64() },
        }).collect();

        let comps_view: Vec<CompartmentView> = bmap.compartments.iter().map(|c| CompartmentView {
            id: c.id.raw(),
            name: c.name.clone(),
            vertices: c.boundary.vertices.iter().map(|v| PositionView {
                x: v.x.as_f64(),
                y: v.y.as_f64(),
            }).collect(),
        }).collect();

        let hatches_view: Vec<HatchwayView> = bmap.hatchways.iter().map(|h| {
            // Get current state from BA mode state, falling back to initial state
            let current_state = ba_hatchway_states
                .and_then(|states| states.get(&h.id))
                .copied()
                .unwrap_or(h.initial_state);
            let state_str = match current_state {
                wh40k_core_types::HatchwayState::Open => "Open",
                wh40k_core_types::HatchwayState::Closed => "Closed",
                wh40k_core_types::HatchwayState::Locked => "Locked",
                wh40k_core_types::HatchwayState::OneWayOpened => "OneWayOpened",
            };
            let orient_str = match h.orientation {
                wh40k_core_types::HatchwayOrientation::Horizontal => "Horizontal",
                wh40k_core_types::HatchwayOrientation::Vertical => "Vertical",
            };
            HatchwayView {
                id: h.id.raw(),
                position: PositionView { x: h.position.x.as_f64(), y: h.position.y.as_f64() },
                orientation: orient_str.to_string(),
                width: h.width.as_f64(),
                state: state_str.to_string(),
            }
        }).collect();

        let zones_view: Vec<EntryZoneView> = bmap.entry_zones.iter().map(|ez| {
            let role_str = match ez.role {
                wh40k_core_types::EntryZoneRole::Main => "Main",
                wh40k_core_types::EntryZoneRole::Underdog => "Underdog",
                wh40k_core_types::EntryZoneRole::Patrol => "Patrol",
                wh40k_core_types::EntryZoneRole::Guard => "Guard",
                wh40k_core_types::EntryZoneRole::Backup => "Backup",
            };
            EntryZoneView {
                id: ez.id,
                name: ez.name.clone(),
                role: role_str.to_string(),
                vertices: ez.boundary.vertices.iter().map(|v| PositionView {
                    x: v.x.as_f64(),
                    y: v.y.as_f64(),
                }).collect(),
                player: ez.player_assignment.map(|p| p.raw()),
            }
        }).collect();

        // Also add boarding objectives to the objectives list
        for bobj in &bmap.objectives {
            objectives.push(ObjectiveView {
                id: bobj.id.raw(),
                position: PositionView { x: bobj.position.x.as_f64(), y: bobj.position.y.as_f64() },
                label: bobj.label.clone(),
                control_status: "Uncontrolled".to_string(),
                controlling_player: None,
            });
        }

        (Some(walls_view), Some(comps_view), Some(hatches_view), Some(zones_view))
    } else {
        (None, None, None, None)
    };

    BoardView {
        width: board.dimensions.width.as_f64(),
        height: board.dimensions.height.as_f64(),
        terrain,
        objectives,
        deployment_zones,
        walls,
        compartments,
        hatchways,
        entry_zones,
    }
}

/// Convert a DeploymentZone into a DeploymentZoneView.
fn deployment_zone_to_view(zone: &wh40k_geometry::DeploymentZone) -> DeploymentZoneView {
    let vertices: Vec<PositionView> = zone.polygon.vertices.iter()
        .map(|v| PositionView { x: v.x.as_f64(), y: v.y.as_f64() })
        .collect();

    let map_type_str = match zone.map_type {
        wh40k_geometry::DeploymentMapType::Standard => "Standard".to_string(),
        wh40k_geometry::DeploymentMapType::SearchAndDestroy => "SearchAndDestroy".to_string(),
    };

    DeploymentZoneView {
        player: zone.player.0,
        vertices,
        map_type: map_type_str,
    }
}

/// Convert a TerrainPiece into a TerrainView.
/// TerrainPiece has no id field, so we use the index in the terrain list.
pub fn terrain_to_view(index: usize, terrain: &TerrainPiece) -> TerrainView {
    TerrainView {
        id: index as u32,
        name: terrain.name.clone(),
        rect: RectView {
            min_x: terrain.footprint.min_x.as_f64(),
            min_y: terrain.footprint.min_y.as_f64(),
            max_x: terrain.footprint.max_x.as_f64(),
            max_y: terrain.footprint.max_y.as_f64(),
        },
        provides_cover: terrain.properties.provides_cover,
        blocks_los: terrain.properties.blocks_los,
        impassable: terrain.properties.impassable,
        height: terrain.properties.height.as_f64(),
    }
}

/// Convert an ObjectiveMarker into an ObjectiveView.
pub fn objective_to_view(obj: &ObjectiveMarker) -> ObjectiveView {
    let (control_status_str, controlling_player) = match obj.control_status {
        ObjectiveControlStatus::Uncontrolled => ("Uncontrolled".to_string(), None),
        ObjectiveControlStatus::ControlledBy(pid) => {
            (format!("ControlledBy({})", pid.0), Some(pid.0))
        }
        ObjectiveControlStatus::Contested => ("Contested".to_string(), None),
    };

    ObjectiveView {
        id: obj.id.0,
        position: PositionView {
            x: obj.position.x.as_f64(),
            y: obj.position.y.as_f64(),
        },
        label: obj.label.clone(),
        control_status: control_status_str,
        controlling_player,
    }
}

// ===========================================================================
// Event conversion
// ===========================================================================

/// Convert an EventLogEntry into an EventView.
pub fn event_to_view(entry: &EventLogEntry) -> EventView {
    let event_type = entry.event.kind_name().to_string();
    let description = format!("{}", entry.event);
    let player = entry.event.primary_player().map(|p| p.0);

    // Collect involved unit IDs from the event's primary_unit.
    let mut unit_ids = Vec::new();
    if let Some(uid) = entry.event.primary_unit() {
        unit_ids.push(uid.0);
    }

    EventView {
        event_type,
        description,
        player,
        unit_ids,
        round: entry.round.number(),
        phase: phase_to_string(&entry.phase),
    }
}

// ===========================================================================
// Decision surface conversion
// ===========================================================================

/// Convert a list of MacroActions (from ActionGenerator) into a DecisionSurfaceView.
pub fn macro_actions_to_decision_surface_view(
    actions: &[MacroAction],
    owner: PlayerId,
    phase: &Phase,
    subphase: &SubPhase,
) -> DecisionSurfaceView {
    let decision_type = decision_type_from_phase_subphase(phase, subphase);

    let action_views: Vec<ActionView> = actions
        .iter()
        .enumerate()
        .map(|(i, ma)| macro_action_to_action_view(i, ma))
        .collect();

    let num_options = action_views.len();
    let is_forced = num_options == 1;
    // Mandatory unless it's a stratagem window or overwatch decision
    let is_mandatory = !matches!(
        subphase,
        SubPhase::StratagemWindow | SubPhase::ReactionWindow
    );

    DecisionSurfaceView {
        decision_type: decision_type.to_string(),
        owner: owner.0,
        actions: action_views,
        is_mandatory,
        num_options,
        is_forced,
    }
}

/// Convert a MacroAction into an ActionView.
fn macro_action_to_action_view(index: usize, ma: &MacroAction) -> ActionView {
    let label = ma.label.clone();

    // Derive command_type from the first command if available
    let command_type = if let Some(first_cmd) = ma.commands.first() {
        first_cmd.category().to_string()
    } else {
        "Unknown".to_string()
    };

    let player = ma
        .commands
        .first()
        .and_then(|cmd| cmd.player())
        .map(|p| p.0);

    let unit_id = ma.actor_units.first().map(|u| u.0);

    // Try to extract target_id from the first command
    let target_id = ma.commands.first().and_then(|cmd| match cmd {
        Command::DeclareShootingTargets { targets, .. } => targets.first().map(|t| t.1 .0),
        Command::DeclareCharge { targets, .. } => targets.first().map(|t| t.0),
        Command::DeclareMeleeTargets { targets, .. } => targets.first().map(|t| t.1 .0),
        Command::ResolveShootingAttack { target_id, .. } => Some(target_id.0),
        Command::ResolveMeleeAttack { target_id, .. } => Some(target_id.0),
        _ => None,
    });

    // Try to extract position from the first command
    let position = ma.commands.first().and_then(|cmd| match cmd {
        Command::NormalMove { destination, .. } => Some(PositionView {
            x: destination.x.as_f64(),
            y: destination.y.as_f64(),
        }),
        Command::AdvanceMove { destination, .. } => Some(PositionView {
            x: destination.x.as_f64(),
            y: destination.y.as_f64(),
        }),
        Command::FallBack { destination, .. } => Some(PositionView {
            x: destination.x.as_f64(),
            y: destination.y.as_f64(),
        }),
        Command::PlaceUnit { position, .. } => Some(PositionView {
            x: position.x.as_f64(),
            y: position.y.as_f64(),
        }),
        Command::ArriveFromReserves { position, .. } => Some(PositionView {
            x: position.x.as_f64(),
            y: position.y.as_f64(),
        }),
        Command::MakeChargeMove { destination, .. } => Some(PositionView {
            x: destination.x.as_f64(),
            y: destination.y.as_f64(),
        }),
        Command::HeroicInterventionMove { destination, .. } => Some(PositionView {
            x: destination.x.as_f64(),
            y: destination.y.as_f64(),
        }),
        _ => None,
    });

    ActionView {
        index,
        label,
        command_type,
        unit_id,
        target_id,
        position,
        player,
    }
}

// ===========================================================================
// AI result conversion
// ===========================================================================

/// Convert a SearchResult into an AiResultView.
pub fn search_result_to_view(result: &SearchResult) -> AiResultView {
    let best_action_label = result.best_action.label.clone();
    let best_action_commands: Vec<String> = result
        .best_action
        .commands
        .iter()
        .map(|cmd| format!("{}", cmd))
        .collect();

    let pv: Vec<String> = result.pv.iter().map(|ma| ma.label.clone()).collect();

    let candidates: Vec<AiCandidateView> = result
        .candidate_scores
        .iter()
        .map(|(ma, score)| AiCandidateView {
            label: ma.label.clone(),
            score: *score,
            intent: tactical_intent_to_string(&ma.intent),
        })
        .collect();

    AiResultView {
        best_action_label,
        best_action_commands,
        score: result.score,
        nodes_evaluated: result.stats.nodes_evaluated,
        max_depth: result.stats.max_depth_reached,
        time_ms: result.stats.time_elapsed_ms,
        pv,
        candidates,
        intent: tactical_intent_to_string(&result.best_action.intent),
    }
}

// ===========================================================================
// Game outcome conversion
// ===========================================================================

/// Convert a GameOutcome into a GameOutcomeView.
pub fn game_outcome_to_view(
    outcome: &GameOutcome,
    players: &[PlayerState; 2],
) -> GameOutcomeView {
    let (status, winner) = match outcome {
        GameOutcome::InProgress => ("in_progress".to_string(), None),
        GameOutcome::Victory(pid) => ("victory".to_string(), Some(pid.0)),
        GameOutcome::Draw => ("draw".to_string(), None),
    };

    GameOutcomeView {
        status,
        winner,
        player_0_vp: players[0].vp.value(),
        player_1_vp: players[1].vp.value(),
    }
}

// ===========================================================================
// Replay conversion
// ===========================================================================

/// Convert a ReplayPlayer into a ReplayInfoView.
/// Requires the ReplayFile's outcome and final_scores for the outcome view.
pub fn replay_info_to_view(
    player: &ReplayPlayer,
    replay: &ReplayFile,
) -> ReplayInfoView {
    let header = player.header();

    let players: Vec<ReplayPlayerInfoView> = header
        .players
        .iter()
        .map(replay_player_info_to_view)
        .collect();

    // Build a GameOutcomeView from the replay's outcome and final_scores.
    let outcome_view = GameOutcomeView {
        status: match replay.outcome {
            GameOutcome::InProgress => "in_progress".to_string(),
            GameOutcome::Victory(_) => "victory".to_string(),
            GameOutcome::Draw => "draw".to_string(),
        },
        winner: match replay.outcome {
            GameOutcome::Victory(pid) => Some(pid.0),
            _ => None,
        },
        player_0_vp: replay.final_scores[0].total_vp,
        player_1_vp: replay.final_scores[1].total_vp,
    };

    ReplayInfoView {
        format_version: replay.format_version,
        total_frames: player.total_frames(),
        current_frame: player.current_frame_index(),
        outcome: outcome_view,
        players,
        has_more: player.has_more(),
    }
}

/// Convert a ReplayPlayerInfo into a ReplayPlayerInfoView.
fn replay_player_info_to_view(info: &ReplayPlayerInfo) -> ReplayPlayerInfoView {
    ReplayPlayerInfoView {
        player_id: info.player_id.0,
        name: info.name.clone(),
        faction_id: info.faction_id.map(|f| f.0),
        enhancement: info.enhancement.map(|e| e.0),
        secondary: info.secondary.map(|s| s.0),
    }
}

// ===========================================================================
// Helper: Phase / SubPhase to string
// ===========================================================================

/// Convert a Phase enum to a display string.
pub fn phase_to_string(phase: &Phase) -> String {
    match phase {
        Phase::PreBattle => "PreBattle".to_string(),
        Phase::Command => "Command".to_string(),
        Phase::Movement => "Movement".to_string(),
        Phase::Shooting => "Shooting".to_string(),
        Phase::Charge => "Charge".to_string(),
        Phase::Fight => "Fight".to_string(),
        Phase::GameEnd => "GameEnd".to_string(),
    }
}

/// Convert a SubPhase enum to a display string.
pub fn subphase_to_string(sp: &SubPhase) -> String {
    match sp {
        SubPhase::DetermineAttackerDefender => "DetermineAttackerDefender".to_string(),
        SubPhase::SelectEnhancements => "SelectEnhancements".to_string(),
        SubPhase::SelectSecondaries => "SelectSecondaries".to_string(),
        SubPhase::SelectPatrolSquad => "SelectPatrolSquad".to_string(),
        SubPhase::Deployment => "Deployment".to_string(),
        SubPhase::DetermineFirstTurn => "DetermineFirstTurn".to_string(),
        SubPhase::CommandPhaseStart => "CommandPhaseStart".to_string(),
        SubPhase::GainCommandPoints => "GainCommandPoints".to_string(),
        SubPhase::BattleShockTests => "BattleShockTests".to_string(),
        SubPhase::CommandPhaseEnd => "CommandPhaseEnd".to_string(),
        SubPhase::MovementPhaseStart => "MovementPhaseStart".to_string(),
        SubPhase::SelectUnitToMove => "SelectUnitToMove".to_string(),
        SubPhase::ResolveMovement => "ResolveMovement".to_string(),
        SubPhase::Reinforcements => "Reinforcements".to_string(),
        SubPhase::MovementPhaseEnd => "MovementPhaseEnd".to_string(),
        SubPhase::ShootingPhaseStart => "ShootingPhaseStart".to_string(),
        SubPhase::SelectUnitToShoot => "SelectUnitToShoot".to_string(),
        SubPhase::SelectTargets => "SelectTargets".to_string(),
        SubPhase::ResolveAttacks => "ResolveAttacks".to_string(),
        SubPhase::ShootingPhaseEnd => "ShootingPhaseEnd".to_string(),
        SubPhase::ChargePhaseStart => "ChargePhaseStart".to_string(),
        SubPhase::SelectUnitToCharge => "SelectUnitToCharge".to_string(),
        SubPhase::DeclareChargeTargets => "DeclareChargeTargets".to_string(),
        SubPhase::ResolveOverwatch => "ResolveOverwatch".to_string(),
        SubPhase::RollChargeDistance => "RollChargeDistance".to_string(),
        SubPhase::MakeChargeMove => "MakeChargeMove".to_string(),
        SubPhase::ChargePhaseEnd => "ChargePhaseEnd".to_string(),
        SubPhase::FightPhaseStart => "FightPhaseStart".to_string(),
        SubPhase::FightsFirst => "FightsFirst".to_string(),
        SubPhase::RemainingCombats => "RemainingCombats".to_string(),
        SubPhase::PileIn => "PileIn".to_string(),
        SubPhase::ResolveMeleeAttacks => "ResolveMeleeAttacks".to_string(),
        SubPhase::Consolidate => "Consolidate".to_string(),
        SubPhase::FightPhaseEnd => "FightPhaseEnd".to_string(),
        SubPhase::ReactionWindow => "ReactionWindow".to_string(),
        SubPhase::StratagemWindow => "StratagemWindow".to_string(),
        SubPhase::EffectResolution => "EffectResolution".to_string(),
    }
}

// ===========================================================================
// Helper: Keyword bit index to name
// ===========================================================================

/// Convert a keyword bit index to its display name.
pub fn keyword_bit_to_name(bit: u8) -> &'static str {
    match bit {
        0 => "Infantry",
        1 => "Monster",
        2 => "Vehicle",
        3 => "Mounted",
        4 => "Beast",
        5 => "Swarm",
        6 => "Terminator",
        7 => "Character",
        8 => "Battleline",
        9 => "Dedicated Transport",
        10 => "Epic Hero",
        11 => "Leader",
        12 => "Imperium",
        13 => "Adeptus Custodes",
        14 => "Space Marines",
        15 => "Astartes",
        16 => "Chaos",
        17 => "Khorne",
        18 => "Daemon",
        19 => "World Eaters",
        20 => "Blade Champion",
        21 => "Tristraen",
        22 => "Custodian Guard",
        23 => "Custodian Wardens",
        24 => "Allarus Custodians",
        25 => "Daemon Prince",
        26 => "Vorrakh",
        27 => "Master of Executions",
        28 => "Berzerkers",
        29 => "Jakhals",
        30 => "Tyranids",
        31 => "Orks",
        32 => "Aeldari",
        33 => "Drukhari",
        34 => "Necrons",
        35 => "T'au Empire",
        36 => "Genestealer Cults",
        37 => "Death Guard",
        38 => "Thousand Sons",
        39 => "Grey Knights",
        40 => "Sisters of Battle",
        41 => "Psyker",
        42 => "Fly",
        43 => "Walker",
        44 => "Grenades",
        45 => "Smoke",
        46 => "Aircraft",
        _ => "Unknown",
    }
}

// ===========================================================================
// Helper: Weapon ability to display string
// ===========================================================================

/// Convert a WeaponAbility enum to a display string.
pub fn weapon_ability_to_string(ability: &WeaponAbility) -> String {
    match ability {
        WeaponAbility::Assault => "Assault".to_string(),
        WeaponAbility::Heavy => "Heavy".to_string(),
        WeaponAbility::RapidFire(n) => format!("Rapid Fire {}", n),
        WeaponAbility::Pistol => "Pistol".to_string(),
        WeaponAbility::Blast => "Blast".to_string(),
        WeaponAbility::Torrent => "Torrent".to_string(),
        WeaponAbility::TwinLinked => "Twin-linked".to_string(),
        WeaponAbility::LethalHits => "Lethal Hits".to_string(),
        WeaponAbility::SustainedHits(n) => format!("Sustained Hits {}", n),
        WeaponAbility::DevastatingWounds => "Devastating Wounds".to_string(),
        WeaponAbility::Hazardous => "Hazardous".to_string(),
        WeaponAbility::Precision => "Precision".to_string(),
        WeaponAbility::Anti(keyword, threshold) => {
            format!("Anti-{:?} {}+", keyword, threshold)
        }
        WeaponAbility::IndirectFire => "Indirect Fire".to_string(),
        WeaponAbility::IgnoresCover => "Ignores Cover".to_string(),
        WeaponAbility::Melta(n) => format!("Melta {}", n),
        WeaponAbility::Lance => "Lance".to_string(),
        WeaponAbility::ExtraAttacks => "Extra Attacks".to_string(),
    }
}

// ===========================================================================
// Helper: Format characteristic strings
// ===========================================================================

/// Format an ArmorSave as a display string (e.g. "3+", "2+").
pub fn format_armor_save(save: &ArmorSave) -> String {
    if save.value() >= 7 {
        "-".to_string()
    } else {
        format!("{}+", save.value())
    }
}

/// Format an InvulnerableSave as a display string (e.g. "4+").
pub fn format_invuln(save: &InvulnerableSave) -> String {
    format!("{}+", save.value())
}

/// Format an AttackCount as a display string (e.g. "3", "D6", "D3+1").
pub fn format_attacks(attacks: &AttackCount) -> String {
    match attacks {
        AttackCount::Fixed(n) => n.to_string(),
        AttackCount::D6 => "D6".to_string(),
        AttackCount::D3 => "D3".to_string(),
        AttackCount::D6Plus(n) => format!("D6+{}", n),
        AttackCount::D3Plus(n) => format!("D3+{}", n),
    }
}

/// Format a Damage value as a display string (e.g. "1", "D3", "D6+1").
pub fn format_damage(damage: &Damage) -> String {
    match damage {
        Damage::Fixed(n) => n.to_string(),
        Damage::D3 => "D3".to_string(),
        Damage::D6 => "D6".to_string(),
        Damage::D3Plus(n) => format!("D3+{}", n),
        Damage::D6Plus(n) => format!("D6+{}", n),
    }
}

/// Format an ArmorPenetration value as a display string (e.g. "0", "-1", "-3").
pub fn format_ap(ap: &ArmorPenetration) -> String {
    let val = ap.value();
    if val == 0 {
        "0".to_string()
    } else {
        format!("{}", val)
    }
}

/// Format a FeelNoPain value as a display string (e.g. "5+").
pub fn format_fnp(fnp: &FeelNoPain) -> String {
    format!("{}+", fnp.value())
}

// ===========================================================================
// Helper: TacticalIntent to string
// ===========================================================================

/// Convert a TacticalIntent enum to a display string.
pub fn tactical_intent_to_string(intent: &TacticalIntent) -> String {
    match intent {
        TacticalIntent::HoldObjective => "Hold Objective".to_string(),
        TacticalIntent::ContestObjective => "Contest Objective".to_string(),
        TacticalIntent::MoveToCover => "Move to Cover".to_string(),
        TacticalIntent::StageCharge => "Stage Charge".to_string(),
        TacticalIntent::Screen => "Screen".to_string(),
        TacticalIntent::Retreat => "Retreat".to_string(),
        TacticalIntent::LineUpShot => "Line Up Shot".to_string(),
        TacticalIntent::DenyReserveZone => "Deny Reserve Zone".to_string(),
        TacticalIntent::HoldPosition => "Hold Position".to_string(),
        TacticalIntent::AdvanceAggressively => "Advance Aggressively".to_string(),
        TacticalIntent::FallBackToShoot => "Fall Back to Shoot".to_string(),
        TacticalIntent::DeepStrikeArrive => "Deep Strike Arrive".to_string(),
        TacticalIntent::MaximizeKills => "Maximize Kills".to_string(),
        TacticalIntent::SoftenChargeTarget => "Soften Charge Target".to_string(),
        TacticalIntent::RemoveScoringUnit => "Remove Scoring Unit".to_string(),
        TacticalIntent::ForceBattleShock => "Force Battle Shock".to_string(),
        TacticalIntent::BracketHardTarget => "Bracket Hard Target".to_string(),
        TacticalIntent::ChargeHighValueTarget => "Charge High Value Target".to_string(),
        TacticalIntent::MultiChargeObjectiveSwing => "Multi-Charge Objective Swing".to_string(),
        TacticalIntent::ChargeSoftenedTarget => "Charge Softened Target".to_string(),
        TacticalIntent::FightMaxDamage => "Fight Max Damage".to_string(),
        TacticalIntent::FightPreserve => "Fight Preserve".to_string(),
        TacticalIntent::ChooseStance => "Choose Stance".to_string(),
        TacticalIntent::ChooseWeaponProfile => "Choose Weapon Profile".to_string(),
        TacticalIntent::DefensiveStratagem => "Defensive Stratagem".to_string(),
        TacticalIntent::OffensiveStratagem => "Offensive Stratagem".to_string(),
        TacticalIntent::MovementReaction => "Movement Reaction".to_string(),
        TacticalIntent::FightOrderManipulation => "Fight Order Manipulation".to_string(),
        TacticalIntent::DeclineStratagem => "Decline Stratagem".to_string(),
        // Boarding Actions intents
        TacticalIntent::OperateHatch => "Operate Hatch".to_string(),
        TacticalIntent::SecureObjective => "Secure Objective".to_string(),
        TacticalIntent::DefendPosition => "Defend Position".to_string(),
        TacticalIntent::ControlChokepoint => "Control Chokepoint".to_string(),
        TacticalIntent::FlankThroughHatch => "Flank Through Hatch".to_string(),
        TacticalIntent::ProjectLeaderAbility => "Project Leader Ability".to_string(),
        TacticalIntent::EnterFromReserves => "Enter From Reserves".to_string(),
        // Misc intents
        TacticalIntent::ScoreObjective => "Score Objective".to_string(),
        TacticalIntent::AllocateBlessings => "Allocate Blessings".to_string(),
        TacticalIntent::PhaseControl => "Phase Control".to_string(),
        TacticalIntent::Generic => "Generic".to_string(),
    }
}

// ===========================================================================
// Helper: Decision type from phase/subphase
// ===========================================================================

/// Derive a decision type string from the current phase and subphase.
fn decision_type_from_phase_subphase(phase: &Phase, subphase: &SubPhase) -> &'static str {
    match (phase, subphase) {
        (Phase::PreBattle, SubPhase::SelectEnhancements) => "Setup Choice",
        (Phase::PreBattle, SubPhase::SelectSecondaries) => "Setup Choice",
        (Phase::PreBattle, SubPhase::SelectPatrolSquad) => "Setup Choice",
        (Phase::PreBattle, SubPhase::Deployment) => "Setup Choice",
        (Phase::PreBattle, SubPhase::DetermineAttackerDefender) => "Setup Choice",
        (Phase::PreBattle, SubPhase::DetermineFirstTurn) => "Setup Choice",
        (Phase::Movement, SubPhase::SelectUnitToMove) => "Movement Action",
        (Phase::Movement, SubPhase::ResolveMovement) => "Movement Action",
        (Phase::Movement, SubPhase::Reinforcements) => "Movement Action",
        (Phase::Shooting, SubPhase::SelectUnitToShoot) => "Shooting Target",
        (Phase::Shooting, SubPhase::SelectTargets) => "Shooting Target",
        (Phase::Shooting, SubPhase::ResolveAttacks) => "Shooting Target",
        (Phase::Charge, SubPhase::SelectUnitToCharge) => "Charge Declaration",
        (Phase::Charge, SubPhase::DeclareChargeTargets) => "Charge Declaration",
        (Phase::Charge, SubPhase::ResolveOverwatch) => "Overwatch Decision",
        (Phase::Fight, SubPhase::FightsFirst) => "Fight Order",
        (Phase::Fight, SubPhase::RemainingCombats) => "Fight Order",
        (Phase::Fight, SubPhase::PileIn) => "Fight Order",
        (Phase::Fight, SubPhase::ResolveMeleeAttacks) => "Melee Target",
        (_, SubPhase::StratagemWindow) => "Stratagem Window",
        (_, SubPhase::ReactionWindow) => "Overwatch Decision",
        _ => "Generic Choice",
    }
}

//! Deterministic state hashing for GameState.
//!
//! Produces a stable u64 hash of the complete game state for:
//! - Replay verification (compare state hashes at each step)
//! - Transposition table keys (search engine)
//! - Bug localization (pinpoint where states diverge)
//! - Determinism contract enforcement
//!
//! Source: implementation_v3.md Section 18.3

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasher, Hash, Hasher};

use crate::state::GameState;

/// Compute a deterministic hash of the complete game state.
///
/// Hashes all game-relevant state fields. Collections (HashSet, HashMap) are
/// sorted before hashing to ensure determinism regardless of iteration order.
///
/// Fields NOT hashed (by design):
/// - `event_bus` - events are a log, not game state that affects future decisions
/// - `command_history` - the hash IS for the command history, including it would be circular
/// - `reaction_windows` - transient, derived from current state
pub fn compute_state_hash(state: &GameState) -> u64 {
    let mut hasher = new_hasher();

    // --- Top-level scalar fields ---
    state.content_version.hash(&mut hasher);
    state.scenario_id.hash(&mut hasher);
    state.battle_round.hash(&mut hasher);
    state.active_player.hash(&mut hasher);
    state.current_phase.hash(&mut hasher);
    state.current_subphase.hash(&mut hasher);
    state.decision_owner.hash(&mut hasher);

    // --- Players ---
    hash_player_state(&mut hasher, &state.players[0]);
    hash_player_state(&mut hasher, &state.players[1]);

    // --- Units (Vec - stable order, iterate in order) ---
    hasher.write_usize(state.units.len());
    for unit in &state.units {
        hash_unit_state(&mut hasher, unit);
    }

    // --- Board ---
    hash_board(&mut hasher, &state.board);

    // --- Deployment config ---
    state.deployment_config.is_some().hash(&mut hasher);
    if let Some(ref config) = state.deployment_config {
        hash_deployment_config(&mut hasher, config);
    }

    // --- Active effects (Vec - stable order) ---
    hasher.write_usize(state.active_effects.len());
    for effect in &state.active_effects {
        effect.hash(&mut hasher);
    }

    // --- Turn flags ---
    hash_turn_flags(&mut hasher, &state.turn_flags);

    // --- Game outcome ---
    state.game_outcome.hash(&mut hasher);

    // --- Deterministic counter ---
    hasher.write_u64(state.deterministic_counter);

    // --- Dice roller ---
    hash_dice_roller(&mut hasher, &state.dice_roller);

    hasher.finish()
}

/// Create a new AHasher with fixed seeds for deterministic cross-platform hashing.
///
/// Using fixed seeds ensures the same state always produces the same hash,
/// regardless of platform, process, or run.
fn new_hasher() -> ahash::AHasher {
    let state = ahash::RandomState::with_seeds(
        0x517cc1b727220a95,
        0x6c62272e07bb0142,
        0x62b821756295c58d,
        0x30474e6c3a4fc481,
    );
    state.build_hasher()
}

/// Hash a HashSet in deterministic order by sorting elements first.
fn hash_sorted_set<T: Hash + Ord>(hasher: &mut ahash::AHasher, set: &HashSet<T>) {
    let mut sorted: Vec<&T> = set.iter().collect();
    sorted.sort();
    hasher.write_usize(sorted.len());
    for item in sorted {
        item.hash(hasher);
    }
}

/// Hash a HashMap in deterministic order by sorting keys first.
fn hash_sorted_map<K: Hash + Ord, V: Hash>(hasher: &mut ahash::AHasher, map: &HashMap<K, V>) {
    let mut sorted_keys: Vec<&K> = map.keys().collect();
    sorted_keys.sort();
    hasher.write_usize(sorted_keys.len());
    for key in sorted_keys {
        key.hash(hasher);
        map[key].hash(hasher);
    }
}

/// Hash all fields of a PlayerState.
fn hash_player_state(hasher: &mut ahash::AHasher, player: &crate::state::PlayerState) {
    player.id.hash(hasher);
    player.name.hash(hasher);
    player.faction_id.hash(hasher);
    player.cp.hash(hasher);
    player.vp.hash(hasher);
    player.enhancement_choice.hash(hasher);
    player.secondary_choice.hash(hasher);
    player.patrol_squad_choice.hash(hasher);
    player.first_turn.hash(hasher);

    // Faction round flags
    hash_faction_round_flags(hasher, &player.faction_round_flags);

    // Stratagem usage
    hash_stratagem_usage(hasher, &player.stratagem_usage);

    // Mission progress
    hash_mission_progress(hasher, &player.mission_progress);
}

/// Hash FactionRoundFlags. Sort active_blessings for determinism.
fn hash_faction_round_flags(
    hasher: &mut ahash::AHasher,
    flags: &crate::state::FactionRoundFlags,
) {
    // active_blessings: Vec<String> - sort for determinism
    let mut sorted_blessings: Vec<&String> = flags.active_blessings.iter().collect();
    sorted_blessings.sort();
    hasher.write_usize(sorted_blessings.len());
    for blessing in sorted_blessings {
        blessing.hash(hasher);
    }

    // blessing_dice: Vec<u8> - in order (roll order is meaningful)
    hasher.write_usize(flags.blessing_dice.len());
    for die in &flags.blessing_dice {
        hasher.write_u8(*die);
    }
}

/// Hash StratagemUsageTracker. Sort all HashSets.
fn hash_stratagem_usage(
    hasher: &mut ahash::AHasher,
    tracker: &crate::state::StratagemUsageTracker,
) {
    hash_sorted_set(hasher, &tracker.used_this_phase);
    hash_sorted_set(hasher, &tracker.used_this_turn);
    hash_sorted_set(hasher, &tracker.used_this_battle);
}

/// Hash MissionProgress.
fn hash_mission_progress(
    hasher: &mut ahash::AHasher,
    progress: &crate::state::MissionProgress,
) {
    progress.primary_vp.hash(hasher);
    progress.secondary_vp.hash(hasher);

    // round_scores: Vec<(BattleRound, VictoryPoints)> - in order
    hasher.write_usize(progress.round_scores.len());
    for (round, vp) in &progress.round_scores {
        round.hash(hasher);
        vp.hash(hasher);
    }
}

/// Hash all fields of a UnitState.
fn hash_unit_state(hasher: &mut ahash::AHasher, unit: &crate::unit::UnitState) {
    unit.id.hash(hasher);
    unit.owner.hash(hasher);
    unit.name.hash(hasher);
    unit.datasheet_id.hash(hasher);
    unit.keywords.hash(hasher);

    // Models (Vec - stable order)
    hasher.write_usize(unit.models.len());
    for model in &unit.models {
        hash_model_state(hasher, model);
    }

    unit.base_movement.hash(hasher);
    unit.base_toughness.hash(hasher);
    unit.base_armor_save.hash(hasher);
    unit.invulnerable_save.hash(hasher);
    unit.base_leadership.hash(hasher);
    unit.base_oc.hash(hasher);

    unit.status.hash(hasher);
    unit.battle_shocked.hash(hasher);
    unit.below_half_strength.hash(hasher);
    unit.engagement_status.hash(hasher);

    unit.attached_leader.hash(hasher);
    unit.bodyguard_for.hash(hasher);
    unit.reserve_type.hash(hasher);

    // Wargear abilities (Vec - in order)
    hasher.write_usize(unit.wargear_abilities.len());
    for ability in &unit.wargear_abilities {
        hash_wargear_ability(hasher, ability);
    }

    unit.enhancement_oc_override.hash(hasher);
    unit.enhancement_oc_requires_engaged.hash(hasher);
}

/// Hash all fields of a ModelState.
fn hash_model_state(hasher: &mut ahash::AHasher, model: &crate::unit::ModelState) {
    model.id.hash(hasher);
    model.unit_id.hash(hasher);
    model.alive.hash(hasher);
    model.wounds_max.hash(hasher);
    model.wounds_remaining.hash(hasher);
    model.position.hash(hasher);
    model.base_size.hash(hasher);

    // Ranged weapons (Vec - in order)
    hasher.write_usize(model.ranged_weapons.len());
    for weapon in &model.ranged_weapons {
        weapon.hash(hasher);
    }

    // Melee weapons (Vec - in order)
    hasher.write_usize(model.melee_weapons.len());
    for weapon in &model.melee_weapons {
        weapon.hash(hasher);
    }

    model.allocation_status.hash(hasher);
    model.is_leader.hash(hasher);
    model.feel_no_pain.hash(hasher);
}

/// Hash a WargearAbilityState (name, description, active).
fn hash_wargear_ability(
    hasher: &mut ahash::AHasher,
    ability: &crate::unit::WargearAbilityState,
) {
    ability.name.hash(hasher);
    ability.description.hash(hasher);
    ability.active.hash(hasher);
}

/// Hash all fields of the Board.
///
/// Board does not derive Hash, so we hash each field manually.
fn hash_board(hasher: &mut ahash::AHasher, board: &wh40k_geometry::Board) {
    // Dimensions (derives Hash)
    board.dimensions.hash(hasher);

    // Terrain pieces (Vec - in order)
    hasher.write_usize(board.terrain.len());
    for terrain in &board.terrain {
        hash_terrain_piece(hasher, terrain);
    }

    // Objective markers (Vec - in order)
    hasher.write_usize(board.objectives.len());
    for objective in &board.objectives {
        hash_objective_marker(hasher, objective);
    }
}

/// Hash a TerrainPiece (name, footprint, properties).
fn hash_terrain_piece(hasher: &mut ahash::AHasher, terrain: &wh40k_geometry::TerrainPiece) {
    terrain.name.hash(hasher);
    // Rect derives Hash (x, y, w, h as Inches which derives Hash)
    terrain.footprint.hash(hasher);
    // TerrainProperties derives Hash
    terrain.properties.hash(hasher);
}

/// Hash an ObjectiveMarker (id, position, height, control_status, label).
fn hash_objective_marker(
    hasher: &mut ahash::AHasher,
    objective: &wh40k_geometry::ObjectiveMarker,
) {
    objective.id.hash(hasher);
    objective.position.hash(hasher);
    objective.height.hash(hasher);
    objective.control_status.hash(hasher);
    objective.label.hash(hasher);
}

/// Hash TurnFlags. All HashSets are sorted before hashing.
fn hash_turn_flags(hasher: &mut ahash::AHasher, flags: &crate::state::TurnFlags) {
    hash_sorted_set(hasher, &flags.units_moved);
    hash_sorted_set(hasher, &flags.units_shot);
    hash_sorted_set(hasher, &flags.units_charged);
    hash_sorted_set(hasher, &flags.units_fought);
    hash_sorted_set(hasher, &flags.charged_this_turn);
    hash_sorted_set(hasher, &flags.fell_back_this_turn);
    hash_sorted_set(hasher, &flags.advanced_this_turn);
    hash_sorted_set(hasher, &flags.remained_stationary);

    flags.overwatch_used_this_turn.hash(hasher);

    // stratagems_used_this_phase: Vec<StratagemId> - hash in order (not a HashSet)
    hasher.write_usize(flags.stratagems_used_this_phase.len());
    for strat_id in &flags.stratagems_used_this_phase {
        strat_id.hash(hasher);
    }

    // declared_charge_targets: HashMap<UnitId, Vec<UnitId>> - sort by key
    hash_sorted_map(hasher, &flags.declared_charge_targets);

    // charge_roll_results: HashMap<UnitId, u8> - sort by key
    hash_sorted_map(hasher, &flags.charge_roll_results);

    // ka_tah_stances: HashMap<UnitId, String> - sort by key
    hash_sorted_map(hasher, &flags.ka_tah_stances);

    // vaultswords_profiles: HashMap<ModelId, String> - sort by key
    hash_sorted_map(hasher, &flags.vaultswords_profiles);

    // fight_on_death_queue: Vec<ModelId> - in order
    hasher.write_usize(flags.fight_on_death_queue.len());
    for model_id in &flags.fight_on_death_queue {
        model_id.hash(hasher);
    }
}

/// Hash the DeploymentConfig.
fn hash_deployment_config(
    hasher: &mut ahash::AHasher,
    config: &wh40k_geometry::DeploymentConfig,
) {
    config.map_type.hash(hasher);
    config.attacker_zone.player.hash(hasher);
    config.attacker_zone.map_type.hash(hasher);
    // Hash polygon vertices
    hasher.write_usize(config.attacker_zone.polygon.vertices.len());
    for v in &config.attacker_zone.polygon.vertices {
        v.hash(hasher);
    }
    config.defender_zone.player.hash(hasher);
    config.defender_zone.map_type.hash(hasher);
    hasher.write_usize(config.defender_zone.polygon.vertices.len());
    for v in &config.defender_zone.polygon.vertices {
        v.hash(hasher);
    }
}

/// Hash the DiceRoller state.
///
/// Hashes the context (seed, stream_kind, state_fingerprint, action_id, resolution_index)
/// and the log summary (next_roll_id via len, records count).
fn hash_dice_roller(hasher: &mut ahash::AHasher, dice_roller: &wh40k_dice::DiceRoller) {
    let ctx = dice_roller.context();

    // Context fields
    ctx.seed.hash(hasher);
    ctx.stream_kind.hash(hasher);
    hasher.write_u64(ctx.state_fingerprint);
    hasher.write_u32(ctx.action_id);
    hasher.write_u32(ctx.resolution_index);

    // Log summary: record count and next_roll_id (approximated by len)
    let log = dice_roller.log();
    hasher.write_usize(log.len());
    // The records count captures the progression of the dice log
    // without hashing every individual record (too expensive and
    // already captured in the dice log audit trail)
    hasher.write_usize(log.records().len());
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{GameState, PlayerState, TurnFlags};
    use wh40k_core_types::{BattleRound, GameOutcome, Phase, PlayerId, SubPhase, UnitId};
    use wh40k_dice::{DiceContext, DiceRoller, StreamKind};

    fn make_test_game_state() -> GameState {
        let seed = [0u8; 32];
        let ctx = DiceContext::new(seed, StreamKind::BattleShockTest, 0, 0);
        let dice_roller = DiceRoller::new(ctx);

        GameState {
            content_version: "test-1.0".to_string(),
            scenario_id: None,
            battle_round: BattleRound::new(1),
            active_player: PlayerId::new(0),
            current_phase: Phase::PreBattle,
            current_subphase: SubPhase::DetermineAttackerDefender,
            decision_owner: PlayerId::new(0),
            players: [
                PlayerState::new(PlayerId::new(0), "Player A".to_string()),
                PlayerState::new(PlayerId::new(1), "Player B".to_string()),
            ],
            units: Vec::new(),
            board: wh40k_geometry::Board::combat_patrol(),
            deployment_config: None,
            event_bus: wh40k_event_system::EventBus::new(),
            command_history: wh40k_command_system::CommandHistory::new(),
            dice_roller,
            active_effects: Vec::new(),
            reaction_windows: Vec::new(),
            turn_flags: TurnFlags::new(),
            game_outcome: GameOutcome::InProgress,
            deterministic_counter: 0,
        }
    }

    #[test]
    fn test_compute_hash_deterministic() {
        let state1 = make_test_game_state();
        let state2 = make_test_game_state();

        let hash1 = compute_state_hash(&state1);
        let hash2 = compute_state_hash(&state2);

        assert_eq!(hash1, hash2, "Same state should produce same hash");
    }

    #[test]
    fn test_compute_hash_nonzero() {
        let state = make_test_game_state();
        let hash = compute_state_hash(&state);

        // Hash should be a non-trivial value
        assert_ne!(hash, 0, "Hash should not be zero for a valid state");
    }

    #[test]
    fn test_compute_hash_changes_with_state() {
        let state1 = make_test_game_state();
        let mut state2 = make_test_game_state();
        state2.battle_round = BattleRound::new(2);

        let hash1 = compute_state_hash(&state1);
        let hash2 = compute_state_hash(&state2);

        assert_ne!(
            hash1, hash2,
            "Different states should produce different hashes"
        );
    }

    #[test]
    fn test_compute_hash_changes_with_player() {
        let state1 = make_test_game_state();
        let mut state2 = make_test_game_state();
        state2.active_player = PlayerId::new(1);

        let hash1 = compute_state_hash(&state1);
        let hash2 = compute_state_hash(&state2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_changes_with_phase() {
        let state1 = make_test_game_state();
        let mut state2 = make_test_game_state();
        state2.current_phase = Phase::Command;

        let hash1 = compute_state_hash(&state1);
        let hash2 = compute_state_hash(&state2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_changes_with_counter() {
        let state1 = make_test_game_state();
        let mut state2 = make_test_game_state();
        state2.deterministic_counter = 42;

        let hash1 = compute_state_hash(&state1);
        let hash2 = compute_state_hash(&state2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_changes_with_turn_flags() {
        let state1 = make_test_game_state();
        let mut state2 = make_test_game_state();
        state2.turn_flags.units_moved.insert(UnitId::new(5));

        let hash1 = compute_state_hash(&state1);
        let hash2 = compute_state_hash(&state2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_via_game_state_method() {
        let state = make_test_game_state();
        let hash_direct = compute_state_hash(&state);
        let hash_method = state.compute_hash();

        assert_eq!(
            hash_direct, hash_method,
            "GameState::compute_hash() should delegate to compute_state_hash()"
        );
    }

    #[test]
    fn test_hash_set_order_independent() {
        // Ensure that inserting elements into a HashSet in different order
        // produces the same hash (because we sort before hashing)
        let mut state1 = make_test_game_state();
        state1.turn_flags.units_moved.insert(UnitId::new(1));
        state1.turn_flags.units_moved.insert(UnitId::new(2));
        state1.turn_flags.units_moved.insert(UnitId::new(3));

        let mut state2 = make_test_game_state();
        state2.turn_flags.units_moved.insert(UnitId::new(3));
        state2.turn_flags.units_moved.insert(UnitId::new(1));
        state2.turn_flags.units_moved.insert(UnitId::new(2));

        let hash1 = compute_state_hash(&state1);
        let hash2 = compute_state_hash(&state2);

        assert_eq!(
            hash1, hash2,
            "HashSet insertion order should not affect hash"
        );
    }

    #[test]
    fn test_compute_hash_changes_with_game_outcome() {
        let state1 = make_test_game_state();
        let mut state2 = make_test_game_state();
        state2.game_outcome = GameOutcome::Victory(PlayerId::new(0));

        let hash1 = compute_state_hash(&state1);
        let hash2 = compute_state_hash(&state2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_changes_with_cp() {
        let mut state1 = make_test_game_state();
        let mut state2 = make_test_game_state();
        state2.players[0].gain_cp(1);

        let hash1 = compute_state_hash(&state1);
        let hash2 = compute_state_hash(&state2);

        assert_ne!(hash1, hash2, "CP change should produce different hash");

        // Also verify the method on GameState works
        state1.players[0].gain_cp(1);
        let hash1_after = compute_state_hash(&state1);
        assert_eq!(
            hash1_after, hash2,
            "Same CP values should produce same hash"
        );
    }

    #[test]
    fn test_hash_sorted_map_order_independent() {
        let mut state1 = make_test_game_state();
        state1
            .turn_flags
            .ka_tah_stances
            .insert(UnitId::new(1), "Dacatarai".to_string());
        state1
            .turn_flags
            .ka_tah_stances
            .insert(UnitId::new(2), "Rendax".to_string());

        let mut state2 = make_test_game_state();
        state2
            .turn_flags
            .ka_tah_stances
            .insert(UnitId::new(2), "Rendax".to_string());
        state2
            .turn_flags
            .ka_tah_stances
            .insert(UnitId::new(1), "Dacatarai".to_string());

        let hash1 = compute_state_hash(&state1);
        let hash2 = compute_state_hash(&state2);

        assert_eq!(
            hash1, hash2,
            "HashMap insertion order should not affect hash"
        );
    }
}

# WH40K Combat Patrol Engine - Implementation TODO

**Status: Phase 9 GUI Productionization COMPLETE. Ready for Phase 10: MCP Server and AlphaGo Expansion.**
**Total tests passing: 1180 | Workspace compiles clean | Web build succeeds | ~58 web files created**

---

## Phase 0: Foundations (COMPLETE)

### 0.1 Repository Scaffold
- [x] Init git repo, create directory tree, .gitignore
- [x] Create Cargo workspace with initial crate stubs, pin workspace deps
- [x] Create content directories structure
- [x] Create docs directory with markdown stubs
- [ ] Set up CI (GitHub Actions)

### 0.2 Core Types Crate
- [x] ID types (PlayerId, UnitId, ModelId, WeaponId, etc.)
- [x] Game state enums (Phase, SubPhase, BattleRound, etc.)
- [x] Measurement types (Inches fixed-point, Position, Distance, BaseSize)
- [x] Resource types (CommandPoints, VictoryPoints, Wounds, etc.)
- [x] Keyword system (Keyword enum + KeywordSet bitflags)
- [x] Weapon ability enums (WeaponAbility + WeaponAbilitySet)

### 0.3 Dice Crate
- [x] SeedBundle + DiceContext with deterministic SmallRng
- [x] DiceRoller methods (roll_d6, roll_2d6, roll_d3, roll_nd6, roll_5d6)
- [x] DiceRollRecord + DiceLog serializable audit trail
- [x] Child seed derivation, property tests for reproducibility

### 0.4 Geometry Crate
- [x] Board (44"x30"), DeploymentZone polygons, Terrain rectangles
- [x] Distance calculation (fixed-point Euclidean), range checks
- [x] Coherency checker (2"/5", 7+ models need 2 neighbors)
- [x] Simplified LOS: line trace vs terrain volumes
- [x] ObjectiveMarker placement, movement legality helpers

### 0.5 Test Harness
- [x] ScenarioBuilder for test game state construction
- [x] StateAssertion framework

---

## Phase 1: Core CP Engine Shell (COMPLETE)

### 1.1 Content Schema and Compiler
- [x] DatasheetSchema, WeaponProfileSchema, AbilitySchema structs
- [x] FactionSchema with data-driven design
- [x] Rule primitive DSL (composable primitives for faction rules)
- [x] Primitive integration tests (Blessings of Khorne, Martial Ka'tah)
- [x] MissionSchema, CombatPatrolPackSchema
- [x] Author Custodes.yaml
- [x] Author Frenzied_Reavers.yaml
- [x] content_compiler: parse, validate, emit ContentPack

### 1.2 Event System
- [x] GameEvent enum (~40 variants)
- [x] EventBus: emit, subscribe, queue, nested emission
- [x] ReactionWindow for decision points

### 1.3 Command System
- [x] Command enum (Setup, PhaseControl, Movement, etc.)
- [x] CommandValidator (15 validators, 1263 lines in game_core/validator.rs)
- [x] CommandExecutor (26+ command handlers, 1115 lines in game_core/executor.rs)
- [x] CommandHistory with state hashes
- [x] Command pipeline integration (propose -> validate -> execute -> emit -> record)

### 1.4 Game State Model
- [x] GameState top-level
- [x] PlayerState (CP, VP, enhancements, faction flags)
- [x] UnitState (models, leader/bodyguard, effects, etc.)
- [x] ModelState (wounds, position, weapons, allocation)
- [x] EffectState + ActiveEffect with duration types

### 1.5 Phase Progression
- [x] Phase state machine (BattleRound 1-5, PlayerTurn, Phase sequence)
- [x] Command Phase (+1 CP, battle-shock tests)
- [x] Movement Phase shell
- [x] DecisionSurface generation

### 1.6 Reserves and Objectives
- [x] CP reserve rules (Deep Strike, timing, distance)
- [x] Objective control (OC sum, controller, securing)

### 1.7 Mission/Scenario Loading
- [x] ScenarioLoader
- [x] Deployment sequence (PreBattle subphases, zone validation, alternate placement, first-turn roll-off)

---

## Phase 2: Full Combat Resolution (COMPLETE)

### 2.1 Shooting Pipeline
- [x] Unit eligibility
- [x] Target selection
- [x] Hit Roll (BS, Critical Hit, modifiers, Heavy, Stealth, Torrent)
- [x] Wound Roll (S vs T table, Critical Wound, modifiers, Lethal Hits, Anti-X)
- [x] Sustained Hits
- [x] Attack allocation (defender allocates, Precision)
- [x] Saving Throw (AP, Invulnerable, Benefit of Cover, Devastating Wounds)
- [x] Damage (wound loss, Feel No Pain, model destruction)
- [x] Weapon abilities (Rapid Fire, Blast, Twin-linked, Hazardous, etc.)
- [x] Executor integration (apply_resolve_attack wired into ResolveShootingAttack/ResolveMeleeAttack)

### 2.2 Charge Pipeline
- [x] Charge eligibility (12" range, not engaged, not AIRCRAFT, not advanced/fell back)
- [x] Charge roll (2D6, distance-based success check vs all declared targets)
- [x] Charge bonus (Fights First via charged_this_turn tracking)
- [x] Charge move execution (sets engagement on charger + targets)
- [x] Overwatch reaction window (opens after DeclareCharge, FireOverwatch/Decline actions)

### 2.3 Fight Pipeline
- [x] Fight Phase structure (Fights First / Remaining Combats via validator)
- [x] Fight eligibility (engaged or charged this turn)
- [x] Pile In (3" distance validation)
- [x] Melee attacks (WS, same sequence as shooting via combat module)
- [x] Consolidation (3" distance validation)
- [x] Martial Ka'tah stances (Dacatarai=Sustained Hits 1, Rendax=Lethal Hits stored and applied)
- [x] Tristraen's Vaultswords profile selection (stored in TurnFlags)
- [x] Counter-Operative reaction window (opens after melee, eligible engaged units, 2CP check)

### 2.4 Blessings of Khorne
- [x] 5D6 roll and allocation (up to 2 blessings)
- [x] Blessing effects (Rage-fuelled, Total Carnage, Martial Excellence)
- [x] Total Carnage fight-on-death (D6 roll per melee kill, 4+ = fight queue, Berserk Resilience +2 stacking)

### 2.5 Model Destruction and Morale
- [x] Model destruction, unit composition update
- [x] Coherency enforcement (after casualties, remove out-of-coherency models)
- [x] Unit destruction consequences (A Worthy Skull MoE+CHARACTER CP, Warrior Exemplar D6 3+ CP)
- [x] Desperate escape tests

### 2.6 Engagement and Attached Units
- [x] Pistol shooting in engagement range
- [x] Big Guns Never Tire (MONSTER/VEHICLE)
- [x] Attached unit wound allocation (bodyguard protects leader, Precision bypasses)
- [x] Lone Operative targeting restriction (>12" blocked unless closest eligible)

---

## Phase 3: Stratagems, Faction Rules, Scoring (COMPLETE)

### 3.1 Core Stratagem Runtime
- [x] StratagemDefinition (StratagemDef struct), StratagemUsageTracker, timing enforcement
- [x] Stratagem timing windows, same-phase restriction (validator upgraded with full validation)
- [x] Command Re-roll (1CP, any phase, re-roll one die)
- [x] Counter-Offensive (2CP, fight phase)
- [x] Fire Overwatch (1CP, movement/charge, 6s only)
- [x] Go to Ground (1CP, shooting, Infantry, 6++ invuln + Cover)
- [x] Heroic Intervention (1CP, charge phase)
- [x] Remaining: Insane Bravery, Epic Challenge, Rapid Ingress, Grenade, Smokescreen, Tank Shock
- [x] ALL_STRATAGEMS static array with 17 definitions
- [x] get_stratagem_def() lookup function
- [x] apply_stratagem_effects() for all stratagems

### 3.2 Faction Stratagems
- [x] FR: Horrifying Butchery (ForceTest on enemy, Ld penalty if below half)
- [x] FR: Berserk Resilience (fight-on-death 4+, +2 with Total Carnage)
- [x] FR: Bloodlust (D6 roll for bonus move distance)
- [x] Custodes: The Gilded Spear (+3" pile-in/consolidation = 6" total)
- [x] Custodes: Inescapable Vengeance (+2 to Advance rolls)
- [x] Custodes: Overawing Magnificence (Ld test, -2 to Charge rolls)
- [x] Battle-shocked stratagem restrictions (except Insane Bravery)
- [x] Berserk Resilience + Total Carnage stacking interaction

### 3.3 Enhancements and Secondary Objectives
- [x] Enhancement system (apply_enhancement with persistent effects)
- [x] FR enhancements: Fearsome Presence (OC 5 not battle-shocked), Bane of the Craven (desperate escape on fall back)
- [x] Custodes enhancements: Watchman of Terra (OC 4 in engagement), Warrior Exemplar (D6 3+ CP on kill)
- [x] Secondary objective system (score_secondary_objectives with timing checks)
- [x] FR secondaries: Champions of Khorne (NML objectives with CHARACTER, 2/3VP, max 12), Skull Takers (3VP per CHARACTER kill in Fight phase, max 12)
- [x] Custodes secondaries: Raise the Vexillas (3VP both edge objectives, BR3+, max 9), Consecrated Ground (+3VP kill / -1VP model loss)

### 3.4 Mission Scoring Runtime
- [x] Primary scoring framework (score_primary_objectives dispatches to mission-specific scorers)
- [x] Mission 1 - Clash of Patrols (5VP per objective BR2-5)
- [x] Mission 2 - Archeotech Recovery (5VP per objective + 10VP endgame center objective)
- [x] Mission 3 - Forward Outpost (5VP per objective + 10VP enemy DZ objective bonus)
- [x] Mission 4 - Scorched Earth (5VP per objective + 10VP raze mechanic)
- [x] Mission 5 - Sweeping Raid (5VP per objective + 5VP per enemy territory endgame)
- [x] Mission 6 - Display of Might (5VP per objective + 5VP CHARACTER claimed bonus)
- [x] End-of-game scoring (calculate_end_of_game_score: mission bonuses + Battle Ready 10VP)
- [x] Winner determination (determine_winner: VP comparison)

**Phase 3 Key Files:**
- `game_core/src/stratagem.rs` - 17 stratagems (11 core + 6 faction), definitions, effects, tests
- `game_core/src/scoring.rs` - Enhancements, secondaries, 6 mission scorers, endgame, winner determination
- `game_core/src/validator.rs` - Upgraded validate_use_stratagem with full CP/phase/timing/keyword/faction checks
- `game_core/src/executor.rs` - apply_use_stratagem wired to stratagem module for CP and effects
- `event_system/src/lib.rs` - Added StratagemEffectApplied event variant
- `dice/src/lib.rs` - Added StratagemEffect RollPurpose variant

### 3.5 Audit Gap Fixes (Post-Audit)
- [x] Fix #1: Stealth -1 to hit modifier applied in combat.rs hit roll (Smokescreen Stealth now functional)
- [x] Fix #2: Warp Blades blessing +1 AP applied during melee attack resolution in executor.rs
- [x] Fix #3: Total Carnage blessing +1 melee attack applied during attack resolution in executor.rs
- [x] Fix #4: Wrathful Devotion blessing 5+ FNP checked during damage resolution in executor.rs
- [x] Fix #5: Cover determination from terrain geometry wired into executor.rs (replaces hardcoded false)
- [x] Fix #6: Heroic Intervention move execution via HeroicInterventionMove command in executor.rs
- [x] Fix #7: stratagem_runtime synced with game_core (all 17 stratagems, delegates to game_core::stratagem)
- [x] Fix #8: Timing window enforcement added to validator.rs validate_use_stratagem
- [x] Fix #9: Enhancement OC effects (Fearsome Presence, Watchman of Terra) wired to unit effective_oc() via active_effects query

---

## Phase 4: Replay and Determinism Hardening (COMPLETE)

### 4.1 Replay System
- [x] Replay format definition (ReplayFile, ReplayHeader, ReplayFrame, ReplayScore, ReplayPlayerInfo)
- [x] ReplayRecorder (captures frames during live play, records commands/events/dice/hashes)
- [x] ReplayPlayer (frame navigation, queries by round/phase/player, verification against live state)
- [x] Export formats: JSON (pretty + compact), binary (bincode), human-readable text summary
- [x] Replay diff tool (ReplayDiff: compare two replays frame-by-frame, detect divergence point)

**Phase 4.1 Key Files:**
- `replay/src/lib.rs` - 2018 lines: full replay system with 28 tests

### 4.2 Determinism Verification
- [x] StateHasher (game_core/src/hasher.rs: compute_state_hash with fixed-seed AHasher, sorts HashSet/HashMap for determinism)
- [x] Executor wired to compute state hash before/after each command (state_hash_before, state_hash_after)
- [x] CommandHistory.set_last_state_hash_after() for post-execution hash recording
- [x] RoundTripVerifier (verify_timelines, states_match, verify_hash_chain)
- [x] DeterminismTestSuite (verify_serialization_roundtrip, verify_hash_stability, verify_clone_determinism, verify_no_float_dependency)
- [x] Cross-platform determinism: fixed-point Inches (no floats), fixed AHasher seeds, sorted collections
- [x] FuzzTester (fuzz_hash_stability: N iterations of direct/clone/serialize hash tests)

**Phase 4.2 Key Files:**
- `game_core/src/hasher.rs` - 571 lines: deterministic state hashing with 12 tests
- `determinism/src/lib.rs` - 1173 lines: verification tooling with 29 tests

### 4.3 Golden Test Scenarios
- [x] Edge cases (13 tests: empty state, clone, serialization roundtrip, fuzz 100 iterations, phase/round/player/CP/VP/wounds/death/battle-shock/outcome/turn-flags sensitivity)
- [x] Faction specifics (8 tests: Custodes faction hash, Ka'tah stances, Vaultswords profiles; World Eaters hash, Blessings of Khorne, fight-on-death queue)
- [x] Mission specifics (6 tests: all 6 missions unique hashes, deterministic setup, scoring progress, round scores)
- [x] Stratagem interactions (4 tests: usage changes hash, phase/turn/battle usage, order independence)
- [x] Active effects (2 tests: effect changes hash, different effect types diverge)
- [x] Command execution (3 tests: single command hash change, hash chain consistency, deterministic replay)
- [x] Replay integration (6 tests: recorder, JSON roundtrip, binary roundtrip, summary, identical diff, divergence detection)
- [x] Full scenario (5 tests: Custodes vs WE deterministic, different seeds diverge, serialization/clone/no-float/fuzz-50)
- [x] Cross-platform (2 tests: fixed-point position, measurement determinism)
- [x] Replay verification (1 test: hash chain continuity in replay frames)

**Phase 4.3 Key Files:**
- `determinism/tests/golden_scenarios.rs` - 51 integration tests

---

## Phase 5: Heuristic AI Engine (COMPLETE)

### 5.1 Action Abstraction Layer
- [x] MacroAction struct (command, intent, label, priority_hint)
- [x] TacticalIntent enum (33 variants covering all phases)
- [x] CandidateSet with owner/phase tracking
- [x] ActionGenerator with phase-specific candidate generation
- [x] Movement candidate generation (advance, retreat, objective grab, screen, reposition)
- [x] Shooting candidate generation (focus fire, split fire, overwatch)
- [x] Charge candidate generation (multi-charge, single target, heroic intervention)
- [x] Fight order candidate generation (pile in, melee attack, consolidation)
- [x] Stratagem candidate generation (all eligible stratagems)
- [x] Phase control candidates (end phase, pass turn)

### 5.2 Heuristic Evaluator
- [x] Feature extraction (EvalFeatures: 15+ feature categories from GameState)
- [x] Transposition table (Zobrist-style hashing, TT entry storage/probe, size config)
- [x] HeuristicWeights struct (20+ tunable weights with default/aggressive/defensive presets)
- [x] HeuristicEvaluator with 13 scoring terms (VP, objectives, kills, survival, position, etc.)
- [x] Evaluator trait (polymorphic evaluation interface for heuristic/future NNUE)
- [x] EvalBreakdown diagnostics with Display impl
- [x] Terminal state detection (victory, draw, tabling)
- [x] Objective evaluation (control, holding strength, contest pressure)
- [x] Unit value estimation (kill potential, survival odds, leader exposure)
- [x] Positional evaluation (charge threat, retaliation risk, reserve leverage)
- [x] Mission-aware weights (early/mid/late game scaling)
- [x] Move ordering priors (tactical intent priority, action priority hints)

### 5.3 Root Search and AI Worker
- [x] SearchConfig with presets (greedy, one_ply, negamax(depth))
- [x] SearchStats tracking (nodes, cutoffs, TT hits, depth)
- [x] SearchResult with PV, candidate scores, best_commands/best_intent helpers
- [x] GreedyAi (depth-0, evaluate all candidates)
- [x] OnePlySearch (depth-1 with heuristic root ordering, max_candidates limit)
- [x] NegamaxSearch (full negamax with alpha-beta pruning, TT probe/store, killer/history ordering)
- [x] SearchRoot orchestration with AiLevel enum (Greedy, OnePly, Negamax, NegamaxDepth)
- [x] KillerTable (two-slot per depth, record/probe/clear)
- [x] HistoryTable (per-TacticalIntent scoring with depth^2 bonus, aging)
- [x] MoveOrderer (TT moves, killers, history, tactical priority)
- [x] Fast state cloning (GameState Clone for search tree)
- [x] Deterministic chance sampling (seeded DiceRoller)
- [x] Node budget enforcement
- [x] AiWorker trait interface
- [x] Convenience functions (greedy_choose, one_ply_choose, negamax_choose)

**Phase 5 Key Files:**
- `eval_features/src/lib.rs` - ~700 lines: feature extraction from GameState
- `transposition/src/lib.rs` - ~620 lines: transposition table with Zobrist hashing
- `search_abstraction/src/lib.rs` - ~1400 lines: MacroAction, CandidateSet, ActionGenerator
- `eval_heuristic/src/lib.rs` - ~680 lines: HeuristicEvaluator with 13 scoring terms, 11 tests
- `search_ordering/src/lib.rs` - ~530 lines: KillerTable, HistoryTable, MoveOrderer, 12 tests
- `search_core/src/lib.rs` - ~640 lines: Greedy/OnePly/Negamax search, SearchRoot, AiWorker, 14 tests

---

## Phase 6: Stockfish-Like Search (COMPLETE)

### 6.1 Iterative Deepening
- [x] Iterative deepening with PV seeding (IterativeDeepeningSearch: depth 1→max with PV from previous iteration seeding move ordering)
- [x] Aspiration windows (narrow window around previous score, doubles on fail-high/fail-low)
- [x] Principal variation tracking (PvLine struct: tracks best line through negamax_pv with update_from)
- [x] Time management (TimeManager: soft/hard limits, dynamic adjustment for phase sharpness, PV stability, score gap, branching)

### 6.2 Transposition Table
- [x] Zobrist-style hash keys (from Phase 5: compute_state_hash in hasher.rs)
- [x] TT entry storage (from Phase 5: TranspositionTable with generation-based aging)
- [x] TT lookup in search (from Phase 5 + Phase 6: negamax_pv probes TT for cutoffs and best move ordering)
- [x] TT diagnostics (TTStats with occupancy, hit_rate, SearchDiagnostics captures TT state per search)

### 6.3 Move Ordering
- [x] Killer move heuristic (from Phase 5: two-slot KillerTable per depth)
- [x] History heuristic table (from Phase 5: HistoryTable with depth^2 bonus and aging)
- [x] Evaluator-assisted priors (from Phase 5: heuristic_order at root with position delta scoring)
- [x] Tactical priority boosts (from Phase 5: TacticalIntent ordering_priority + action priority_hint)

### 6.4 Tactical Extensions and Quiescence
- [x] Quiescence search (continues searching unstable positions at leaf nodes, stand-pat cutoff, limited qs depth)
- [x] Instability detection (is_position_unstable: reaction windows, mid-combat, charge resolution, fight sequencing)
- [x] Selective depth extensions (charges, fight order, stratagems, reactions: +1 ply, max_extensions cap)
- [x] Search diagnostics (SearchDiagnostics + IterationInfo: per-iteration depth/score/pv/nodes/time/nps/aspiration/pv_changes)

### 6.5 Performance
- [x] NPS calculation and performance tracking (TimeManager.nps, SearchStats.nps/time_elapsed_ms)
- [x] Parallel search preparation (SharedSearchState + LazySmpSearch: atomic stop flag, shared TT skeleton, worker coordination)

**Phase 6 Key Files:**
- `search_core/src/lib.rs` - ~3400 lines: IterativeDeepeningSearch, quiescence, extensions, PvLine, TimeManager, SearchDiagnostics, LazySmpSearch, 58 tests
- `search_core/Cargo.toml` - Added wh40k_geometry and wh40k_event_system dependencies

---

## Phase 7: NNUE Runtime (COMPLETE)

### 7.1 Feature Extraction
- [x] Feature schema (global 31, per-objective 30×6=180, per-unit 62×16=992, total 1203 features)
- [x] Relative positional features (distance-to-objectives, nearest-enemy, objective-pressure)
- [x] Matchup features (anti-armor relevance, anti-infantry relevance, melee threat matchup per unit)
- [x] extract_features() sparse representation (SparseFeature {index, value}, SparseFeatureVec)
- [x] Incremental feature diff (FeatureDiff: added/removed/changed features, compute_feature_diff())

### 7.2 NNUE Inference
- [x] Model artifact format (NnueModelArtifact with QuantizedWeights, ModelMetadata, NnueDimensions)
- [x] Model loading with schema validation (version check, dimension validation, weight count verification)
- [x] Forward pass (sparse→accumulator→hidden→scalar with ClippedReLU, i16/i8/i32 quantization)
- [x] Accumulator cache with incremental updates (NnueAccumulator: apply_diff for add/remove/change)
- [x] Evaluator trait (AnyEvaluator enum for zero-cost heuristic/NNUE swap, NnueEvaluator implements Evaluator)

### 7.3 Model Registry
- [x] Store, load, list, validate model artifacts (ModelRegistry with .nnue files + JSON index)
- [x] Bootstrap heuristic as generation 0 (bootstrap() creates random-initialized generation 0 model)

### 7.4 Integration
- [x] Wire NNUE into search (GreedyAiNnue, greedy_choose_nnue(), greedy_choose_nnue_model())
- [x] Evaluation benchmark (benchmark_heuristic_vs_nnue with BenchmarkResult, compare_evaluators)

**Phase 7 Key Files:**
- `eval_features/src/lib.rs` - Extended with NNUE sparse features: 1203-dim feature space, extract_sparse_features(), compute_feature_diff(), 6 new tests
- `eval_nnue/src/lib.rs` - Complete NNUE runtime: NnueModel, NnueEvaluator, ModelRegistry, quantized forward pass (1203→128→32→32→1), AnyEvaluator, 48 tests
- `search_core/src/lib.rs` - NNUE search integration: GreedyAiNnue, benchmark_heuristic_vs_nnue(), re-exports

---

## Phase 8: Self-Play and Training Bridge (COMPLETE)

### 8.1 Training Data Export
- [x] Shard format (TrainingShard, ShardHeader with version/schema/timestamps)
- [x] encode_state() (dense f32 vector, 1203 features)
- [x] encode_sparse_features() (sparse (u16, i16) pairs)
- [x] encode_legal_mask() (LegalMask over 528 fixed vocab with candidate mapping)
- [x] Action vocabulary encoding (33 TacticalIntent × 16 unit slots = 528 fixed vocabulary)
- [x] ShardWriter (batched bincode output with configurable shard size)
- [x] ShardReader (read_shard, read_all_shards, count_samples, validate_shard)
- [x] TrainingSample (sparse_features, legal_mask, chosen_action, score, outcome, perspective, progress)

### 8.2 Self-Play Runner
- [x] Match orchestration (play_single_game with full game loop, action selection, command execution)
- [x] Outcome labeling (+1.0 win, -1.0 loss, 0.0 draw per perspective)
- [x] Search diagnostic capture (SearchDiagnosticEntry with SearchStatsSnapshot per move)
- [x] Batch stepping (SelfPlayRunner.run() with SelfPlayReport aggregation)
- [x] Game variation (GameVariation.generate_configs: faction alternation, mission cycling, enhancement/secondary selection)
- [x] AI types (Greedy, OnePly, Negamax(depth), IterativeDeepening, Timed, GreedyNnue)
- [x] Throughput benchmark (benchmark_selfplay_throughput, 100+ games/hr)
- [x] Convenience functions (run_test_game, collect_training_data)

### 8.3 Gating Harness
- [x] Candidate vs baseline evaluation (GatingHarness.evaluate with faction alternation)
- [x] NNUE vs heuristic evaluation (evaluate_heuristic_vs_nnue)
- [x] Promotion gate (>55% win rate threshold, configurable)
- [x] Model lineage tracking (ModelLineage with save/load JSON, generation queries)
- [x] Elo estimation (expected_score, update_rating, estimate_elo_delta, confidence_interval, calculate_ratings)
- [x] GatingResult with win_rate, elo_delta, confidence intervals

### 8.4 Python Bridge
- [x] PyO3 trainer_bridge crate (cdylib + rlib, cargo check passes)
- [x] PyGameState class (reset, step, step_by_vocab_index, encode_state_dense/sparse, encode_legal_mask, terminal_result)
- [x] PyNnueWeights class (zeros, load, save, dimensions)
- [x] PyMatchResult, PyGatingResult, PyModelLineage classes
- [x] Module functions (play_game, run_selfplay_batch, benchmark, evaluate_candidate, load_shard, load_all_shards, count_samples, elo_delta, engine_constants)
- [x] Python training scripts (model.py, shard_loader.py, train.py, export_weights.py)
- [x] PyTorch NNUE model matching Rust architecture (1203→128→32→32→1, ClippedReLU, quantize/dequantize)
- [x] Training loop (MSE loss on outcome, LR scheduler, checkpointing, validation)
- [x] Weight export (float→quantized with proper scaling, .nnue artifact format)
- [x] Full pipeline CLI (generate → train → gate)
- [x] Maturin build config (pyproject.toml)

**Phase 8 Key Files:**
- `selfplay/src/lib.rs` - ~2900 lines: training data export, self-play runner, gating harness, Elo rating, model lineage, 60 tests
- `trainer_bridge/src/lib.rs` - ~1100 lines: PyO3 FFI bridge exposing engine API to Python, 10 tests
- `trainer_bridge/pyproject.toml` - Maturin build configuration
- `python/train_nnue/model.py` - PyTorch NNUE model definition with quantization
- `python/train_nnue/shard_loader.py` - DataLoader for training shards
- `python/train_nnue/train.py` - Training loop with validation, checkpointing, gating
- `python/train_nnue/export_weights.py` - Float↔quantized weight conversion and .nnue export

---

## Phase 9: GUI Productionization

### 9.1 WASM API
- [x] wasm_api crate via wasm-bindgen
- [x] View models (state -> TS-friendly JSON)
- [x] Error types with JsValue conversion (error.rs)
- [x] Conversion functions: GameState/Player/Unit/Model/Weapon/Board/Event -> ViewModels (conversions.rs)
- [x] WASM exports: create_match, load_scenario, get_state_snapshot, get_decision_surface, validate_action, apply_action, run_ai_decision, apply_ai_action, export_replay, load_replay, replay_step_forward, replay_step_backward, replay_get_info (lib.rs)
- [x] Thread-local state management (GameState, ReplayRecorder, ReplayPlayer, DecisionCache, AI result)
- [x] WASM compilation pipeline (wasm-pack build succeeds, getrandom js/wasm_js features, .cargo/config.toml for wasm32 target)
- [x] AI in Web Worker (engineWorker.ts dispatches all engine calls in dedicated worker thread)

### 9.2 React UI Shell
- [x] Project setup (Vite + React + TS + PixiJS + Zustand + Tailwind)
- [x] TypeScript types mirroring Rust view models (types/game.ts, types/worker-messages.ts)
- [x] WASM bridge + Web Worker engine communication (wasmBridge.ts, engineWorker.ts, workerClient.ts)
- [x] Zustand stores (gameStore.ts, setupStore.ts, replayStore.ts)
- [x] Game setup UI (SetupScreen, FactionSelect, EnhancementSelect, SecondarySelect, MissionSelect, DeploymentPanel)
- [x] Phase indicator + turn tracker (PhaseIndicator.tsx, TurnTracker.tsx)
- [x] Unit info panel (UnitInfoPanel.tsx)
- [x] Combat log (CombatLog.tsx)
- [x] Stratagem panel (StratagemPanel.tsx)
- [x] Action panel (ActionPanel.tsx)
- [x] Score board (ScoreBoard.tsx)
- [x] Blessing panel (BlessingPanel.tsx)
- [x] Shared components (Button, Panel, Tooltip, Modal, AppShell, Header, Sidebar)
- [x] Utilities (formatters.ts, colors.ts)

### 9.3 PixiJS Battlefield Renderer
- [x] Canvas renderer (44"x30", pan/zoom) (BattlefieldCanvas.tsx, constants.ts)
- [x] Board rendering (BoardRenderer.ts)
- [x] Terrain rendering (TerrainRenderer.ts)
- [x] Deployment zone overlays (DeploymentOverlay.ts)
- [x] Objective markers (ObjectiveRenderer.ts)
- [x] Unit/model sprites (UnitRenderer.ts, SpriteFactory.ts)
- [x] Movement previews (MovementPreview.ts)
- [x] Attack visualization (AttackVisualization.ts)
- [x] Charge preview (ChargePreview.ts)
- [x] Camera controls (CameraController.ts)

### 9.4 Input and Touch
- [x] Click/tap controls (InteractionLayer.ts)
- [x] Touch: long-press, pinch-zoom, drag-pan (TouchHandler.ts)

### 9.5 AI Controls and Replay Viewer
- [x] AI controls (AiControls.tsx, AiEvalBar.tsx)
- [x] Replay viewer (ReplayScreen.tsx, ReplayControls.tsx, ReplayTimeline.tsx)
- [x] Game end screen (GameEndScreen.tsx)

### 9.6 Verification
- [x] WASM compilation with wasm-pack (wasm-pack build succeeds, output in web/wasm-pkg)
- [x] Web TypeScript compilation (tsc --noEmit passes clean)
- [x] Web production build (vite build succeeds, 680KB main bundle + 20KB CSS)
- [x] Dev server runs (vite dev starts on port 3000)
- [x] All 1180 workspace tests still pass (no regressions)

---

## Phase 10: MCP Server and AlphaGo Expansion

### 10.1 MCP Server
- [ ] mcp_server crate (10 MCP tools)
- [ ] Observation model (public, private, debug)
- [ ] Session management
- [ ] MCP protocol compliance
- [ ] Cross-surface state verification

### 10.2 Native CLI
- [ ] CLI commands (play, benchmark, verify, selfplay)
- [ ] Headless execution

### 10.3 AlphaGo Expansion
- [ ] Stabilize action vocabulary
- [ ] Refine state tensor export
- [ ] Policy/value training in Python
- [ ] MCTS hybrid prototype
- [ ] Policy-guided search experiments

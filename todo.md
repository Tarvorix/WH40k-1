# WH40K Combat Patrol Engine - Implementation TODO

**Status: Phase 4 COMPLETE. Ready for Phase 5: Heuristic AI Engine.**
**Total tests passing: 956 | Workspace compiles clean | 0 clippy warnings**

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

## Phase 5: Heuristic AI Engine

### 5.1 Action Abstraction Layer
- [ ] MacroAction struct
- [ ] Movement candidate generation
- [ ] Shooting candidate generation
- [ ] Charge candidate generation
- [ ] Fight order candidate generation
- [ ] Stratagem candidate generation

### 5.2 Heuristic Evaluator
- [ ] Weighted scoring
- [ ] Objective evaluation
- [ ] Unit value estimation
- [ ] Positional evaluation
- [ ] Mission-aware weights
- [ ] Move ordering priors

### 5.3 Root Search and AI Worker
- [ ] Greedy AI
- [ ] One-ply search
- [ ] SearchRoot orchestration
- [ ] Fast state cloning
- [ ] Deterministic chance sampling
- [ ] Negamax 2-3 ply with alpha-beta
- [ ] AiWorker interface
- [ ] AI soak testing (1000+ games)

---

## Phase 6: Stockfish-Like Search

### 6.1 Iterative Deepening
- [ ] Iterative deepening with PV seeding
- [ ] Aspiration windows
- [ ] Principal variation tracking
- [ ] Time management

### 6.2 Transposition Table
- [ ] Zobrist-style hash keys
- [ ] TT entry storage
- [ ] TT lookup in search
- [ ] TT diagnostics

### 6.3 Move Ordering
- [ ] Killer move heuristic
- [ ] History heuristic table
- [ ] Evaluator-assisted priors
- [ ] Tactical priority boosts

### 6.4 Tactical Extensions and Quiescence
- [ ] Quiescence search
- [ ] Instability detection
- [ ] Selective depth extensions
- [ ] Search diagnostics

### 6.5 Performance
- [ ] Profile and optimize (10K+ NPS target)
- [ ] Parallel search preparation (lazy SMP design)

---

## Phase 7: NNUE Runtime

### 7.1 Feature Extraction
- [ ] Feature schema (global, per-objective, per-unit)
- [ ] Relative positional features
- [ ] Matchup features
- [ ] extract_features() sparse representation
- [ ] Incremental feature diff

### 7.2 NNUE Inference
- [ ] Model artifact format
- [ ] Model loading with schema validation
- [ ] Forward pass (sparse -> accumulator -> hidden -> scalar)
- [ ] Accumulator cache with incremental updates
- [ ] Evaluator trait (heuristic/NNUE swap)

### 7.3 Model Registry
- [ ] Store, load, list, validate model artifacts
- [ ] Bootstrap heuristic as generation 0

### 7.4 Integration
- [ ] Wire NNUE into search
- [ ] Evaluation benchmark (1000 positions)

---

## Phase 8: Self-Play and Training Bridge

### 8.1 Training Data Export
- [ ] Shard format
- [ ] encode_state()
- [ ] encode_legal_mask()
- [ ] Shard writer

### 8.2 Self-Play Runner
- [ ] Match orchestration
- [ ] Outcome labeling
- [ ] Search diagnostic capture
- [ ] Batch stepping
- [ ] Game variation
- [ ] Throughput benchmark (100+ games/hr)

### 8.3 Gating Harness
- [ ] Candidate vs baseline evaluation
- [ ] Promotion gate (>55% win rate)
- [ ] Model lineage tracking
- [ ] Elo estimation

### 8.4 Python Bridge
- [ ] FFI via PyO3
- [ ] Training loop in Python

---

## Phase 9: GUI Productionization

### 9.1 WASM API
- [ ] wasm_api crate via wasm-bindgen
- [ ] View models (state -> TS-friendly JSON)
- [ ] WASM compilation pipeline
- [ ] AI in Web Worker

### 9.2 React UI Shell
- [ ] Project setup (Vite + React + TS + PixiJS + Zustand + Tailwind)
- [ ] Game setup UI
- [ ] Phase indicator + turn tracker
- [ ] Unit info panel
- [ ] Combat log
- [ ] Stratagem panel

### 9.3 PixiJS Battlefield Renderer
- [ ] Canvas renderer (44"x30", pan/zoom)
- [ ] Terrain rendering
- [ ] Deployment zone overlays
- [ ] Objective markers
- [ ] Unit/model sprites
- [ ] Movement previews
- [ ] Attack visualization
- [ ] Charge preview

### 9.4 Input and Touch
- [ ] Click/tap controls
- [ ] Touch: long-press, pinch-zoom, drag-pan

### 9.5 AI Controls and Replay Viewer
- [ ] AI controls (toggle, difficulty, eval bar)
- [ ] Replay viewer (load, step, speed)

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

# WH40K Engine - Implementation TODO

**Status: Phase 16 COMPLETE — Boarding Actions Fully Implemented**
**Total tests passing: 1567+ (all workspace tests) | Workspace compiles clean | Web TypeScript clean**

#### Phase 16.8: Integration, Polish & Full Testing for Boarding Actions (2026-03-15) COMPLETE
- [x] Step 1: Extend replay system — added `game_mode` to ReplayHeader metadata, populated from GameState, included in diff comparison and summary export
- [x] Step 2: Extend selfplay — added `game_mode` field to MatchConfig, SelfPlayConfig, GatingConfig; branched on BA mode in play_single_game to call load_boarding_actions_scenario; updated GameVariation::generate_configs signature
- [x] Step 3: Extend native_api selfplay command — added `--mode` CLI flag (combat_patrol/boarding_actions/ba) to SelfPlayCmdConfig and clap arg
- [x] Step 4: Verify JSON data files — confirmed include_str! paths from wasm_api resolve correctly to content/boarding_actions/factions/*.json and root-level JSON files
- [x] Step 5: Update objectives JSON — filled in BA-21 (Power Network 4 conditions at 5VP), BA-33 (Salvation Shrines 15VP + Cut Off the Head 15VP), BA-05 (Access the Data 10VP/terminal + Rout threshold table + 20VP warlord), BA-06 (Humble the Machine 20VP/corrupted marker)
- [x] Step 6: Full workspace build and test — 1607 tests pass, 0 failures, workspace compiles clean, TypeScript clean

#### Phase 16.7: Boarding Actions AI Support — Search & Evaluation (2026-03-15) COMPLETE
- [x] Add BA-specific TacticalIntent variants to `search_abstraction/src/lib.rs` (OperateHatch, SecureObjective, DefendPosition, ControlChokepoint, FlankThroughHatch, ProjectLeaderAbility, EnterFromReserves)
- [x] Add ordering priorities for new BA intents
- [x] Add BA-specific candidate generation in ActionGenerator (hatchway ops, tactical manoeuvres, entry zone arrivals, battlefield command)
- [x] Add `classify_command` entries for BA commands (OperateHatchway, PerformTacticalManoeuvre, UseBattlefieldCommand, ArriveFromEntryZone)
- [x] Add `boarding_actions_heuristic()` function to `eval_heuristic/src/lib.rs` with secured objective, hatchway control, tactical manoeuvre, and battlefield command scoring terms
- [x] Integrate BA heuristic into main evaluate() with mode guard
- [x] Add BA heuristic weights to HeuristicWeights struct
- [x] Add `BoardingFeatures` struct and `extract_boarding_features()` to `eval_features/src/lib.rs`
- [x] Update search_ordering intent_index for new BA variants
- [x] Update selfplay intent_to_index / index_to_intent / INTENT_COUNT for BA variants
- [x] Update wasm_api tactical_intent_to_string for BA variants
- [x] Build workspace clean (0 warnings, 0 errors)
- [x] All existing tests still pass (1559+ tests, 0 failures)

#### Phase 16.6: Web Frontend — Main Menu, Army Builder, Board Renderer (2026-03-15) ✅ COMPLETE
- [x] Add Boarding Actions types to `web/src/types/game.ts` (BoardingFaction, BoardingDetachment, BoardingUnitDatasheet, SelectedUnit, etc.)
- [x] Update `Screen` type to include 'menu' and 'boarding_setup'
- [x] Change initial screen to 'menu' in `web/src/store/gameStore.ts`
- [x] Create `web/src/components/menu/MainMenu.tsx` — main menu with Combat Patrol and Boarding Actions buttons
- [x] Create `web/src/store/boardingSetupStore.ts` — Zustand store for full boarding setup flow (8 steps)
- [x] Create `web/src/components/boarding-setup/BoardingSetupScreen.tsx` — complete 8-step army builder:
  - [x] Step 1: select_faction — faction cards with army rules
  - [x] Step 2: select_detachment — detachment cards with stratagems/enhancements count
  - [x] Step 3: build_army — dual-panel roster builder with points bar, size selectors, add/remove
  - [x] Step 4: select_enhancements — universal + detachment enhancements assigned to CHARACTER units (max 2)
  - [x] Step 5: designate_warlord — CHARACTER unit selection as Warlord
  - [x] Step 6: opponent_setup — AI opponent faction/detachment selection with auto-pick
  - [x] Step 7: select_mission — 15 missions with type/tag filters
  - [x] Step 8: ready — full roster summary with start battle button
- [x] Create `web/src/components/boarding/BoardingBoardView.tsx` — board renderer with hatchway status display
- [x] Update `web/src/App.tsx` — add MainMenu and BoardingSetupScreen routes
- [x] TypeScript compiles clean with zero errors

#### Phase 16.5: Mission Loader & Scoring Engine (2026-03-15) ✅ COMPLETE
- [x] Create `boarding_rules/src/mission_loader.rs` — unified BoardingMissionPackage from 3 JSON assets (11 tests)
- [x] Create `boarding_rules/src/scoring.rs` — BoardingScoringEngine for progressive/end-game scoring (18 tests)
- [x] Create `boarding_rules/src/mission_mechanics.rs` — per-mission mechanics (radiation, lighting, corruption, etc.) (48 tests)
- [x] Update `boarding_rules/src/lib.rs` to declare new modules
- [x] All 198 boarding_rules tests pass including integration tests with actual JSON files

---

### Phase 16: Boarding Actions (2026-03-15)

Add Boarding Actions as a second game mode with main menu, army builder, all 16 missions, 4 factions / 6 detachments / ~40 units, and AI support.

#### Phase 16.1: Game Mode Foundation & Core Type Extensions ✅ COMPLETE
- [x] Add `HatchwayId`, `CompartmentId`, `RegionId`, `DetachmentId` to `core_types/src/ids.rs`
- [x] Add `GameMode`, `HatchwayState`, `TacticalManoeuvre`, `EntryZoneRole` enums to `core_types/src/enums.rs`
- [x] Add `MovementAction::OperateHatchway`, `ArriveFromEntryZone` variants
- [x] Add `HatchwayOrientation`, `BoardingMissionType` enums
- [x] Add `Inches::BA_MOVE_CAP`, `BA_HATCHWAY_ENGAGEMENT_RANGE`, `BA_OBJECTIVE_RANGE`, `BA_HATCHWAY_OPERATE_RANGE`, `BA_BATTLEFIELD_COMMAND_RANGE`, `BoardDimensions::BOARDING_ACTIONS` to measurements
- [x] Add `game_mode: GameMode` and `mode_state: Option<ModeState>` to `GameState`
- [x] Define `ModeState`, `CombatPatrolModeState`, `BoardingActionsModeState` with full runtime fields
- [x] Define `BattlefieldCommandLink`, `BoardingMissionSpecificState`
- [x] Add helper methods: `is_boarding_actions()`, `boarding_state()`, `boarding_state_mut()`
- [x] Update all 20+ `GameState` constructors across 10 files with new fields
- [x] All existing tests pass (1336+ tests, 0 failures)

#### Phase 16.2: Boarding Map Geometry & Content Schema ✅ COMPLETE
- [x] Create `geometry/src/boarding.rs` — `BoardingMap`, `Compartment`, `WallSegment`, `Hatchway`, `EntryZone`, `SpecialRegion` (19 methods + 3 helpers + 20 tests)
- [x] Implement spatial queries: `compartment_containing()`, `is_wall_between()`, `check_los_boarding()`, `shortest_path_distance()` (Dijkstra), `check_cover_boarding()`, etc.
- [x] Create `boarding_content` crate — deserialize maps/objectives/tags JSON (11 tests, all JSON files load)
- [x] Create faction asset types: `BoardingFactionDef`, `BoardingDetachmentDef`, `BoardingUnitDatasheet`
- [x] Convert all 6 faction markdown files → JSON under `content/boarding_actions/factions/`
  - space_marines_terminator_assault.json (16 datasheets)
  - world_eaters_boarding_butchers.json (8 datasheets)
  - world_eaters_skullsworn.json (5 datasheets)
  - csm_champions_of_chaos.json (6 datasheets)
  - csm_underdeck_uprising.json (7 datasheets)
  - astra_militarum_tempestus.json (5 datasheets)
- [x] Comprehensive datasheet validation test (all 47 datasheets verified: weapons, stats, points)

#### Phase 16.3: Army Builder Engine (Roster Validation) ✅ COMPLETE
- [x] Create `boarding_rules` crate with `roster.rs` — `BoardingPatrol`, `SelectedUnit`, `EnhancementAssignment`
- [x] Implement `BoardingRosterValidator::validate()` — 12 validation rules:
  - Points cap (500), faction/detachment match, unit legality, unit count limits
  - No duplicate EPIC HEROES, model count validity, points cost accuracy (half-points rule)
  - Warlord requirements, enhancement rules (max 2, no dupes, CHARACTER only, no EPIC HEROES)
  - 6 universal BA enhancements recognized, conditional limits (Jakhals/Berzerkers)
- [x] 26 tests covering all validation rules against actual faction JSON data
- [x] Tested with World Eaters, Space Marines, and Astra Militarum rosters

#### Phase 16.4: Boarding Actions Engine Core (Rules Overlay) ✅ COMPLETE
- [x] Add 5 Command variants: `OperateHatchway`, `PerformTacticalManoeuvre`, `UseBattlefieldCommand`, `ArriveFromEntryZone`, `BoardingMissionAction`
- [x] Updated all Command impl methods (player, units_involved, category, Display)
- [x] Add 3 DecisionType variants: `BoardingTacticalManoeuvre`, `BoardingHatchwayOperation`, `BoardingMissionAction`
- [x] Add 8 GameEvent variants: `HatchwayOperated`, `TacticalManoeuvrePerformed`, `BattlefieldCommandActivated`, `EntryZoneArrival`, `LightingChanged`, `CompartmentVented`, `ObjectiveCorrupted`, `BoardingMissionActionPerformed`
- [x] Basic validator/executor match arms for all new commands
- [x] `hatchway.rs` — can_operate, resolve_operation (roll-off), can_close (split-unit), check_opening_engagement (12 tests)
- [x] `movement.rs` — effective_movement (9" cap), is_legal_move (wall/hatch/distance), deep_strike ignoring walls, scouts (16 tests)
- [x] `visibility.rs` — check_visibility (walls/hatches/models), check_cover, indirect_fire_removed, blast_visible_count, charge_must_be_visible (11 tests)
- [x] `leader_adapter.rs` — leaders_do_not_attach, validate_battlefield_command, led_by_disabled (10 tests)
- [x] `stratagems.rs` — 5 universal BA stratagems as data, availability checks (13 tests)
- [x] `tactical_manoeuvres.rs` — can_perform, can_secure_site, apply_secure_site, apply_set_to_defend, apply_set_overwatch (17 tests)
- [x] `BoardingActionsModeState` fully defined in Phase 16.1 with all runtime fields
- [x] Total: 1,486 tests passing, 0 failures

#### Phase 16.5: All 16 Missions ✅ COMPLETE
- [x] `mission_loader.rs` — loads all 15 missions from JSON, BA-1/BA-01 normalization (11 tests)
- [x] `scoring.rs` — progressive scoring (rounds 2-5), 7 end-game methods, VP cap 90+10 (18 tests)
- [x] `mission_mechanics.rs` — 22 mechanic variants, radiation table, corruption, lighting, venting, prison break, multi-level (48 tests)
- [x] `ScenarioLoader::load_boarding_actions_scenario()` in game_core (2 tests)
- [x] BA-21, BA-33, BA-05, BA-06 scoring data captured from Wahapedia
- [x] Total: 1,567 tests passing, 0 failures

#### Phase 16.6: WASM API & Web Frontend ✅ COMPLETE
- [x] Add 5 WASM functions: `create_boarding_match()`, `get_boarding_factions()`, `get_boarding_missions()`, `validate_boarding_roster()`, `get_game_mode()`
- [x] Extend GameView with `game_mode`, `hatchway_states`, `secured_objectives`
- [x] Add main menu screen to `App.tsx` — Combat Patrol / Boarding Actions selection
- [x] Create `MainMenu.tsx` component
- [x] Create `BoardingSetupScreen.tsx` — 8-step army builder UI
- [x] Create `boardingSetupStore.ts` — Zustand store for setup state
- [x] Create `BoardingBoardView.tsx` — board view with hatchway display
- [x] TypeScript compiles clean

#### Phase 16.7: AI Support ✅ COMPLETE
- [x] 7 new TacticalIntent variants + BA candidate generation in ActionGenerator
- [x] BA-specific heuristic: secured objectives, hatchway control, manoeuvres, battlefield command, chokepoints
- [x] BoardingFeatures extraction (11 fields)
- [x] Updated search_ordering, selfplay intent mappings, WASM intent strings

#### Phase 16.8: Integration, Polish & Full Testing ✅ COMPLETE
- [x] `boarding_content`, `boarding_rules` added to workspace
- [x] Replay system: `game_mode` in ReplayHeader
- [x] Selfplay: `game_mode` in MatchConfig/SelfPlayConfig/GatingConfig, BA branch in play_single_game
- [x] Native API: `--mode` CLI flag for selfplay
- [x] Objectives JSON updated: BA-21, BA-33, BA-05, BA-06 scoring complete
- [x] All tests passing, no CP regressions

---

### Phase 15: Rules Audit Fixes (2026-03-14)

Full audit of code vs 40k_revised.md, CP_Rules.md, Custodes.md, and Frenzied_Reavers.md.
~90% of core mechanics correctly implemented. The following bugs and gaps were identified.

#### BUGS — Critical (affect game correctness)

- [x] **BUG 1: Battle-shock not cleared at start of Command Phase**
  - File: `game_core/src/phase.rs:236-247`
  - Rule: 40k_revised.md §4.2 — "Duration: Until start of that player's next Command Phase"
  - Issue: `start_command_phase()` only tests units below half-strength. It does NOT clear `battle_shocked = false` for ALL of the active player's units before running new tests. If a unit was battle-shocked last round but is no longer below half-strength, the flag persists incorrectly.
  - Fix: At start of Command Phase, clear `battle_shocked = false` for ALL active player's units before re-testing.

- [x] **BUG 2: Skull Takers secondary NEVER scores**
  - File: `game_core/src/scoring.rs:277`
  - Rule: Frenzied_Reavers.md — "At the end of the Fight phase"
  - Issue: `score_secondary_objectives()` is only called at end of Command Phase (phase.rs:139-146). Skull Takers checks `state.current_phase == Phase::Fight` which is never true when called during Command Phase. VP is never awarded.
  - Fix: Call secondary scoring at end of Fight Phase for fight-phase secondaries, or add a separate end-of-fight scoring hook.

- [x] **BUG 3: Raise the Vexillas scores at wrong timing**
  - File: `game_core/src/scoring.rs:326-365`
  - Rule: Custodes.md — "at the end of YOUR turn" (not end of Command Phase)
  - Issue: Scored during `score_secondary_objectives()` called at end of Command Phase. Should be scored at end of the player's turn (after Fight Phase). Evaluates objective control before the player has had Movement/Shooting/Charge/Fight to capture objectives.
  - Fix: Score Raise the Vexillas at end of turn instead of end of Command Phase.

- [x] **BUG 4: Devastating Wounds excess mortal wounds carry over (should be lost)**
  - File: `game_core/src/combat.rs:1261-1390`
  - Rule: 40k_revised.md §8.7 — "Excess damage CARRIES OVER to other models UNLESS from HAZARDOUS or DEVASTATING WOUNDS"
  - Issue: In `apply_mortal_wounds_devastating()`, when a model is destroyed (line 1371 `break`), the outer `while mortal_pool > 0` loop continues to the next model, carrying over remaining mortal wounds. For Devastating Wounds, excess per model should be LOST.
  - Example: DW deals 3 damage to a 2W model. Model dies after 2 wounds. The remaining 1 MW incorrectly carries over to next model.
  - Fix: After model destroyed in DW, set `mortal_pool = 0` or break from outer loop.

- [x] **BUG 5: Go to Ground duration too long**
  - File: `game_core/src/stratagem.rs:476-477`
  - Rule: CP_Rules.md §11 — "Until end of phase" (opponent's Shooting phase)
  - Issue: Code uses `EffectDuration::UntilStartOfNextCommandPhase`, persisting the 6++ invulnerable save and Benefit of Cover through the entire rest of the opponent's turn AND into the next Command Phase. Should only last until end of current Shooting Phase.
  - Fix: Change duration to `UntilEndOfPhase`.

#### MISSING MECHANICS — Medium Priority

- [x] **MISSING 1: Battle-shock Desperate Escape enforcement on Fall Back**
  - Rule: 40k_revised.md §5.5 — "Battle-shocked Fall Back: Take Desperate Escape test for EVERY model in the unit (before any models move)"
  - Status: ALREADY IMPLEMENTED in executor.rs:637-771. Both battle-shocked (all models) and non-battle-shocked (per enemy model passed) paths exist with FLY/TITANIC exemptions and dedicated tests.

- [x] **MISSING 2: Display of Might — Break Their Spirit mission rule**
  - Rule: CP_Rules.md Mission 6 — "Insane Bravery can only be used if target unit within 6\" of WARLORD"
  - Status: Not implemented. Insane Bravery has no Mission 6-specific restriction in stratagem validation.
  - File: `game_core/src/stratagem.rs` (Insane Bravery validation)

- [x] **MISSING 3: Archeotech Recovery — automatic objective removal at round boundaries**
  - Rule: CP_Rules.md Mission 2 — "Start of round 3: Defender removes Gamma NML objective. Start of round 4: Gamma removed; Attacker selects Beta NML objective. Start of round 5: Beta removed."
  - Status: Scoring function exists but automatic objective removal at specific round boundaries is not wired into phase transitions.
  - File: `game_core/src/phase.rs` (end_battle_round or start of round hooks)

- [x] **MISSING 4: CP gain cap per battle round**
  - Rule: 40k_revised.md §4.1 — "Outside the standard 1 CP gain, each player can only gain 1 additional CP per Battle Round from any source"
  - Status: Not enforced. Players can gain unlimited extra CP from abilities (Warrior Exemplar, Supply Lines, Retrieve Intelligence, A Worthy Skull).
  - File: `game_core/src/state.rs` (PlayerState::gain_cp) — need to track extra CP gained per round and cap at 1.

- [x] **MISSING 5: Grenade stratagem full restrictions**
  - Rule: CP_Rules.md — "Unit hasn't Advanced, Fallen Back, or shot; not in Engagement Range; target visible and not in ER of friendly units"
  - Status: Grenade stratagem rolls 6D6 without validating these conditions.
  - File: `game_core/src/stratagem.rs` (Grenade validation/execution)

- [x] **MISSING 6: Heroic Intervention WALKER restriction for VEHICLEs**
  - Rule: CP_Rules.md — "VEHICLE must be WALKER" to use Heroic Intervention
  - Status: Not validated. Any eligible unit can use Heroic Intervention regardless of VEHICLE/WALKER status.
  - File: `game_core/src/stratagem.rs` (Heroic Intervention validation)

- [x] **MISSING 7: Consecrated Ground incremental scoring wiring**
  - Rule: Custodes.md — "+3VP each time enemy unit destroyed, -1VP each time Custodes model destroyed"
  - Status: Helper functions `score_consecrated_ground_kill()` and `score_consecrated_ground_loss()` exist in scoring.rs but need to be confirmed wired into the executor when units/models are destroyed.
  - File: `game_core/src/executor.rs` (unit/model destruction handlers)

- [x] **MISSING 8: Bloodlust condition — Jakhals must have lost models**
  - Rule: Frenzied_Reavers.md — "One JAKHALS unit that lost one or more models from the attacking unit's attacks"
  - Status: The condition that the Jakhals unit must have lost models from the shooting is not validated before allowing the stratagem.
  - File: `game_core/src/stratagem.rs` (Bloodlust validation)

#### MINOR ISSUES

- [x] **Typo**: `stratagem.rs:429` — Comment says "Counter-Operative" instead of "Counter-Offensive"
- [x] **Tank Shock player agency**: Improved auto-selection to pick best target (most models). TODO for human play: expose as player choice via UI.
- [x] **Tank Shock MONSTER eligibility**: Changed from static VEHICLE keyword to custom validation accepting VEHICLE or MONSTER. Vorrakh can now use Tank Shock.
- [x] **Secured Objective logic review**: Rewrote with proper `secured_by` field on ObjectiveMarker. Once secured by BATTLELINE, persists even when BATTLELINE moves away. Only broken when opponent actively controls via OC superiority.

---

### Phase 14: Rules Compliance, Combat Resolution, Scoring & Perturabo Pipeline (2026-03-14)

#### Engine Bug Fixes
- [x] Fix AI charge phase infinite loop (full sub-phase flow: Declare → Roll 2D6 → Move)
- [x] Fix charge rolls — use actual 2D6 dice instead of AI-chosen values
- [x] Fix deployment skipping — block EndPhase during PreBattle when units still undeployed
- [x] Fix MakeChargeMove loop — validate ER against actual target models + non-target enemy proximity
- [x] Fix Ka'tah stance loop — filter units that already chose a stance
- [x] Fix Vaultswords profile loop — filter models that already chose a profile
- [x] Add generic pre-validation filter on ALL ActionGenerator exit paths
- [x] Cap deployment search depth to 1 (prevents 3+ min search per game)
- [x] Add stuck-state detection to selfplay (state hash + repeated action)
- [x] Add safety limits to web UI runAiTurn (max iterations + repeated action detection)

#### Scoring
- [x] Wire scoring into phase transitions (primary + secondary at end of Command Phase)
- [x] Fix double-scoring — only active player scores at their own Command Phase
- [x] Wire EndOfTurn scoring for BR5 2nd player (split timing)
- [x] Wire endgame scoring (Battle Ready bonus, mission-specific bonuses)
- [x] Implement objective Secured mechanic with BATTLELINE requirement (CP_Rules §12.3)
- [x] Update objective control_status at end of each Command Phase

#### Combat Resolution
- [x] Wire shooting resolution — ResolveShootingAttack per weapon-target pair
- [x] Wire melee resolution — DeclareMeleeTargets + ResolveMeleeAttack per weapon
- [x] Only models in engagement range can fight (CP_Rules §8.3)
- [x] Add Pile In (3" toward closest enemy before attacks)
- [x] Add Consolidate (3" toward closest enemy after attacks)
- [x] Wire Vaultswords profile — only chosen profile's weapon attacks

#### Missing Mechanics Wired
- [x] Deadly Demise — Vorrakh D3 mortal wounds on destruction (roll D6, trigger on 6)
- [x] Overwatch firing — reaction windows now generate FireOverwatch candidates
- [x] Heroic Intervention — reaction windows generate UseStratagem for characters
- [x] Missing stratagem candidates: Epic Challenge, Insane Bravery, Berserk Resilience, Bloodlust, Inescapable Vengeance, Overawing Magnificence
- [x] Verified already working: Ka'tah stances in combat, Martial Excellence blessing, Total Carnage fight-on-death

#### Perturabo Training Pipeline
- [x] IterativeDeepeningSearch now supports AnyEvaluator (heuristic OR NNUE)
- [x] New `with_nnue()` constructor for ID search with NNUE model
- [x] Gating uses ID+NNUE vs ID+Heuristic (proper Perturabo evaluation)
- [x] Gen 0 trained: 1K ID games → 121K samples → 50 epochs → 78.2% accuracy
- [x] Fix PyTorch 2.9 compatibility (ReduceLROnPlateau, torch.round)
- [x] Apple Metal (MPS) GPU auto-detected for training

#### Pipeline Commands
```bash
# Generate training data (ID selfplay, ~350 games/hr)
cargo run -p wh40k_native_api --release -- selfplay --games 1000 --ai id --output-dir ./shards/gen0

# Train NNUE (~2 min on Apple Metal)
cd python && python3 -m train_nnue.train train --shard-dir ../engine/shards/gen0 --output-dir ../engine/checkpoints/gen0 --epochs 50

# Gate (ID+NNUE vs ID+Heuristic)
python3 -m train_nnue.train gate ../engine/checkpoints/gen0/gen0.nnue --num-games 25
```

#### Remaining (future work)
- Deployment positions could be smarter (corners vs near objectives)
- Movement GUI: range preview on unit selection

---

### Phase 13: Per-Model Rendering & Formation System (2026-03-13)

- [x] Step 1: Formation generator in geometry crate (`geometry/src/formation.rs`)
- [x] Step 2: Engine deploy with formation (`game_core/src/executor.rs` — `apply_place_unit`, `apply_arrive_from_reserves`)
- [x] Step 3: Movement validation per-model board bounds (`game_core/src/validator.rs`)
- [x] Step 4: Frontend per-model rendering (`web/src/renderer/UnitRenderer.ts`, `constants.ts`)
- [x] Step 5: Frontend interaction & preview updates (`InteractionLayer.ts`, `MovementPreview.ts`)
- [x] Step 6: Full build & test verification

---

### Phase 12: Free Movement/Deployment + Mobile Responsive Layout (2026-03-13)

#### Issue #1: Free Movement & Deployment (click-to-move/deploy)
- [x] Step 1: Add `Inches::from_f64()` helper in `core_types/src/measurements.rs`
- [x] Step 2: Add `submit_place_unit` WASM endpoint in `wasm_api/src/lib.rs`
- [x] Step 3: Add `submit_normal_move` WASM endpoint in `wasm_api/src/lib.rs`
- [x] Step 4: WASM Bridge + Worker + Client TypeScript wiring
- [x] Step 5: Game Store `submitDeploy` and `submitMove` actions
- [x] Step 6: Board click handler in InteractionLayer
- [x] Step 7: Wire board click in BattlefieldCanvas
- [x] Step 8: Update ActionPanel for deployment unit selection
- [x] Step 9: Deployment zone visual feedback
- [x] Step 10: Touch tap/drag differentiation in CameraController

#### Issue #2: Mobile Responsive Layout
- [x] Step 11: Create `useIsMobile` hook
- [x] Step 12: Create `useContainerSize` hook
- [x] Step 13: Make BattlefieldCanvas responsive (ResizeObserver)
- [x] Step 14: Create BottomTabBar component
- [x] Step 15: Create SlideUpPanel component
- [x] Step 16: Create MobileGameLayout component
- [x] Step 17: Update GameScreen with responsive switching
- [x] Step 18: Update Sidebar for responsive visibility
- [x] Step 19: Compact Header for mobile
- [x] Step 20: Viewport meta tag update
- [x] Step 21: CSS safe area adjustments

---

### Critical Fix: WASM Build Path (2026-03-13)
- [x] Fixed `wasm:build` npm script — `--out-dir` was relative to crate root, not working directory
  - Old: `--out-dir ../../web/wasm-pkg` → resolved to `engine/web/wasm-pkg` (WRONG)
  - New: `--out-dir ../../../web/wasm-pkg` → resolves to `web/wasm-pkg` (CORRECT)
- [x] All previous WASM rebuilds were silently writing to wrong directory
- [x] Cleaned stale `engine/web/wasm-pkg` directory
- [x] WASM now correctly exports `create_match` with all 10 parameters (faction, mission, seed, enhancements, secondaries, patrol squads)
- [x] Deployment, enhancement selection, secondary selection, and patrol squad selection should now work end-to-end

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
- [x] Primary scoring framework (score_primary_objectives dispatches to mission-specific scorers with ScoringTiming parameter)
- [x] ScoringTiming enum (EndOfCommandPhase, EndOfTurn) for BR5 split timing across missions
- [x] Mission 1 - Clash of Patrols: Take and Hold, 5VP/objective (max 15VP) BR2-5, BR5 split timing, Retrieve Intelligence rule (WARLORD + controlled objective = 1CP, each objective once)
- [x] Mission 2 - Archeotech Recovery: Recover Archeotech, 5VP/objective (max 15VP) BR2-5, Irradiated Power Cells (NML objectives removed BR3-5), +10VP last NML objective
- [x] Mission 3 - Forward Outpost: Vital Ground, 5VP/NML obj + 10VP enemy DZ obj (max 15VP), BR5 split timing, Sabotage Enemy Comms (block Command Re-roll)
- [x] Mission 4 - Scorched Earth: Raze and Ruin threshold scoring (5VP control 1+ / 5VP control more / 10VP razed), BR5 split timing, raze validation (Attacker can't raze A, Defender can't raze B, no enemies within 3")
- [x] Mission 5 - Sweeping Raid: Priority Targets, 5VP/objective (max 15VP) BR2-4 only, Supply Lines (4+ D6 = 1CP), end-of-battle bonus (Attacker 5VP C/10VP D, Defender 5VP B/10VP A)
- [x] Mission 6 - Display of Might: Symbolic Sites, 5VP×4 scoring categories (max 20VP), BR5 split timing, Break Their Spirit (Insane Bravery within 6" WARLORD), Claim Sites (CHARACTER on NML objectives)
- [x] create_mission() lookup by MissionId (1-6)
- [x] All missions: EndOfCommandPhase scoring per CP_Rules.md, Round 5 second player at end of turn via ScoringTiming
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

## Phase 10: MCP Server and AlphaGo Expansion (COMPLETE)

### 10.1 MCP Server
- [x] mcp_server crate (10 MCP tools: create_session, load_scenario, get_observation, list_legal_actions, apply_action, step_until_decision, get_replay_log, get_score, reset, end_session)
- [x] Observation model (Public, Player(N), Debug view modes with GameObservation, PlayerObservation, UnitObservation, ModelObservation, etc.)
- [x] Session management (SessionManager with create/get/end, SessionConfig, cached action generation)
- [x] MCP protocol compliance (JSON-RPC 2.0 over stdio, Content-Length framing, initialize/tools/list/tools/call)
- [x] Cross-surface state verification (state hash in observations, replay frame verification)

**Phase 10.1 Key Files:**
- `mcp_server/src/protocol.rs` - JSON-RPC 2.0 types, MCP protocol types, 10 tool argument structs, tool definitions with JSON schemas, 11 tests
- `mcp_server/src/error.rs` - McpError enum (14 variants), JSON-RPC/tool error conversion, 6 tests
- `mcp_server/src/observation.rs` - ViewMode (Public/Player/Debug), GameObservation builder, 3 tests
- `mcp_server/src/session.rs` - Session/SessionManager, scenario loading, action generation/execution, 11 tests
- `mcp_server/src/tools.rs` - Tool dispatch, 10 handler implementations, AI stepping, 11 tests
- `mcp_server/src/server.rs` - McpServer, stdio transport, message routing, 10 tests
- `mcp_server/src/main.rs` - Binary entry point with tracing

### 10.2 Native CLI
- [x] CLI commands (play, benchmark, verify, selfplay) with clap argument parsing
- [x] Headless execution (AI vs AI games, performance benchmarks, replay verification, self-play data generation)

**Phase 10.2 Key Files:**
- `native_api/src/play.rs` - PlayConfig/PlayResult, run_play() game loop with AI search, replay export
- `native_api/src/benchmark.rs` - BenchmarkConfig/BenchmarkResult, run_benchmark() with win/loss/VP statistics
- `native_api/src/verify.rs` - VerifyConfig/VerifyResult, replay determinism verification
- `native_api/src/selfplay_cmd.rs` - SelfPlayCmdConfig/Result, training data generation with ShardWriter
- `native_api/src/main.rs` - Clap CLI with 4 subcommands

### 10.3 AlphaGo Expansion
- [x] Stabilize action vocabulary (528 fixed vocab = 33 TacticalIntent × 16 unit slots, action_to_vocab_index, encode_legal_mask)
- [x] Refine state tensor export (1203 sparse features, encode_state, encode_sparse_features for policy/value training)
- [x] Policy/value training in Python (PolicyValueNet dual-head, PolicyValueLoss combined loss, quantization, JSON export)
- [x] MCTS hybrid prototype (MctsSearch with PUCT selection, arena-based tree, Dirichlet noise, heuristic priors, AiWorker integration)
- [x] Policy-guided search experiments (PolicyValueModel in Rust with quantized inference, MCTS training pipeline, AlphaGo-style generate→train→gate loop)

**Phase 10.3 Key Files:**
- `search_core/src/mcts.rs` - MctsConfig, MctsNode, MctsTree (arena), MctsSearch (PUCT/MCTS), AiWorker impl, Dirichlet noise, heuristic priors, 13 tests
- `eval_nnue/src/lib.rs` - PolicyValueModel (quantized inference: 1203→128→64→528 policy + 64→1 value), masked_softmax, save/load JSON
- `selfplay/src/lib.rs` - PolicyValueSample, PolicyValueShard, encode_policy_target, encode_uniform_policy_target
- `python/train_nnue/policy_value_model.py` - PolicyValueNet (PyTorch), PolicyValueLoss, quantization, JSON export
- `python/train_nnue/mcts_train.py` - Full AlphaGo training pipeline: PolicyValueShardDataset, train/validate epochs, LR scheduling, checkpointing, generate→train→gate pipeline, model inspection, CLI

---

## Content Audit & Corrections (Post-Phase 10)

### Fabricated Content Fix
- [x] Removed 3 fabricated BlessingOfKhorne variants (WrathfulDevotion, WarpBlades, UnbridledBloodlust) — only 3 per Frenzied_Reavers.md
- [x] Renamed RageFuelledInvaders → RageFuelledInvigoration per Frenzied_Reavers.md
- [x] Removed 2 fabricated KaTahStance variants (Salvus, Kaptaris) — only Dacatarai + Rendax per Custodes.md
- [x] Fixed blessing effects: RageFuelledInvigoration = pile-in/consolidation 6", TotalCarnage = fight on death 4+, MartialExcellence = Sustained Hits 1
- [x] Fixed UI setupStore.ts: all enhancement names, secondary objectives, mission names/descriptions sourced from faction YAMLs and CP_Rules.md
- [x] Fixed mission_runtime scoring timing: EndOfTurn → EndOfCommandPhase per CP_Rules.md
- [x] Implemented all 6 missions in mission_runtime with correct rules, scoring, and special rules per CP_Rules.md Section 13
- [x] All references updated across event_system, faction_runtime, game_core/executor.rs
- [x] 29 mission_runtime tests passing, all engine tests passing

### Core Rules Audit Fixes (40k_revised.md)
- [x] CRITICAL: Fixed battle-shock pass/fail inversion in phase.rs — was `roll <= Ld` (wrong), now `roll >= Ld` per §4.2
- [x] Added Benefit of Cover AP 0 restriction in combat.rs — Sv 3+ or better models don't get cover vs AP 0 per §13.1
- [x] Fixed Hazardous damage in combat.rs — was 3 MW for CHARACTER/MONSTER/VEHICLE and 1 MW for others, now flat 3 MW for all per §11.13
- [x] Added advance move distance validation in validator.rs — checks M + advance_roll per §5.4
- [x] Added 4 new cover tests (AP 0 denied for Sv 3+, AP 0 allowed for Sv 4+, AP-1 allowed for Sv 3+, AP 0 denied for Sv 2+)
- [x] Updated hazardous test to verify infantry also takes 3 MW
- [x] All workspace tests passing (217 game_core tests, 1200+ total)

### Scoring Logic Fixes (scoring.rs) — ALL 6 Missions Fully Implemented
- [x] Added ScoringTiming enum (EndOfCommandPhase, EndOfTurn) for BR5 split timing
- [x] Added players_to_score() helper for BR5 1st/2nd player timing logic
- [x] Updated score_primary_objectives() to accept ScoringTiming parameter
- [x] Added MissionProgress tracking fields: retrieved_intelligence_objectives, razed_this_turn, command_reroll_blocked, symbolic_site_claims

**Mission 1 - Clash of Patrols (Take and Hold):**
- [x] score_clash_of_patrols(): Rewritten with 5VP per objective, max 15VP cap, BR5 split timing
- [x] evaluate_retrieve_intelligence(): New — from BR2, select controlled objective, gain 1CP if WARLORD on battlefield, each objective once only

**Mission 2 - Archeotech Recovery (Recover Archeotech):**
- [x] score_archeotech_recovery(): Added 15VP cap per CP_Rules.md
- [x] score_archeotech_endgame(): Fixed to check remaining NML objectives (was incorrectly using center objective)

**Mission 3 - Forward Outpost (Vital Ground):**
- [x] score_forward_outpost(): CORRECTED formula — 5VP per NML objective + 10VP for enemy DZ objective (max 15VP). Was wrongly 5VP per any objective + 10VP bonus. Added BR5 split timing
- [x] evaluate_sabotage_enemy_comms(): New — if controlling opponent's DZ objective at end of turn, blocks opponent's Command Re-roll until end of battle

**Mission 4 - Scorched Earth (Raze and Ruin):**
- [x] score_scorched_earth(): COMPLETELY REWRITTEN to threshold-based scoring per CP_Rules.md:
  - 5VP if control 1+ objectives
  - 5VP if control more objectives than opponent
  - 10VP if razed an objective this turn
  - Added BR5 split timing
- [x] validate_raze_objective(): New — full raze validation: BR2+, 2+ objectives remain, must control, no enemies within 3", Attacker can't raze A, Defender can't raze B
- [x] score_raze_objective(): Updated — now returns tracking event (0VP), 10VP incorporated into threshold scoring

**Mission 5 - Sweeping Raid (Priority Targets):**
- [x] score_sweeping_raid(): Fixed to BR 2-4 only (was BR 2-5), added max 15VP cap
- [x] score_sweeping_raid_endgame(): Fixed to use specific objective labels — Attacker 5VP for C + 10VP for D, Defender 5VP for B + 10VP for A
- [x] evaluate_supply_lines(): Added Command Phase mechanic — if controlling own DZ objective, roll D6, on 4+ gain 1CP
- [x] Added is_objective_in_own_dz() helper function

**Mission 6 - Display of Might (Symbolic Sites):**
- [x] score_display_of_might(): Rewritten with 4 threshold-based categories (5VP each, max 20VP), added BR5 split timing:
  - Category 1: Control 1+ objectives → 5VP
  - Category 2: Control 2+ objectives → 5VP
  - Category 3: 1+ symbolic sites (NML objectives) claimed by CHARACTER → 5VP
  - Category 4: Same CHARACTER claimed same site for 2+ consecutive turns → 5VP
- [x] record_display_of_might_claims(): Added claim recording function for tracking CHARACTER claims across rounds

**Tests:**
- [x] 46 scoring tests passing (25 new tests covering all 6 missions, VP caps, BR5 split timing, raze validation, sabotage comms)
- [x] All workspace tests passing (1200+ total, 0 failures)

---

## Full Rules Compliance Audit (2026-03-12)

### CRITICAL — Wrong Unit Stats (Code vs Datasheets)

- [x] #1: Vorrakh Movement 10" — FIXED: scenario.rs MoveCharacteristic::from_inches(10)
- [x] #2: Vorrakh Invulnerable Save 4+ — FIXED: scenario.rs InvulnerableSave::FOUR_PLUS
- [x] #3: Master of Executions Movement 8" — FIXED: scenario.rs MoveCharacteristic::from_inches(8)
- [x] #4: Khorne Berzerkers Movement 8" — FIXED: scenario.rs MoveCharacteristic::from_inches(8)
- [x] #5: Khorne Berzerkers Model Count 10 — FIXED: scenario.rs (0..10).map()
- [x] #6: Jakhals Movement 7" — FIXED: scenario.rs MoveCharacteristic::from_inches(7)
- [x] #7: Allarus Custodians Model Count 2 — FIXED: scenario.rs (0..2).map()

### CRITICAL — Missing Implementations (Placeholders)

- [x] #8: ALL weapon profiles populated — FIXED: Full weapon data for all units in both factions (scenario.rs)
- [x] #9: Engagement range check on move destinations — FIXED: validate_normal_move, validate_advance_move, validate_fall_back all check within_engagement_range_2d against all enemy models (validator.rs)
- [x] #10: Weapon range check for shooting targets — FIXED: validate_declare_shooting_targets checks dist > weapon_range (validator.rs)
- [x] #11: LOS/visibility check for shooting targets — FIXED: validate_declare_shooting_targets uses board.check_los(), Indirect Fire exception handled (validator.rs)
- [x] #12: 9" distance check for Deep Strike/Reserves — FIXED: validator.rs validates distance from all enemies
- [x] #13: Reserves destroyed end of Round 3 — FIXED: phase.rs destroys reserve units at end of Round 3
- [x] #14: Charge move geometric validation — FIXED: validate_charge_move checks distance cap, ER with targets, no ER with non-targets (validator.rs)
- [x] #15: Desperate Escape tests — FIXED: executor.rs apply_fall_back() has both battle-shocked (every model) and non-battle-shocked (per enemy in ER) with TITANIC/FLY exemptions
- [x] #16: Fall Back distance validated — FIXED: validator.rs checks distance <= M characteristic
- [x] #17: Command Re-roll integrated — FIXED: combat.rs has command_reroll_active/defender_command_reroll_active fields, re-roll logic in hit/wound/save/damage rolls, GameEvent::CommandRerollUsed emitted
  - [x] Add EffectType::CommandReroll variant to effect.rs
  - [x] Update stratagem.rs to use EffectType::CommandReroll instead of Custom(...)
  - [x] Add GameEvent::CommandRerollUsed to event_system
  - [x] Add command_reroll_active field to AttackContext in combat.rs
  - [x] Integrate re-roll into resolve_hit_roll (re-roll failed hit)
  - [x] Integrate re-roll into resolve_wound_roll (re-roll failed wound)
  - [x] Integrate re-roll into save roll section (re-roll failed save)
  - [x] Integrate re-roll into damage roll (re-roll damage)
  - [x] Return command_reroll_consumed flag from resolve_attack_batch
  - [x] Update executor.rs to pass command_reroll_active and consume effect

### CRITICAL — Rules Logic Bugs

- [x] #18: Devastating Wounds individual — FIXED: combat.rs tracks each DW individually, excess lost per model per attack
- [x] #19: Stealth all ranges — FIXED: combat.rs applies -1 to hit unconditionally (no distance check)
- [x] #20: Hazardous verified correct — 40k_revised.md §11.13 says 3 mortal wounds for ALL models. Code was already correct.
- [x] #21: Go to Ground invulnerable — FIXED: stratagem.rs uses EffectType::GrantInvulnerableSave(6), executor.rs applies it
- [x] #22: Advanced non-Assault blocked — FIXED: validate_resolve_shooting_attack checks can_fire_after_advance() (validator.rs)

### SIGNIFICANT — Rules Not Fully Enforced

- [x] #23: Fight phase alternation — FIXED: validate_select_unit_to_fight checks fight_alternation_next_player, validates designated player has eligible units (validator.rs)
- [x] #24: Pile-in/Consolidate closer to enemy — FIXED: validate_pile_in_closer_to_enemy checks each model ends closer to nearest enemy, 3" max move (validator.rs)
- [x] #25: Precision targets CHARACTER — FIXED: select_allocation_target targets wounded leader first, then any alive leader (combat.rs)
- [x] #26: Precision visibility — FIXED: unit-level LOS validated at shooting declaration (validator.rs), per-model visibility satisfied by unit-level check. Documented in combat.rs
- [x] #27: Blast restriction — FIXED: validate_declare_shooting_targets checks no friendly unit in ER of target for Blast weapons (validator.rs)
- [x] #28: Pistol exclusivity — FIXED: validate_declare_shooting_targets prevents mixing Pistol and non-Pistol for non-MONSTER/VEHICLE (validator.rs)
- [x] #29: Heroic Intervention as charge — FIXED: executor.rs apply_heroic_intervention_move uses roll_2d6 charge roll, charge distance check, no Charge bonus
- [x] #30: Tank Shock toughness — FIXED: stratagem.rs uses vehicle_toughness from the VEHICLE unit, mortal wounds applied to enemy unit
- [x] #31: Mission-specific objectives — FIXED: scenario.rs create_mission_objectives() with per-mission layouts
- [x] #32: Titanic keyword — FIXED: keywords.rs Titanic = 47, KeywordSet TITANIC = 1 << 47

### MODERATE — Faction Content Gaps

- [x] #33: Secondary objectives populated — FIXED: content_schema has full SecondaryObjectiveSchema for both factions with scoring rules, conditions, timing, and max VP
- [x] #34: Enhancement effects data-driven — FIXED: EnhancementSchema.effects populated with RulePrimitive definitions; scoring.rs apply_enhancement() implements runtime effects
- [x] #35: Mission rule logic connected — FIXED: scoring.rs has dedicated scoring functions for all 6 missions; content_schema has ScoringRule definitions
- [x] #36: Praesidium Shield +1W — FIXED: scenario.rs Custodian Guard Wounds::new(4) (3 base + 1 shield)

### MINOR — Reporting/Edge Cases

- [x] #37: SaveRollMade modified field — VERIFIED CORRECT: in 10th ed, AP modifies save characteristic not roll. Raw roll IS the correct value.
- [x] #38: Advance roll validated 1-6 — FIXED: validator.rs checks advance_roll < 1 || > 6
- [x] #39: Charge roll validated 2-12 — FIXED: validator.rs checks roll < 2 || > 12
- [x] #40: Indirect Fire cover — FIXED: executor.rs computes indirect_fire_no_los from weapon ability + board.check_los(), auto-grants cover for non-visible targets
- [x] #41: Benefit of Cover ranged-only — FIXED: executor.rs sets target_has_cover = false for melee
- [x] #42: Hazardous comment — FIXED: weapons.rs "3 mortal wounds allocated to selected model" with §11.13 citation

---

## Phase 11: Full Wiring Audit — Fix All Stubs and Broken Data Flows

**Status: COMPLETE**
**Source: Complete codebase audit 2026-03-13**
**Total tests passing: 1317 | Workspace compiles clean | All 30 items fixed**

### CRITICAL — Game Cannot Be Played

- [x] #C1: Deployment non-functional — FIXED: `generate_setup_candidates()` fully implemented with PlaceUnit commands, deployment zone sampling, defender-first alternation, and SetupComplete transition (search_abstraction/lib.rs)
- [x] #C2: Deployment zones sent to frontend — FIXED: `board_to_view_with_deployment()` populates deployment zone polygons from DeploymentConfig (wasm_api/conversions.rs)
- [x] #C3: Enhancement and secondary selections wired — FIXED: Full chain UI → workerClient → engineWorker → wasmBridge → WASM create_match → apply_enhancement → PlayerState.secondary_choice (wasm_api/lib.rs, gameStore.ts, workerClient.ts, engineWorker.ts, wasmBridge.ts)
- [x] #C4: Shooting target/weapon selection implemented — FIXED: `generate_shooting_candidates()` generates DeclareShootingTargets with weapon+target pairs, range/LOS/pistol checks (search_abstraction/lib.rs)
- [x] #C5: Stratagem generation implemented — FIXED: `generate_stratagem_candidates()` generates UseStratagem for Command Re-roll, Go to Ground, Grenade, Counter-offensive, Rapid Ingress, Gilded Spear, Horrifying Butchery (search_abstraction/lib.rs)

### HIGH — Core Mechanics Missing or Broken

- [x] #H1: ScoreObjective executor — FIXED: `apply_score_objective()` awards 5 VP, updates primary_vp, records round score (executor.rs)
- [x] #H2: RazeObjective executor — FIXED: `apply_raze_objective()` awards 5 VP, removes objective from board, sets razed_this_turn (executor.rs)
- [x] #H3: AllocateWound executor — FIXED: `apply_allocate_wound()` sets model AllocationStatus::WoundedAllocated (executor.rs)
- [x] #H4: AssignOverwatchTarget — FIXED: `apply_assign_overwatch_target()` derives charging unit from ChargeDeclared event log, marks overwatch used, pops reaction window (executor.rs)
- [x] #H5: PlaceUnit deployment zone validation — FIXED: Checks `state.deployment_config.zone_for(player).contains(position)` (validator.rs)
- [x] #H6: DeclareMeleeTargets validation — FIXED: Validates unit exists, belongs to player, targets in engagement range, alive, on battlefield (validator.rs)
- [x] #H7: ResolveMeleeAttack validation — FIXED: Validates attacker/target exist, on battlefield, weapon exists (validator.rs)
- [x] #H8: ChooseKaTahStance validation — FIXED: Verifies AdeptusCustodes keyword, valid stance name (validator.rs)
- [x] #H9: ChooseVaultswordsProfile validation — FIXED: Verifies BladeChampion keyword, valid profile name (validator.rs)
- [x] #H10: AllocateWound validation — FIXED: Validates model exists and is alive (validator.rs)
- [x] #H11: AssignOverwatchTarget validation — FIXED: Validates reaction window, overwatch not used, unit exists (validator.rs)
- [x] #H12: ScoreObjective/RazeObjective validation — FIXED: Verifies objective exists on board (validator.rs)
- [x] #H13: StartPhase phase ordering — FIXED: Enforces correct sequence PreBattle→Command→Movement→Shooting→Charge→Fight→Command (validator.rs)

### MEDIUM — Features Exist But Incomplete

- [x] #M1: Consecrated Ground secondary wired — FIXED: `score_consecrated_ground_kill()` called on enemy unit destruction (+3VP), `score_consecrated_ground_loss()` called per Custodes model destroyed (-1VP) (executor.rs)
- [x] #M2: Warrior Exemplar triggered — FIXED: D6 roll on unit kill, 3+ = 1CP, already wired at executor.rs:1799-1821
- [x] #M3: Scorched Earth objective removal — FIXED: `apply_raze_objective()` removes objective via `board.objectives.retain()` (executor.rs:1997)
- [x] #M4: ActionPanel empty state — FIXED: Shows "Waiting for opponent..." when decisionSurface has 0 actions (ActionPanel.tsx)
- [x] #M5: Advance roll variable — FIXED: Generates candidates for rolls [2, 4, 6] covering D6 range instead of hardcoded 4 (search_abstraction/lib.rs)
- [x] #M6: Fallback positions expanded — FIXED: Generates 7 retreat positions: straight back, diagonal back-left/right, lateral left/right, nearest objective, away from nearest enemy (search_abstraction/lib.rs)
- [x] #M7: Ka'tah stances — VERIFIED CORRECT: Dacatarai and Rendax are the only two stances per Custodes.md. Hardcoded values match the rules exactly.
- [x] #M8: Vaultswords profiles — VERIFIED CORRECT: Behemor, Hurricanus, and Victus are the only three profiles per Custodes.md. Hardcoded values match the rules exactly.
- [x] #M9: Charge candidates unlimited — FIXED: Removed `.take(3)` limit, all viable targets within 12" get charge candidates (search_abstraction/lib.rs)
- [x] #M10: BlessingPanel player filtering — FIXED: Filters by `decision_owner` instead of first player with blessings (BlessingPanel.tsx)

### LOW — UI Polish Gaps

- [x] #L1: Setup Ready screen shows selections — FIXED: Displays enhancement name, secondary objective name, and mission number alongside factions (SetupScreen.tsx)
- [x] #L2: Game End screen shows choices — FIXED: Added "Battle Choices" card with enhancement and secondary for each player (GameEndScreen.tsx)
- [x] #L3: Weapon dedup by ID — FIXED: Changed `findIndex` from `x.name === w.name` to `x.id === w.id` for both ranged and melee (UnitInfoPanel.tsx)
- [x] #L4: MovementPreview 0 movement — FIXED: Added `&& unit.movement > 0` guard to skip drawing circle for immobile units (MovementPreview.ts)

### POST-AUDIT — Custodes Faction Composition Fix

- [x] Custodes patrol squad selection (Choose One: Wardens OR Allarus) — FIXED: Full stack implementation
  - Engine: `create_custodes_units()` now accepts `patrol_squad_choice` param, conditionally creates Wardens (squad 0) or Allarus (squad 1). Default = Wardens.
  - Engine: `load_scenario_with_squads()` added to accept patrol squad choices per player
  - WASM API: `create_match()` now accepts `patrol_squad_a`/`patrol_squad_b` params
  - Frontend: New `PatrolSquadSelect.tsx` component with Wardens vs Allarus selection UI
  - Frontend: Setup flow updated: faction → enhancement → secondary → patrol squad → mission → ready
  - Frontend: `setupStore.ts` extended with `playerPatrolSquad` state + `selectPatrolSquad` action
  - Frontend: `gameStore.ts` passes patrol squad choice through to engine
  - Frontend: Worker pipeline (workerClient → engineWorker → wasmBridge) updated with patrol squad params
  - Frontend: GameEndScreen shows patrol squad choice per player
  - Types: `PlayerView` (Rust + TS) extended with `patrol_squad_choice` field
  - Tests: 4 new tests (allarus selection, wardens selection, allarus model count, updated unit counts)
  - Source: Custodes.md §2 — Fixed (Tristraen + Guard) + Choose One (Wardens 3 models OR Allarus 2 models)
  - World Eaters: No patrol squad choice needed (all units fixed per Frenzied_Reavers.md)
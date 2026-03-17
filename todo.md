# WH40K Engine — FULL AUDIT RESULTS

**Date: 2026-03-16 | Updated: 2026-03-17**
**Audited against: 40k_revised.md, CP_Rules.md, boarding_actions_complete_v3.md, boarding_actions_missions_complete_v3.md, Frenzied_Reavers.md, Custodes.md, AI_Primer.md**

**Progress: ~52 of 93 issues FIXED across 10 commits (2026-03-16/17)**
**See docs/rule_coverage_matrix.md for current status of every rule**

**FIXED items marked with ~~strikethrough~~ below.**

---

# SECTION 1: PERTURABO PIPELINE (CRITICAL — TRAINING IS BROKEN)

## P-1: CRITICAL — Self-play uses GreedyAi instead of IterativeDeepeningSearch
- **Spec**: AI_Primer §12-13 says `ai_type="iterative_deepening", max_depth=3`
- **Code**: `collect_training_data()` hardcodes `SelfPlayConfig::default_greedy()`
- **File**: selfplay/src/lib.rs:2847
- **Impact**: ALL training data is from depth-0 random-quality play. Model learns from noise.

## P-2: CRITICAL — collect_training_data API missing params
- **Spec**: `collect_training_data(num_games, output_dir, ai_type, max_depth, model_generation)`
- **Code**: `run_selfplay_batch(num_games, output_dir)` — only 2 params
- **File**: trainer_bridge/src/lib.rs:1114
- **Impact**: Cannot configure search depth or model from Python

## P-3: CRITICAL — Gating uses GreedyNnue vs Heuristic, not proper search
- **Spec**: AI_Primer §14 — NNUE vs NNUE with proper search
- **Code**: `evaluate_heuristic_vs_nnue()` — depth-0 greedy on both sides
- **File**: trainer_bridge/src/lib.rs:1200
- **Impact**: Gating doesn't test whether NNUE improves search quality

## P-4: CRITICAL — Game mode hardcoded to CombatPatrol
- **Code**: `game_mode: GameMode::CombatPatrol` hardcoded in match config
- **File**: trainer_bridge/src/lib.rs:1086
- **Impact**: Cannot generate Boarding Actions training data at all

## P-5: HIGH — Training hyperparams don't match spec
- **Spec**: weight_decay=1e-4, gradient_clipping=1.0, warmup_epochs=5, patience=10, min_lr=1e-6
- **Code**: weight_decay=1e-5, NO clipping, NO warmup, patience=5, NO min_lr
- **File**: python/train_nnue/train.py:158,241

## P-6: HIGH — Feature normalization mismatch
- **Code path 1**: encode_state() divides sparse values by 1000.0
- **Code path 2**: shard_loader.py passes raw i16 values as float (no division)
- **Files**: selfplay/src/lib.rs:320, python/train_nnue/shard_loader.py:59
- **Impact**: Training and inference see different feature scales

## P-7: MEDIUM — AI_Primer.md stale in ~20 places
- Says 1203 features (should be 1209), 528 action vocab (should be 640), 33 intents (should be 40)
- SearchConfig defaults differ from spec (max_candidates 30 vs 64, chance_samples 3 vs 8)

## P-8: MEDIUM — AiWorker trait missing Send bound (blocks parallel selfplay)

---

# SECTION 2: CORE 40K RULES (movement, shooting, charge, fight, morale)

## WRONG implementations (produce incorrect results):

## R-1: Fight phase alternation starts with WRONG player
- **Rule**: Non-active player picks first (40k_revised.md §10.1)
- **Code**: `init_fight_alternation(state.active_player)` — active player picks first
- **File**: phase.rs:412-414

## R-2: Charge move only requires reaching ONE target, should require ALL
- **Rule**: Must end within ER of EVERY declared target (§9.4)
- **Code**: `in_er_of_any_target` — breaks on first match
- **File**: validator.rs:1504-1527

## R-3: ALL models fight in melee regardless of position
- **Rule**: Only models within ER of enemy or in b2b chain can fight (§10.5)
- **Code**: Counts ALL alive models with weapon as attacking_model_count
- **File**: executor.rs:1639-1644

## R-4: Deadly Demise always rolls D3 instead of using datasheet value
- **Rule**: Deadly Demise X deals X mortal wounds (§12.2)
- **Code**: Always rolls D3, ignores unit.deadly_demise field
- **File**: executor.rs:1970-1979

## R-5: Counter-Offensive stratagem has wrong timing and wrong effect
- **Rule**: After enemy fights, one unit fights NEXT in alternation
- **Code**: DuringPhase timing, grants FightsFirst (wrong step)
- **File**: stratagem.rs:162-174, 430-441

## R-6: Desperate Escape test counts enemies in ER, should count model-crossings
- **Rule**: Test per model moving OVER an enemy (§5.5)
- **Code**: Rolls per enemy model in ER
- **File**: executor.rs:745-833

## R-7: Blessings of Khorne "Double 2+" requires matching dice, should be any two >= 2
- **Rule**: Frenzied_Reavers.md §3 example shows 2+3 for "Double 2+"
- **Code**: `a == b && a >= 2` — requires matching pair
- **File**: faction_runtime/src/lib.rs:188-199

## R-8: Archeotech Recovery (Mission 2) Beta objective timing wrong
- **Rule**: Beta selected at round 4, removed at round 5
- **Code**: Beta selected AND removed at round 5
- **File**: phase.rs:574-626

## R-9: Big Guns Never Tire -1 hit missing for units shooting AT engaged Monster/Vehicle
- **Rule**: External units shooting at engaged Monster/Vehicle also suffer -1 (§7.5)
- **Code**: Only applies -1 when the Monster/Vehicle itself shoots out
- **File**: combat.rs:647-652

## MISSING implementations (not coded at all):

## R-10: No path collision detection — units teleport through models
- **Rule**: Cannot move through enemy models or friendly Monster/Vehicle (§5.3)
- **File**: executor.rs, validator.rs — no intermediate point checks

## R-11: No terrain height/climbing system
- **Rule**: Terrain >2" must climb, count vertical distance (§5.7)
- Not implemented anywhere

## R-12: No pivot costs for non-round bases (§5.6)

## R-13: No Surge Moves (§5.10)

## R-14: No Transport system — entire subsystem absent (§6.1-6.5)

## R-15: No Aircraft movement rules (min 20", no stationary/advance/fallback) (§16.1-16.2)

## R-16: No Infiltrators deployment (§12.4)

## R-17: No Scouts pre-game move (§12.5)

## R-18: No Redeployments mechanism (§1.4)

## R-19: No FightsFirst-to-RemainingCombats subphase transition
- SubPhase::RemainingCombats exists but is never assigned

## R-20: No per-model closer-to-target enforcement for charge moves (§9.4)

## R-21: No unit coherency check during charge/pile-in/consolidation moves

## R-22: No "must end in base-to-base contact if possible" for charge/pile-in/consolidation

## R-23: No melee target validation per engagement range (§10.5)
- Any enemy unit on the board can be declared as melee target

## R-24: No per-model weapon selection enforcement in melee (§10.5)

## R-25: No Extra Attacks weapon ability enforcement (§11.18)

## R-26: No "model destroyed if forced to end in ER" rule (§1.6)

## R-27: No end-of-turn coherency check with model removal (§1.7)

## R-28: No per-phase stratagem dedup — same stratagem usable multiple times (§15.1)

## R-29: No battle-shocked stratagem targeting block (§4.3, §15.1)

## R-30: Locked in Combat — non-Pistol/BGNT shooting at engaged enemies not blocked (§7.4)

## R-31: No Psychic weapon PSYKER restriction (§12.11)

## R-32: No generic aura ability system (§12.10)

## R-33: No simultaneous rule ordering system (§2.5)

## R-34: Strategic Reserves missing "within 6" of battlefield edge" (§14.2)

## R-35: No Torrent + Indirect Fire prohibition (§11.16)

## R-36: Save improvement cap (+1 max) not generalized beyond cover (§8.4)

## R-37: Heroic Intervention missing 6" proximity check and target validation

## R-38: Epic Challenge stratagem stored but never applies Precision to melee attacks

## R-39: Stealth ability doesn't verify ALL models in unit have it

## R-40: Pile-in/Consolidation — models already in b2b should not pile in (§10.4, §10.6)

---

# SECTION 3: COMBAT PATROL SPECIFIC

## C-1: Plasma pistol supercharge profile missing for Khorne Berzerkers
- **File**: scenario.rs:1004-1018

## C-2: Vorrakh Deadly Demise is D3 (variable), stored as flat 3
- **File**: scenario.rs:913

## C-3: Master of Executions "A Worthy Skull" — re-roll hits/wounds vs CHARACTER not wired

## C-4: Scorched Earth raze decision not triggered from phase machine
- Validation exists but phase.rs never offers the raze choice

## C-5: Display of Might (Mission 6) — no per-round 20VP cap enforced

## C-6: Missions 2-6 all use standard deployment instead of mission-specific layouts
- **File**: scenario.rs:181-197

## C-7: Bane of the Craven missing MONSTER/VEHICLE exclusion and Battle-shocked -1 modifier

## C-8: Martial Ka'tah Dacatarai + Hurricanus Sustained Hits stacking unverified

## C-9: Grenade stratagem targeting restrictions not validated

---

# SECTION 4: BOARDING ACTIONS SPECIFIC

## BA-1: CRITICAL — Prison Break test logic INVERTED
- **Rule**: Roll >= Toughness = FAIL
- **Code**: Roll >= Toughness = SUCCESS
- **File**: boarding_rules/src/mission_mechanics.rs:498-503

## BA-2: FLY suppression not implemented — FLY keyword never stripped

## BA-3: Deep Strike round/count limits not enforced (rounds 2-3 only, max 1/round)

## BA-4: Distance measured straight-line instead of path-around-walls
- **Rule**: Measure shortest legal path around walls (§3.2)
- **Code**: Uses Euclidean distance
- **File**: boarding_rules/src/movement.rs:141-145

## BA-5: Returning destroyed models limit missing (max 1 per unit per round)

## BA-6: Pile-In/Consolidation BA visibility changes not implemented (§3.6)

## BA-7: Attack allocation restriction for unwounded targets (visible model) missing (§3.3)

## BA-8: Objective marker range should be 1" horizontal in BA (§3.2)

## BA-9: ALL 6 BA enhancement effects unimplemented — names only, no game effects

## BA-10: BA-32 Desperate Measures CP gain mechanic missing

## BA-11: BA-32 Furnace burner mortal wound 3/unit cap missing

## BA-12: BA reserves via empty entry zones not implemented (§7.5)

## BA-13: Secured objective state machine not implemented (§3.4)

## BA-14: BA-01 Inaccessible Area movement restriction missing

## BA-15: BA-04 Prison Cells unit-placement restrictions missing

## BA-16: BA-05 Multi-level cross-board visibility block missing

## BA-17: BA-06 All 4 corruption consequence EFFECTS unimplemented (data only)

## BA-18: BA deployment sequence (alternating one unit, defender first) missing

## BA-19: Underdog 30-point threshold function missing

## BA-20: Destroyed points threshold table may be wrong (0/15/40/60/80 vs spec 0/15/30/45)

---

# SECTION 5: FRONTEND / WASM WIRING

## F-1: BROKEN — InteractionLayer click-catcher hardcoded to 880x600 (CP size)
- BA board is 960x560 — far-right 4" of BA board is unclickable
- **File**: web/src/renderer/InteractionLayer.ts:32

## F-2: All 6 BA factions render as gray/"Unknown"
- Only Custodes (0) and World Eaters (1) have colors/names
- **File**: web/src/utils/colors.ts:2, web/src/utils/formatters.ts:56

## F-3: "Play Again" after BA game sends to CP setup, not BA setup
- **File**: web/src/components/game-end/GameEndScreen.tsx:43

## F-4: GameEndScreen hardcodes Custodes/WorldEaters colors for all games

## F-5: ChargePreview renderer implemented but never wired into BattlefieldCanvas

## F-6: TypeScript GameView type missing game_mode, hatchway_states, secured_objectives fields

## F-7: No BA-specific scoring display (secured objectives, tactical manoeuvres)

## F-8: BoardingBoardView component is dead code (475 lines, never imported)

---

# SECTION 6: TEST QUALITY

## T-1: rule_coverage_matrix.md is EMPTY — tracking doc never filled in

## T-2: ~35 tests are TRIVIAL — test flags/constants instead of actual rule enforcement
- Examples: fell_back_cannot_shoot only tests TurnFlags, not validator
- charge_roll_valid_range tests literal 2 <= 12, not actual dice
- battle_shock only sets flag true/false, never rolls 2D6

## T-3: Deadly Demise test only checks field exists, never tests actual mechanic

## T-4: Transport tests use Position::within_range, never invoke embark/disembark commands

## T-5: BA Deep Strike timing test creates local Vec[2,3] and asserts .contains(2)

## T-6: Underdog bonus test has tautology — `is_some() || is_none()` always true

## T-7: Zero behavioral tests for any detachment-specific stratagem effect

## T-8: Zero behavioral tests for any BA enhancement effect

## MAJOR COVERAGE GAPS (rules with zero real test coverage):
- Desperate Escape mortal wounds
- Battle-shock test resolution (2D6 vs Leadership)
- Deadly Demise mechanic (roll on death, 6" range, MW application)
- Damage overflow rules (excess doesn't carry to next model)
- Mortal wounds bypass saves
- Transport capacity enforcement
- Modifier cap enforcement (-1/+1 on hit/wound)
- Unit coherency enforcement (2" for 2-6 models, 2 models for 7+)
- Advance roll (M + D6 distance)

---

# TOTAL ISSUE COUNT

| Category | Critical | High | Medium | Low | Total |
|----------|----------|------|--------|-----|-------|
| Perturabo Pipeline | 4 | 2 | 2 | 0 | 8 |
| Core 40K Rules (WRONG) | 3 | 4 | 2 | 0 | 9 |
| Core 40K Rules (MISSING) | 2 | 8 | 21 | 0 | 31 |
| Combat Patrol | 2 | 3 | 4 | 0 | 9 |
| Boarding Actions | 2 | 6 | 12 | 0 | 20 |
| Frontend/WASM | 1 | 3 | 4 | 0 | 8 |
| Test Quality | 0 | 3 | 5 | 0 | 8 |
| **TOTAL** | **14** | **29** | **50** | **0** | **93** |

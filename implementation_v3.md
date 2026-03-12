# 40K Combat Patrol Engine - Full Implementation Plan

## Purpose

This document is a production-oriented implementation plan for a **rules-accurate Warhammer 40,000 Combat Patrol engine** with a **Stockfish-style search engine**, **NNUE-style evaluator**, **TypeScript sprite-capable GUI**, and a clean path to a later **AlphaGo/AlphaZero-style policy/value self-play system**.

The design target is **not** a prototype, tutorial, or beginner engine. The target is a complete architecture that can be implemented into a serious, testable, replayable, extensible engine.

The initial game target is **Combat Patrol** because it gives a bounded, fixed-force rules surface while preserving the core turn/phasing, timing windows, stratagem interactions, reserve flow, objective scoring, battle-shock, and mission logic of 10th edition 40K. The engine architecture is designed so that Combat Patrol is the first content layer, not a dead-end fork.

---

# 1. Strategic Goals

## 1.1 Primary goals

1. **100% rules accuracy for Combat Patrol**
2. **Deterministic authoritative engine**
3. **Headless simulation and replay verification**
4. **Strong search AI with NNUE-style evaluation**
5. **TypeScript front end usable on browser, tablet, and phone**
6. **Sprite-capable rendering path**
7. **Architecture extensible to full 40K**
8. **Architecture ready for AlphaGo/AlphaZero-style training later**

## 1.2 Non-goals for v1

1. Full 2000-point list building
2. All factions at launch
3. Full 3D scene rendering
4. Literal continuous freeform search over every possible tabletop micro-position
5. AlphaZero-first development before the deterministic engine is complete

## 1.3 Why Combat Patrol first

Combat Patrol is the right first surface because it keeps:
- fixed forces
- fixed 5-round game length
- pre-battle declarations
- reserve logic
- mission scoring
- normal 40K phase structure
- faction stratagems and enhancements
- enough complexity to force a real engine

But it avoids the immediate explosion of:
- full list building
- detachments at scale
- huge board states
- maximal faction combinatorics

---

# 2. Final Recommended Stack

## 2.1 Core recommendation

- **Engine core:** Rust
- **Rules/content compiler:** Rust
- **Search engine:** Rust
- **NNUE inference:** Rust
- **Headless tools / self-play runner:** Rust
- **GUI:** TypeScript
- **Renderer:** TypeScript + WebGL or Canvas 2D
- **Mobile/tablet deployment:** browser-first PWA, optionally wrapped with Capacitor
- **AlphaGo-style trainer later:** Python initially, with a clean Rust environment bridge

## 2.2 Why this stack

Rust is the authoritative simulation/search language because it is best suited for:
- deterministic state transitions
- performance-critical legality checking
- deep search
- low-level memory control
- safe concurrency
- native and WASM deployment

TypeScript is the presentation language because it is best suited for:
- browser deployment
- UI iteration speed
- sprite and asset workflows
- touch input support
- responsive layout for phone/tablet/web

Python is used later for AlphaGo-style training because:
- the ML ecosystem is still strongest there
- PyTorch-based experimentation is faster to iterate
- the engine can remain in Rust while training consumes engine-exposed state/action tensors

This is not because Python is faster than Rust. It is because training productivity and ecosystem support are stronger in Python while the real gameplay/search runtime still lives in Rust.

---

# 3. Product Shape

The product should be built as a **single authoritative simulation platform** with multiple surfaces.

## 3.1 Runtime surfaces

1. **GUI client**
   - browser-first
   - touch + mouse
   - sprite-capable battlefield renderer
   - local play and AI play

2. **Headless engine**
   - deterministic simulations
   - tests
   - replay generation
   - AI benchmarking
   - self-play data generation

3. **Search worker/runtime**
   - native worker threads in desktop/server contexts
   - web worker in browser context

4. **Training bridge**
   - exports state/action/outcome data
   - runs self-play and evaluation matches

5. **Future service layer**
   - optional match hosting
   - AI service
   - model registry service

---

# 4. Core Design Principles

## 4.1 Authoritative engine first

The GUI must never be the source of truth.

All legality, movement, combat, scoring, and timing must come from the Rust engine.

The GUI is a projection and command client.

## 4.2 Real command processor

The search engine must search by simulating through the **same command processor used in live play**.

No shadow simulator.
No simplified AI-only rules implementation.
No duplicated game logic.

This is one of the most important architecture decisions.

## 4.3 Event/timing-window driven rules

40K cannot be implemented cleanly as a giant linear phase script with hardcoded conditionals everywhere.

The engine must have:
- phase state
- subphase / decision window state
- event emission
- trigger evaluation
- reaction windows
- rule effect application
- duration tracking

## 4.4 Determinism by default

For a fixed:
- game setup
- content version
- seed bundle
- player inputs
- model version

The engine must replay identically.

This is essential for:
- debugging
- AI testing
- replay verification
- search reproducibility
- training data quality

## 4.5 Data-driven content, code-driven rules runtime

Faction content should be data-driven as much as possible.

The engine runtime should provide composable rule primitives and effect types; faction and mission content should mostly assemble those primitives through validated structured data.

Hardcode content only when a rule is truly too irregular to encode declaratively.

## 4.6 Search over tactical abstractions, not raw geometry

A literal search over every legal tabletop micro-position will explode.

The engine must separate:
- **authoritative move legality**
- **AI candidate generation**

The engine knows every legal action.
The AI searches only a curated, meaningful, tactical subset of them.

## 4.7 AlphaGo-ready but not AlphaGo-first

Do not delay the engine waiting for a future policy/value system.

Instead build the engine so that it already exposes:
- stable state serialization
- legal action masks
- deterministic stepping
- fast clone or undo
- tensor export
- replay shards
- self-play harness

---

# 5. Repository Architecture

```text
/40k-engine
  /crates
    core_types
    game_core
    geometry
    dice
    command_system
    event_system
    combat_patrol_rules
    full40k_rules
    content_schema
    content_compiler
    mission_runtime
    faction_runtime
    stratagem_runtime
    search_core
    search_abstraction
    search_ordering
    transposition
    eval_features
    eval_heuristic
    eval_nnue
    model_registry
    replay
    determinism
    test_harness
    selfplay
    trainer_bridge
    wasm_api
    native_api
  /content
    /schemas
    /compiled
    /sources
      /combat_patrol
        cp_rules
        missions
        factions
  /web
    /app
    /renderer
    /workers
    /ui_components
    /asset_pipeline
  /python
    /train_nnue
    /train_policy_value
    /selfplay_tools
    /analysis
  /docs
    implementation.md
    engine_spec.md
    content_spec.md
    ai_spec.md
    determinism_spec.md
```

---

# 6. Layered Engine Architecture

## 6.1 Layer A - Core types

Defines stable foundational types:
- ids
- enums
- tags
- coordinates
- measurements
- small value wrappers
- seed bundles
- score types
- timestamps / turn indices
- typed resource counters

Examples:
- `PlayerId`
- `UnitId`
- `ModelId`
- `ObjectiveId`
- `BattleRound`
- `Phase`
- `SubPhase`
- `CommandPoint`
- `VictoryPoints`
- `Inches`

This crate should have almost no rules logic.

## 6.2 Layer B - Geometry and board representation

Responsible for:
- board dimensions
- deployment zones
- objective markers
- terrain volumes/footprints
- visibility checks
- engagement range checks
- coherency checks
- movement path legality
- arrival placement legality
- sprite anchor coordinates for rendering

### Recommended geometry model

Use a **continuous-inch simulation model** internally, not a pure square-grid abstraction.

Suggested representation:
- positions stored in fixed-point integer thousandths of an inch or tenths/hundredths of a millimeter
- circular/simplified base footprints
- terrain represented as polygons or rectangles with tagged properties
- LOS using simplified line traces against terrain geometry and model silhouettes/base proxies

This gives enough precision for correctness while remaining deterministic.

## 6.3 Layer C - Deterministic dice and randomness

A dedicated dice subsystem must own all random behavior.

Requirements:
- seeded reproducible RNG
- per-resolution deterministic child seed derivation
- auditable roll logging
- search sampling support
- replay fidelity

### Suggested design

```text
DiceContext
  root_seed
  stream_kind
  state_fingerprint
  action_id
  resolution_index
```

Every random event must be attributable and reproducible.

## 6.4 Layer D - Command system

The live engine and search engine both operate on the same command pipeline.

### Command lifecycle

1. command proposal
2. validation
3. execution
4. emitted events
5. state updates
6. reaction windows, if any
7. command result log

### Command categories

- setup commands
- phase-control commands
- movement commands
- reserve/deep strike commands
- transport commands
- target declaration commands
- attack resolution commands
- charge declaration commands
- fight-order commands
- stratagem commands
- trigger response commands
- scoring commands

## 6.5 Layer E - Event system

The event system is what makes the rules runtime scalable.

### Event responsibilities

- emit semantic game events
- let rules subscribe by timing/conditions
- queue or resolve reactions
- apply effects
- create follow-up decision windows

### Event examples

- `BattleRoundStarted`
- `TurnStarted`
- `PhaseStarted`
- `CommandPointsGained`
- `BattleShockTestRequired`
- `UnitSelectedToMove`
- `MoveCompleted`
- `UnitSelectedToShoot`
- `AttackSequenceStarted`
- `HitsRolled`
- `WoundsRolled`
- `DamageInflicted`
- `ModelDestroyed`
- `UnitDestroyed`
- `ChargeDeclared`
- `ChargeSucceeded`
- `UnitSelectedToFight`
- `FightResolved`
- `ObjectiveControlChanged`
- `TurnEnded`
- `BattleEnded`

## 6.6 Layer F - Rules runtime

The rules runtime consumes commands and events and enforces:
- phase flow
- eligibility rules
- targeting rules
- timing windows
- effect durations
- scoring windows
- mission logic
- faction logic
- stratagem logic

This layer should be split into:
- **core 40K runtime rules**
- **Combat Patrol format rules**
- **mission pack runtime**
- **faction runtime**

---

# 7. Exact Scope for v1

## 7.1 v1 game scope

v1 should support:
- full Combat Patrol turn structure
- fixed 44" x 30" board
- pre-battle declarations
- fixed forces
- enhancements
- Combat Patrol reserve rules
- objective control and scoring
- battle-shock
- all core attack resolution
- core stratagems
- at least 2 faction patrols to start
- deterministic replay
- AI vs AI and human vs AI

## 7.2 Recommended initial factions

Use two contrasting Combat Patrols first so the engine proves asymmetry:
- **World Eaters**
- **Custodes**

This gives a strong first implementation surface:
- elite vs pressure
- melee pressure and timing windows
- reactive faction rule behavior
- small but tactically rich matchups

## 7.3 Initial mission scope

Implement only 2-3 Combat Patrol missions initially, then expand after the scoring/runtime path is stable.

Recommended first missions:
- one straightforward central-objective mission
- one spread-objective mission
- one mission with stronger end-turn scoring pressure

---

# 8. Rules Engine Specification

## 8.1 State model

The state model must be explicit, serializable, versioned, and searchable.

### Top-level game state

```text
GameState
  content_version
  ruleset_mode
  scenario_id
  battle_round
  active_player
  current_phase
  current_subphase
  decision_owner
  initiative_metadata
  vp
  cp
  turn_flags
  reaction_windows
  board_state
  objective_state
  mission_state
  player_states
  unit_states
  model_states
  transport_state
  reserve_state
  effect_state
  command_history_tail
  deterministic_counters
  game_outcome
```

## 8.2 Player state

Contains:
- cp total
- vp total
- enhancement choice
- hidden declaration data before reveal
- available stratagem usages
- faction round flags
- mission progress
- reserve permissions/restrictions

## 8.3 Unit state

Contains:
- unit identity and owner
- datasheet link
- current model membership
- attached leader/bodyguard relations
- transport relation
- reserve/on-board status
- movement/fight/shoot state flags for turn/phase
- battle-shocked flag and expiry
- below-half-strength status
- current objective control value
- current effects and modifiers
- current coherency status
- current engagement status

## 8.4 Model state

Contains:
- model identity
- unit identity
- alive/destroyed state
- wounds remaining
- board position and facing if needed
- base size
- weapon availability if per-model
- current allocation status in attack sequence

## 8.5 Effect state

Must represent:
- rule source
- target(s)
- trigger source
- duration type
- stacking behavior
- priority/precedence
- expiry condition

### Duration examples

- until end of phase
- until end of turn
- until start of next command phase
- once this attack sequence completes
- once this unit is selected to fight
- persistent mission effect

## 8.6 Decision windows

The engine must explicitly represent decision windows.

Examples:
- choose enhancement
- choose reserve placement
- choose movement action for unit
- choose target(s)
- choose stratagem usage
- choose pile-in path
- choose casualty allocation where needed
- choose fight order
- choose mission-specific scoring decision if any

Search nodes should correspond to these meaningful decisions, not to every bookkeeping step.

---

# 9. Content System

## 9.1 Content philosophy

Content must be externalized and compiled.

Raw handwritten content files should not be directly consumed by runtime. Instead:
- source files are parsed and validated
- transformed into compiled content packs
- runtime loads compiled packs with stable ids

## 9.2 Content categories

1. datasheets
2. weapon profiles
3. abilities
4. stratagems
5. enhancements
6. missions
7. deployment maps
8. faction-wide rules
9. Combat Patrol package definitions

## 9.3 Content source format

Use structured YAML or JSON5 for source authoring, then compile into a compact binary or JSON artifact.

### Why compile content

Because it allows:
- validation
- reference resolution
- canonical ids
- stable version hashes
- fast runtime loading
- deterministic search/model compatibility tagging

## 9.4 Rule primitive system

Most faction and mission rules should be composed from primitives like:
- add modifier
- set status
- grant ability
- apply reroll rule
- add attack property
- restrict target eligibility
- trigger move reaction
- trigger fight reaction
- score VP
- alter OC
- alter reserve permissions

## 9.5 Escape hatch

Some irregular rules will need code hooks.

Support this through:
- a `CustomRuleHandlerId`
- explicit audited code hooks
- minimal usage only

---

# 10. Rules Accuracy Strategy

## 10.1 100% rules accuracy definition

For this project, 100% rules accuracy means:
- all implemented Combat Patrol factions, missions, and core rules behave identically to the intended tabletop rule interactions within the digital simulation assumptions
- no illegal actions can be executed
- all mandatory effects are applied
- all reaction windows are exposed when legal
- all random procedures are correctly resolved
- scoring and game end conditions are correct

## 10.2 Required assumptions for digitalization

True tabletop 40K contains physically fuzzy concepts.
To become a digital engine, some assumptions must be fixed precisely.

Examples requiring explicit digital policy:
- how line of sight is approximated
- how base contact and exact paths are determined
- whether model facings matter for any custom content
- terrain interaction implementation details
- tie-breaking where tabletop expects players to resolve ambiguous placement physically

These assumptions must be documented and remain stable.

## 10.3 Rules conformance process

Every rule implemented should have:
1. source rule reference
2. engine interpretation note
3. test coverage
4. replay sample
5. AI legality validation

## 10.4 Golden tests

Create golden test suites for:
- battle-shock edge cases
- reserve arrival restrictions
- transport destruction handling
- engagement range and coherency edge cases
- charge legality and movement
- fight ordering
- stratagem timing
- objective control changes
- end-of-turn scoring

---

# 11. Search Engine Design

## 11.1 High-level search style

The engine should be **Stockfish-like in discipline**, not a literal clone of Stockfish’s chess assumptions.

The search stack should include:
- iterative deepening
- aspiration windows
- transposition table
- killer/history heuristics
- principal variation tracking
- quiescence/tactical extensions
- selective move generation
- budget-aware search control
- deterministic sampled chance handling

## 11.2 Why not literal full-tree search

40K has:
- randomness
- wider action branching
- geometric movement
- sequencing-heavy combats
- reaction windows
- objective scoring and mission pressure

So the engine must search over **good tactical abstractions** rather than every possible atomic micro-command.

## 11.3 Search node types

Use explicit node classes:
- strategic node
- tactical node
- reaction node
- chance node
- procedural node

### Strategic node
Player chooses among candidate macro-actions.

### Tactical node
Player chooses among combat-relevant or immediate-resolution actions.

### Reaction node
Opponent may choose a stratagem/interrupt/decline.

### Chance node
A sampled deterministic rollout branch for dice outcomes.

### Procedural node
No meaningful choice; auto-advance through bookkeeping.

## 11.4 Macro-action model

A macro-action should contain:
- stable action id
- label
- actor ids
- command sequence
- tactical intent category
- preconditions
- approximate eval priors
- invalidation fingerprint strategy

### Example categories

#### Movement intents
- hold objective
- contest objective
- move to cover
- stage charge
- screen lane
- retreat preserve unit
- line up shot
- deny reserve drop area

#### Shooting intents
- maximize kill probability
- bracket hard target
- remove scoring unit
- soften charge target
- force battle-shock pressure
- clear screen

#### Charge/fight intents
- charge highest-value target
- multi-charge objective swing
- interrupt-preserving fight order
- trade-up activation

#### Stratagem intents
- defensive save preservation
- movement reaction
- offensive damage spike
- fight order manipulation

## 11.5 Candidate generation

The search engine should not generate every legal move destination.

Instead:
- authoritative engine can enumerate legality
- search abstraction layer produces a bounded candidate set

### Movement candidate generation

For each unit, generate destinations from tactical anchors:
- objectives
- cover spots
- LOS spots vs valuable enemies
- charge staging spots
- screening arcs
- retreat safepoints
- reserve denial zones

Then validate exact placement through the authoritative engine.

### Shooting candidate generation

Bound targets to a top-K set by:
- expected damage
- VP impact
- threat value
- mission relevance
- ability denial

### Charge candidate generation

Bound to likely relevant charge declarations:
- objective swing targets
- fragile high-value targets
- interrupt-sensitive fights
- units already softened by shooting

### Stratagem generation

Only surface legal windows and high-value candidates.

## 11.6 Search algorithms

### Baseline engine search

Start with:
- negamax/alpha-beta over macro-action nodes
- selective breadth limits
- deterministic rollout samples at chance points
- tactical extension for unstable combats

### Hybrid search direction

The long-term engine should become hybrid:
- alpha-beta style tactical search in stable/sequential exchanges
- MCTS-style exploration later where policy/value guidance becomes available

This hybrid direction is what makes the AlphaGo-style future path clean.

## 11.7 Time management

The engine needs real time management from day one.

Inputs:
- remaining match clock if any
- phase sharpness
- root branching
- tactical volatility
- confidence gap between best and second best move
- board complexity

Spend more time when:
- charge/fight sequencing is sharp
- objective swing is immediate
- lethal or near-lethal outcomes are possible
- multiple valid reactions exist

Spend less time when:
- procedural state
- obvious clean-up turns
- one move dominates heavily

## 11.8 Quiescence/tactical extension

Do not stop search at unstable states.

Extend in:
- ongoing attack sequences
- charge resolutions
- fight order decisions
- lethal retaliation windows
- major scoring-swing reactions

---

# 12. NNUE-Style Evaluator Design

## 12.1 Goal

The evaluator should be a **CPU-fast, incrementally maintainable, search-friendly value function**.

Not a giant deep network.
Not a browser-scale transformer.
Not a raw pixel model.

## 12.2 What “NNUE-style” means here

In this project, NNUE means:
- sparse structured features
- compact network
- quantized or CPU-efficient inference
- incremental update friendliness
- embedded directly in the Rust engine

It does **not** mean a literal copy of chess piece-square NNUE.

## 12.3 Architecture

Recommended runtime architecture:

```text
Sparse feature extractor
  -> feature accumulator / embedding buckets
  -> hidden layer 1
  -> hidden layer 2
  -> scalar value head
```

Possible later extension:
- add policy prior head in a separate training/runtime path

## 12.4 Feature classes

The evaluator needs a much richer feature basis than a tiny summary vector.

### Global features
- battle round
- phase and subphase
- side to move
- current VP differential
- current CP differential
- mission scoring state
- reserve availability and deadlines
- active round/phase buffs

### Objective features
- current objective control
- objective holding strength
- sticky objective status if relevant
- contest pressure
- projected swing potential
- nearest-unit quality to each objective

### Unit features
- unit type id
- owner
- current wounds/models remaining
- OC contribution
- battle-shocked state
- engaged state
- in-cover state
- in-transport state
- in-reserves state
- turn action flags
- active modifiers/effects
- scoring-role tags
- durability score
- threat score

### Relative positional features
- distance bucket to nearest objective
- distance bucket to nearest enemy
- chargeability bucket
- LOS availability bucket against key targets
- threatened-by bucket
- nearby ally support bucket
- screening quality bucket

### Matchup features
- anti-armor relevance
- anti-infantry relevance
- melee threat pairing
- attrition race indicators
- exposed leader/warlord indicators

## 12.5 Incremental updates

A major requirement is incremental evaluator update.

After a command, usually only a subset of features changed:
- one unit moved
- one unit lost models/wounds
- one objective changed status
- one buff toggled
- one reserve arrived

The engine should compute feature diffs and update cached accumulator state rather than rebuilding everything from scratch at every node whenever feasible.

## 12.6 Output target

The evaluator output should be a scalar representing one of:
- expected final VP differential
- win probability transformed to search score
- blended tactical value + expected outcome

Recommendation:
- train against **expected final VP differential** and/or blended outcome targets
- normalize into a signed search score for alpha-beta use

## 12.7 Quantization and deployment

Use quantized weights or a tightly CPU-friendly format for runtime.

The trained evaluator artifact should include:
- model id
- feature schema version
- dimensions
- weights checksum
- metadata
- training provenance

---

# 13. Heuristic Evaluator Phase

Before NNUE becomes strong, ship a serious heuristic evaluator.

## 13.1 Why

A non-trivial heuristic evaluator is needed for:
- early engine strength
- debugging search
- training bootstrap targets
- benchmark baselines

## 13.2 Heuristic terms

Use weighted terms for:
- VP differential
- projected scoring next turn
- objective holding strength
- kill potential
- survival odds
- leader exposure
- reserve leverage
- battle-shock pressure
- charge threat
- retaliation risk
- mission-specific leverage
- CP utility remaining

## 13.3 Heuristic role

The heuristic evaluator remains useful permanently for:
- move ordering
- fallback behavior
- sanity checks
- auxiliary training labels

---

# 14. AlphaGo / AlphaZero-Ready Path

## 14.1 What readiness means

The engine is “AlphaGo-ready” if it already exposes:
- deterministic reset
- deterministic step
- legal action mask
- stable state encoding
- search/value logs
- replay export
- self-play batch capability
- model plug-in points

## 14.2 Required engine API

The trainer bridge should expose something conceptually like:

```text
reset(seed, scenario_config) -> state
legal_actions(state) -> action_ids
step(state, action_id) -> transition
encode_state(state) -> tensors
encode_legal_mask(state) -> mask
terminal_result(state) -> outcome
```

## 14.3 Future AlphaGo-style architecture

Later system can add:
- policy network over abstracted actions
- value network over state
- MCTS guided by policy prior + value leaf evaluation
- self-play pipeline with gating and promotion

## 14.4 Why not build it first

Because policy/value self-play is wasted if:
- legality is incomplete
- determinism is broken
- reaction windows are wrong
- content is unstable
- action vocabulary keeps changing

The Stockfish-like engine is the correct foundation.

## 14.5 Transition plan

1. heuristic search engine
2. NNUE-enhanced search engine
3. self-play data collection
4. policy/value action abstraction stabilization
5. MCTS hybrid experiments
6. candidate-vs-baseline gating
7. gradual promotion into stronger engine modes

---

# 15. GUI and Renderer Plan

## 15.1 GUI architecture

Use TypeScript with a browser-first app.

Recommended stack:
- React or Solid for UI shell
- TypeScript strict mode
- web worker for AI/search
- WebAssembly bridge to Rust engine
- PixiJS or Phaser for sprite-capable rendering

PixiJS is a very strong fit if the game is more board/tabletop presentation than arcade simulation.
Phaser is also viable if you already want scene/state tooling and sprite workflows.

## 15.2 Rendering goals

The renderer must support:
- 2D battlefield
- terrain overlays
- deployment zone overlays
- objective markers
- model/unit sprites
- range/LOS previews
- move ghosts/path previews
- attack arrows/target highlights
- status markers
- combat log drill-down

## 15.3 Sprite pipeline

Units should be represented through a flexible sprite system.

### Recommended model

- unit root anchor
- model sub-sprites for multi-model units where needed
- facing-independent top-down or angled sprite variants
- animation-lite state transitions
- optional selection rings and state badges

### Asset format

- sprite atlas JSON + PNG/WebP
- stable content ids
- per-faction palettes/themes if desired
- optional low-memory atlases for mobile

## 15.4 Input model

Support:
- click/tap select unit
- click/tap destination or target
- preview legal candidates
- confirm/cancel commands
- long-press tooltips on mobile

## 15.5 Why browser-first

Because browser-first gives:
- web deployment
- tablet compatibility
- phone compatibility
- low-friction testing
- easy sprite rendering
- PWA path

## 15.6 Mobile deployment

Phase 1: responsive browser/PWA
Phase 2: Capacitor wrapper if app-store style distribution is desired

---

# 16. WASM / Native Boundary

## 16.1 Rust/WASM interface requirements

The WASM boundary should expose only stable, high-value calls.

Examples:
- create match
- load scenario
- get current state snapshot
- get current decision surface
- validate candidate action
- apply action
- run AI decision
- export replay

## 16.2 Snapshot design

Avoid exposing raw internal Rust state to TS.

Instead expose:
- view models for UI
- compact serialized snapshots for debugging
- typed command DTOs for requests

## 16.3 AI execution placement

### Browser
- run AI in web worker
- keep heavy search off main thread

### Native/server/headless
- run AI in native thread pool
- share code paths with benchmark tooling

---

# 17. Replay, Logging, and Diagnostics

## 17.1 Replay system

Every match should emit a complete replay artifact.

Replay must include:
- content version
- seed bundle
- initial setup
- command list
- random resolutions
- score timeline
- state hashes/checkpoints
- final outcome

## 17.2 Why replay matters

Replays are needed for:
- bug reports
- deterministic verification
- regression tests
- self-play training data
- spectator/review tools

## 17.3 Search diagnostics

The engine should expose:
- depth reached
- node count
- nps
- best line
- score timeline
- candidate scores
- rollout count usage
- evaluator stats
- TT hit rate

## 17.4 User-visible diagnostics

The GUI can optionally show:
- AI thinking panel
- principal variation summary
- evaluation bar
- confidence estimate
- top candidate explanations

---

# 18. Determinism and Verification

## 18.1 Determinism contract

A deterministic verification suite must be a first-class subsystem.

Given identical:
- content hash
- engine version
- seed
- command sequence

The final state hash must match.

## 18.2 Verification tooling

Create tooling to:
- run replay round-trip verification
- compare state hashes at every step
- verify search reproducibility for fixed seeds/config
- verify no illegal command passes validation

## 18.3 Save-state hashing

Every major state transition should allow stable hash generation.

This is useful for:
- queued plan invalidation
- TT keys
- replay checking
- bug localization

---

# 19. Testing Strategy

## 19.1 Testing pyramid

### Unit tests
- geometry primitives
- dice resolution
- command validators
- effect stacking
- feature extraction
- evaluator consistency

### Scenario tests
- specific rules interactions
- mission scoring examples
- reserve timing edge cases
- transport destruction cases
- battle-shock persistence cases

### Replay tests
- load and reproduce historical bug cases

### AI legality tests
- long self-play runs
- assert no illegal command accepted
- assert no unresolved dead-end decision states

### Performance tests
- search speed
- memory usage
- replay throughput
- WASM latency

## 19.2 Required automated suites

1. **Rules conformance suite**
2. **Determinism suite**
3. **Search legality suite**
4. **Self-play soak suite**
5. **Performance regression suite**

## 19.3 Soak testing

Run thousands of self-play matches continuously and treat:
- abnormal termination
- command rejection after search selection
- replay mismatch
- unresolved decision windows

as engine bugs to be fixed before scaling content.

---

# 20. Implementation Phases

## Phase 0 - Foundations

Deliverables:
- repo scaffold
- content schema draft
- state model draft
- command system skeleton
- deterministic RNG subsystem
- geometry primitives
- test harness shell

Exit criteria:
- empty match can initialize, serialize, and hash deterministically

## Phase 1 - Core Combat Patrol engine shell

Deliverables:
- phase order
- player turns
- command phase
- movement phase shell
- reserve declarations shell
- objective representation
- mission/scenario loader
- two patrol package loaders

Exit criteria:
- scripted matches can progress through 5 rounds without combat

## Phase 2 - Full combat resolution

Deliverables:
- shooting pipeline
- attack allocation
- saves and damage
- melee pipeline
- charge pipeline
- model destruction
- coherency checks
- battle-shock

Exit criteria:
- two patrols can play complete lethal games headlessly

## Phase 3 - Stratagems, faction rules, scoring

Deliverables:
- core stratagem runtime
- enhancement choices
- faction runtime hooks
- mission scoring runtime
- end-of-turn and end-of-game scoring

Exit criteria:
- representative Combat Patrol games resolve correctly end-to-end

## Phase 4 - Replay and determinism hardening

Deliverables:
- replay files
- replay verification
- state hash checkpoints
- scenario regression suite

Exit criteria:
- replay round-trip is boringly reliable

## Phase 5 - Heuristic engine

Deliverables:
- candidate generation
- macro-action pipeline
- root search
- move ordering
- heuristic evaluator
- AI worker bridge

Exit criteria:
- AI can finish games legally and non-randomly

## Phase 6 - Stockfish-like engine features

Deliverables:
- iterative deepening
- aspiration windows
- TT
- killer/history ordering
- tactical extensions
- deterministic chance sampling
- diagnostics panel/logging

Exit criteria:
- search AI materially outperforms heuristic-only baseline

## Phase 7 - NNUE runtime

Deliverables:
- feature extractor
- model artifact spec
- Rust inference runtime
- model registry
- evaluator swap path

Exit criteria:
- engine can run with heuristic or NNUE evaluator interchangeably

## Phase 8 - Self-play and training bridge

Deliverables:
- replay shard export
- state/action/outcome export
- self-play runner
- gating harness

Exit criteria:
- repeatable candidate-vs-baseline training cycle works

## Phase 9 - GUI productionization

Deliverables:
- TS UI shell
- sprite battlefield renderer
- touch support
- replay viewer
- AI controls
- PWA packaging

Exit criteria:
- browser/mobile/tablet playable release candidate

## Phase 10 - AlphaGo-style expansion

Deliverables:
- state tensor export refinement
- legal action vocab stabilization
- policy/value training experiment path
- MCTS hybrid prototype

Exit criteria:
- first policy-guided search experiments possible without rewriting engine core

---

# 21. Search and AI Module Breakdown

## 21.1 `search_core`

Responsibilities:
- root search orchestration
- iterative deepening
- node expansion
- budget handling
- principal variation tracking

## 21.2 `search_abstraction`

Responsibilities:
- generate macro-actions from state
- movement candidate templating
- target pruning
- stratagem candidate pruning
- tactical intent classification

## 21.3 `search_ordering`

Responsibilities:
- hash move ordering
- killer/history heuristic
- evaluator-assisted priors
- tactical priority boosts

## 21.4 `transposition`

Responsibilities:
- TT key generation
- entry storage
- bound typing
- replacement policy
- collision diagnostics

## 21.5 `eval_heuristic`

Responsibilities:
- deterministic weighted scoring
- move ordering priors
- bootstrap labels

## 21.6 `eval_features`

Responsibilities:
- feature schema definitions
- state -> sparse feature extraction
- incremental diff extraction
- schema versioning

## 21.7 `eval_nnue`

Responsibilities:
- model load
- inference
- accumulator cache
- incremental update hooks
- quantized runtime path

## 21.8 `selfplay`

Responsibilities:
- match orchestration
- replay generation
- outcome labeling
- search diagnostic capture
- shard emission

---

# 22. Data and Model Versioning

## 22.1 Version categories

Track distinct versions for:
- engine build
- content schema
- compiled content pack
- replay format
- evaluator feature schema
- model artifact format

## 22.2 Why this matters

A model trained on one feature schema must not silently load against another.
A replay generated against one content pack must not silently claim compatibility with another.

## 22.3 Registry approach

Use registries for:
- content packs
- missions
- factions
- evaluator models

All must validate compatibility explicitly.

---

# 23. Performance Plan

## 23.1 Performance targets

The engine should target:
- instant UI feedback for human command previews
- usable AI thinking time on consumer devices
- deeper search on desktop/native/headless
- efficient self-play throughput

## 23.2 Key optimization points

1. state cloning / undo model
2. geometry caching
3. LOS cache reuse
4. candidate pruning quality
5. incremental evaluator updates
6. TT efficiency
7. command validation hot paths

## 23.3 Clone vs undo

Design so either approach is possible:
- fast immutable-ish clone for safety and simplicity at first
- optional apply/undo later for deeper performance

Recommendation:
- start with disciplined fast clone / arena-backed structure
- only add true undo if profiling proves necessary

---

# 24. Security and Integrity

## 24.1 Runtime integrity

The UI must not be able to force illegal actions.
All commands must round-trip through engine validation.

## 24.2 Multiplayer readiness

Even if multiplayer is later, architect now so the engine can become server-authoritative.

That means:
- deterministic command application
- clean command DTOs
- no trust in client state

---

# 25. Concrete First Build Order

## 25.1 First 12 major milestones

1. Rust workspace and core types
2. deterministic RNG and replay skeleton
3. board/geometry/objective system
4. state model and serialization
5. command processor skeleton
6. phase progression and decision windows
7. movement/reserve legality
8. combat resolution pipelines
9. scoring and mission runtime
10. two faction Combat Patrol packs
11. heuristic AI and macro-actions
12. search engine + NNUE hook points

## 25.2 First “real game” milestone

The first real milestone should be:

**Headless Custodes vs World Eaters Combat Patrol match, 5 rounds, full replay, no UI, legal from start to finish.**

That is the moment the engine is truly alive.

## 25.3 First “real product” milestone

The first product milestone should be:

**Browser-playable match with TypeScript GUI, sprite renderer, human vs AI, complete replay export, deterministic bug report package.**

---

# 26. Risks and Mitigations

## 26.1 Risk: rules engine complexity explodes

Mitigation:
- event-driven runtime
- content compiler
- rule primitive system
- start with bounded Combat Patrol content

## 26.2 Risk: search branching explodes

Mitigation:
- tactical macro-actions
- candidate pruning
- movement anchors
- top-K targeting
- selective tactical extensions

## 26.3 Risk: NNUE feature design is too weak

Mitigation:
- keep strong heuristic baseline
- version feature schemas aggressively
- train iteratively from self-play and rollouts
- expand feature richness over time

## 26.4 Risk: browser AI performance is weak

Mitigation:
- worker-based search
- configurable depth/time presets
- WASM build optimization
- optional server/native AI path later

## 26.5 Risk: “100% rules accuracy” drifts

Mitigation:
- rules conformance docs
- golden scenarios
- replay-backed bug fixing
- every new rule paired with tests

---

# 27. Final Recommended Direction

## 27.1 The correct implementation choice

Build:
- a **Rust authoritative Combat Patrol engine**
- a **Rust Stockfish-like search system** using macro-actions, selective search, TT, and NNUE-style evaluation
- a **TypeScript browser-first GUI** with sprite-capable rendering
- a **headless deterministic replay and self-play toolchain**
- a **Python training bridge later** for NNUE refinement and eventual AlphaGo-style policy/value experimentation

## 27.2 The correct sequence

1. deterministic rules engine
2. replay + tests
3. heuristic AI
4. serious search engine
5. NNUE runtime
6. self-play/gating
7. GUI productionization
8. AlphaGo-style future path

## 27.3 Final principle

Do **not** build a fake simple engine to “get to AI faster.”

The real engine is the foundation.
The search engine must sit on the real engine.
The NNUE must evaluate real states.
The future AlphaGo-style system must train on real transitions.

That is how you get a serious 40K engine rather than a demo.


---

## Rule Source Contract and Traceability

This implementation is grounded in two authoritative source documents that must be treated as the baseline rules contract for the initial engine build:

- `/mnt/data/40k_revised.md` — authoritative core 40K rules baseline for the engine's shared mechanics.
- `/mnt/data/CP_Rules.md` — authoritative Combat Patrol format layer, including Combat Patrol-specific setup, constraints, and scenario flow.

These two files must be referenced directly throughout implementation, testing, and validation. Faction patrol files (for example `Custodes.md`, `World_Eaters.md`, `Frenzied_Reavers.md`, and `ArkothsGorepact.md`) are content-layer authorities that sit on top of the baseline core and Combat Patrol rules.

### Mandatory rule-source policy

1. The engine must never treat GUI behavior, AI assumptions, or convenience scripting as authoritative over the rules documents.
2. Every implemented mechanic must be traceable back to one or more concrete source passages from `40k_revised.md` and/or `CP_Rules.md`, with faction patrol files referenced where applicable.
3. Every rule implementation module must include a source annotation block that records:
   - source file name
   - heading or section name
   - relevant passage summary
   - tests that validate the implemented behavior
4. Every unresolved ambiguity must be logged into a `rules_decisions.md` file with the exact source conflict or ambiguity noted.
5. “Works like tabletop memory” is not acceptable as a rules source. The uploaded rule files are the implementation baseline.

### Required traceability artifacts

The repo must include:

- `docs/rule_index.md`
  - maps each engine subsystem to the source files and sections it implements
- `docs/rules_decisions.md`
  - records clarifications, assumptions, and temporary rulings when source text is ambiguous
- `docs/rule_coverage_matrix.md`
  - tracks implemented / partially implemented / not implemented rules
- `tests/rules/`
  - source-linked rules tests

Suggested `rule_coverage_matrix.md` columns:

- Rule ID
- Source file
- Section / heading
- Mechanic summary
- Engine module
- Test file(s)
- Status
- Notes

### Initial authority layering

The recommended authority stack is:

1. `40k_revised.md` — shared core mechanics
2. `CP_Rules.md` — Combat Patrol overlay and restrictions
3. faction patrol files — patrol-specific units, enhancements, stratagems, secondaries, and special rules
4. mission/scenario content files implemented under the same traceability rules

If a faction file conflicts with core rules, the conflict must be resolved explicitly in `rules_decisions.md` rather than silently hardcoded.

---

## Headless Runtime Is a First-Class Product Surface

The engine must be fully playable without the GUI.

This is not just for testing. It is required for:

- search AI
- self-play
- batch simulation
- replay validation
- server-hosted matches
- MCP / agent integration
- CI rules regression testing

### Required headless entry points

Implement all of the following:

- `play-headless`
  - runs a complete game from scenario config and player controllers
- `replay-headless`
  - replays a saved action log deterministically from seed
- `selfplay-headless`
  - runs engine-vs-engine matches in batch
- `rules-test-headless`
  - executes narrow scripted rules scenarios
- `mcp-host`
  - exposes a structured external control interface over the same authoritative runtime

### Headless runtime requirements

The headless engine must:

- load official rules content
- expose all legal actions for the current decision window
- resolve all outcomes authoritatively
- auto-progress through non-decision windows where appropriate
- preserve deterministic action/replay logs
- operate identically whether driven by UI, AI, tests, or MCP

There must be no “GUI-only rules.”

---

## MCP Integration Requirements

MCP must be treated as a first-class interface layer, not an afterthought.

The purpose is to allow external reasoning agents such as ChatGPT, Claude, Gemini, or other tool-using systems to connect to the authoritative game runtime and play complete games legally through structured tool calls.

### Architectural rule

All external agents must interact with the same headless authoritative runtime used by:

- local search AI
- self-play harnesses
- regression tests
- replay tools
- browser/native clients

The MCP layer must never reimplement game rules. It is only an interface adapter.

### MCP design goals

The MCP server must support:

- creating and loading matches
- querying visible game state
- querying legal actions
- submitting actions or macro-actions
- stepping the game to the next decision point
- reading logs/replays
- inspecting public board state
- optionally inspecting privileged debug state for development modes only

### Required MCP tool surface

At minimum, expose tools/resources equivalent to:

#### Match lifecycle
- `match.create`
- `match.load`
- `match.reset`
- `match.clone`
- `match.delete`

#### State queries
- `state.summary`
- `state.public_view`
- `state.debug_view`
- `state.current_window`
- `state.score`
- `state.phase`

#### Action queries
- `actions.list_legal`
- `actions.describe`
- `actions.expand_macro`
- `actions.validate`

#### Gameplay
- `game.submit_action`
- `game.submit_macro_action`
- `game.step_until_decision`
- `game.pass_if_legal`
- `game.undo_last_action` (debug-only unless fully safe)

#### Replay/logging
- `replay.export`
- `replay.import`
- `replay.step`
- `log.tail`

#### Analysis/debug
- `debug.seed`
- `debug.snapshot`
- `debug.diff_snapshots`
- `debug.list_triggers`

### Public vs private information

The MCP layer must support multiple view policies:

- `public`
  - only information a legal player is allowed to know
- `player_n`
  - information visible to one side
- `omniscient_debug`
  - complete internal state for testing and debugging

This matters for future scenarios, reserve states, hidden choices, and any later full-40K mechanics that may involve non-public information. The interface must be designed correctly now even if Combat Patrol starts with limited hidden information.

### MCP action contract

The preferred contract is:

1. agent requests current public/player view
2. agent requests legal actions
3. engine returns action IDs plus structured payloads
4. agent selects an action or macro-action
5. engine validates and applies it
6. engine resolves resulting event chain and steps forward until the next decision point
7. logs and replay entries are recorded automatically

This makes the engine safe for external agents because the engine, not the model, remains the source of truth.

### MCP transport and schema

Use stable JSON schemas for all MCP payloads.

Repo must include:

- `schemas/mcp/*.json`
- `docs/mcp_api.md`
- `examples/mcp_session_transcript.md`

Every MCP message type should be versioned. Breaking changes must bump the schema version.

### MCP acceptance criteria

The MCP integration is not complete until all of the following work:

1. Chat-style external agent can create a match and play a legal full game with no GUI.
2. A scripted agent can request legal actions and never be forced to infer legality itself.
3. Replay exported from an MCP-driven game replays deterministically in headless mode.
4. The same saved game can be resumed in GUI mode without state mismatch.

---

## Handoff Contract for Codex / Claude / Implementation Agents

This document must be sufficient for a capable implementation agent to build the system without inventing major architecture.

### The implementation agent must deliver

1. A Rust authoritative engine that can play full Combat Patrol games headlessly.
2. A TypeScript GUI suitable for sprites and browser/mobile use.
3. A Stockfish-like search stack with a heuristic evaluator first and NNUE-ready evaluator path.
4. Full replay determinism and source-traceable rules coverage.
5. MCP support over the same authoritative runtime.
6. An architecture that can later host AlphaGo/AlphaZero-style self-play and training.

### Required deliverable set

The implementation agent must produce at minimum:

- engine crates / packages
- GUI client
- MCP host
- rules content loaders
- traceability documents
- test suite
- replay tools
- self-play harness
- evaluator/training bridge stubs
- developer setup instructions

### Required repository documents

The repo produced by the implementation agent must include:

- `README.md`
- `docs/architecture.md`
- `docs/rule_index.md`
- `docs/rule_coverage_matrix.md`
- `docs/rules_decisions.md`
- `docs/replay_format.md`
- `docs/mcp_api.md`
- `docs/ai_search.md`
- `docs/nnue_plan.md`
- `docs/alphago_readiness.md`

### Required acceptance gates

Before the implementation is considered “usable,” it must pass these gates:

#### Gate 1 — rules core
- load `40k_revised.md` and `CP_Rules.md` derived content successfully
- run deterministic Combat Patrol setup
- play a complete legal game headlessly
- export replay and replay it identically

#### Gate 2 — gameplay correctness
- objective control, phases, shooting, charging, fighting, battle-shock, reserves, stratagem windows, and scoring covered by tests
- faction packages for at least two patrols implemented and validated

#### Gate 3 — AI baseline
- legal random player finishes games
- heuristic player beats random reliably
- search player beats heuristic baseline in controlled matchups

#### Gate 4 — interface parity
- GUI, headless, and MCP all produce identical legal state transitions
- an MCP-driven external agent can finish a full legal game

#### Gate 5 — AlphaGo readiness
- state export exists
- action masks exist
- batch stepping exists
- self-play harness exists
- evaluator training data export exists

### Explicit non-goals for the first release

To keep the implementation agent focused, these are not required for first release unless explicitly scheduled:

- full all-factions full-40K content coverage
- ranked online service
- cinematic VFX-heavy presentation
- advanced account systems
- giant transformer evaluator in-engine

The first release goal is a correct, deterministic, headless-authoritative Combat Patrol engine with strong search architecture and clear full-40K extension paths.


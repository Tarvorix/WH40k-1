# WH40k Digital Engine — AI Primer

Complete technical reference for the AI subsystem: Stockfish-style search engine, AlphaZero-style MCTS, NNUE neural evaluation, self-play training pipeline, and supporting infrastructure.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Feature Extraction](#2-feature-extraction)
3. [Heuristic Evaluator](#3-heuristic-evaluator)
4. [NNUE Neural Evaluator](#4-nnue-neural-evaluator)
5. [Macro-Action Abstraction](#5-macro-action-abstraction)
6. [Move Ordering](#6-move-ordering)
7. [Transposition Table](#7-transposition-table)
8. [Stockfish-Style Search Engine](#8-stockfish-style-search-engine)
9. [AlphaZero-Style MCTS](#9-alphazero-style-mcts)
10. [PolicyValue Dual-Head Network](#10-policyvalue-dual-head-network)
11. [Self-Play Framework](#11-self-play-framework)
12. [PyO3 Trainer Bridge](#12-pyo3-trainer-bridge)
13. [Training Pipelines](#13-training-pipelines)
14. [Gating & Elo Rating](#14-gating--elo-rating)
15. [Model Lineage & Registry](#15-model-lineage--registry)
16. [Determinism Guarantees](#16-determinism-guarantees)
17. [Configuration Reference](#17-configuration-reference)
18. [Crate Map](#18-crate-map)

---

## 1. Architecture Overview

The AI subsystem is a layered system combining classical game-tree search with neural network evaluation and reinforcement learning, adapted for the unique challenges of Warhammer 40,000 (imperfect information, stochastic dice, multi-phase turns, variable branching factor).

```
┌──────────────────────────────────────────────────────────┐
│                    Search Frontends                       │
│  ┌──────────┐  ┌───────────┐  ┌──────────┐  ┌────────┐ │
│  │GreedyAi  │  │OnePlySrch │  │Negamax   │  │  MCTS  │ │
│  │(depth 0) │  │(depth 1)  │  │(depth d) │  │(PUCT)  │ │
│  └────┬─────┘  └─────┬─────┘  └────┬─────┘  └───┬────┘ │
│       │              │              │             │       │
│  ┌────┴──────────────┴──────────────┴─────────────┘      │
│  │        IterativeDeepeningSearch (Stockfish-style)      │
│  │  Aspiration Windows · Quiescence · Extensions          │
│  │  PV Tracking · Time Management · Lazy SMP (skeleton)   │
│  └────────────────────────┬───────────────────────────┘  │
│                           │                               │
│  ┌────────────────────────┴───────────────────────────┐  │
│  │              Search Infrastructure                  │  │
│  │  MoveOrderer (Killer+History+TT)                    │  │
│  │  TranspositionTable (Always-Replace, Generation)    │  │
│  │  ActionGenerator → CandidateSet (MacroActions)      │  │
│  └────────────────────────┬───────────────────────────┘  │
│                           │                               │
│  ┌────────────────────────┴───────────────────────────┐  │
│  │                   Evaluation                        │  │
│  │  ┌─────────────────┐   ┌────────────────────────┐  │  │
│  │  │HeuristicEvaluator│   │NnueEvaluator           │  │  │
│  │  │(15 weighted terms│   │(1203→128→32→32→1)      │  │  │
│  │  │ hand-tuned)      │   │ quantized i16/i8/i32   │  │  │
│  │  └─────────┬───────┘   │ incremental accumulator │  │  │
│  │            │            └────────────┬───────────┘  │  │
│  │            └─────────┬──────────────┘               │  │
│  │                      ▼                              │  │
│  │              AnyEvaluator (runtime switch)           │  │
│  └────────────────────────┬───────────────────────────┘  │
│                           │                               │
│  ┌────────────────────────┴───────────────────────────┐  │
│  │              Feature Extraction                     │  │
│  │  GameState → 1203 sparse features                   │  │
│  │  31 global + 180 objective (6×30) + 992 unit (16×62)│  │
│  └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────┐
│                  Training Pipeline                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐│
│  │Self-Play │→ │Training  │→ │ Gating   │→ │ Lineage  ││
│  │Framework │  │(PyTorch) │  │(Elo-based│  │ Registry ││
│  │(Rust)    │  │NNUE or PV│  │ harness) │  │ (JSON)   ││
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘│
│       ↕ PyO3 FFI (trainer_bridge cdylib)                 │
└──────────────────────────────────────────────────────────┘
```

### AI Worker Hierarchy

| Level | Class | Depth | Features | Use Case |
|-------|-------|-------|----------|----------|
| 0 | `GreedyAi` | 0 | Static eval only | Fallback, testing |
| 1 | `OnePlySearch` | 1 | Heuristic ordering | Moderate play |
| 2 | `NegamaxSearch` | 2-3+ | Alpha-beta, TT, move ordering | Strong tactical |
| 3 | `IterativeDeepeningSearch` | 1→d | Full Stockfish feature set | Strongest play |
| 4 | `LazySmpSearch` | 1→d | Multi-threaded ID (skeleton) | Future parallelism |
| Alt | `MctsSearch` | N/A | PUCT tree search | Long-range planning |

### Named AI Tiers (WASM/Web)

The web frontend exposes named difficulty tiers via `run_ai_decision(difficulty)`:

| Tier | Difficulty Levels | Search Method |
|------|------------------|---------------|
| **Basic** | Recruit, Battle_Ready, Veteran, Elite | Greedy → OnePly → Negamax(2) → Negamax(3) |
| **Perturabo** | Shallow, Regular, Deep | Iterative Deepening depth 4 → 6 → 8 |
| **Alpharius** | Operative, Headhunter, Primarch | MCTS fast → standard → competition |

Perturabo is the primary Stockfish-style tier for NNUE training. Alpharius is the AlphaZero-style tier for policy/value training.

All workers implement the `AiWorker` trait:
```rust
pub trait AiWorker: Send {
    fn choose_action(&mut self, state: &GameState, perspective: PlayerId)
        -> Option<SearchResult>;
    fn name(&self) -> &str;
}
```

### SearchResult — Unified Output

```rust
pub struct SearchResult {
    pub best_action: MacroAction,                    // Chosen move
    pub score: Score,                                 // Evaluation (centipawns)
    pub stats: SearchStats,                           // Telemetry
    pub pv: Vec<MacroAction>,                        // Principal variation
    pub candidate_scores: Vec<(MacroAction, Score)>, // All candidates scored
}
```

---

## 2. Feature Extraction

**Crate:** `wh40k_eval_features` — Converts raw `GameState` into a fixed-size numeric representation for both heuristic evaluation and neural network input.

### Feature Space Layout (1203 total)

```
Offset    Count   Description
──────    ─────   ───────────
0-4         5     Battle round one-hot (rounds 1-5)
5-11        7     Phase one-hot (7 phases)
12-30      19     Scalar global features
31-210    180     Objective features (6 objectives × 30 features)
211-1202  992     Unit features (16 units × 62 features)
──────    ─────
Total    1203
```

### Global Features (31)

| Index | Feature | Encoding |
|-------|---------|----------|
| 0-4 | Battle round | One-hot (5 rounds) |
| 5-11 | Phase | One-hot (7 phases: PreBattle, Command, Movement, Shooting, Charge, Fight, GameEnd) |
| 12 | VP differential | Signed scalar, own − enemy |
| 13 | CP differential | Signed scalar, own − enemy |
| 14-15 | Models remaining (own/enemy) | Absolute count |
| 16-17 | Model ratio (own/enemy) | Fixed-point 0-1000 (1000 = 100% alive) |
| 18-19 | Wounds remaining (own/enemy) | Absolute sum |
| 20-21 | Reserves count (own/enemy) | Count of units in reserves |
| 22-23 | Battle-shocked count (own/enemy) | Count |
| 24-25 | Objectives controlled (own/enemy) | Count |
| 26 | Contested objectives | Count |
| 27-28 | Total OC (own/enemy) | Sum of all units' OC on board |
| 29 | Is first player | Binary |
| 30 | Game progress | Fixed-point 0-1000 (round/5 × 1000) |

### Per-Objective Features (30 each, max 6 objectives)

| Offset | Feature | Encoding |
|--------|---------|----------|
| 0 | Controller | -127 (enemy) to +127 (own), 0 = contested/none |
| 1-2 | OC in range (own/enemy) | Normalized count within 3" |
| 3 | OC advantage | Signed differential |
| 4-5 | Secured flags (own/enemy) | Binary |
| 6-14 | Nearest own unit distance | One-hot over 9 distance buckets |
| 15-23 | Nearest enemy unit distance | One-hot over 9 distance buckets |
| 24-25 | Units within 6" (own/enemy) | Normalized count |
| 26-28 | Zone flags | No Man's Land / own DZ / enemy DZ |
| 29 | VP value | Scalar (mission-dependent) |

### Per-Unit Features (62 each, max 16 units)

| Offset | Feature | Encoding |
|--------|---------|----------|
| 0 | Is own | Binary |
| 1-3 | Model ratio, wound ratio, effective OC | Normalized 0-127 |
| 4-11 | Status flags | Battle-shocked, engaged, reserves, on-board, CHARACTER, BATTLELINE, WARLORD, invuln save |
| 12-20 | Combat stats | Toughness, save, melee/ranged attacks, max range, strengths, best AP, max damage |
| 21 | Best AP | Signed (most negative = strongest) |
| 22 | Max damage | Normalized |
| 23-25 | Composite scores | Threat (0-1000), durability (0-1000), value (0-1000) |
| 26-34 | Nearest objective distance | One-hot over 9 buckets |
| 35-43 | Nearest enemy distance | One-hot over 9 buckets |
| 44-52 | Nearest ally distance | One-hot over 9 buckets |
| 53 | Can charge | Binary |
| 54-55 | Enemies in range | Shooting range / charge range (12") |
| 56-60 | Action flags | Moved, shot, fought, charged, advanced this turn |
| 61-63 | Matchup features | Anti-armor relevance, anti-infantry relevance, melee threat matchup |

### Distance Buckets (9 levels)

| Bucket | Range | Tactical Meaning |
|--------|-------|-----------------|
| Engaged | ≤1" | In melee combat |
| MeleeReach | 1-3" | Pile-in/consolidation range |
| ShortCharge | 3-6" | Near-guaranteed charge |
| MediumCharge | 6-9" | Probable charge |
| LongCharge | 9-12" | Maximum charge declaration |
| ShortRange | 12-18" | Short-range weapons |
| MediumRange | 18-24" | Standard rapid fire range |
| LongRange | 24-36" | Heavy/sniper weapons |
| OutOfRange | >36" | No immediate interaction |

### Sparse Representation

Features are stored as `(index: u16, value: i16)` pairs — only non-zero entries. Values are normalized:
- **Binary features:** value = 1
- **Ratios (0-1000):** normalized to 0-127 via `ratio × 127 / 1000`
- **Differentials:** signed, normalized to [-127, 127]
- **Counts:** normalized to [0, 127] with configurable max

### Composite Scoring Formulas

**Threat Score (0-1000):**
```
Per model:
  melee = attacks × skill_factor × strength × |AP| × damage × ability_mult
  ranged = 0.6 × (attacks × skill_factor × strength × |AP| × damage × ability_mult)
ability_mult: LethalHits/SustainedHits → +2%, DevastatingWounds → +3%
CHARACTER bonus: ×1.2
```

**Durability Score (0-1000):**
```
base = wounds × toughness × save_factor
save_factor: 2+=5, 3+=4, 4+=3, 5+=2, 6+=1, 7+=0
FnP bonus: wounds × (7 - fnp_val) × 10/6
Invulnerable bonus: × (12 + (6 - inv_save)) / 12
```

**Value Score (0-1000):**
```
(threat×40 + durability×30) / 70 + OC×20 + CHARACTER(+100) + BATTLELINE(+50)
```

### Feature Diff (Incremental Updates)

For efficient NNUE accumulator updates during search:
```rust
pub struct FeatureDiff {
    pub added: Vec<SparseFeature>,     // Present in new, not old
    pub removed: Vec<SparseFeature>,   // Present in old, not new
    pub changed: Vec<(SparseFeature, SparseFeature)>,  // (old, new) pairs
}
```

---

## 3. Heuristic Evaluator

**Crate:** `wh40k_eval_heuristic` — A handcrafted, weighted-sum evaluator designed for WH40K Combat Patrol. Returns `Score` (i32, centipawn-scale).

### Evaluation Terms (15 weighted components)

| Term | Default Weight | Description |
|------|---------------|-------------|
| VP Differential | 150 | Direct victory point advantage (most important) |
| Projected Scoring | 80 | Estimated VP from objective control next turn |
| Objective Control | 120 | Bonus per controlled objective, penalty per enemy-held |
| Objective Holding Strength | 15 | OC advantage at each objective, proximity bonuses |
| Objective Contest | 60 | Partial credit for contested objectives |
| CP Utility | 30 | Command Point differential (for stratagem use) |
| Kill Potential | 8 | Aggregate threat scores of own − enemy units |
| Survival Odds | 6 | Aggregate durability scores of own − enemy units |
| Unit Value | 4 | Combined threat + durability + keywords |
| Leader Exposure | -50 | Penalty for WARLORD units exposed to danger |
| Reserve Leverage | 25 | Future potential from units in reserves |
| Battle-Shock Pressure | 40 / -35 | Enemy shocked (+40), own shocked (-35) |
| Charge Threat | 45 | Own units threatening charges |
| Retaliation Risk | -30 | Penalty for exposure to enemy charges |
| Mission Leverage | 60 | Mission-specific scoring factors |
| Model Advantage | 50 | Model count ratio differential |
| Wound Advantage | 30 | Total wound count differential |

### Game Phase Scaling

Weights are scaled by battle round:
- **Early game (rounds 1-2):** 1.0× (default)
- **Mid game (rounds 3-4):** 1.0× (default)
- **Late game (round 5):** 1.0× (default)

Phase factors are configurable in thousandths (1000 = 1.0×).

### Projected Scoring Detail

```
For each objective:
  if controlled by own → +5 VP contribution
  if contested → split credit by OC ratio: own_oc / (own_oc + enemy_oc) × 5
  urgency_bonus: applied in rounds 3+ (later rounds worth more)
```

### Weight Presets

- **Balanced** (default): As shown above
- **Aggressive:** kill_potential=12, charge_threat=65, objective_control=100
- **Defensive:** objective_control=150, survival_odds=10, kill_potential=5

### Move Ordering Score (lightweight variant)

```
score = vp_differential × 100
      + objective_advantage × 80
      + model_ratio_advantage / 5
      + shock_count_diff × 20
```

Used by `OnePlySearch` and for static candidate ranking when full evaluation is too expensive.

---

## 4. NNUE Neural Evaluator

**Crate:** `wh40k_eval_nnue` — A quantized neural network evaluator inspired by Stockfish's Efficiently Updatable Neural Network architecture.

### Network Architecture

```
Input Layer: 1203 sparse features
      │
      ▼
Feature Transformer + Accumulator (128 neurons)
      │  [ClippedReLU → clamp to [0, 127]]
      ▼
Hidden Layer 1 (32 neurons)
      │  [ClippedReLU → clamp to [0, 127]]
      ▼
Hidden Layer 2 (32 neurons)
      │  [ClippedReLU → clamp to [0, 127]]
      ▼
Output Layer (1 neuron → Score)
```

**Total parameters:** ~159K
```
Feature transformer:  1203 × 128 weights + 128 biases  = 154,112
Hidden 1:               128 × 32  weights + 32  biases  =   4,128
Hidden 2:                32 × 32  weights + 32  biases  =   1,056
Output:                  32 × 1   weights + 1   bias    =      33
                                                  Total = 159,329
```

### Quantization Scheme

All arithmetic uses integer types — no floating point during inference:

| Layer | Weight Type | Bias Type | Activation Range |
|-------|------------|-----------|-----------------|
| Feature Transformer | `i16` (±32,767) | `i32` | Raw accumulator (i32) |
| Hidden 1 | `i8` (±127) | `i32` | ClippedReLU [0, 127] |
| Hidden 2 | `i8` (±127) | `i32` | ClippedReLU [0, 127] |
| Output | `i16` (±32,767) | `i32` | Score (i32) |

### Quantization Constants

```rust
const QA: i32 = 127;           // ClippedReLU maximum (activation quantum)
const FT_SCALE: i32 = 64;      // Feature transformer scale divisor
const HIDDEN_SCALE: i32 = 64;  // Hidden layer scale divisor
const OUTPUT_SCALE: i32 = 400;  // Final output multiplier to centipawns
```

### Forward Pass (Integer Arithmetic)

**Step 1 — Feature Transformer (sparse → dense):**
```
For each neuron j in [0..128):
  accumulator[j] = bias[j] + Σ (feature_weights[i][j] × feature_value[i])
                                for each active sparse feature i
  clipped[j] = clamp(accumulator[j] / FT_SCALE, 0, QA)    // [0, 127]
```

**Step 2 — Hidden Layer 1:**
```
For each neuron k in [0..32):
  h1[k] = bias[k] + Σ (weights[j][k] × clipped_acc[j])    for j in [0..128)
  clipped_h1[k] = clamp(h1[k] / HIDDEN_SCALE, 0, QA)      // [0, 127]
```

**Step 3 — Hidden Layer 2:**
```
For each neuron l in [0..32):
  h2[l] = bias[l] + Σ (weights[k][l] × clipped_h1[k])     for k in [0..32)
  clipped_h2[l] = clamp(h2[l] / HIDDEN_SCALE, 0, QA)      // [0, 127]
```

**Step 4 — Output:**
```
output = bias + Σ (weights[l] × clipped_h2[l])             for l in [0..32)
final_score = output × OUTPUT_SCALE / QA                    // centipawn-scale Score
```

### Incremental Accumulator Updates

The key NNUE optimization: when a move changes only a few features, the accumulator is updated incrementally rather than recomputed from scratch.

```
Given FeatureDiff (added, removed, changed):

For removed features:
  accumulator[j] -= feature_weights[i][j] × old_value[i]

For added features:
  accumulator[j] += feature_weights[i][j] × new_value[i]

For changed features:
  accumulator[j] += feature_weights[i][j] × (new_value[i] - old_value[i])

Then forward from updated accumulator (skip step 1).
```

This reduces leaf evaluation from O(1203 × 128) to O(Δ × 128) where Δ is the number of changed features — typically 5-20 for a single move.

### Model Artifact Format

```rust
pub struct NnueModelArtifact {
    metadata: ModelMetadata,       // ID, generation, Elo, etc.
    schema: NnueFeatureSchema,     // Feature layout version
    weights: QuantizedWeights,     // All weight/bias tensors
    checksum: u64,                 // ahash integrity check
}
```

Serialized via `bincode` (binary, production) or `serde_json` (debugging).

### Bootstrap (Generation 0)

Random initialization with small weights:
- Feature weights: uniform [-8, 8] (i16)
- Feature biases: uniform [-16, 16] (i32)
- Hidden weights: uniform [-8, 8] (i8)
- Hidden biases: uniform [-16, 16] (i32)
- Output weights: uniform [-16, 16] (i16)
- Deterministic seed: `0x40_AAEE_B007`

### AnyEvaluator (Runtime Switch)

```rust
pub enum AnyEvaluator {
    Heuristic(HeuristicEvaluator),
    Nnue(Box<NnueEvaluator>),
}
```

Enables switching between heuristic and NNUE evaluation at runtime without recompilation. Both backends implement the `Evaluator` trait.

---

## 5. Macro-Action Abstraction

**Crate:** `wh40k_search_abstraction` — Wraps low-level engine commands into high-level tactical decisions to control branching factor.

### The Problem

Warhammer 40,000 has an enormous branching factor. A single "move unit forward and shoot" involves multiple atomic commands (select unit, choose destination, confirm move, select shooting target, etc.). Searching at the atomic command level is computationally infeasible.

### MacroAction

```rust
pub struct MacroAction {
    pub id: MacroActionId,         // Unique ID within a candidate set
    pub label: String,             // Human-readable diagnostic label
    pub commands: Vec<Command>,    // 1+ atomic engine commands
    pub intent: TacticalIntent,    // Classified tactical purpose
    pub actor_units: Vec<UnitId>,  // Units involved
    pub priority_hint: i32,        // Static ordering priority from intent
}
```

A single macro-action encapsulates an entire tactical decision (e.g., "Move Intercessors to Objective B" or "Charge Angron into Custodian Guard") as one node in the search tree.

### TacticalIntent Classification (33 variants)

**Movement (12 intents):**
| Intent | Priority | Description |
|--------|----------|-------------|
| HoldObjective | 70 | Stay on a controlled objective |
| ContestObjective | 85 | Move to contest an enemy objective |
| MoveToCover | 55 | Seek terrain cover |
| StageCharge | 80 | Position within 9-12" for next-turn charge |
| Screen | 60 | Block enemy approach vectors |
| Retreat | 40 | Fall back from danger |
| LineUpShot | 65 | Position for optimal shooting |
| DenyReserveZone | 50 | Block deep-strike landing zones |
| HoldPosition | 35 | Remain stationary |
| AdvanceAggressively | 75 | Push forward with advance move |
| FallBackToShoot | 45 | Disengage to shoot |
| DeepStrikeArrive | 85 | Deploy from reserves |

**Shooting (5 intents):**
| Intent | Priority | Description |
|--------|----------|-------------|
| MaximizeKills | 70 | Shoot the most vulnerable target |
| SoftenChargeTarget | 75 | Weaken a unit you plan to charge |
| RemoveScoringUnit | 80 | Kill a unit on an objective |
| ForceBattleShock | 65 | Push a unit below half-strength |
| BracketHardTarget | 60 | Reduce wounds on high-wound models |

**Charge (3 intents):**
| Intent | Priority | Description |
|--------|----------|-------------|
| ChargeHighValueTarget | 100 | Charge the most valuable target |
| MultiChargeObjectiveSwing | 95 | Multi-charge to flip objective control |
| ChargeSoftenedTarget | 90 | Charge a weakened unit |

**Fight (4 intents):**
| Intent | Priority | Description |
|--------|----------|-------------|
| FightMaxDamage | 80 | Maximize damage output |
| FightPreserve | 60 | Minimize exposure in combat |
| ChooseStance | 70 | Custodes Ka'tah stance selection |
| ChooseWeaponProfile | 65 | Tristraen Vaultsword profile selection |

**Stratagem (5 intents):**
| Intent | Priority | Description |
|--------|----------|-------------|
| DefensiveStratagem | 55 | Use defensive stratagem (e.g., Insane Bravery) |
| OffensiveStratagem | 60 | Use offensive stratagem |
| MovementReaction | 50 | React to enemy movement (e.g., Fire Overwatch) |
| FightOrderManipulation | 45 | Manipulate fight order |
| DeclineStratagem | 5 | Pass on stratagem opportunity |

**Miscellaneous (4 intents):**
| Intent | Priority | Description |
|--------|----------|-------------|
| ScoreObjective | 85 | Claim VP at objective |
| AllocateBlessings | 70 | World Eaters blessing dice allocation |
| PhaseControl | 10 | End phase / pass |
| Generic | 0 | Unclassified |

### ActionGenerator

Generates `CandidateSet` (collection of `MacroAction` options) per phase:

- **Command Phase:** Blessing allocations (Rage-fuelled Advance, Total Carnage, Martial Excellence combos) + end phase
- **Movement Phase:** Per-unit destinations from tactical anchors (objectives, charge staging 9-12", screening positions, cover, advance routes, fall-back positions, reserve arrival >9" from enemies)
- **Shooting Phase:** One macro-action per unit with best target heuristically selected
- **Charge Phase:** Single-target and multi-charge options (up to 3 targets ranked by distance)
- **Fight Phase:** Fight order selection, Ka'tah stance, Vaultsword profiles
- **Setup/Reaction:** Legal commands from `DecisionSurface`

**Charge Sub-Phase Flow:** Before falling through to phase-specific generation, `generate()` checks for mandatory sub-steps in priority order:
1. **Reaction windows** → DeclineStratagem (or UseOverwatch in future)
2. **Pending charge rolls** → ResolveChargeRoll with `roll: 0` sentinel (executor rolls actual 2D6)
3. **Pending charge moves** → MakeChargeMove with validated destination (checks ER against actual target models, non-target enemy proximity, board bounds)
4. Normal phase-specific generation

**Deployment Gating:** EndPhase is blocked during PreBattle if any player still has undeployed units. This prevents the AI from skipping deployment.

**Duplicate Action Prevention:** Candidates for Ka'tah stances and Vaultswords profiles are only generated for units/models that haven't already chosen this turn (checked via `TurnFlags`).

**Pre-Validation Filter:** All generated candidates are validated against `CommandValidator` before being returned. Invalid candidates are silently removed, preventing the UI from showing illegal options and preventing AI stuck loops:
```rust
candidates.candidates.retain(|action| {
    action.commands.iter().all(|cmd| {
        CommandValidator::validate(state, cmd).is_legal()
    })
});
```

### Action Vocabulary (528 = 33 × 16)

For training data encoding, each macro-action maps to a vocabulary index:

```
vocab_index = intent_index × 16 + unit_slot_index
```

- **33 intents** × **16 unit slots** (max units per player) = **528 possible actions**
- Used as the policy head output dimension in the PolicyValueModel

---

## 6. Move Ordering

**Crate:** `wh40k_search_ordering` — Prioritizes candidates to maximize alpha-beta cutoffs.

### Ordering Priority (Highest → Lowest)

| Priority | Source | Bonus | Description |
|----------|--------|-------|-------------|
| 1 | TT Best Move | +20,000 | From transposition table hit |
| 2 | Primary Killer | +9,000 | Most recent cutoff move at this ply |
| 3 | Secondary Killer | +8,000 | Second-most recent cutoff at this ply |
| 4 | History Heuristic | 0-5,000 | Logarithmic depth-weighted success score |
| 5 | Tactical Priority | 0-100 | From `TacticalIntent::ordering_priority()` |
| 6 | Action Hint | Variable | From candidate generation priority |

### Killer Move Table

Two-slot scheme per ply depth:

```
On beta cutoff at ply P:
  if action ≠ killers[P][0]:
    killers[P][1] = killers[P][0]    // Demote primary → secondary
    killers[P][0] = action            // New primary

During ordering at ply P:
  if action matches killers[P][0]: +9,000
  if action matches killers[P][1]: +8,000
```

Matching uses `TacticalIntent` comparison (not exact action equality) for reliability across transpositions.

### History Heuristic

64-entry table indexed by tactical intent. Tracks cumulative search success:

```
On beta cutoff (depth d):
  history[intent] += (d + 1)²         // Quadratic depth bonus

On move searched but no cutoff (depth d):
  history[intent] -= (d + 1)          // Linear failure penalty

Overflow prevention:
  if any entry > 1,000,000: all entries halved

Scoring for move ordering:
  bonus = log(max(1, history[intent])) × 500   // Range [0, 5000]
```

Persists across searches within a game; cleared between games.

### Ordering Functions

- `order_moves(candidates, ply, tt_best_move)` → Sorted `Vec<OrderedMove>` (descending score)
- `heuristic_order(candidates, state, evaluator)` → Rank by position evaluation delta (expensive, root only)
- `static_order(candidates)` → Rank by tactical intent priority only (fast)
- `record_cutoff(ply, action)` → Updates both killer table and history
- `record_searched(depth, action)` → Applies history failure penalty

---

## 7. Transposition Table

**Crate:** `wh40k_transposition` — Hash table storing previously searched positions to avoid redundant subtree evaluation.

### Entry Format (24 bytes)

```rust
pub struct TTEntry {
    pub hash_key: u64,         // Full hash for collision detection
    pub score: i32,            // Evaluation (centipawns)
    pub depth: u8,             // Search depth this entry was computed at
    pub bound: TTBound,        // Type of bound (Exact/Lower/Upper)
    pub best_move_index: u16,  // Index into candidate list (u16::MAX = none)
    pub generation: u16,       // Search generation for aging
}
```

### Bound Types

| Bound | Meaning | Created When |
|-------|---------|-------------|
| `Exact` | Score is the true minimax value at this depth | Node fully searched, score between alpha and beta |
| `LowerBound` | True score ≥ stored score | Beta cutoff (score ≥ beta) |
| `UpperBound` | True score ≤ stored score | Fail-low (score ≤ alpha) |

### Probe Logic

```
probe(hash, required_depth) → TTProbeResult:
  entry = table[hash & (size - 1)]

  if entry.hash_key == hash:
    if entry.depth >= required_depth:
      → Hit(entry)     // Can use for score cutoff
    else:
      → ShallowHit(entry)  // Can use best_move for ordering only
  else:
    → Miss
```

**Using a Hit for cutoff:**
```
match entry.bound:
  Exact  → return entry.score                    // Exact match
  Lower  → if entry.score >= beta: return score   // Beta cutoff
  Upper  → if entry.score <= alpha: return score   // Alpha cutoff
  _      → use entry.best_move_index for ordering only
```

### Replacement Policy (Always-Replace with Generation Aging)

Replace existing entry if:
1. Slot is empty (`hash_key == 0`)
2. Same position (hash keys match) with equal or deeper depth
3. Entry is from an older generation (stale)
4. New entry has strictly greater depth

### Size and Indexing

- Power-of-two bucket count for efficient `hash & (size - 1)` modular indexing
- Minimum 1024 entries
- `TranspositionTable::with_mb(n)` creates a table sized to `n` megabytes
- Generation counter incremented per root search via `new_generation()`
- Wraps at `u16::MAX` (~65,535 searches)

### Statistics

```rust
pub struct TTStats {
    pub capacity: usize,
    pub generation: u16,
    pub stored_count: u64,
    pub hit_count: u64,
    pub miss_count: u64,
    pub overwrite_count: u64,
    pub occupancy: f64,     // Sampled, 0.0-1.0
    pub hit_rate: f64,      // 0.0-1.0
}
```

---

## 8. Stockfish-Style Search Engine

**Crate:** `wh40k_search_core` — The primary search system, implementing a full Stockfish-style iterative deepening framework.

### 8.1 Greedy AI (Depth 0)

Simplest search: evaluate every candidate at the root, pick the highest-scoring one.

```
for action in generate_candidates(state):
    state' = apply(state, action)
    score = evaluate(state', perspective)
return argmax(score)
```

Complexity: O(C) where C = number of candidates.

### 8.2 One-Ply Search (Depth 1)

One-move lookahead with heuristic ordering:

```
candidates = generate_candidates(state)
ordered = heuristic_order(candidates)
for action in ordered[..max_candidates]:
    state' = apply(state, action)
    score = evaluate(state', perspective)
return argmax(score)
```

### 8.3 Negamax with Alpha-Beta Pruning

The core recursive search algorithm:

```
function negamax(state, depth, α, β, perspective, ply):
    nodes_evaluated += 1

    // Terminal checks
    if state is terminal:
        return terminal_score(state, perspective, ply)
    if player_tabled(perspective):
        return SCORE_LOSS + ply       // Prefer losing later

    // Leaf evaluation
    if depth == 0:
        return evaluate(state, perspective)

    // Transposition table probe
    if TT enabled:
        probe = tt.probe(state.hash, depth)
        if probe provides cutoff: return probe.score
        tt_best_move = probe.best_move_index

    // Generate and order candidates
    candidates = generate(state, perspective)
    ordered = order_moves(candidates, ply, tt_best_move)

    best_score = SCORE_LOSS + ply
    best_move = None
    original_α = α

    for action in ordered[..max_candidates]:
        state' = apply(state, action)
        next_perspective = state'.decision_owner

        if next_perspective == perspective:
            score = negamax(state', depth-1, α, β, perspective, ply+1)
        else:
            score = -negamax(state', depth-1, -β, -α, opponent, ply+1)

        if score > best_score:
            best_score = score
            best_move = action
            α = max(α, score)

        if α ≥ β:                 // Beta cutoff
            record_cutoff(action, ply)
            break

    // Store in transposition table
    bound = if best_score ≤ original_α: UpperBound
            elif best_score ≥ β: LowerBound
            else: Exact
    tt.store(hash, best_score, depth, bound, best_move)

    return best_score
```

### 8.4 Iterative Deepening (Full Feature Set)

The strongest search mode, combining all Stockfish-inspired enhancements:

#### Iterative Deepening Loop

```
previous_score = 0
previous_pv = []

for depth in 1..=max_depth:
    if soft_time_exceeded():
        break                       // Don't start new iteration

    score = search_with_aspiration(state, depth, previous_score)

    record_iteration(depth, score, pv, nodes, time)
    previous_score = score
    previous_pv = pv

    adjust_time_for_position(phase, pv_stability, score_gap, candidates)

    if |score| ≥ SCORE_WIN - 100:
        break                       // Found forced win/loss
```

Each completed iteration provides a usable result. If time runs out mid-iteration, the previous iteration's result is used. This gives graceful degradation under time pressure.

#### Aspiration Windows

Narrows the search window around the previous iteration's score:

```
α = previous_score - initial_delta
β = previous_score + initial_delta
delta = initial_delta                // Typically 50 centipawns

loop:
    score = negamax_pv(state, depth, α, β, ...)

    if score ≤ α:                    // Fail-low
        α = max(SCORE_LOSS, α - delta)
        delta *= 2
        aspiration_resets += 1
        continue

    if score ≥ β:                    // Fail-high
        β = min(SCORE_WIN, β + delta)
        delta *= 2
        aspiration_resets += 1
        continue

    break                            // Score within window
```

Benefits: Reduces search tree size when the previous score is accurate. Typically requires 0-3 re-searches in sharp positions.

#### Principal Variation (PV) Tracking

The PV is the predicted best line of play. It is tracked through the recursive search:

```
function negamax_pv(..., pv_line):
    ...
    for action in ordered_moves:
        child_pv = PvLine::new()
        score = negamax_pv(child_state, ..., child_pv)

        if score > α:
            pv_line = [action] + child_pv    // Update PV
    ...
```

**PV Seeding:** The previous iteration's PV is used to seed move ordering at each ply in the next iteration. When no TT best move is available, the search looks for the PV move that matches the current ply and uses it as the first candidate. This dramatically improves move ordering efficiency between iterations.

#### Quiescence Search

Continues searching "unstable" positions beyond the normal depth limit to avoid horizon effects:

```
function quiescence(state, α, β, perspective, ply, qs_depth):
    // Stand-pat: assume we can achieve at least static eval
    stand_pat = evaluate(state, perspective)

    if stand_pat ≥ β:
        return β                     // Position already good enough

    α = max(α, stand_pat)

    // Check stability
    if position_is_stable(state) or qs_depth ≥ max_qs_depth:
        return stand_pat

    // Search unstable moves
    candidates = generate(state, perspective)
    ordered = order_moves(candidates, ply, None)

    for action in ordered[..max_candidates/2]:
        state' = apply(state, action)
        score = (recursive quiescence)

        α = max(α, score)
        if α ≥ β:
            return β                 // Cutoff

    return α
```

**Position Instability Detection:**

A position is considered unstable if:
- An active reaction window exists (e.g., Overwatch, Fights First)
- The game is in a mid-resolution sub-phase:
  - `ResolveAttacks`, `RollChargeDistance`, `MakeChargeMove`, `ResolveOverwatch`
  - `PileIn`, `ResolveMeleeAttacks`, `Consolidate`, `FightsFirst`, `RemainingCombats`
  - `EffectResolution`, `ReactionWindow`, `StratagemWindow`

These sub-phases represent positions where significant tactical consequences are pending and cutting off evaluation would produce unreliable scores.

#### Selective Extensions

Critical moves receive extra search depth to avoid missing tactical consequences:

```
function compute_extension(action, state, extensions_used):
    if extensions_used ≥ max_extensions:
        return 0                     // Prevent search explosion

    match action.intent:
        ChargeHighValueTarget | MultiChargeObjectiveSwing | ChargeSoftenedTarget:
            return 1                 // Charge outcomes are critical
        FightMaxDamage if phase == Fight:
            return 1                 // Combat resolution matters
        DefensiveStratagem | OffensiveStratagem:
            return 1                 // Stratagem effects can swing games
        FightOrderManipulation:
            return 1                 // Fight order manipulation is decisive
        MovementReaction if has_reaction_window:
            return 1                 // Overwatch, Fire Overwatch, etc.
        _:
            return 0
```

Extensions are applied before the recursive call: `child_depth = depth - 1 + extension`. A cumulative `extensions_used` counter prevents runaway depth increases.

#### Time Management

```rust
pub struct TimeManager {
    hard_limit_ms: u64,     // Must stop immediately
    soft_limit_ms: u64,     // Don't start new iteration
    node_budget: u64,       // Alternative stopping criterion
    start_time: Instant,
}
```

**Dynamic time allocation based on position characteristics:**

| Factor | Multiplier | Condition |
|--------|-----------|-----------|
| Phase sharpness | ×1.4 | Charge or Fight phase |
| | ×1.2 | Shooting phase |
| | ×1.0 | Movement phase |
| | ×0.6 | Command or GameEnd phase |
| PV stability | ×0.7 | PV didn't change from last iteration |
| Score gap | ×0.6 | Best move dominates by >200 centipawns |
| Branching factor | ×1.3 | >20 root candidates |

Applied to soft limit: `soft_limit_ms = base_soft_ms × factor`; capped at hard limit.

**Soft time factor:** Default 60% of total budget. Configurable via `soft_time_factor` (in 1/1000ths, so 600 = 60%).

#### Position Volatility

Numeric estimate (0-10+) used for time allocation:

| Factor | Score |
|--------|-------|
| Active reaction window | +3 |
| Fight phase | +4 |
| Charge phase | +3 |
| Shooting phase | +2 |
| Movement phase | +1 |
| Unstable position | +2 |

### 8.5 Lazy SMP (Skeleton)

Multi-threaded search framework (currently single-threaded, awaiting lock-free TT):

```rust
pub struct LazySmpSearch {
    config: SearchConfig,
    shared: SharedSearchState,    // Arc<AtomicBool> stop flag
    main_worker: IterativeDeepeningSearch,
}
```

**Design (from Stockfish):**
1. Main thread runs iterative deepening at depth 1, 2, 3, ...
2. Helper threads run with depth offsets for subtree diversity
3. All threads share a transposition table (requires lock-free atomic operations)
4. Main thread's time limit stops all helpers via shared `AtomicBool`
5. Best result comes from main thread's deepest completed iteration

**Blocking requirement:** The `TranspositionTable` must be made thread-safe with lock-free atomics before enabling true parallel search.

### 8.6 Search Telemetry

```rust
pub struct SearchStats {
    pub nodes_evaluated: u64,
    pub leaf_evaluations: u64,
    pub beta_cutoffs: u64,
    pub tt_cutoffs: u64,
    pub tt_ordering_hits: u64,
    pub max_depth_reached: u8,
    pub root_candidates: usize,
    pub iterations_completed: u8,
    pub aspiration_resets: u64,
    pub quiescence_nodes: u64,
    pub extensions: u64,
    pub time_elapsed_ms: u64,
    pub nps: u64,                   // Nodes per second
    pub pv_changes: u64,
    pub seldepth: u8,               // Selective depth (max ply with extensions/QS)
}

pub struct SearchDiagnostics {
    pub iterations: Vec<IterationInfo>,  // Per-iteration breakdown
    pub final_stats: SearchStats,
    pub tt_occupancy: f64,
    pub tt_hit_rate: f64,
    pub stopped_by_time: bool,
    pub stopped_by_nodes: bool,
    pub search_phase: Phase,
    pub volatility: u8,
}
```

### 8.7 Typical Performance

| Algorithm | Typical Nodes | Branching ~30-40 candidates |
|-----------|--------------|----------------------------|
| Greedy | ~40 | O(C) |
| One-ply | ~1,200 | O(C²) |
| Negamax(3) | 10K-30K | O(C^d) → O(C^(d/2)) with alpha-beta |
| ID to depth 3 | 35K-50K | Cumulative across iterations |
| ID to depth 4 | 200K-400K | Deeper iterations dominate |

---

## 9. AlphaZero-Style MCTS

**Crate:** `wh40k_search_core` (module `mcts`) — Monte Carlo Tree Search with PUCT selection, Dirichlet exploration noise, and training data extraction.

### Configuration

```rust
pub struct MctsConfig {
    pub num_simulations: u32,       // Tree walks per search (default 800)
    pub c_puct: f32,                // PUCT exploration constant (1.5-2.5)
    pub temperature: f32,           // Action selection temperature
    pub dirichlet_alpha: f32,       // Root noise concentration
    pub dirichlet_epsilon: f32,     // Noise blend weight (typically 0.25)
    pub max_depth: u32,             // Forced leaf evaluation depth
    pub use_heuristic_prior: bool,  // Use eval scores as policy prior
    pub rollout_depth: u32,         // Random rollout steps
    pub fpu_reduction: f32,         // First Play Urgency penalty for unvisited
}
```

**Presets:**

| Preset | Simulations | c_puct | Temperature | Use |
|--------|------------|--------|-------------|-----|
| `default_exploration()` | 800 | 2.0 | 1.0 | Training self-play |
| `fast()` | 100 | 2.0 | 1.0 | Testing |
| `competition()` | 1,600 | 1.5 | 0.1 | Match play |

### Arena-Based Tree

```rust
struct MctsTree {
    nodes: Vec<MctsNode>,   // Flat array (arena allocation)
    root: NodeIndex,        // Index 0
}

struct MctsNode {
    action: Option<MacroAction>,    // Incoming edge action
    parent: Option<NodeIndex>,
    children: Vec<NodeIndex>,
    visits: u32,                    // N(s)
    value_sum: f64,                 // Σ V(s) backpropagated values
    prior: f32,                     // P(s,a) from policy
    is_expanded: bool,
    is_terminal: bool,
    terminal_value: f64,
    perspective: PlayerId,
    depth: u32,
}

// Average value: Q(s,a) = value_sum / visits
```

Arena allocation avoids per-node heap allocation overhead and provides cache-friendly memory layout.

### MCTS Algorithm

Each search runs `num_simulations` iterations. Each iteration has four phases:

#### Phase 1: Selection (PUCT)

Walk down the tree from root, at each node selecting the child that maximizes the PUCT score:

```
PUCT(s, a) = Q(s,a) + c_puct × P(s,a) × √N(parent) / (1 + N(child))

Where:
  Q(s,a) = average value (value_sum / visits), or FPU for unvisited nodes
  P(s,a) = prior probability from policy
  N(s)   = visit count
  c_puct = exploration constant

For unvisited children (N=0):
  Q = parent.Q - fpu_reduction    // First Play Urgency
```

The PUCT formula balances exploitation (high Q) with exploration (high prior P divided by visit count N). As a node is visited more, its exploration bonus shrinks, causing the search to shift to less-explored alternatives.

#### Phase 2: Expansion

When selection reaches an unexpanded node:
```
Generate all legal macro-actions for the current state
Create a child node for each action
Set priors (uniform, heuristic, or neural — see below)
```

#### Phase 3: Evaluation

Evaluate the leaf position:
- **Terminal state:** Assign win (+1) / loss (0) / draw (0.5)
- **Max depth reached:** Use heuristic evaluation, normalized to [0, 1]
- **Normal leaf:** Heuristic evaluation, normalized via sigmoid

**Score normalization (centipawns → [0, 1]):**
```
normalized = 1 / (1 + exp(-score / 1000))
```

#### Phase 4: Backpropagation

Update all nodes from leaf to root:

```
current = leaf_node

while current is not None:
    current.visits += 1

    if current.perspective == root_perspective:
        current.value_sum += value
    else:
        current.value_sum += (1.0 - value)    // Flip for opponent

    current = current.parent
```

The perspective-aware backpropagation ensures that Q-values always represent "goodness from the node's perspective" while the root evaluates moves from a consistent viewpoint.

### Action Selection from Root

After all simulations complete, select the action from the root node:

**Temperature-based selection:**
```
visit_counts = [child.visits for child in root.children]

if temperature ≤ 0.01:
    best = argmax(visit_counts)          // Deterministic (competition)
else:
    powered = [v^(1/temperature) for v in visit_counts]
    probs = powered / sum(powered)
    best = argmax(probs)                 // Stochastic (training)
```

Higher temperature → more exploration / more random; lower temperature → more exploitation / more deterministic.

### Policy Priors

Three prior modes:

**Uniform:**
```
prior = 1.0 / num_children    // Equal for all
```

**Heuristic (from evaluator):**
```
for each child action:
    state' = apply(state, action)
    score = evaluate(state', perspective)

priors = softmax(scores)       // Higher score → higher prior
```

**Neural (from PolicyValueModel):**
Policy head outputs logits over 528-action vocabulary, masked to legal actions and softmax-normalized.

### Dirichlet Noise

Applied to root priors for exploration diversity during self-play:

```
noise = Dirichlet(α)              // Generate n samples from Dir(α)
ε = dirichlet_epsilon              // Typically 0.25

for each root child i:
    child.prior = (1 - ε) × child.prior + ε × noise[i]
```

The Dirichlet distribution is sampled via Marsaglia-Tsang Gamma rejection sampling, providing correct Dirichlet samples without external library dependencies.

### Training Data Extraction

After each MCTS search during self-play:

```
policy_target = [(i, child.visits / total_visits)
                 for i, child in enumerate(root.children)]
```

This visit-count distribution becomes the policy training target — the fundamental idea from AlphaZero: the neural network learns to match MCTS's aggregated search results.

---

## 10. PolicyValue Dual-Head Network

**Crate (Rust):** `wh40k_eval_nnue` (PolicyValueModel) — **File (Python):** `python/train_nnue/policy_value_model.py`

The AlphaZero-style dual-head network that produces both a policy (action probabilities) and a value (position evaluation) from a single forward pass.

### Architecture

```
Input (1203 sparse features)
        │
        ▼
  Shared Trunk: Linear(1203 → 128) + ReLU
        │
        ├──────────────────┐
        ▼                  ▼
   Policy Head         Value Head
   Linear(128→64)      Linear(128→64)
   + ReLU              + ReLU
   Linear(64→528)      Linear(64→1)
   → logits            + tanh → [-1, 1]
```

### Dimensions

```rust
pub const DEFAULT: PolicyValueDimensions = {
    input_size: 1203,           // Same feature space as NNUE
    trunk_size: 128,            // Shared representation
    policy_hidden_size: 64,     // Policy head hidden layer
    policy_output_size: 528,    // ACTION_VOCAB_SIZE (33 intents × 16 slots)
    value_hidden_size: 64,      // Value head hidden layer
    value_output_size: 1,       // Scalar evaluation
};
```

### Forward Pass

```python
def forward(x, legal_mask=None):
    # Shared trunk
    trunk = relu(trunk_linear(x))           # [batch, 128]

    # Policy head
    policy_hidden = relu(policy_fc1(trunk))  # [batch, 64]
    policy_logits = policy_fc2(policy_hidden) # [batch, 528]

    # Mask illegal actions to -∞
    if legal_mask is not None:
        policy_logits[~legal_mask] = -1e9

    # Value head
    value_hidden = relu(value_fc1(trunk))    # [batch, 64]
    value = tanh(value_fc2(value_hidden))    # [batch, 1] in [-1, 1]

    return policy_logits, value
```

### Quantization for Rust Inference

```python
def quantize_policy_value_model(model):
    # Trunk weights: i16, scaled by 64
    # Hidden layer weights: i8, scaled by 64
    # Biases: i32 (full precision)
    # Output (value): i8, scaled by 64
```

Export format: JSON with `"dims"` and `"weights"` sections matching Rust `PolicyValueWeights` struct.

### Loss Function

```
L = α × L_policy + β × L_value - γ × H_entropy

Where:
  L_policy  = CrossEntropy(predicted_distribution, target_distribution)
  L_value   = MSE(predicted_value, target_value)
  H_entropy = -Σ p(a) × log(p(a))    // Entropy bonus encourages exploration

Default: α=1.0, β=1.0, γ=0.01
```

Illegal actions are masked before computing the policy loss — only legal actions contribute.

---

## 11. Self-Play Framework

**Crate:** `wh40k_selfplay` — Orchestrates games between AI agents, collects training data, and writes shards.

### Constants

```rust
const ACTION_VOCAB_SIZE: usize = 528;       // 33 intents × 16 slots
const INTENT_COUNT: usize = 33;
const MAX_UNIT_SLOTS: usize = 16;
const TOTAL_FEATURES: usize = 1203;
const SHARD_FORMAT_VERSION: u32 = 1;
const DEFAULT_SHARD_SIZE: usize = 4096;     // Samples per shard file
const MAX_COMMANDS_PER_GAME: usize = 10_000;
```

### Training Sample Format

Each decision point in a game produces one sample:

```rust
pub struct TrainingSample {
    pub sparse_features: Vec<(u16, i16)>,  // Feature (index, value) pairs
    pub legal_action_indices: Vec<u32>,     // Vocab indices of legal actions
    pub chosen_action_index: u32,           // Vocab index of action taken
    pub search_score: Score,                // Engine evaluation at this point
    pub outcome: f32,                       // Game result: +1 win, -1 loss, 0 draw
    pub perspective: u8,                    // Acting player (0 or 1)
    pub game_progress: f32,                 // Normalized: battle_round / 5
    pub state_hash: u64,                    // Deterministic hash for dedup
    pub battle_round: u8,
    pub phase: u8,
}
```

### Shard Format

```rust
pub struct TrainingShard {
    pub header: ShardHeader,
    pub samples: Vec<TrainingSample>,
}

pub struct ShardHeader {
    pub version: u32,                  // SHARD_FORMAT_VERSION
    pub engine_version: String,        // "0.1.0"
    pub feature_schema_version: u32,   // Feature layout version
    pub sample_count: usize,
    pub game_count: usize,
    pub model_generation: u32,         // Which model generated this data
    pub created_at: u64,               // Unix timestamp
    pub total_features: usize,         // 1203
    pub action_vocab_size: usize,      // 528
}
```

File naming: `shard_XXXXXX.bin` (bincode) / `shard_XXXXXX.json` (debug).

### Self-Play Game Loop

```
1. Configure both players (AI type, search depth, model)
2. Load scenario (seed, factions, mission, enhancements, secondaries)
3. Main loop:
   a. Get decision owner from game state
   b. Safety checks:
      - Command limit: break at MAX_COMMANDS_PER_GAME (10,000)
      - State hash stagnation: break if state unchanged for 5 iterations
   c. Call AI's choose_action() → SearchResult
   d. Repeated action detection: break if same action chosen 5x consecutively
   e. If collecting training data:
      - Extract sparse features for current state
      - Encode legal action mask (528 bools)
      - Record chosen action's vocab index
      - Store search score
   f. Execute each command in the macro-action
   g. Continue until game ends or safety limit hit
4. Label all samples with game outcome (+1/-1/0 from each player's perspective)
5. Return MatchResult (includes error_message if safety limit triggered)
```

**Scoring:** Primary and secondary objectives are scored automatically during phase transitions (end of Command Phase). No explicit scoring commands needed — the phase state machine in `phase.rs` calls `score_primary_objectives()` and `score_secondary_objectives()` at the end of each player's Command Phase.

**Dice Rolls:** Charge rolls use actual 2D6 via `state.dice_roller` (not AI-chosen values). The `ResolveChargeRoll { roll: 0 }` sentinel triggers real dice in the executor.

### Game Variation for Diversity

Each game gets varied configuration:
- **Mission cycling:** Rotates through 6 available missions
- **Faction alternation:** Swaps Custodes ↔ World Eaters every other game
- **Random enhancement:** Picks from faction's enhancement pool
- **Random secondary:** Picks from faction's secondary objective pool
- **Deterministic seeding:** `seed_base + game_index` ensures reproducibility

### Action Vocabulary Encoding

```
vocab_index = intent_index × 16 + unit_slot_index

Where:
  intent_index: TacticalIntent.to_index() → 0..32
  unit_slot_index: unit_id mapped to player-relative slot 0..15
```

This flattened representation allows the policy network to output a fixed-size vector regardless of the variable number of legal actions.

### Legal Action Mask

```rust
pub struct LegalMask {
    pub mask: [bool; 528],              // true = legal
    pub num_legal: usize,
    pub vocab_to_candidate: HashMap<u32, usize>,  // Vocab index → candidate list index
}
```

The mask is applied during training (illegal actions get -∞ logits) and during inference (only legal actions considered for selection).

---

## 12. PyO3 Trainer Bridge

**Crate:** `wh40k_trainer_bridge` (cdylib) — Provides Python bindings to the Rust engine via PyO3, enabling the Python training pipeline to interact with the game.

### PyGameState Class

Python-accessible game state wrapper:

```python
# Constructor
game = PyGameState(
    faction_a=0,       # 0=Custodes, 1=World Eaters
    faction_b=1,
    mission=None,      # Optional mission ID (0-5)
    seed=None          # Optional 32-byte seed, None for random
)

# Properties
game.is_in_progress        # bool
game.outcome               # "in_progress" | "victory_p0" | "victory_p1" | "draw"
game.winner                # -1 (no winner) | 0 | 1
game.battle_round          # u8 (1-5)
game.current_phase         # str
game.decision_owner        # u32 (0 or 1)
game.player1_vp            # i16
game.player2_vp            # i16
game.state_hash            # u64

# Feature extraction
game.encode_state_dense(perspective=None)    # → List[float], length 1203
game.encode_state_sparse(perspective=None)   # → List[(u16, i16)]
game.encode_legal_mask(perspective=None)     # → List[bool], length 528

# Action interface
game.num_legal_actions()                     # → int
game.legal_action_labels()                   # → List[(index, label, intent_name)]
game.legal_action_vocab_indices()            # → List[int]

# Step (apply action by index)
result = game.step(action_index)             # → {"reward": float, "done": bool, "info": str}
```

### Module-Level Functions

```python
import wh40k_trainer_bridge as wtb

# Constants
wtb.TOTAL_FEATURES          # 1203
wtb.ACTION_VOCAB_SIZE       # 528

# Shard loading
samples = wtb.load_all_shards("path/to/shard_dir")     # Load all .bin shards
samples = wtb.load_shard_json("path/to/shard.json")     # Load single JSON shard

# Self-play
wtb.collect_training_data(
    num_games=100,
    output_dir="path/to/shards",
    ai_type="iterative_deepening",
    max_depth=3,
    model_generation=0
)

# Gating
result = wtb.evaluate_candidate(
    candidate_path="path/to/model.nnue",
    num_games=100,
    threshold=0.55
)

# Benchmarking
wtb.benchmark(num_games=10)
```

### Design Patterns

- Wraps Rust engine state as opaque Python objects
- Caches legal actions to avoid regeneration on repeated queries
- Returns Python-native types (dicts, lists, strings)
- Error handling via `PyErr` / `PyResult`
- Deterministic game replay via seed control

---

## 13. Training Pipelines

### 13.1 NNUE Training (Single-Head Value Network)

**Script:** `python/train_nnue/train.py`

Trains the scalar evaluation network (1203→128→32→32→1) on self-play data.

```
┌───────────┐     ┌──────────────┐     ┌───────────┐     ┌──────────┐
│ Self-Play │ ──→ │ Shard Files  │ ──→ │ Training  │ ──→ │  .nnue   │
│ (Rust)    │     │ (.bin/.json) │     │ (PyTorch) │     │ Artifact │
└───────────┘     └──────────────┘     └───────────┘     └──────────┘
```

**Training loop:**
1. Load training shards from directory
2. Create DataLoaders with train/validation split
3. Build NnueModel (single output head)
4. Adam optimizer with `ReduceLROnPlateau` scheduler
5. Per-epoch:
   - **Primary loss:** MSE between prediction and game outcome
   - **Auxiliary loss (optional):** MSE between prediction and `tanh(search_score / 10000)`
   - Combined: `loss = primary + search_score_weight × auxiliary`
6. Validation: sign-match accuracy (predicted vs actual outcome)
7. Checkpoint every N epochs
8. Export best model to `.nnue` (bincode) format

**Key parameters:**
```
batch_size: 256
learning_rate: 1e-3
weight_decay: 1e-4
gradient_clipping: max_norm 1.0
warmup_epochs: 5
lr_patience: 10
min_lr: 1e-6
```

### 13.2 PolicyValue Training (AlphaZero Dual-Head)

**Script:** `python/train_nnue/mcts_train.py`

Trains the AlphaZero-style dual-head network (policy + value) on MCTS self-play data.

```
┌───────────┐     ┌──────────────┐     ┌───────────┐     ┌──────────┐
│ MCTS      │ ──→ │ PV Shard     │ ──→ │ Training  │ ──→ │ PV Model │
│ Self-Play │     │ Files        │     │ (PyTorch) │     │ (.pt+.json)
│ (Rust)    │     │              │     │           │     │          │
└───────────┘     └──────────────┘     └───────────┘     └──────────┘
```

**Training data format (per sample):**
```json
{
    "sparse_features": [[idx, val], ...],
    "legal_mask": {"mask": [true, false, ...]},
    "policy_target": [0.0, 0.1, ..., 0.05],      // MCTS visit distribution
    "value_target": 1.0,                           // Game outcome
    "progress": 0.6,                               // battle_round / 5
    "perspective": 0,
    "search_score": 1500
}
```

**Training loop:**
1. Load PolicyValue shard samples
2. Split train/val (90/10 default)
3. Create PolicyValueNet
4. Training:
   - Loss = α × CrossEntropy(policy) + β × MSE(value) - γ × Entropy
   - Gradient clipping at norm 1.0
   - Warmup: linear LR ramp for first N epochs
   - `ReduceLROnPlateau` scheduler (patience=10)
   - Checkpoint every 10 epochs
   - Early stopping if LR bottoms out with no improvement
5. Outputs:
   - **Best model checkpoint** (lowest validation loss)
   - **Quantized JSON export** (for Rust PolicyValueModel)
   - **Training log** (per-epoch metrics in JSON)

**Validation metrics:**
- **Policy accuracy:** argmax of predicted vs argmax of target distribution
- **Value accuracy:** sign match (predicted vs target)
- **Combined loss**

### 13.3 Full Training Pipeline

The complete reinforcement learning loop:

```
                    ┌──────────────────┐
                    │   Bootstrap      │
                    │   (Gen 0: random │
                    │    weights)      │
                    └────────┬─────────┘
                             │
           ┌─────────────────▼──────────────────┐
           │                                     │
           ▼                                     │
    ┌─────────────┐                              │
    │  Self-Play   │ Current best model plays    │
    │  (Rust engine)│ against itself              │
    └──────┬──────┘                              │
           │ Training shards                     │
           ▼                                     │
    ┌─────────────┐                              │
    │  Training    │ PyTorch trains new model    │
    │  (Python)    │ on self-play data           │
    └──────┬──────┘                              │
           │ Candidate model                     │
           ▼                                     │
    ┌─────────────┐                              │
    │   Gating     │ Candidate vs baseline       │
    │   (Elo-based)│ head-to-head evaluation     │
    └──────┬──────┘                              │
           │                                     │
           ├─── Win rate ≥ 55% ──→ Promoted ─────┘
           │                       (becomes new baseline)
           │
           └─── Win rate < 55% ──→ Rejected
                                   (try again with more data)
```

**CLI commands in train.py:**
1. `generate` — Run self-play to create training data
2. `train` — Train model on shards
3. `gate` — Evaluate candidate vs baseline
4. `benchmark` — Throughput testing
5. `pipeline` — Run generate → train → gate sequentially

**Native CLI commands (Rust):**
```bash
# Self-play data generation
cargo run -p wh40k_native_api --release -- selfplay \
  --games 10000 --ai greedy --output-dir ./shards/gen0

# Play a single game (verbose)
cargo run -p wh40k_native_api --release -- play \
  --ai-a id --ai-b greedy -v

# Benchmark (head-to-head)
cargo run -p wh40k_native_api --release -- benchmark \
  --games 100 --ai-a id --ai-b greedy
```

**Measured Throughput (Apple M-series):**

| Step | Speed | Notes |
|------|-------|-------|
| Greedy selfplay | ~139K games/hr | ~120 samples/game, 50/50 completion |
| ID selfplay | ~70 games/hr | Deeper search, higher quality data |
| NNUE training (50 epochs, 1.2M samples) | ~19 min | Apple Metal (MPS) GPU |
| Gating (100 games) | ~2 sec | Greedy NNUE vs heuristic |

**Gen 0 Baseline Results:**
- 10,000 greedy games → 1,201,758 samples → 294 shards (~4 min)
- Training: loss 0.428 → 0.319, accuracy 59.4% (50 epochs, ~19 min)
- Gating: 7% win rate vs heuristic (expected for Gen 0 — bootstrap only)
- Improvement loop: Gen 1+ uses stronger AI (one-ply/negamax) for better training signal

---

## 14. Gating & Elo Rating

**Crate:** `wh40k_selfplay` — Evaluates candidate models against the current baseline to determine promotion.

### Gating Configuration

```rust
pub struct GatingConfig {
    pub num_games: usize,              // Default 100
    pub promotion_threshold: f64,      // Default 0.55 (55% win rate)
    pub seed_base: u64,
    pub missions: Vec<MissionId>,
    pub alternate_factions: bool,      // Swap players each game
    pub collect_training_data: bool,   // Gather samples during gating
}
```

### Gating Process

1. **Setup:** Configure candidate (new model) and baseline (current best) as AI players
2. **Match play:** Run `num_games` games, alternating which slot (player 0/1) the candidate occupies
3. **Scoring:**
   ```
   candidate_win_rate = (candidate_wins + 0.5 × draws) / total_games
   ```
4. **Decision:** If `candidate_win_rate ≥ promotion_threshold`: promote

### Elo Rating System

```
Expected score:
  E(A) = 1 / (1 + 10^((R_B - R_A) / 400))

Rating update:
  R'_A = R_A + K × (S_A - E(A))
  Where: K=32, S_A = actual score (1 for win, 0.5 for draw, 0 for loss)

Elo delta from win rate:
  Δ_Elo = 400 × log10(win_rate / (1 - win_rate))
  Clamped to [-800, 800]

95% confidence interval:
  SE = √(win_rate × (1 - win_rate) / num_games)
  CI = [Δ_Elo - 1.96×SE×scale, Δ_Elo + 1.96×SE×scale]
```

### Gating Result

```rust
pub struct GatingResult {
    pub candidate_wins: u32,
    pub baseline_wins: u32,
    pub draws: u32,
    pub candidate_win_rate: f64,
    pub promoted: bool,
    pub elo_delta: i32,
    pub candidate_elo: f64,
    pub baseline_elo: f64,
    pub elo_confidence_interval: (f64, f64),  // 95% CI
    pub duration_ms: u64,
    pub total_samples: usize,
}
```

### Promotion Thresholds

| Win Rate | Approx. Elo Delta | Interpretation |
|----------|-------------------|----------------|
| 50% | 0 | Equal strength |
| 55% | +35 | Default threshold — modest improvement |
| 60% | +72 | Clear improvement |
| 65% | +110 | Strong improvement |
| 70% | +150 | Dominant |

The 55% threshold corresponds to roughly +35 Elo, enough to confirm real improvement while being achievable within 100 games.

---

## 15. Model Lineage & Registry

**Crate:** `wh40k_eval_nnue` (registry) + `wh40k_selfplay` (lineage)

### Model Metadata

```rust
pub struct ModelMetadata {
    pub model_id: String,               // e.g., "gen0", "gen1_20240315"
    pub generation: u32,                // 0 = bootstrap, 1+ = trained
    pub schema_version: u32,            // Feature layout compatibility
    pub dimensions: NnueDimensions,
    pub description: String,
    pub parent_model_id: Option<String>,
    pub training_positions: u64,
    pub training_loss: f64,
    pub elo_estimate: i32,
    pub created_at: u64,                // Unix epoch seconds
    pub total_parameters: usize,
}
```

### Registry Directory Structure

```
base_path/
├── models/
│   ├── gen0_bootstrap.nnue
│   ├── gen1_20240315.nnue
│   ├── gen2_20240401.nnue
│   └── ...
└── registry_index.json
```

**Operations:**
- `store(artifact)` — Saves model artifact and updates index
- `load(model_id)` — Loads and validates artifact (checksum verification)
- `list()` — Returns all model metadata
- `latest()` — Gets highest generation model
- `bootstrap()` — Creates gen0 if not present

### Lineage Tracking

```rust
pub struct LineageEntry {
    pub model_id: String,
    pub generation: u32,
    pub parent_model_id: Option<String>,
    pub training_positions: u64,
    pub training_loss: f64,
    pub elo_estimate: i32,
    pub promoted: bool,
    pub gating_win_rate: Option<f64>,
    pub gating_games: Option<u32>,
    pub gating_elo_delta: Option<i32>,
    pub created_at: u64,
    pub description: String,
}

pub struct ModelLineage {
    pub entries: Vec<LineageEntry>,
}
```

**Lineage methods:**
- `latest()` — Most recent entry
- `latest_promoted()` — Most recent promoted model (current baseline)
- `promoted_models()` — Filter to promoted only
- `next_generation()` — Next generation number
- `current_elo()` — Cumulative Elo of latest promoted model
- `total_training_positions()` — Sum across all generations
- `save()` / `load()` — JSON persistence

### Promotion Flow

```
Gen 0 (bootstrap):
  random weights → promoted (baseline by default)

Gen N (training cycle):
  1. Self-play with Gen N-1 baseline → training shards
  2. Train candidate model on shards
  3. Gate: candidate vs Gen N-1 baseline
  4. If win_rate ≥ 0.55:
     → Create LineageEntry(promoted=true, elo=parent_elo + elo_delta)
     → Candidate becomes new baseline
  5. If win_rate < 0.55:
     → Create LineageEntry(promoted=false, elo=parent_elo)
     → Retry with more data or different hyperparameters
```

---

## 16. Determinism Guarantees

The engine provides bit-perfect deterministic replay — critical for self-play training, debugging, and verification.

### Mechanisms

| Concern | Solution |
|---------|----------|
| Floating point | Fixed-point arithmetic throughout game logic (no floats in game state) |
| Hash maps | Fixed-seed AHasher (`ahash::RandomState::with_seeds(...)`) |
| Collection ordering | Sorted collections wherever iteration order matters |
| Random number generation | Seeded RNG per game (`[u8; 32]` seed) |
| Feature extraction | Deterministic sparse feature vectors from identical game states |
| Replay verification | Hash chain: `state_hash` recorded at each decision point |

### Verification

The `verify` command in the native CLI:
1. Plays a game to completion, recording all decisions
2. Replays the game from the same seed, replaying all decisions
3. Compares `state_hash` at each decision point
4. Fails if any hash mismatch detected

Multiple seeds tested per verification run for confidence.

---

## 17. Configuration Reference

### SearchConfig Presets

```rust
SearchConfig::greedy()                          // Depth 0
SearchConfig::one_ply()                         // Depth 1
SearchConfig::negamax(depth)                    // Fixed depth, alpha-beta + TT
SearchConfig::iterative_deepening(max_depth)    // Full ID
SearchConfig::iterative_deepening_timed(ms)     // Time-limited ID
```

### SearchConfig Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_depth` | `u8` | varies | Maximum search depth in plies |
| `max_candidates` | `usize` | 64 | Maximum moves to consider per node |
| `tt_size_mb` | `usize` | 16 | Transposition table memory budget |
| `use_alpha_beta` | `bool` | true | Enable alpha-beta pruning |
| `use_tt` | `bool` | true | Enable transposition table |
| `use_move_ordering` | `bool` | true | Enable killer/history heuristics |
| `chance_samples` | `u32` | 8 | Samples for stochastic nodes (dice) |
| `use_iterative_deepening` | `bool` | false | Enable ID loop |
| `use_aspiration_windows` | `bool` | false | Narrow search windows |
| `aspiration_window_initial` | `Score` | 50 | Initial aspiration delta |
| `aspiration_window_delta` | `Score` | 50 | Window expansion on reset |
| `use_quiescence` | `bool` | false | Continue in unstable positions |
| `quiescence_max_depth` | `u8` | 4 | Max additional QS depth |
| `use_extensions` | `bool` | false | Selective depth extension |
| `max_extensions` | `u8` | 3 | Max cumulative extensions per path |
| `time_budget_ms` | `u64` | 0 | Total time budget (0 = unlimited) |
| `soft_time_factor` | `u32` | 600 | Soft limit % (thousandths, 600=60%) |
| `smp_threads` | `usize` | 1 | Lazy SMP thread count |

### AiLevel Enum (Unified Entry Point)

```rust
pub enum AiLevel {
    Greedy,
    OnePly,
    Negamax,
    NegamaxDepth(u8),
    IterativeDeepening,
    IterativeDeepeningDepth(u8),
    IterativeDeepeningTimed(u64),
    LazySmp,
    Mcts,
    MctsSimulations(u32),
}
```

Used via `SearchRoot::new(level)` for simplified API access.

### MctsConfig Presets

| Preset | Simulations | c_puct | Temperature | Use |
|--------|------------|--------|-------------|-----|
| `default_exploration()` | 800 | 2.0 | 1.0 | Training |
| `fast()` | 100 | 2.0 | 1.0 | Testing |
| `competition()` | 1,600 | 1.5 | 0.1 | Match play |

---

## 18. Crate Map

| Crate | Lines | Purpose |
|-------|-------|---------|
| `wh40k_eval_features` | ~2,800 | Feature extraction (1203-dim sparse vectors) |
| `wh40k_eval_heuristic` | ~1,600 | Handcrafted 15-term weighted evaluator |
| `wh40k_eval_nnue` | ~3,400 | NNUE (1203→128→32→32→1) + PolicyValueModel + Registry |
| `wh40k_search_abstraction` | ~3,200 | MacroAction, TacticalIntent, ActionGenerator, CandidateSet |
| `wh40k_search_ordering` | ~800 | Killer moves, history heuristic, move ordering |
| `wh40k_transposition` | ~600 | Transposition table (always-replace, generation aging) |
| `wh40k_search_core` | ~3,900 | All search algorithms (Greedy→ID→MCTS) + Lazy SMP |
| `wh40k_selfplay` | ~8,000 | Self-play framework, training data, gating, Elo, lineage |
| `wh40k_trainer_bridge` | ~1,500 | PyO3 Rust↔Python FFI bindings |
| **Python** | | |
| `policy_value_model.py` | ~350 | PyTorch dual-head network + quantization + export |
| `mcts_train.py` | ~1,300 | AlphaZero training pipeline |
| `train.py` | ~800 | NNUE single-head training pipeline |

**Total AI subsystem:** ~28,250 lines (Rust + Python)

---

## Appendix: Score Scale

| Score | Meaning |
|-------|---------|
| `+100,000` | `SCORE_WIN` — guaranteed victory |
| `+100,000 - ply` | Win in `ply` moves (prefer shorter wins) |
| `+1,000` to `+5,000` | Significant advantage |
| `+100` to `+1,000` | Moderate advantage |
| `0` | Even / draw |
| `-100,000` | `SCORE_LOSS` — guaranteed loss |
| `-100,000 + ply` | Loss in `ply` moves (prefer longer losses) |
| `i32::MIN + 1` | `SCORE_NONE` — not yet evaluated |

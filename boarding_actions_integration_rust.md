# Boarding Actions Integration for Rust Engine

## Purpose

This document replaces the earlier language-agnostic integration note with a Rust-specific implementation plan for integrating Boarding Actions into the existing Combat Patrol engine.

The goal is **not** to fork the rules engine into a separate game. The goal is to keep one Rust simulation core and add a `BoardingActions` rules mode that reuses the Combat Patrol engine where possible and overrides only the systems Boarding Actions actually changes.

---

## 1. Architectural position

Your current project already has a normal 40k / Combat Patrol rules baseline with:
- phased turn structure
- objective control
- strategic reserves
- terrain interaction
- standard stratagem plumbing
- pre-battle flow
- five battle rounds

Those baseline concepts should stay in the Rust engine as the shared core. Combat Patrol and Boarding Actions should be implemented as separate **rules overlays** on top of the same simulation kernel. Core 40k turn structure and objective handling are already present in your rules baseline. fileciteturn11file7 Boarding Actions then adds mode-specific changes such as enclosed board topology, hatchways, secured objectives, no normal Leader attachments, Boarding Actions stratagems, and a 500-point Boarding Patrol muster. fileciteturn11file6 fileciteturn11file3

Recommended high-level Rust layering:

```text
crate::core
  - ids
  - math
  - geometry
  - events
  - serialization

crate::rules40k
  - phases
  - attacks
  - objectives
  - command_points
  - reserves
  - terrain

crate::modes::combat_patrol
  - patrol roster rules
  - CP missions
  - CP faction packs

crate::modes::boarding_actions
  - boarding muster rules
  - hatchways
  - compartments / zones
  - boarding stratagems
  - leader battlefield-command adapter
  - mission loaders
```

---

## 2. What to reuse from Combat Patrol

These systems should remain shared Rust engine systems, not duplicated:

### 2.1 Shared core simulation
- turn / phase sequencing
- command point accounting
- unit activation state
- attack sequence
- damage resolution
- morale / Battle-shock plumbing
- reinforcement timing hooks
- victory point accumulation framework
- event dispatch / trigger resolution

### 2.2 Shared unit content
Use the same Rust-side datasheet/domain model for:
- units
- models
- weapons
- keywords
- abilities
- faction identifiers
- wound and save profiles

### 2.3 Shared mission runtime interfaces
Your mission runner can stay common if it already supports:
- mission setup hooks
- scoring hooks
- custom turn-start / turn-end triggers
- map-defined objective markers
- custom mission state

That is consistent with your current Boarding Actions content split into missions, maps, objectives, and mission tags. fileciteturn11file8 fileciteturn11file18 fileciteturn11file12

---

## 3. What Rust systems must be overridden for Boarding Actions

### 3.1 Battlefield representation
A normal open-table Combat Patrol map is not enough.

For Boarding Actions, the Rust map layer must support:
- hard wall segments
- hatchway edges between compartments
- open / closed hatch state
- special zone labels
- mission-specific inaccessible areas
- objective markers that can become secured and remain sticky until flipped

Boarding Actions missions explicitly depend on fixed mission-map data for wall placement, hatchway placement, objective marker placement, starting hatch states, and special labeled regions. fileciteturn11file8

### 3.2 Movement and visibility
The normal movement/LOS code needs a Boarding Actions rules adapter.

Rust systems needed:
- pathfinding on a compartment-and-hatch graph
- hatchway traversal gating
- visibility checks through walls / hatchways
- charge target visibility restriction
- fight movement restriction toward visible enemies only

Boarding Actions changes charging and fight movement compared with the normal rules baseline. Boarding Actions requires charge visibility and restricts pile-in / consolidation relative to visible enemy units. fileciteturn11file6 Normal 40k charge flow is broader and should remain in the shared base rules for non-boarding modes. fileciteturn11file5

### 3.3 Leader behavior
Combat Patrol / normal 40k pre-battle assumptions about attached Leaders cannot be reused as-is.

Rust requirement:
- disable normal attach-at-start behavior when `GameMode::BoardingActions`
- add a temporary projected-leader-effect system for `Battlefield Command`

Boarding Actions states that Leaders do not attach normally and instead project selected Leader abilities temporarily via the Battlefield Command Stratagem. fileciteturn11file6turn11file3

### 3.4 Stratagem availability
Do not reuse generic Combat Patrol stratagem availability rules verbatim.

Boarding Actions should load:
- universal Boarding Actions stratagem set
- chosen Boarding Actions detachment stratagem set
- not the ordinary core/Codex set unless a Boarding rule explicitly permits it

The Boarding Actions digest explicitly states that normal core/Codex stratagem sets are not used there. fileciteturn11file6

### 3.5 Mustering / roster validation
Rust roster validation for Boarding Actions must be a dedicated validator, not just Combat Patrol validation with a point cap tweak.

Boarding validator needs at minimum:
- max 500 points
- boarding-legal detachment selection
- Boarding Actions enhancement rules
- no illegal unit classes if prohibited by source content
- Warlord and enhancement legality checks

The Boarding Actions digest defines Boarding Patrol roster flow and 500-point army size separately from normal play. fileciteturn11file3

---

## 4. Rust crate/module recommendations

Recommended structure:

```text
src/
  core/
    game_id.rs
    event.rs
    geometry.rs
    serialization.rs

  sim/
    state.rs
    reducer.rs
    command.rs
    query.rs
    validator.rs

  rules40k/
    phases.rs
    combat.rs
    morale.rs
    objectives.rs
    reserves.rs
    terrain.rs

  modes/
    combat_patrol/
      mod.rs
      roster.rs
      missions.rs
      deployment.rs

    boarding_actions/
      mod.rs
      roster.rs
      map.rs
      hatchway.rs
      zones.rs
      mission_loader.rs
      mission_state.rs
      scoring.rs
      stratagems.rs
      leader_adapter.rs
      validator.rs
      queries.rs

  data/
    boarding_actions/
      missions_complete_v3.md
      maps_complete_v3.json
      objectives_complete_v3.json
      mission_tags_complete_v3.json
```

---

## 5. Recommended Rust domain model

### 5.1 Game mode enum
```rust
pub enum GameMode {
    CombatPatrol,
    BoardingActions,
}
```

### 5.2 Mission definition split
```rust
pub struct MissionPackage {
    pub mission_id: MissionId,
    pub mode: GameMode,
    pub map: MissionMap,
    pub scoring: MissionScoring,
    pub tags: MissionTags,
    pub scripted_rules: Vec<MissionRule>,
}
```

### 5.3 Boarding map model
```rust
pub struct BoardingMap {
    pub compartments: Vec<Compartment>,
    pub walls: Vec<WallSegment>,
    pub hatchways: Vec<Hatchway>,
    pub entry_zones: Vec<EntryZone>,
    pub objective_markers: Vec<ObjectiveMarker>,
    pub special_regions: Vec<SpecialRegion>,
}

pub struct Hatchway {
    pub id: HatchwayId,
    pub between: (CompartmentId, CompartmentId),
    pub state: HatchwayState,
    pub tags: Vec<HatchwayTag>,
}

pub enum HatchwayState {
    Open,
    Closed,
    Locked,
    OneWayOpened,
}
```

### 5.4 Mission-scoped state
```rust
pub struct BoardingMissionState {
    pub lighting_areas: HashMap<RegionId, LightingState>,
    pub secure_objectives: HashMap<ObjectiveId, SecuredBy>,
    pub corrupted_objectives: HashSet<ObjectiveId>,
    pub opened_airlocks: HashSet<HatchwayId>,
    pub mission_flags: HashMap<String, MissionValue>,
}
```

### 5.5 Leader projection adapter
```rust
pub struct BattlefieldCommandLink {
    pub leader_unit: UnitId,
    pub target_unit: UnitId,
    pub granted_rule: LeaderAbilityId,
    pub expires_at: PhaseBoundary,
}
```

---

## 6. Runtime flow in Rust

### 6.1 Match creation
1. Load shared datasheets/factions.
2. Parse selected mode.
3. If `CombatPatrol`, run CP validator and CP mission loader.
4. If `BoardingActions`, run Boarding validator and Boarding mission package loader.
5. Build `GameState` with a `ModeState` enum:

```rust
pub enum ModeState {
    CombatPatrol(CombatPatrolState),
    BoardingActions(BoardingMissionState),
}
```

### 6.2 During play
The reducer should branch only when necessary:
- `can_move_through_edge(...)`
- `is_target_visible(...)`
- `can_charge_target(...)`
- `available_stratagems(...)`
- `score_command_phase_objectives(...)`
- `apply_mission_endgame_scoring(...)`

This keeps most of the engine shared while isolating Boarding Actions behavior in mode-aware query functions.

### 6.3 Event-first approach
Use a Rust event model so mission rules are not hardcoded into one giant turn loop.

Examples:
- `PhaseStarted`
- `MovementEnded`
- `HatchwayOperated`
- `ObjectiveSecured`
- `CommandPhaseScoring`
- `UnitDestroyed`
- `BattleEnded`

Mission tags and objectives files can then bind handlers to these events rather than being manually checked everywhere.

---

## 7. JSON-to-Rust ingestion

Your Boarding content is already split in the right direction:
- mission digest
- map data
- objective data
- mission tags

Those should become Rust deserializable assets using `serde`.

Recommended pattern:
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct BoardingMissionMapAsset { /* ... */ }

#[derive(Debug, Clone, Deserialize)]
pub struct BoardingMissionObjectiveAsset { /* ... */ }

#[derive(Debug, Clone, Deserialize)]
pub struct BoardingMissionTagAsset { /* ... */ }
```

Load pipeline:
1. parse JSON assets
2. validate cross-file mission IDs
3. validate referenced objective/hatch/zone tags
4. build a `MissionPackage`
5. reject startup if any mission asset is inconsistent

---

## 8. AI implications for Rust engine

A Rust engine is actually a good fit here because Boarding Actions is highly stateful and spatially constrained.

The AI layer should consume engine queries, not raw JSON.

Useful Rust AI queries:
- shortest path to objective through current hatch states
- value of opening hatchway X
- expected exposure after moving into compartment Y
- contestability of secured objectives
- risk map by room / corridor / choke point
- mission action availability

Because Boarding Actions is compartmentalized, graph search and choke-point analysis become more important than open-table heuristics. Boarding Actions maps explicitly depend on labeled zones, hatchways, and entry zones. fileciteturn11file8turn11file12

---

## 9. UI contract from Rust core

Even if your front end is TypeScript or another language, the Rust engine should expose Boarding-specific render/query data.

Recommended query outputs:
- compartment graph
- wall and hatchway segments
- open/closed state per hatchway
- region labels and overlays
- objective secured state
- mission-special markers
- legal hatch interactions for selected unit

That prevents the UI from re-implementing rules logic.

---

## 10. Validation checklist for Rust integration

### Engine-level
- each Boarding mission loads without asset mismatch
- every referenced hatchway/objective/region exists
- starting hatch states initialize correctly
- mission entry zones are legal and role-aware

### Rules-level
- charges fail when target not visible in Boarding mode
- pile-in/consolidation respects visible-enemy restriction
- Leaders do not attach at battle start in Boarding mode
- Battlefield Command grants only permitted Leader ability effects
- secured objectives persist correctly until flipped

### Mission-level
- progressive scoring fires at the right step
- round-5 second-player timing changes work
- asymmetric attacker/defender scoring works
- special mission actions mutate state correctly
- end-game scoring reaches correct VP totals

---

## 11. What the previous integration doc should be corrected to say

Because the engine is Rust, the integration layer should not be described as generic “engine glue.” It should be described as:
- a **Rust rules-mode extension**
- with **serde-loaded Boarding assets**
- using **mode-specific validators, reducers, and queries**
- while keeping one shared simulation kernel

That is the cleanest architecture for your project.

---

## 12. Recommended next artifact

After this document, the next most useful file is:

`boarding_actions_engine_contract_rust.md`

That document should define exact Rust enums, structs, traits, event names, validator rules, and `serde` schemas for all Boarding Actions assets and runtime systems.

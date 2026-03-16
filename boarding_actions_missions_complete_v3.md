# Boarding Actions Missions

This document is an implementation-oriented companion to `boarding_patrol.md`. It focuses specifically on the **Boarding Actions Missions** section from the Wahapedia Boarding Action Rules page for Warhammer 40,000 10th Edition Boarding Actions.

It is intended as a **developer-facing mission rules digest**:
- mission taxonomy
- mission flow
- battlefield/setup notes that are specific to missions
- per-mission special rules
- per-mission scoring structure
- implementation notes for a digital rules engine

## Scope note

This document captures the **text rules and scoring structure** of the mission section in a clean reference format.

Exact mission-map visuals are paired with the companion `boarding_actions_maps_complete_v3.json`, which records a best-effort machine-readable transcription of mission-map labels, special regions, objective labels, and battlefield-structure notes.

---

# 1. Mission framework

## 1.1 Mission categories

Boarding Actions missions on the source page are split into two broad groups:

### Symmetric missions
Players use mirrored or effectively equivalent setups and pursue the same scoring structure.
- 11. Access Junction Primus
- 12. Deck Sweepers
- 13. The Pipeline
- 21. Power Struggle
- 22. Death in the Dark
- 23. Hull Breach
- 31. Control Centre
- 32. The Furnace
- 33. Rad Leak

### Asymmetric missions
Attacker and Defender have different setup permissions, turn order, special rules, and/or scoring incentives.
- 1. Void the Ship
- 2. Pull Their Teeth
- 3. Strongrooms
- 4. Jailbreak
- 5. Power the Generators
- 6. Corrupt the Machine Spirit

## 1.2 Universal mission scoring reminder

Across Boarding Actions missions:
- each player can score a maximum of **90 VP** from mission objectives
- a Battle Ready army can earn **+10 VP**
- normal winner determination is highest VP total

## 1.3 Mission map dependency

Every Boarding Actions mission depends on its mission map for:
- board arrangement
- wall placement
- hatchway placement
- objective marker placement
- starting open/closed hatchway states
- entry zones and role-specific entry zones
- special labeled regions such as Lighting Areas, Compartments, Furnace zones, Access zones, sectors, prison cells, strongrooms, etc.

For a digital implementation, every mission should therefore be represented by:
1. a mission rules object
2. a mission geometry/map object
3. labeled special-region metadata

---

# 2. Core mission setup/data model notes

## 2.1 Terrain/objective setup from mission map
When creating the battlefield for a Boarding Actions mission:
- terrain features are fixed by the mission deployment map
- objective markers are fixed by the mission deployment map
- the mission also specifies which hatchways start open and which start closed

## 2.2 Entry zone behavior
Entry Zones are mission-defined deployment zones. Some missions add special Entry Zones such as:
- Underdog Entry Zone
- Patrol Entry Zone
- Guard Entry Zone
- Backup Entry Zone
- role-specific attacker/defender entry zones
- access-level transfer zones

## 2.3 Digital engine recommendation
Each mission should be serialized as something like:

```yaml
id: "BA-11"
name: "Access Junction Primus"
type: "symmetric"
roles:
  attacker: null
  defender: null
turn_order_override: null
special_regions:
  - junction
  - objective_markers
entry_zones:
  - player_a_main
  - player_b_main
mission_rules:
  - underdog_bonus_extra_cp
objectives:
  progressive:
    - capture_the_junction
  end_game:
    - purge_the_ship
map_ref: "boarding_actions/mission_11"
```

---

# 3. Symmetric missions

# 3.1 Mission 11 — Access Junction Primus

## Narrative premise
Both forces advance from opposite ends of a vital arterial access corridor. The side that secures the junction gains access to adjoining decks and critical systems.

## Mission rules
- **Underdog Bonus:** if one player is the Underdog, that player starts the battle with **+1 CP**.

## Mission objectives

### Capture the Junction
**Type:** Progressive

At the end of each player's Command phase, that player scores **5 VP** for each of the following they satisfy:
- control one or more objective markers
- control two or more objective markers
- control more objective markers than their opponent

### Purge the Ship
**Type:** End Game

At the end of the battle, each player totals the points value of enemy units destroyed and scores **15 VP** for each threshold condition specified by the mission.  
**Implementation note:** the source snippet available here confirms threshold-based destroyed-points scoring, but the exact threshold list should be stored from the mission page/map extraction pass.

## Engine notes
- straightforward mirrored objective-control mission
- requires end-game destroyed-points threshold table

---

# 3.2 Mission 12 — Deck Sweepers

## Narrative premise
A derelict vessel is being swept and purged deck by deck by rival boarding forces seeking control of salvage, resources, or technology.

## Mission rules
- **Underdog Bonus:** in Deploy Armies, a player receiving the Underdog Bonus may also use the **Underdog Entry Zone**.
- Once battle round 1 starts, that special zone cannot be used for Strategic Reserve entry.

## Mission objectives

### Take and Hold
**Type:** Progressive

In battle rounds 2, 3, and 4, at the end of each player's Command phase, that player scores **5 VP** for each of the following:
- control one or more objective markers
- control more objective markers than their opponent

In battle round 5:
- the player with first turn scores in the normal step above
- the player with second turn scores at the end of their turn instead of at the end of their Command phase

### Sweep the Decks
**Type:** End Game

At the end of the battle, each player scores **15 VP for each objective marker they control**.

## Engine notes
- unlike many missions, the progressive condition shown in the source excerpt does **not** include a “control two or more” bullet in the retrieved text
- needs special deployment-zone enable/disable behavior for the underdog

---

# 3.3 Mission 13 — The Pipeline

## Narrative premise
Boarders fight for the conduits, fuel lines, and power-transfer systems that keep the ship functioning.

## Mission rules
- **Underdog Bonus:** if one player is the Underdog, that player starts the battle with **+1 CP**.
- Objective markers are connected by **Power Lines**. Some are directly linked; that linkage matters for scoring.

## Mission objectives

### Power Network
**Type:** Progressive

In battle rounds 2, 3, and 4, at the end of each player's Command phase, that player scores **5 VP** for each of the following:
- control one or more objective markers
- control two or more objective markers
- control more objective markers than their opponent
- control two or more objective markers that are **directly linked by the Power Lines**

In battle round 5:
- the player with first turn scores in the normal step
- the player with second turn scores at the end of their turn instead of at the end of their Command phase

## Engine notes
- mission-specific graph data is required for objective linkage
- map object should explicitly encode objective adjacency

---

# 3.4 Mission 21 — Power Struggle

## Narrative premise
Combatants contest linked power nodes and generator systems to seize control of the vessel’s motive force and key systems.

## Mission rules
- **Underdog Bonus:** if one player is the Underdog, that player starts the battle with **+1 CP**.

## Mission objectives

### Power Network
**Type:** Progressive

The retrieved source text for this mission confirms a progressive “power network” style objective structure, but the full bullet list is not fully visible in the text slices captured here. Treat this mission as requiring:
- objective-control scoring
- additional reward tied to linked/sequential node control

### Cut Off the Head
**Type:** End Game

At the end of the battle, each player scores **10 VP if their opponent’s Warlord is destroyed**.

## Engine notes
- likely similar data needs to The Pipeline: objective linkage metadata
- full scoring bullets should be finalized when exact map/text extraction is completed

---

# 3.5 Mission 22 — Death in the Dark

## Narrative premise
A battle fought across malfunctioning or sabotaged decks where lighting fails and visibility becomes unreliable.

## Mission rules
### Lights Out
At the end of each player's Movement phase, that player rolls one D6 for each **Lighting Area**:
- on **1–3**, the lights in that area turn off until end of turn

If either the attacker or target unit is wholly within a Lighting Area with lights off when making a ranged attack:
- if target is more than **9"** away, the attack cannot be made
- if target is within **9"**, subtract **1** from the Hit roll

If the charging unit or target is wholly within a Lighting Area with lights off when selecting charge targets:
- if target is more than **9"** away, that unit cannot be a charge target
- if target is within **9"**, subtract **1** from the Charge roll (max -1)

- **Underdog Bonus:** if one player is the Underdog, that player starts the battle with **+1 CP**.

## Mission objectives

### Lock It Down!
**Type:** Progressive

In battle rounds 2, 3, and 4, at the end of each player's Command phase, that player scores **5 VP** for each of the following:
- control one or more objective markers
- control two or more objective markers
- control more objective markers than their opponent

In battle round 5:
- first player scores normally
- second player scores at end of turn

### Seek Sanctuary
**Type:** End Game

At the end of the battle, each player scores **15 VP** for each of the following:
- they control both objective markers within Lighting Area 1
- they control both objective markers within Lighting Area 2

## Engine notes
- lighting areas need per-turn on/off state
- visibility and charge eligibility must check local lighting state

---

# 3.6 Mission 23 — Hull Breach

## Narrative premise
A ship is breaking apart under catastrophic external or internal stress. Troops must recover critical data while also preventing total collapse.

## Mission rules

### Deadly Decompression
From battle round 2 onward, at the start of the battle round:
- the Attacker rolls one D6
- on **3+**, one Compartment is vented to the void until the end of that battle round
- if vented, the Defender rolls to determine which Compartment

When a Compartment is vented:
- close all open hatchways within that Compartment
- each unit within it suffers **D3 mortal wounds**
- when a unit in that Compartment is selected to make a Normal or Advance move, subtract **1"** from Move until end of turn

## Mission objectives

### Data Retrieval
**Type:** Progressive

Each time a unit from your army **downloads data from an objective marker**, you score **15 VP**.

### Temporary Stabilisation
**Type:** Progressive

In battle rounds 2, 3, and 4, at the end of each player's Command phase, that player scores **5 VP** for each of the following:
- control one or more objective markers
- control two or more objective markers
- control more objective markers than their opponent

In battle round 5:
- first player scores normally
- second player scores at end of turn

## Engine notes
- requires compartment metadata
- requires per-objective “downloaded/not downloaded” state and interaction logic
- if download is one-shot per marker, model that explicitly in mission state

---

# 3.7 Mission 31 — Control Centre

## Narrative premise
A critical control hub or relay center is under assault, but cunning forces may also seize adjacent secondary points to secure the wider battle.

## Mission rules

### Unlock Overrides
In each player's Command phase, if that player:
- controls objective marker **B**
- has one or more non-Battle-shocked units within range of it
- and at least one such unit is not within Engagement Range of enemy models

that player may **unlock the door overrides**. If they do:
- **open every Hatchway on the battlefield**

- **Underdog Bonus:** if one player is the Underdog, that player starts the battle with **+1 CP**.

## Mission objectives

### Secure at All Costs
**Type:** Progressive

In battle rounds 2, 3, and 4, at the end of each player's Command phase, that player scores **5 VP** for each of the following:
- control one or more objective markers
- control two or more objective markers
- control more objective markers than their opponent
- control objective marker **A**

In battle round 5:
- first player scores normally
- second player scores at end of turn

### Cut Off the Head
**Type:** End Game

At the end of the battle, each player scores **10 VP if their opponent’s Warlord is destroyed**.

## Engine notes
- global hatchway state change is mission-critical
- marker A and marker B must be distinguished in mission metadata

---

# 3.8 Mission 32 — The Furnace

## Narrative premise
A key objective lies in a deadly furnace zone; commanders can manipulate the burners while fighting over fuel exchangers and critical positions.

## Mission rules

### Furnace / Furnace Control Zones
The mission defines:
- **Furnace** zones
- **Furnace Control Zones**

### Rites of Incineration
Once in each player's turn, one unit from that player's army within a Furnace Control Zone can perform this Tactical Manoeuvre:

#### Turn on the Burners
At the end of the turn, for each unit within the Furnace:
- roll one D6 for each model in that unit
- for each **4+**, that unit suffers **1 mortal wound**
- maximum **3 mortal wounds per unit per turn**

### Worth Dying For
Any unit may perform **Secure Site** on an objective marker within the Furnace, even if it does not have BATTLELINE.

### Desperate Measures
In each player's Command phase, if that player controls one or more objective markers within the Furnace:
- roll one D6
- on **5+**, gain **1 CP**

### Underdog Bonus
- if one player is the Underdog, that player starts the battle with **+1 CP**

## Mission objectives

### Promethium Exchangers
**Type:** Progressive

In battle rounds 2, 3, and 4, at the end of each player's Command phase, that player scores **5 VP** for each of the following:
- control one or more objective markers
- control two or more objective markers
- control more objective markers than their opponent
- control one or more objective markers within the Furnace

In battle round 5:
- first player scores normally
- second player scores at end of turn

## Engine notes
- furnace and furnace-control subregions are required
- burner activation is a once-per-player-turn triggered action
- secure-site exception must override normal BATTLELINE requirement

---

# 3.9 Mission 33 — Rad Leak

## Narrative premise
A reactor leak spreads hazardous radiation across the battlefield in expanding sectors, degrading the fighting capacity of units caught within it.

## Mission rules

### Rad Exposure
The battlefield is divided into **Sectors A–D**. Starting with Sector A in battle round 1, radiation spreads and intensifies as the battle continues.

Exposure levels:
- **Mild Rad:** worsen Leadership by 1
- **High Rad:** subtract 1" from Move and also suffer Mild Rad
- **Extreme Rad:** subtract 1 from Toughness and also suffer High Rad and Mild Rad

If a unit starts a battle round in multiple sectors, it suffers the most extreme applicable level.

### Underdog Bonus
- if one player is the Underdog, that player starts the battle with **+1 CP**

## Mission objectives

### Urgent Takeover
**Type:** Progressive

In battle rounds 2, 3, and 4, at the end of each player's Command phase, that player scores **5 VP** for each of the following:
- control one or more objective markers
- control two or more objective markers
- control more objective markers than their opponent

In battle round 5:
- first player scores normally
- second player scores at end of turn

### Salvation Shrines
**Type:** End Game

The retrieved source text confirms an end-game objective named **Salvation Shrines**, but the exact scoring bullets are not present in the captured text excerpt. This should be finalized during full mission-text extraction.

## Engine notes
- sector overlays and round-based contamination states are required
- start-of-battle-round debuff application matters

---

# 4. Asymmetric missions

# 4.1 Mission 1 — Void the Ship

## Roles
- **Attacker**
- **Defender**

## Narrative premise
Rather than simply seizing the ship, the attackers try to open airlocks and expose defenders to vacuum.

## Mission rules

### Security Patrol
The **Patrol Entry Zone** can only be used by the Defender during Deploy Armies. After battle round 1 begins, the Defender cannot use that Entry Zone to set up Strategic Reserve units.

### Set Defence
- the **Defender has the first turn**

### Exposed to the Void
- models cannot enter the **Inaccessible Area** for any reason

### Airlocks
Hatchways labeled **Airlock**:
- behave as normal hatchways except:
  - only the **Attacker** can open them
  - once opened, they **cannot be closed**

### Underdog Bonus
- if one player is the Underdog, that player starts with **+1 CP**

## Mission objectives

### Maintain Ship Integrity
**Type:** End Game

At end of battle:
- **Attacker:** scores **20 VP** for each Airlock opened
- **Defender:** scores **20 VP** for each Airlock still closed

### Cut Off the Head
**Type:** End Game

At the end of the battle, each player scores **10 VP if their opponent’s Warlord is destroyed**.

## Engine notes
- mission-specific hatchway subtype: Airlock
- one-way open-state transition
- role-restricted hatch operation

---

# 4.2 Mission 2 — Pull Their Teeth

## Roles
- **Attacker**
- **Defender**

## Narrative premise
Boarders attack shipboard weapon systems to prevent devastating bombardment support.

## Mission rules

### Rapid Offence
- the **Attacker has the first turn**

### Control Node
- the **Control Node** is the only objective marker that can be **secured by either player’s army**
- any unit may perform **Secure Site** on the Control Node, even without BATTLELINE

### Underdog Bonus
- if one player is the Underdog, that player starts with **+1 CP**

## Mission objectives

### Destroy Ground Targets
**Type:** Progressive

- the **Attacker starts the battle with 60 VP**
- at the start of the Defender's Command phase:
  - if the Defender controls the Control Node, then for each Loader objective marker they control:
    - subtract **10 VP** from the Attacker (minimum 0)
    - the Defender scores **10 VP**
  - otherwise, for each Loader objective marker the Defender controls:
    - subtract **5 VP** from the Attacker (minimum 0)
    - the Defender scores **5 VP**

### Seize the Guns
**Type:** Progressive

At the start of the Attacker's Command phase, if they control the Control Node:
- they score **5 VP**

## Engine notes
- nonstandard VP transfer / VP depletion mechanic
- Control Node and Loader markers need distinct tags
- secure-site exception must be implemented

---

# 4.3 Mission 3 — Strongrooms

## Roles
- **Attacker**
- **Defender**

## Narrative premise
Attackers seek to seize valuables or relics hidden within secure vaults.

## Mission rules

### Rapid Offence
- the **Attacker has the first turn**

### Guard Duty
- at the start of Deploy Armies, for each **Strongroom**, the Defender selects one unit and sets it up wholly within that Strongroom

### Underdog Bonus
- if one player is the Underdog, that player starts with **+1 CP**

## Mission objectives

### They Are Ours
**Type:** End Game

At the end of the battle:
- the **Attacker scores 45 VP for each objective marker they control**

### Purge the Thieves
**Type:** End Game

The retrieved text confirms this defender-side end-game objective, but the exact VP line is not visible in the captured excerpt here. Store/finalize it during the full extraction pass.

## Engine notes
- strongroom-tagged chambers need role-specific predeployment placement
- asymmetric scoring is likely high-swing and objective-count sensitive

---

# 4.4 Mission 4 — Jailbreak

## Roles
- **Attacker**
- **Defender**

## Narrative premise
The Attacker attempts to free an imprisoned unit before the Defender can contain or kill it.

## Mission rules

### Imprisoned
At the start of Deploy Armies:
- the Attacker selects one unit from their army with a minimum cost of **60 points**
- that unit is set up within the **Prison Cells**

The imprisoned unit:
- cannot be removed from the battlefield unless destroyed
- cannot use rules allowing movement through Walls/terrain while the Prison Cells hatchway is closed
- can only attempt to operate the Prison Cells hatchway while it is closed if there are no Defender units wholly within the Guard Entry Zone
- when it attempts this, roll one D6:
  - if result is **greater than or equal to** its Toughness, the attempt fails and hatch remains closed
  - otherwise, open the hatchway normally

### Prison Cells
- units other than the imprisoned unit cannot use abilities that would set them up inside the Prison Cells
- once the Prison Cells hatchway is opened, it cannot be closed
- Defender units cannot attempt to operate the Prison Cells hatchway

### Prison Guards
During Deploy Armies:
- the Defender can only set up within the **Guard Entry Zone** and **Patrol Entry Zones**
- the Defender cannot use **Backup Entry Zones** during initial deployment
- one Defender unit must be set up within the Guard Entry Zone
- after battle round 1 starts, Backup Entry Zones can be used for Strategic Reserve entry

### Rapid Offence
- the **Attacker has the first turn**

### Silent Infiltration
At the start of battle round 1:
- the Attacker selects one unit and rolls one D6
- on **2+**, the alarm is not set off and that unit can make a **Normal Move up to 6"**
- they may then select another unselected unit and roll again, subtracting **1** from the roll each time

## Mission objectives

This mission uses a situation table based on the final status of the imprisoned unit:

- imprisoned unit still within Prison Cells -> **Attacker 0 VP / Defender 90 VP**
- imprisoned unit not within Prison Cells, but destroyed -> **Attacker 30 VP / Defender 60 VP**
- imprisoned unit not within Prison Cells, not destroyed, but within Engagement Range of one or more Defender models -> **Attacker 60 VP / Defender 30 VP**
- imprisoned unit not within Prison Cells, not destroyed, and not within Engagement Range of any Defender models -> **Attacker 90 VP / Defender 0 VP**

## Engine notes
- this mission needs a bespoke end-state evaluator instead of normal additive objective scoring
- prison cell hatchway is a custom stateful mission object
- silent infiltration is a chained pre-round movement sequence with escalating failure chance

---

# 4.5 Mission 5 — Power the Generators

## Roles
- **Attacker**
- **Defender**

## Narrative premise
Attackers try to power data-fanes and access intelligence while fighting across two separate levels of the ship.

## Mission rules

### Set Defence
- the **Defender has the first turn**

### Multi-level
When setting up the battlefield:
- each game board represents a different level within the ship
- units cannot move from one board to the other except via **Change Level**
- units on one board are **not visible** to units on the other

### Change Level
At the end of each player's Movement phase:
- one or more units from that player's army may change level if every model in the unit is within an **Access Zone**
- remove the unit and set it up within the corresponding Access Zone on the other game board

## Mission objectives

The retrieved text for this mission confirms its multi-level movement rules, but the objective text is not visible in the captured excerpts above. Mark for completion during the exact mission-text extraction pass.

## Engine notes
- requires a two-layer board model
- LOS and movement across layers are entirely mission-gated
- paired Access Zone mapping is essential

---

# 4.6 Mission 6 — Corrupt the Machine Spirit

## Roles
- **Attacker**
- **Defender**

## Narrative premise
The Attacker attempts to corrupt machine spirits at objective sites, disabling key systems and inflicting escalating strategic consequences.

## Mission rules

### Set Defence
- the **Defender has the first turn**

### Corrupt the Machine Spirit
At the end of the Attacker's Command phase:
- for each objective marker they control, they may attempt to corrupt it
- roll one D6
- on **2+**, that objective marker is corrupted and removed from the battlefield
- each time an objective marker is corrupted, the Attacker selects one consequence from the mission's consequence list that has not already been selected

**Implementation note:** the consequence list exists in the mission rules but is not fully visible in the captured snippet here; store it from the source page during exact extraction.

## Mission objectives

### Thwart the Corrupters
**Type:** End Game

At the end of the battle:
- the **Defender scores 30 VP for each objective marker they control**

The attacker-side complementary objective is implied by the mission structure but is not fully visible in the captured excerpt above.

## Engine notes
- objective markers can be permanently removed from play
- corruption consequences appear to be one-time unique picks
- mission state must track which consequences have already been selected

---

# 5. Implementation checklist for a digital Boarding Actions engine

## 5.1 Mission data requirements
For every mission, store:
- mission id
- mission name
- symmetric/asymmetric type
- role definitions
- turn-order override
- special entry zones
- labeled objective markers
- labeled zones/regions
- hatchway initial states
- mission-specific actions and triggers
- mission-specific scoring objects
- round-specific scoring timing
- end-game scoring evaluation

## 5.2 Region/label system
Your engine should support reusable region labels such as:
- Lighting Area 1 / 2 / etc.
- Compartment
- Furnace
- Furnace Control Zone
- Sector A / B / C / D
- Access Zone A / B / etc.
- Strongroom
- Prison Cells
- Airlock
- Inaccessible Area
- Guard Entry Zone / Patrol Entry Zone / Backup Entry Zone / Underdog Entry Zone

## 5.3 Scoring engine requirements
Support:
- standard progressive scoring in Command phase
- round-5 alternate timing for second player
- end-game threshold scoring
- asymmetric role-based VP awards
- VP transfer / VP subtraction mechanics
- table-driven outcome scoring
- one-shot interaction scoring (downloads, corruption, etc.)

## 5.4 Action/interaction requirements
Support bespoke mission interactions such as:
- unlock all hatchways
- turn on burners
- download data
- corrupt and remove objective markers
- change level between boards
- prison break hatch operations
- airlock opening restrictions
- region-based environmental effects

## 5.5 Remaining extraction tasks
To make this fully production-ready, do a second pass that captures:
- exact mission map geometry
- exact objective coordinates/labels
- exact hatchway start states
- all omitted bullet lists and missing consequence tables
- mission-specific diagrams such as power links, sector progression, and access-zone pairings

---

# 6. Recommended file split

Use:
- `boarding_patrol.md` -> full Boarding Actions core rules adaptation
- `boarding_actions_missions.md` -> this mission digest
- `boarding_actions_maps.json` -> exact deployment maps, walls, hatchways, objectives, zone labels
- `boarding_actions_objectives.json` -> machine-readable scoring logic
- `boarding_actions_mission_tags.json` -> mission-region metadata and trigger bindings



---

# v3 completion addendum

The following mission details were explicitly normalised in the v3 pass so that the text layer aligns with the Wahapedia Boarding Actions page:

- **Access Junction Primus**: `Purge the Ship` uses three 15 VP thresholds at 125+, 250+, and 375+ enemy points destroyed.
- **Power Struggle**: `Power Network` includes the direct-link condition for objective markers connected by the printed Power Lines with no other objective marker in between.
- **The Furnace**: `Worth Dying For`, `Desperate Measures`, and the underdog bonus are included in full.
- **Rad Leak**: `Salvation Shrines` and `Cut Off the Head` are both included in full.
- **Strongrooms**: defender VP threshold table is normalised to 0 / 30 / 60 / 90 across 0-124 / 125-249 / 250-374 / 375+ attacker points destroyed.
- **Power the Generators**: deck/sub-level split, Access Zone traversal, cross-level fighting, and all objective timing are included in full.
- **Corrupt the Machine Spirit**: all four corruption consequences and both end-game objectives are included in full.

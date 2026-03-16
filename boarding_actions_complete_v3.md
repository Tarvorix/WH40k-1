# Warhammer 40,000 10th Edition - Boarding Actions / Boarding Patrol Rules Reference

> Purpose: implementation-oriented reference for **Boarding Actions** rules, adapted into a structured markdown document for digital rules work.
>
> Source basis: Wahapedia Boarding Action Rules page and related rules context. This document is a **paraphrased rules reference**, not a verbatim copy. It focuses on mechanical completeness for play logic and engine implementation. Mission map graphics are represented in the companion `boarding_actions_maps_complete_v3.json` as a best-effort machine-readable transcription layer. Exact physical layout in play remains governed by the Wahapedia mission maps and labels.

---

## Table of Contents

1. Introduction and scope
2. Boarding Actions battlefield fundamentals
3. Boarding Actions rules adaptations
   - Before the battle
   - Movement phase
   - Shooting phase
   - Tactical Manoeuvres
   - Charge phase
   - Fight phase
   - Leaders and attached-unit changes
   - Returning destroyed models
4. Boarding Actions universal Stratagems
5. Boarding Actions universal Enhancements
6. Mustering your Boarding Patrol
7. Mission structure and battle sequence
8. Mission objectives and scoring framework
9. Implementation notes and edge cases
10. Quick-reference summary

---

## 1. Introduction and scope

Boarding Actions is a close-quarters Warhammer 40,000 format built around:

- compact **500-point** forces
- bespoke **Boarding Actions Detachments**, not normal Codex/Index detachments
- modular ship-interior terrain built from **Walls** and **Hatchways**
- **tight movement, constrained sightlines, hatch control, and objective securing**
- bespoke Boarding Actions **Stratagems**, **Enhancements**, and faction detachment rules

Core 10th edition rules still apply **unless Boarding Actions specifically changes them**.

Boarding Actions also changes army composition expectations:

- normal Codex/Index **Detachment rules, Enhancements, and Stratagems are not used**
- instead, use:
  - Boarding Actions universal rules
  - your chosen Boarding Actions Detachment’s mustering rules
  - that detachment’s bespoke enhancements/stratagems

---

## 2. Boarding Actions battlefield fundamentals

### 2.1 Battlefield composition

A Boarding Actions battlefield is built from **two Boarding Actions boards** placed side by side, or otherwise arranged exactly as a mission map specifies.

Each board is divided into **Zones**. Missions use these zones for:

- deployment / **Entry Zones**
- objective placement
- special mission areas
- reinforcement arrival points

### 2.2 Terrain types

Most terrain consists of:

- **Walls** (including attached pillars)
- **Hatchways**
- **Hatches** that can be either open or closed

### 2.3 Hatchway states

A Hatchway has two states:

**Closed Hatchway**
- blocks movement
- blocks visibility
- cannot be measured through

**Open Hatchway**
- can be moved through freely
- can be seen through
- is ignored for measurement/visibility purposes except where specific Boarding Actions rules say otherwise

### 2.4 Terrain setup and mission maps

Boarding Actions mission maps define:

- exact wall and hatchway placement
- which hatchways start open or closed
- objective locations
- entry zones
- any mission-specific zones or special markers

For exact digital implementation, mission map geometry remains authoritative.

---

## 3. Boarding Actions rules adaptations

## 3.1 Before the battle

### Deep Strike timing change

Units with **Deep Strike** are not committed to Reserves during normal Declare Battle Formations timing. Instead:

- players decide whether to place Deep Strike units into Reserves at the start of **Deploy Armies**
- at least **half the units** in a player’s army must be deployed normally and cannot all be placed into Reserves this way

### Deep Strike arrival limit

Unless a specific rule says otherwise:

- Deep Strike arrivals may only occur in **battle rounds 2 and 3**
- a player may set up **no more than one unit per battle round** using Deep Strike
- any such unit not deployed by the end of battle round 3 is destroyed

---

## 3.2 Movement phase

### Impassable terrain and models

The following are treated as **impassable**:

- Walls
- closed Hatchways
- other models

A model cannot:

- move through them
- end a move overlapping them

### Open Hatchways and movement

Models can move through an **open Hatchway** freely, even if their base is wider than the doorway opening.

Restriction:
- a model cannot end a move with its base positioned in the middle of an open Hatchway

### Measurement in Boarding Actions

Distances cannot be measured through:

- Walls
- closed Hatchways

Instead, distance is measured by the **shortest legal path around them**.

If no legal path can be traced between two points, those points are treated as being at **infinite distance** from one another.

### Engagement Range through open Hatchways

When measuring through an **open Hatchway**, models are treated as within Engagement Range of each other if they are within **2" horizontally**.

This is a major Boarding Actions change from standard 40k engagement geometry.

### Scouts ability

While on the battlefield, units lose the ability to make pre-battle Scouts reposition moves in any way that would ignore Boarding Actions layout restrictions. For implementation, use Boarding Actions geometry and impassable rules over any broader open-board assumptions.

### Deep Strike distance measurement change

When measuring Deep Strike distance to enemy models in Boarding Actions:

- **ignore Walls and closed Hatchways**

This also applies to rules that prevent Deep Strike placement.

Practical result:
- a model cannot Deep Strike into a room “safely” just because walls force ordinary movement measurement around them
- Deep Strike checks use effective straight horizontal proximity through structure

### Opposite sides of a Hatchway

Two models are on **opposite sides of a Hatchway** if the shortest line between them would pass through that Hatchway if it were open, regardless of whether the Hatchway is currently open or closed.

Two units are **wholly on opposite sides of a Hatchway** if every model in one unit is on the opposite side of that Hatchway from every model in the other.

This matters for:

- hatchway control contests
- certain positional checks
- emergent combat after opening a door

### Flying is suppressed

While on the battlefield in Boarding Actions:

- models **lose FLY** if they have it
- they cannot Fly
- if a model’s Move characteristic is greater than **9"**, reduce it to **9"** at the start of the battle

### Objective marker interaction

Boarding Actions changes objective interaction:

- models can move over objective markers
- models **can end a move on top of** an objective marker in Boarding Actions
- a model is within range of an objective marker if it is within **1" horizontally** of it

This is a notable departure from standard 40k objective handling.

### Operating Hatchways

At the **end of the Move Units step** of each player’s Movement phase:

- one or more eligible units from that player’s army can each attempt to operate **one Hatchway**
- units within Engagement Range cannot attempt this
- the selected Hatchway must be within **1"** of the acting unit

#### Enemy prevention

If there are enemy units:

- on the opposite side of the Hatchway, and
- within **1"** of it,

then the opponent may select one such enemy unit to try to prevent the operation.

#### Roll-off to operate

If prevented, players roll off and each adds the **Toughness** of one model from their selected unit.

- if the operating player wins, the Hatchway changes state
- if the defender declines to contest, the Hatchway changes state automatically

#### Hatchway state change

Operating a Hatchway toggles it:

- open -> closed
- closed -> open

#### Opening into combat

If opening a Hatchway causes units on opposite sides to now be within Engagement Range:

- both units will be eligible to fight in the next Fight phase
- **none** of them count as having charged that turn

#### Preventable vs non-preventable effects

If a rule says a unit can **operate** a Hatchway, enemy prevention still works normally.

If a rule explicitly says to **open** or **close** a Hatchway, that cannot be prevented.

#### Cannot shut on split unit positioning

A Hatchway can never be closed if models from the **same unit** are on opposite sides of it.

---

## 3.3 Shooting phase

### Boarding Actions visibility

A model is visible to an observing model only if a straight line can be traced from any part of one base to any part of the other base **without passing through**:

- a Wall
- a closed Hatchway
- a model not in the target model’s unit

An open Hatchway’s door itself does not block visibility.

### Visibility through other models

In Boarding Actions, models from **other units** block line of sight in a much stricter way than in open-table 40k.

### Indirect Fire removed

While on the battlefield:

- weapons lose the **Indirect Fire** ability

### Blast restriction

For Blast attacks:

- only count models **visible** to the attacker when determining target unit size for Blast bonus attacks

### Attack allocation restriction

For ranged attacks allocated to an unwounded unit:

- the attack must be allocated to a model visible to at least one model in the shooting unit
- if no such model exists, the attack sequence ends

### Benefit of Cover in Boarding Actions

A target model has Benefit of Cover against a ranged attack unless it is **fully visible** to at least one model in the attacking unit.

This makes partial peeking, doorway fights, and body-blocking very important.

---

## 3.4 Tactical Manoeuvres

At the start of your **Shooting phase**, you may select one or more units from your army to each perform **one Tactical Manoeuvre**.

### Tactical Manoeuvre eligibility

A unit can perform a Tactical Manoeuvre only if it:

- is **not Battle-shocked**
- is **not within Engagement Range** of an enemy unit
- did **not Advance** this turn
- did **not Fall Back** this turn
- was **not set up on the battlefield this turn**

A unit performing a Tactical Manoeuvre:

- is not eligible to shoot this turn
- cannot declare a charge this turn

Some missions add additional Tactical Manoeuvres; the same baseline restrictions still apply.

### Secure Site

**BATTLELINE only** unless a mission says otherwise.

Procedure:

- select an objective marker you control within range of that unit
- at the start of your next Command phase, if that unit:
  - is not Battle-shocked
  - is still within range of that objective
  - and you still control that objective
- then that unit **Seizes** the objective marker

### Set to Defend

Until the start of your next Command phase:

- each melee attack made by that unit gets **+1 to Hit**

### Set Overwatch

Until the end of your opponent’s next turn, after an enemy unit:

- is set up
- ends a Normal / Advance / Fall Back move
- declares a charge
- or opens a Hatchway

that unit may fire Overwatch at it.

Boarding Actions Overwatch rules:

- shoot as if it were your Shooting phase
- attacks may target only that triggering enemy unit
- only **unmodified 6s** hit
- Critical Hits only occur on unmodified 6s
- a unit cannot fire Overwatch more than once per turn

### Seizing and Securing objective markers

When a unit **Seizes** an objective marker:

- that marker becomes **Secured** by that player’s army
- it remains controlled by that army even if no friendly models stay near it
- it stays that way until the opponent controls it at the **start or end of any turn**

Some mission rules can Secure an objective without requiring a unit to Seize it directly.

---

## 3.5 Charge phase

### Charge visibility requirement

A unit can only be selected as a charge target if it is **visible** to the charging unit.

This is stricter than standard 40k, where some charges can be declared without actual visual contact.

---

## 3.6 Fight phase

### Pile-in and Consolidation changes

Each time a model makes a Pile-in or Consolidation move:

- it cannot end that move within Engagement Range of a unit that was **not visible** to its own unit at the start of that move
- it does not have to end closer to the **closest enemy model**, provided it ends as close as possible to the **closest visible enemy unit**

Additional restriction:

- if a unit cannot end such a move within Engagement Range of an enemy unit while remaining in coherency, it also **cannot** make a Consolidation move toward the nearest objective marker

### Fighting through newly opened Hatchways

If a Hatchway is opened and opposing units become engaged across it:

- both will be eligible to fight in the following Fight phase
- neither counts as having charged

---

## 3.7 Leaders and attached units in Boarding Actions

### Leaders do not attach normally

In Boarding Actions:

- Leaders do **not** attach to Bodyguard units at the start of the battle
- they remain separate units all game

### Leader abilities still matter

Abilities worded like:

- “while this model is leading a unit”
- “while this unit is leading a unit”

are still usable through the **Battlefield Command** Stratagem.

When that Stratagem is used:

- pick a valid nearby Bodyguard-type unit the Leader could normally join
- that unit gains the selected Leader ability temporarily
- the Leader itself does **not** benefit from the conferred effect
- after application, the Bodyguard unit does **not** need to remain within 6" unless the conferred rule itself says so

### Led-by abilities do not function

Abilities that only work while a unit is **being led** do **nothing** in Boarding Actions, even if Battlefield Command is used.

---

## 3.8 Returning destroyed models

Unless a Boarding Actions mission or detachment explicitly says otherwise:

- rules that return destroyed models to a unit cannot return **more than one model per unit per battle round**

---

## 4. Boarding Actions universal Stratagems

You cannot use normal core/Codex stratagem sets here. Only:

- the universal Boarding Actions stratagems below
- stratagems from your chosen Boarding Actions Detachment

### 4.1 Command Re-roll — 1CP

**When:** any phase, after a qualifying roll/test/save is made.

Valid re-roll types include:

- Hit roll
- Wound roll
- Damage roll
- saving throw
- Advance roll
- Charge roll
- Desperate Escape test
- Hazardous test
- random attacks roll

**Effect:** re-roll that roll/test/save.

### 4.2 Battlefield Command — 1CP

**When:** start or end of any phase.

**Target:**
- one Leader unit from your army
- one friendly Bodyguard unit within 6" that the Leader could normally join

**Effect:**
- choose one of that Leader’s Leader abilities
- until your next Command phase, the chosen Bodyguard unit is treated as being led by that Leader for that Leader ability only

**Restriction:**
- once a unit has been targeted by Battlefield Command, it cannot be targeted by it again until your next Command phase

### 4.3 Counter-Offensive — 2CP

**When:** Fight phase, after an enemy unit has fought.

**Target:** one of your units that:
- is within Engagement Range of an enemy
- has not yet fought this phase

**Effect:** that unit fights next.

### 4.4 Insane Bravery — 1CP

**When:** Battle-shock step of your Command phase, immediately after one of your units fails a Battle-shock test.

**Effect:** that unit is treated as having passed instead and is not Battle-shocked.

### 4.5 Explosive Clearance — 1CP

**When:** your Shooting phase.

**Target:** one unit from your army that has not been selected to shoot.

**Effect:**
- pick one model in that unit equipped with a **Blast** weapon
- until end of phase, for that weapon:
  - count models that are not visible when determining Blast target size
  - attacks from that weapon can be allocated to models not visible to the attacker

This is one of the most important universal Boarding Actions ranged tools.

---

## 5. Boarding Actions universal Enhancements

Normal Codex/Index Enhancements are not used. Boarding Actions uses universal enhancements plus detachment-specific ones.

Enhancements:

- cost **0 points** in Boarding Actions
- may only be taken by eligible CHARACTER models
- detachment-specific enhancements can only go on models with that detachment’s faction keyword

### 5.1 Superior Boarding Tactics

Effect:
- you start the battle with **2CP**

### 5.2 Close-Quarters Killer

Effect:
- the bearer can **re-roll Wound rolls** for melee attacks

### 5.3 Peerless Leader

Effect:
- once per battle round, the bearer can be targeted by **Battlefield Command for 0CP**, even if you already used Battlefield Command on a different unit that phase

### 5.4 Expert Breacher

Effect:
- the bearer’s unit can attempt to operate a Hatchway at the **start or end** of the Move Units step of your Movement phase
- that unit still cannot attempt to operate more than one Hatchway per turn

This is stronger than the standard end-of-move hatch timing.

### 5.5 Personal Teleporter

Effect:
- the bearer gains **Deep Strike**

### 5.6 Trademark Weapon

Effect:
- pick one non-Torrent ranged weapon on the bearer when mustering
- improve that weapon by **+1 Strength** and **+1 Damage**

---

## 6. Mustering your Boarding Patrol

### 6.1 Army size

A Boarding Patrol contains up to **500 points**.

### 6.2 Army roster sequence

1. **Start your Army Roster**
   - record units, wargear, model counts, and points
   - show completed roster to your opponent before battle

2. **Select Army Faction**
   - choose one keyword as your Army Faction

3. **Select Detachment Rules**
   - choose a Boarding Actions Detachment that matches your Army Faction
   - only Boarding Actions Detachments are valid here

4. **Select Units**
   - choose units allowed by that detachment’s mustering rules
   - your army does **not** have to include a CHARACTER
   - respect maximum copies and permitted starting strengths listed by the detachment

### 6.3 Unit inclusion restrictions

You may only include a unit if:

- you have enough points left
- your detachment’s mustering rules allow it
- you have not exceeded that unit’s allowed maximum count

Additional restrictions:

- you cannot include the same **EPIC HERO** more than once

### 6.4 Non-standard unit sizing and points

If a detachment allows a unit size not printed directly in the Munitorum Field Manual, but the Manual has a points entry for a unit at **double that size**, then:

- the smaller size costs **half** the larger size’s points
- round up

### 6.5 Warlord and Enhancements

- choose one model from your army to be your **Warlord**
- if your army includes any CHARACTER models, the Warlord must be a CHARACTER
- your Warlord gains the WARLORD keyword

Enhancement rules:

- up to **two CHARACTER models** in your army may each take **one Enhancement**
- **EPIC HEROES cannot take Enhancements**
- you cannot duplicate the same Enhancement
- enhancements cost **no points**

### 6.6 Points total and underdog status

Your roster must show total army points.

In missions, if one player is at least **30 points lower** than the other, that player is the **Underdog** and may receive a mission-specific Underdog bonus.

---

## 7. Mission structure and battle sequence

### 7.1 Symmetric mission flow

A standard symmetric Boarding Actions mission follows this general sequence:

1. Muster Boarding Patrols
2. Determine mission
3. Determine Attacker / Defender
4. Read mission briefing and rules
5. Create battlefield and place objective markers
6. Determine Entry Zones
7. Deploy armies
8. Determine Reserves
9. Determine first turn
10. Begin battle
11. End after five battle rounds
12. Determine victor

### 7.2 Mission selection

Players may:

- agree on a mission, or
- randomly determine one from the mission table

### 7.3 Battlefield creation

Mission maps define:

- wall and hatchway placement
- which hatchways begin open/closed
- objective markers
- special zones
- entry zones

### 7.4 Deployment

Players alternate deploying **one unit per Entry Zone**, starting with the Defender.

If one player finishes first, the opponent continues until either:

- all units are deployed, or
- all players have filled the allowed entry placements

### 7.5 Reserves

Any unit not starting on the battlefield begins in **Strategic Reserves**.

In the Reinforcements step of the Movement phase, starting from **battle round 1 onward**, a player can select one Strategic Reserves unit **for each of their Entry Zones that contains no models** and set those units up within those Entry Zones.

This is a major Boarding Actions difference from normal 40k reserves logic.

### 7.6 First turn and battle length

- players roll off to determine first turn
- battle lasts **five battle rounds**
- if one army is wiped at the start of its turn, the other player may keep taking turns until the game ends

### 7.7 Victory cap and Battle Ready bonus

- mission objectives can score up to **90 VP**
- Battle Ready standard gives **+10 VP**
- practical maximum score is **100 VP**

---

## 8. Mission objectives and scoring framework

### 8.1 Objective types

Boarding Actions missions use two broad objective types:

- **Progressive objectives** — scored during the battle at defined timings
- **End Game objectives** — scored at battle end

### 8.2 Common progressive scoring pattern

Many Boarding Actions missions use this recurring progressive pattern:

In battle rounds **2, 3, and 4**, at the end of each player’s Command phase, that player scores **5 VP** for each satisfied condition, often including:

- controlling one or more objectives
- controlling two or more objectives
- controlling more objectives than the opponent
- mission-specific control conditions

In battle round **5**:

- the first player scores using the same framework at the usual timing
- the second player usually scores at **end of turn** instead of end of Command phase

### 8.3 Representative mission rules from the Boarding Actions mission pack

The source page includes a large mission set with bespoke maps and scenario rules. Core recurring special-rule patterns include:

- **Lighting Area / lights-off rules** that can:
  - limit charge target selection beyond 9"
  - impose **-1 to Charge** when charging into darkness within 9"
- **Furnace / environmental hazard missions** where:
  - units in specific sectors suffer escalating penalties by battle round
  - radiation can worsen Leadership, reduce Move, and reduce Toughness
- **Airlock missions** where:
  - some Hatchways are Airlocks
  - only the Attacker may open them
  - once opened they can never be closed
  - battle-end scoring depends on whether Airlocks remain sealed or are opened
- **Control Node / special objective missions** where:
  - only one designated marker can be Secured
  - any unit may Secure Site that marker regardless of BATTLELINE
- **Cross-deck / between-decks missions** where:
  - models in paired Access Zones across the two boards count as visible and in Engagement Range during the Fight phase
  - pile-in/consolidation in those zones moves toward the centre of the zone
- **Mission-specific Tactical Manoeuvre overrides** where:
  - any unit, not only BATTLELINE, may Secure Site certain objectives
- **Mission-specific CP/Underdog rules** where:
  - the lower-point player often starts with +1 CP

### 8.4 Examples of mission objective structures present on the page

Examples of objective frameworks visible on the source page include:

- holding a vital junction or transit route
- controlling linked objectives or paired lettered objectives
- stabilising or dominating compromised lighting sectors
- controlling critical shrines or reactor controls at battle end
- opening or defending Airlocks
- stealing powered terminal data by controlling generators and terminals
- killing the enemy Warlord as an end-game objective
- preventing enemy extraction or inflicting specified losses for scaling VP

### 8.5 Important practical note for implementation

For a digital adaptation, mission logic should be split into:

1. **shared Boarding Actions engine rules**
2. **mission geometry** (walls, hatchways, entry zones, objective locations)
3. **mission scripts**
   - special areas
   - turn timing hooks
   - scoring conditions
   - mission-only manoeuvre or door rules

The mission maps on the source page are graphical, so exact geometry should be transcribed from those maps separately.

---

## 9. Implementation notes and edge cases

### 9.1 Key engine differences from standard 40k

Boarding Actions requires custom handling for all of the following:

- pathfinding around **impassable walls and closed doors**
- line-of-sight blocked by **models from other units**
- **2" hatchway Engagement Range** checks through open doors
- hatch control and **contested operate-door roll-offs**
- separate logic for **secured** objectives vs merely controlled objectives
- suppression of **FLY** and Move characteristic cap at 9"
- removal of **Indirect Fire**
- modified **Blast** visibility counting
- range to objectives measured at **1" horizontally**
- Leaders as independent units with **virtual leader buff projection** via Battlefield Command
- Deep Strike distance measured **ignoring walls and closed doors**
- mission-specific reserve entry via **empty Entry Zones**

### 9.2 Door-opening combat edge case

If a unit opens a Hatchway and now becomes engaged through it:

- mark both units as fight-eligible this phase
- do **not** give charge benefits
- visibility and pile-in legality must still respect Boarding Actions visibility rules

### 9.3 Cover edge case

Benefit of Cover is functionally common in Boarding Actions because:

- a model only loses it if **fully visible** to at least one attacker
- partial doorway peeks should usually still count as cover

### 9.4 Secure vs control

A unit can **control** an objective now, but only **Secure** it through Secure Site (or mission rule equivalents). Once secured, control persists even if the unit leaves, until the opponent retakes control at the start or end of a turn.

### 9.5 Battlefield Command modeling

Battlefield Command should not create an attached unit. It should instead apply:

- a timed temporary buff package
- restricted to one Leader ability
- to a nearby eligible Bodyguard-type unit

Led-by abilities must not activate as a side effect.

---

## 10. Quick-reference summary

### Core Boarding Actions changes at a glance

- **500-point** game size
- use **Boarding Actions Detachments only**
- no normal Codex/Index detachment rules, enhancements, or stratagems
- battlefield made from **two boards**, Walls, and Hatchways
- **closed doors/walls are impassable and opaque**
- **open doors** allow movement and visibility
- distance cannot be measured through walls/closed doors
- Engagement Range through open doors is **2" horizontally**
- FLY is lost; Move above 9" becomes **9"**
- objective range is **1" horizontally**
- models **can end on objectives**
- Leaders do not attach
- Battlefield Command temporarily projects Leader abilities
- Led-by abilities do nothing
- Indirect Fire is lost
- Blast counts only visible models unless **Explosive Clearance** is used
- Tactical Manoeuvres replace a unit’s shooting/charge output for the turn
- Secure Site creates **persistent secured control**
- many missions use **Entry Zone** reserve arrivals
- many missions use recurring score conditions in rounds 2–5

---

## Appendix A - Mission and map transcription status

This document captures the **core textual rules system** from the Boarding Actions rules page and enough mission-framework detail to implement the common engine logic.

What still needs a separate pass if you want a fully machine-usable mission pack:

- every mission map’s exact wall/hatchway geometry
- every entry zone coordinate
- every objective coordinate and label
- every mission’s full bespoke special-rule script and scoring table in structured JSON/YAML form

That is best handled as a second artifact, e.g.:

- `boarding_actions_missions.md`
- `boarding_actions_maps.json`
- `boarding_actions_scenarios.yaml`



---

## Companion Artifacts (v3)

This rules reference is intended to be used together with:
- `boarding_actions_missions_complete_v3.md`
- `boarding_actions_maps_complete_v3.json`
- `boarding_actions_objectives_complete_v3.json`
- `boarding_actions_mission_tags_complete_v3.json`
- `boarding_actions_audit_complete_v3.md`

### Source authority note

For Boarding Actions, the source mission maps define the two-board arrangement, terrain layout, hatchway placement, objective placement, entry zones, and any starting hatch states. This document therefore carries the text rules layer, while the companion map/objective/tag files hold the implementation-facing mission layer.

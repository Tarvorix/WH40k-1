# Warhammer 40,000 10th Edition - Complete Rules Reference

**Purpose:** This document provides a comprehensive breakdown of Warhammer 40,000 10th Edition rules for computer game adaptation. All mechanics, phases, and systems are detailed with implementation-relevant specifics.

---

## Table of Contents

1. [Core Concepts](#1-core-concepts)
   - 1.1 [Game Structure Overview](#11-game-structure-overview)
   - 1.2 [Units and Models](#12-units-and-models)
   - 1.3 [Keywords](#13-keywords)
   - 1.4 [Redeployments](#14-redeployments)
   - 1.5 [Datasheets and Profiles](#15-datasheets-and-profiles)
   - 1.6 [Engagement Range](#16-engagement-range)
   - 1.7 [Unit Coherency](#17-unit-coherency)
   - 1.8 [Measuring Distances](#18-measuring-distances)
   - 1.9 [Visibility and Line of Sight](#19-visibility-and-line-of-sight)

2. [Dice Mechanics](#2-dice-mechanics)
   - 2.1 [Standard Dice Rolls](#21-standard-dice-rolls)
   - 2.2 [Re-rolls](#22-re-rolls)
   - 2.3 [Roll-offs](#23-roll-offs)
   - 2.4 [Modifiers](#24-modifiers)
   - 2.5 [Sequencing](#25-sequencing)
   - 2.6 [Random Characteristics](#26-random-characteristics)

3. [The Battle Round](#3-the-battle-round)
   - 3.1 [Turn Structure](#31-turn-structure)
   - 3.2 [Persisting Effects](#32-persisting-effects)
   - 3.3 [Out-of-Phase Rules](#33-out-of-phase-rules)

4. [Command Phase](#4-command-phase)
   - 4.1 [Command Points](#41-command-points)
   - 4.2 [Battle-shock Tests](#42-battle-shock-tests)
   - 4.3 [Battle-shocked Status Effects](#43-battle-shocked-status-effects)

5. [Movement Phase](#5-movement-phase)
   - 5.1 [Movement Types](#51-movement-types)
   - 5.2 [Remain Stationary](#52-remain-stationary)
   - 5.3 [Normal Moves](#53-normal-moves)
   - 5.4 [Advance Moves](#54-advance-moves)
   - 5.5 [Fall Back Moves](#55-fall-back-moves)
   - 5.6 [Pivoting](#56-pivoting)
   - 5.7 [Moving Over Terrain](#57-moving-over-terrain)
   - 5.8 [Flying Models](#58-flying-models)
   - 5.9 [Reinforcements](#59-reinforcements)
   - 5.10 [Surge Moves](#510-surge-moves)

6. [Transport Rules](#6-transport-rules)
   - 6.1 [Transport Capacity](#61-transport-capacity)
   - 6.2 [Embarking](#62-embarking)
   - 6.3 [Disembarking](#63-disembarking)
   - 6.4 [Destroyed Transports](#64-destroyed-transports)
   - 6.5 [Firing Deck](#65-firing-deck)

7. [Shooting Phase](#7-shooting-phase)
   - 7.1 [Eligibility to Shoot](#71-eligibility-to-shoot)
   - 7.2 [Target Selection](#72-target-selection)
   - 7.3 [Making Ranged Attacks](#73-making-ranged-attacks)
   - 7.4 [Locked in Combat](#74-locked-in-combat)
   - 7.5 [Big Guns Never Tire](#75-big-guns-never-tire)

8. [Attack Resolution](#8-attack-resolution)
   - 8.1 [Hit Rolls](#81-hit-rolls)
   - 8.2 [Wound Rolls](#82-wound-rolls)
   - 8.3 [Attack Allocation](#83-attack-allocation)
   - 8.4 [Saving Throws](#84-saving-throws)
   - 8.5 [Invulnerable Saves](#85-invulnerable-saves)
   - 8.6 [Inflicting Damage](#86-inflicting-damage)
   - 8.7 [Mortal Wounds](#87-mortal-wounds)

9. [Charge Phase](#9-charge-phase)
   - 9.1 [Charge Eligibility](#91-charge-eligibility)
   - 9.2 [Declaring Charges](#92-declaring-charges)
   - 9.3 [Charge Rolls](#93-charge-rolls)
   - 9.4 [Charge Moves](#94-charge-moves)
   - 9.5 [Charge Bonus](#95-charge-bonus)
   - 9.6 [Charging Over Terrain](#96-charging-over-terrain)
   - 9.7 [Charging with Flying Models](#97-charging-with-flying-models)

10. [Fight Phase](#10-fight-phase)
    - 10.1 [Fight Phase Structure](#101-fight-phase-structure)
    - 10.2 [Fights First Step](#102-fights-first-step)
    - 10.3 [Remaining Combats Step](#103-remaining-combats-step)
    - 10.4 [Pile In](#104-pile-in)
    - 10.5 [Making Melee Attacks](#105-making-melee-attacks)
    - 10.6 [Consolidate](#106-consolidate)

11. [Weapon Abilities](#11-weapon-abilities)
    - 11.1 [Assault](#111-assault)
    - 11.2 [Heavy](#112-heavy)
    - 11.3 [Rapid Fire](#113-rapid-fire)
    - 11.4 [Pistol](#114-pistol)
    - 11.5 [Blast](#115-blast)
    - 11.6 [Torrent](#116-torrent)
    - 11.7 [Melta](#117-melta)
    - 11.8 [Lance](#118-lance)
    - 11.9 [Twin-linked](#119-twin-linked)
    - 11.10 [Lethal Hits](#1110-lethal-hits)
    - 11.11 [Sustained Hits](#1111-sustained-hits)
    - 11.12 [Devastating Wounds](#1112-devastating-wounds)
    - 11.13 [Hazardous](#1113-hazardous)
    - 11.14 [Precision](#1114-precision)
    - 11.15 [Anti-X](#1115-anti-x)
    - 11.16 [Indirect Fire](#1116-indirect-fire)
    - 11.17 [Ignores Cover](#1117-ignores-cover)
    - 11.18 [Extra Attacks](#1118-extra-attacks)

12. [Unit Abilities](#12-unit-abilities)
    - 12.1 [Feel No Pain](#121-feel-no-pain)
    - 12.2 [Deadly Demise](#122-deadly-demise)
    - 12.3 [Deep Strike](#123-deep-strike)
    - 12.4 [Infiltrators](#124-infiltrators)
    - 12.5 [Scouts](#125-scouts)
    - 12.6 [Lone Operative](#126-lone-operative)
    - 12.7 [Stealth](#127-stealth)
    - 12.8 [Leader and Attached Units](#128-leader-and-attached-units)
    - 12.9 [Fights First](#129-fights-first)
    - 12.10 [Aura Abilities](#1210-aura-abilities)
    - 12.11 [Psychic Weapons and Abilities](#1211-psychic-weapons-and-abilities)
    - 12.12 [Leadership Tests](#1212-leadership-tests)

13. [Terrain](#13-terrain)
    - 13.1 [Benefit of Cover](#131-benefit-of-cover)
    - 13.2 [Terrain Heights](#132-terrain-heights)

14. [Strategic Reserves](#14-strategic-reserves)
    - 14.1 [Placing Units in Reserves](#141-placing-units-in-reserves)
    - 14.2 [Arriving from Reserves](#142-arriving-from-reserves)

15. [Stratagems](#15-stratagems)
    - 15.1 [Using Stratagems](#151-using-stratagems)
    - 15.2 [Core Stratagems](#152-core-stratagems)

16. [Aircraft Rules](#16-aircraft-rules)
    - 16.1 [Aircraft Movement](#161-aircraft-movement)
    - 16.2 [Aircraft in Combat](#162-aircraft-in-combat)

17. [Unit States and Conditions](#17-unit-states-and-conditions)
    - 17.1 [Starting Strength](#171-starting-strength)
    - 17.2 [Below Half-strength](#172-below-half-strength)
    - 17.3 [Destroyed](#173-destroyed)

18. [Objective Markers](#18-objective-markers)
    - 18.1 [Objective Marker Basics](#181-objective-marker-basics)
    - 18.2 [Controlling Objectives](#182-controlling-objectives)
    - 18.3 [Objective Markers and Terrain](#183-objective-markers-and-terrain)
    - 18.4 [Sticky Objectives](#184-sticky-objectives)

19. [Muster Your Army](#19-muster-your-army)
    - 19.1 [Army Construction Basics](#191-army-construction-basics)
    - 19.2 [Detachments](#192-detachments)
    - 19.3 [Army Composition](#193-army-composition)
    - 19.4 [Warlord](#194-warlord)
    - 19.5 [Enhancements](#195-enhancements)

---

## 1. Core Concepts

### 1.1 Game Structure Overview

Warhammer 40,000 is played in a series of **Battle Rounds**. Each Battle Round consists of two **Player Turns** (one per player). The same player always takes the first turn in each Battle Round.

Each Player Turn consists of five phases executed in strict order:

1. Command Phase
2. Movement Phase
3. Shooting Phase
4. Charge Phase
5. Fight Phase

Victory is achieved by scoring **Victory Points (VP)** through mission objectives.

### 1.2 Units and Models

| Term         | Definition                                                   |
| ------------ | ------------------------------------------------------------ |
| **Model**    | A single Citadel miniature with its own characteristics      |
| **Unit**     | One or more models from the same datasheet that act together |
| **Army**     | All models under a player's command                          |
| **Friendly** | Models/units from the same army                              |
| **Enemy**    | Models/units from the opponent's army                        |

### 1.3 Keywords

All datasheets have two categories of keywords:

- **Faction Keywords:** Used for army composition rules
- **Other Keywords:** Used for rules interactions

Keywords appear in **KEYWORD BOLD** format. Rules that reference a keyword (e.g., "affects INFANTRY units") only apply to units with that specific keyword.

### 1.4 Redeployments

Some rules allow players to redeploy certain units after both armies have been deployed.

**Timing:** Redeployments are resolved:

1. After the Deploy Armies step
2. Before the Determine First Turn step

**Procedure:**

- Players alternate resolving redeployment rules, starting with the Attacker
- When a unit is redeployed, remove it from the battlefield and set it up again following all normal deployment rules
- If a unit has deployment abilities (e.g., Infiltrators), those can be used when redeploying

### 1.5 Datasheets and Profiles

Every unit has a datasheet containing:

**Model Profile:**
| Characteristic | Abbreviation | Description |
|----------------|--------------|-------------|
| Move | M | Maximum movement distance in inches |
| Toughness | T | Resistance to damage |
| Save | Sv | Armor save value (lower is better) |
| Wounds | W | Health points before destruction |
| Leadership | Ld | Morale/command resistance |
| Objective Control | OC | Ability to control objectives |

**Weapon Profile:**
| Characteristic | Abbreviation | Description |
|----------------|--------------|-------------|
| Range | - | Maximum attack distance in inches |
| Attacks | A | Number of attacks made |
| Ballistic Skill | BS | Accuracy for ranged attacks (target number) |
| Weapon Skill | WS | Accuracy for melee attacks (target number) |
| Strength | S | Power of the attack |
| Armour Penetration | AP | Modifier to target's save |
| Damage | D | Wounds inflicted per successful attack |

### 1.6 Engagement Range

**Definition:** A model is within Engagement Range of an enemy model if it is within **1" horizontally AND 5" vertically** of that enemy model.

**Critical Rules:**

- Models cannot be set up within Engagement Range of enemies
- Models cannot end Normal, Advance, or Fall Back moves within Engagement Range of enemies
- If a model cannot avoid ending a move within Engagement Range, that model is **destroyed**

### 1.7 Unit Coherency

Units must maintain coherency at all times:

| Unit Size  | Coherency Requirement                                                                |
| ---------- | ------------------------------------------------------------------------------------ |
| 2-6 models | Each model within 2" horizontally and 5" vertically of at least **one** other model  |
| 7+ models  | Each model within 2" horizontally and 5" vertically of at least **two** other models |

**End of Turn Check:** At the end of every turn, if a unit is not in coherency, the controlling player removes models (one at a time) until coherency is restored. These models count as destroyed but do not trigger "when destroyed" rules.

### 1.8 Measuring Distances

- Distances are measured in **inches**
- Measure between the closest points of model **bases**
- Models without bases: measure to closest point of model
- **"Within X inches"** means any distance not more than X

### 1.9 Visibility and Line of Sight

The game uses **True Line of Sight**. Check visibility from the observing model's perspective.

| Visibility State        | Definition                                                           |
| ----------------------- | -------------------------------------------------------------------- |
| **Model Visible**       | Any part of a model can be seen from any part of the observing model |
| **Unit Visible**        | At least one model in the unit is visible                            |
| **Model Fully Visible** | Every facing part of the model can be seen                           |
| **Unit Fully Visible**  | Every model in the unit is fully visible                             |

**Special Rules:**

- A model can see through other models in its own unit
- A model's base is considered part of the model for visibility
- For determining if an enemy unit is fully visible, an observing model can see through other models in the unit it is observing

---

## 2. Dice Mechanics

### 2.1 Standard Dice Rolls

| Notation       | Meaning                                              |
| -------------- | ---------------------------------------------------- |
| D6             | Roll one six-sided die                               |
| 2D6, 3D6, etc. | Roll multiple dice, sum the results                  |
| D3             | Roll D6, divide by 2 (round up): 1-2=1, 3-4=2, 5-6=3 |
| X+             | Roll must equal or exceed X to succeed               |
| 1-3            | Range of consecutive values                          |

### 2.2 Re-rolls

- A die can **never be re-rolled more than once**
- Re-rolls happen **before** modifiers are applied
- When re-rolling multiple dice (2D6, etc.), **all dice must be re-rolled** unless stated otherwise
- **Unmodified result:** The value after any re-rolls but before modifiers

### 2.3 Roll-offs

When rules call for a roll-off:

1. Both players roll one D6
2. Highest roll wins
3. Ties: Roll again until resolved
4. No re-rolls or modifiers allowed

### 2.4 Modifiers

**Modifier Caps:**
| Roll Type | Maximum Modifier |
|-----------|------------------|
| Hit Roll | -1 to +1 |
| Wound Roll | -1 to +1 |
| Saving Throw | +1 improvement maximum |

### 2.5 Sequencing

When two or more rules must be resolved at the same time, use the following:

| Timing                                | Resolution                                    |
| ------------------------------------- | --------------------------------------------- |
| During the battle                     | The player whose turn it is chooses the order |
| Before or after the battle            | Players roll off; winner decides the order    |
| At the start or end of a battle round | Players roll off; winner decides the order    |

### 2.6 Random Characteristics

Some characteristics are presented as random values (e.g., D6, D3+1, 2D6).

**When to Roll:**

- Roll at the point the characteristic is needed
- Roll separately for each attack/model unless otherwise stated

**Examples:**

- Weapon with Damage D6: Roll D6 separately for each successful wound
- Weapon with Attacks 2D6: Roll 2D6 once when selecting targets to determine total attacks

---

## 3. The Battle Round

### 3.1 Turn Structure

```
BATTLE ROUND
â”œâ”€â”€ Player 1 Turn
â”‚   â”œâ”€â”€ 1. Command Phase
â”‚   â”œâ”€â”€ 2. Movement Phase
â”‚   â”œâ”€â”€ 3. Shooting Phase
â”‚   â”œâ”€â”€ 4. Charge Phase
â”‚   â””â”€â”€ 5. Fight Phase
â””â”€â”€ Player 2 Turn
    â”œâ”€â”€ 1. Command Phase
    â”œâ”€â”€ 2. Movement Phase
    â”œâ”€â”€ 3. Shooting Phase
    â”œâ”€â”€ 4. Charge Phase
    â””â”€â”€ 5. Fight Phase
```

### 3.2 Persisting Effects

Effects with specific durations (e.g., "until start of next turn") are tracked as persisting effects:

- Continue to apply when a unit embarks on a Transport
- Continue to apply if an Attached unit separates (Leader/Bodyguard destroyed)
- Note the effect and its expiration

### 3.3 Out-of-Phase Rules

Some rules allow actions outside normal phase order (e.g., Fire Overwatch allows shooting in opponent's turn).

**Critical Restriction:** When performing out-of-phase actions:

- Only the specified action can be performed
- No other rules normally triggered in that phase activate
- Cannot use other Stratagems from that phase

---

## 4. Command Phase

### 4.1 Command Points

**At the start of each Command Phase:**

- Both players gain **1 Command Point (CP)**
- CP is used to activate Stratagems

**CP Gain Limit:**

- Outside the standard 1 CP at Command Phase start, each player can only gain **1 additional CP per Battle Round** from any source

### 4.2 Battle-shock Tests

**When to Test:** Test each unit that is **Below Half-strength** at the start of its controlling player's Command Phase.

**Test Procedure:**

1. Roll 2D6
2. Compare result to unit's best **Leadership (Ld)** characteristic
3. If result **â‰¥ Ld:** Test passed
4. If result **< Ld:** Test failed, unit is Battle-shocked

**Duration:** Battle-shocked status lasts until the start of that player's next Command Phase.

### 4.3 Battle-shocked Status Effects

A Battle-shocked unit:

- Has **Objective Control (OC) = 0** for all models
- Must take **Desperate Escape tests** if it Falls Back
- **Cannot be affected by friendly Stratagems**

---

## 5. Movement Phase

### 5.1 Movement Types

| Situation                   | Available Movement Options                 |
| --------------------------- | ------------------------------------------ |
| Not within Engagement Range | Normal Move, Advance, or Remain Stationary |
| Within Engagement Range     | Fall Back or Remain Stationary only        |

### 5.2 Remain Stationary

- No models in the unit move
- Unit counts as having Remained Stationary for other rules
- Can still shoot and charge normally

### 5.3 Normal Moves

- Each model moves up to its **Move (M)** characteristic in inches
- Cannot end within Engagement Range of enemy models
- A unit can only make one Normal Move per phase

**Movement Rules:**

- Can move in any combination of straight lines and pivots
- Cannot move through enemy models
- Cannot move through friendly MONSTER or VEHICLE models (must go around)
- Can move through other friendly models
- Cannot end movement on top of another model

### 5.4 Advance Moves

**Procedure:**

1. Roll 1D6 (Advance roll)
2. Add result to each model's Move characteristic for this phase
3. Move each model up to this total distance
4. Cannot end within Engagement Range of enemies

**Restrictions for Advancing Units:**

- Cannot shoot (except with Assault weapons)
- Cannot declare charges this turn

### 5.5 Fall Back Moves

**When Available:** Only when unit is within Engagement Range of enemies

**Procedure:**

- Each model moves up to its Move characteristic
- Can move within Engagement Range during the move
- Must end outside Engagement Range of all enemies (if impossible, cannot Fall Back)

**Restrictions for Falling Back Units:**

- Cannot shoot this turn
- Cannot declare charges this turn

**Desperate Escape Tests:**
When a model moves over an enemy model during Fall Back:

- Roll 1D6 before any models move
- On 1-2: One model from the unit is destroyed (player's choice)
- Exception: TITANIC and FLY models do not require this test
- **Important:** The same model can only ever trigger one Desperate Escape test per phase

**Battle-shocked Fall Back:**
If a Battle-shocked unit Falls Back, take a Desperate Escape test for **every model** in the unit (before any models move).

### 5.6 Pivoting

**Pivot Definition:** Rotating a model around its central vertical axis.

**Pivot Values (subtracted from remaining movement on first pivot):**

| Model Type                                   | Pivot Value |
| -------------------------------------------- | ----------- |
| Non-round base models (not MONSTER/VEHICLE)  | 1"          |
| MONSTER/VEHICLE on non-round base            | 2"          |
| VEHICLE on round base >32mm with flying stem | 2"          |
| AIRCRAFT                                     | 0"          |
| All other models                             | 0"          |

**Note:** Only the first pivot costs movement; additional pivots during the same move are free.

### 5.7 Moving Over Terrain

- Terrain **2" or less** in height: Move over freely as if not there
- Terrain **taller than 2":** Must climb (count vertical distance as movement)
- Cannot move **through** terrain (walls, etc.)
- Cannot end a move mid-climb

### 5.8 Flying Models

Models with the **FLY** keyword:

- Can move over enemy models during Normal, Advance, or Fall Back moves
- Can move over other MONSTER/VEHICLE models
- Cannot end move on top of another model
- Cannot end within Engagement Range of enemies

**Diagonal Movement:** When starting or ending on terrain, measure distance "through the air" (straight diagonal line).

### 5.9 Reinforcements

Units in **Reserves** arrive during the Reinforcements step of the Movement Phase.

**Reinforcement Rules:**

- Set up as specified by the ability that placed them in Reserves
- Distance restrictions from enemies are always **horizontal distance**
- Count as having made a Normal Move this turn
- Can act normally (shoot, charge, etc.) after arriving

**Important:** Any Reserves units not on the battlefield when the battle ends count as destroyed.

### 5.10 Surge Moves

Some abilities allow "surge" moves triggered by specific events.

**Surge Move Restrictions:**

- Each unit can only make one surge move per phase
- Cannot surge while Battle-shocked
- Cannot surge while within Engagement Range of enemies

---

## 6. Transport Rules

### 6.1 Transport Capacity

- Listed on the Transport's datasheet
- Specifies model types and maximum number that can embark
- Units can start the battle embarked (declared before setup)

### 6.2 Embarking

**Requirements:**

- Unit makes a Normal, Advance, or Fall Back move
- Every model ends that move **within 3"** of a friendly Transport
- Unit has not disembarked this phase

**Procedure:**

- Remove unit from battlefield
- Unit is now embarked within the Transport
- Embarked units cannot do anything or be affected while embarked (unless rules state otherwise)

### 6.3 Disembarking

**Timing:** Start of owning player's Movement Phase (before Transport moves)

**Procedure:**

- Set up unit **wholly within 3"** of Transport
- Must be set up **not within Engagement Range** of enemies
- If impossible to set up, unit cannot disembark

**Movement Interaction:**

| Transport Action                    | Disembarking Unit Can...                                                       |
| ----------------------------------- | ------------------------------------------------------------------------------ |
| Not yet moved / Remained Stationary | Act normally (move, shoot, charge) but cannot Remain Stationary                |
| Made Normal Move                    | Count as having made Normal Move; cannot move further or charge, but can shoot |
| Advanced                            | Cannot disembark                                                               |
| Fell Back                           | Cannot disembark                                                               |

### 6.4 Destroyed Transports

When a Transport is destroyed:

1. Embarked units **must** disembark immediately (before removing Transport)
2. Roll 1D6 for each disembarking model: on a **1**, that model's unit suffers **1 mortal wound**
3. Disembarking unit is **Battle-shocked** until start of controlling player's next Command Phase
4. Unit counts as having made a Normal Move this turn
5. Unit **cannot charge** this turn

**Emergency Disembarkation:**
If normal disembarkation is impossible (cannot fit wholly within 3"):

- Set up **wholly within 6"** instead
- Mortal wounds on rolls of **1-3** (instead of just 1)
- Any model that still cannot be set up is **destroyed**

**Note:** Disembarking units are not affected by the Transport's Deadly Demise ability.

### 6.5 Firing Deck

Transports with "Firing Deck X" ability:

- When Transport shoots, select up to X models from embarked units (whose units haven't shot)
- Select one ranged weapon from each (not ONE SHOT weapons)
- Transport counts as equipped with those weapons in addition to its own
- Selected models' units cannot shoot this phase

---

## 7. Shooting Phase

### 7.1 Eligibility to Shoot

A unit is **eligible to shoot** unless:

- That unit Advanced this turn
- That unit Fell Back this turn

Additional requirement: At least one model must have an eligible target (enemy within range and visible).

### 7.2 Target Selection

**Before resolving attacks:**

1. Declare all targets for all weapons in the unit
2. For each weapon, target must be:
   - Within the weapon's **Range**
   - **Visible** to the attacking model

**Multiple Weapons/Targets:**

- A model with multiple weapons can shoot each at different targets
- Cannot split attacks from the **same weapon** across multiple targets
- Models in the same unit can shoot at different targets
- Declare all weapon profile choices if applicable

### 7.3 Making Ranged Attacks

**Attack Resolution Order:**

1. Resolve all attacks against one target before moving to next
2. When shooting multiple weapons at same target with different profiles: resolve same-profile weapons before different-profile weapons

**Persistent Targeting:** If a target was valid when selected (visible and in range), attacks can still be made even if target becomes invalid later (e.g., models destroyed by earlier attacks).

### 7.4 Locked in Combat

**Critical Rules:**

- Units **within Engagement Range of enemies cannot shoot** (exception: see Big Guns Never Tire)
- Units **cannot shoot at enemies** that are within Engagement Range of friendly units (exception: see Big Guns Never Tire)

### 7.5 Big Guns Never Tire

**MONSTER and VEHICLE units** have special shooting rules:

| Situation                                     | Rule                                                                        |
| --------------------------------------------- | --------------------------------------------------------------------------- |
| Within Engagement Range of enemies            | Still eligible to shoot                                                     |
| Targeting enemies within own Engagement Range | Allowed; -1 to Hit (except Pistols)                                         |
| Being targeted while within Engagement Range  | Can be targeted by other units; attacker suffers -1 to Hit (except Pistols) |

---

## 8. Attack Resolution

### 8.1 Hit Rolls

**Procedure:**

1. Roll 1D6 per attack
2. Compare to attacking weapon's **BS** (ranged) or **WS** (melee)
3. Result **â‰¥ BS/WS:** Hit scored
4. Result **< BS/WS:** Attack fails, sequence ends

**Critical Hit:** Unmodified roll of **6** - always successful
**Automatic Fail:** Unmodified roll of **1** - always fails
**Modifier Cap:** -1 to +1

### 8.2 Wound Rolls

**Procedure:**

1. Compare weapon's **Strength (S)** to target's **Toughness (T)**
2. Roll 1D6
3. Result â‰¥ required value: Wound scored

**Wound Roll Table:**

| Strength vs Toughness      | Required Roll |
| -------------------------- | ------------- |
| S â‰¥ 2Ã—T (twice or more) | 2+            |
| S > T (greater)            | 3+            |
| S = T (equal)              | 4+            |
| S < T (less)               | 5+            |
| S â‰¤ Â½T (half or less)   | 6+            |

**Critical Wound:** Unmodified roll of **6** - always successful
**Automatic Fail:** Unmodified roll of **1** - always fails
**Modifier Cap:** -1 to +1

### 8.3 Attack Allocation

The defending player allocates each successful wound to a model in the target unit.

**Allocation Rules:**

- If a model has already lost wounds OR had attacks allocated this phase: **must** allocate to that model
- Otherwise: May allocate to any model in the unit
- Allocation does not require visibility or range to the attacker

### 8.4 Saving Throws

**Procedure:**

1. Roll 1D6
2. Subtract weapon's **AP** value from result
3. Compare modified result to model's **Save (Sv)** characteristic

**Results:**

- Modified result **â‰¥ Sv:** Save successful, attack sequence ends
- Modified result **< Sv:** Save failed, model suffers damage

**Automatic Fail:** Unmodified roll of **1** always fails
**Improvement Cap:** Saving throws can never be improved by more than +1

### 8.5 Invulnerable Saves

Some models have an **Invulnerable Save** (noted on datasheet).

**Rules:**

- When allocated an attack, choose to use **either** normal Save **or** Invulnerable Save
- Invulnerable saves are **never modified by AP**
- If a model has multiple invulnerable saves, choose one to use
- All other saving throw rules apply

### 8.6 Inflicting Damage

**Damage Application:**

- Model loses wounds equal to weapon's **Damage (D)** characteristic
- When wounds reach **0 or below:** Model is destroyed
- **Excess damage is lost** (does not carry over to other models)

### 8.7 Mortal Wounds

Mortal wounds are special damage that bypasses normal attack resolution.

**Mortal Wound Rules:**

- Each mortal wound = 1 point of damage
- Allocated like normal attacks (wounded model first, etc.)
- **No saving throws allowed** (including invulnerable saves)
- Excess damage **carries over** to other models in the unit (unless from HAZARDOUS or DEVASTATING WOUNDS)

**Mortal Wounds from Attacks:**

- If an attack causes mortal wounds **in addition to** normal damage:
  - No Wound roll or save against mortal wounds
  - Normal damage resolved first, then mortal wounds applied
  - Mortal wounds still apply even if normal damage was saved

**Mortal Wounds from HAZARDOUS/DEVASTATING WOUNDS:**

- Excess damage is **lost** when model is destroyed (same as normal attacks)

---

## 9. Charge Phase

### 9.1 Charge Eligibility

A unit is eligible to charge if:

- At least one enemy unit is **within 12"**
- Unit did **not** Advance this turn
- Unit did **not** Fall Back this turn
- Unit is **not** within Engagement Range of enemies
- Unit is **not** an AIRCRAFT

### 9.2 Declaring Charges

**Procedure:**

1. Select an eligible unit
2. Declare one or more enemy units within 12" as charge targets
3. Targets do **not** need to be visible

### 9.3 Charge Rolls

**Procedure:**

1. Roll 2D6 (Charge roll)
2. Result = maximum inches each model can move

### 9.4 Charge Moves

**Success Conditions (ALL must be met):**

- Can end within **Engagement Range of every declared target**
- Does **not** move within Engagement Range of non-targets
- Ends in **Unit Coherency**

**If conditions cannot be met:** Charge fails, no models move

**Successful Charge Movement:**

- Each model moves up to the Charge roll distance
- Each model must end **closer to** one of the charge targets
- If possible, must end in **base-to-base contact** with enemy models

### 9.5 Charge Bonus

Units that make a successful Charge move gain the **Fights First** ability until end of turn.

### 9.6 Charging Over Terrain

- Terrain **â‰¤ 2"** height: Move over freely
- Terrain **> 2"** height: Climb (count vertical distance)
- Cannot end mid-climb (if unavoidable, charge fails)

### 9.7 Charging with Flying Models

Models with **FLY:**

- Can move over other models during Charge moves
- When starting/ending on terrain, measure "through the air"
- Cannot end on top of another model

---

## 10. Fight Phase

### 10.1 Fight Phase Structure

```
FIGHT PHASE
â”œâ”€â”€ Step 1: Fights First
â”‚   â””â”€â”€ All eligible units with Fights First ability fight
â”‚       (alternating, starting with non-active player)
â””â”€â”€ Step 2: Remaining Combats
    â””â”€â”€ All remaining eligible units fight
        (alternating, starting with non-active player)
```

**Eligibility to Fight:**

- Within Engagement Range of enemy units, OR
- Made a Charge move this turn

**Critical Rules:**

- Players **alternate** selecting units, starting with player whose turn it is **not**
- Cannot pass when eligible units remain
- No unit can fight more than once per Fight phase

### 10.2 Fights First Step

Units with the **Fights First** ability fight in this step.
This includes all units that charged this turn (due to Charge bonus).

### 10.3 Remaining Combats Step

All other eligible units fight.
Includes Fights First units that became eligible after Step 1.

### 10.4 Pile In

**Pile-in Move:** Up to **3"**

**Requirements:**

- Only models not in base-to-base contact with enemies may Pile In
- Unit must end Pile In within Engagement Range of at least one enemy unit
- Unit must end in Unit Coherency

**Movement Rules:**

- Each model must end **closer to the closest enemy model**
- Must end in **base-to-base contact** with an enemy if possible

**If conditions cannot be met:** No models may Pile In; proceed directly to making attacks.

### 10.5 Making Melee Attacks

**Which Models Fight:**
A model can fight if:

- It is within Engagement Range of an enemy unit, OR
- It is in base-to-base contact with a friendly model that is in base-to-base contact with an enemy

**Weapon Selection:**

- Each model selects **one** melee weapon to attack with
- If weapon has multiple profiles, select one profile
- Number of attacks = weapon's **Attacks (A)** characteristic

**Target Selection:**

- Must target an enemy unit the model is within Engagement Range of, OR
- Target an enemy unit that a friendly model in base-to-base contact is within Engagement Range of

**Attack Resolution:**
Same as ranged attacks (Hit roll using WS, Wound roll, Save, Damage).

### 10.6 Consolidate

After all models in the unit have fought:

**Consolidation Move:** Up to **3"**

**Who Consolidates:** Models not in base-to-base contact with enemies

**Movement Rules:**

- Must end **closer to the closest enemy model**
- Must end in **base-to-base contact** with enemy if possible

---

## 11. Weapon Abilities

### 11.1 Assault

**Effect:** Unit can shoot this weapon even if it Advanced this turn.
**Restriction:** If unit Advanced, can only shoot Assault weapons.

### 11.2 Heavy

**Effect:** If bearer's unit Remained Stationary this turn, add **+1 to Hit rolls**.

### 11.3 Rapid Fire

**[RAPID FIRE X]**
**Effect:** When targeting units within **half range**, increase Attacks characteristic by X.

### 11.4 Pistol

**Effects:**

- Can shoot even while within Engagement Range of enemies
- Must target an enemy unit within Engagement Range
- Can target enemies even if friendly units are also within Engagement Range

**Restriction:** Unless MONSTER or VEHICLE, model must choose to shoot Pistols OR other ranged weapons (not both).

### 11.5 Blast

**Effect:** Add +1 to Attacks for every 5 models in target unit (rounded down).

**Restriction:** Cannot target units within Engagement Range of friendly units.

### 11.6 Torrent

**Effect:** Attacks automatically hit (no Hit roll required).

### 11.7 Melta

**[MELTA X]**
**Effect:** When targeting units within **half range**, increase Damage characteristic by X.

### 11.8 Lance

**Effect:** If bearer made a Charge move this turn, add **+1 to Wound rolls**.

### 11.9 Twin-linked

**Effect:** Can re-roll the Wound roll for each attack.

### 11.10 Lethal Hits

**Effect:** Critical Hits (unmodified 6 to hit) automatically wound the target (skip Wound roll).

### 11.11 Sustained Hits

**[SUSTAINED HITS X]**
**Effect:** Critical Hits score X additional hits on the target.

### 11.12 Devastating Wounds

**Effect:** On a Critical Wound (unmodified 6 to wound):

- No saving throw of any kind allowed (including invulnerable)
- Attack is allocated after all other attacks resolved
- Inflicts mortal wounds equal to weapon's Damage (instead of normal damage)
- Excess mortal wounds lost if model destroyed

### 11.13 Hazardous

**Procedure:**

1. After unit finishes shooting/fighting, roll one **Hazardous test** (D6) per Hazardous weapon used
2. On a **1:** Test failed

**Failed Test Resolution:**

1. Select a model equipped with Hazardous weapon (prioritize: wounded models â†’ non-CHARACTER â†’ CHARACTER)
2. Unit suffers **3 mortal wounds** allocated to that model

### 11.14 Precision

**Effect:** When successfully wounding an Attached unit, if a CHARACTER model is visible, attacker may choose to allocate the attack to that CHARACTER.

### 11.15 Anti-X

**[ANTI-KEYWORD X+]**
**Effect:** When attacking a target with the specified keyword, unmodified Wound rolls of X+ score Critical Wounds.

### 11.16 Indirect Fire

**Effect:** Can target units not visible to the attacker.

**Penalties when no models visible:**

- Subtract **1 from Hit rolls**
- Unmodified Hit rolls of **1-3 always fail**
- Target gains **Benefit of Cover**

**Restriction:** Weapons with TORRENT cannot use Indirect Fire.

### 11.17 Ignores Cover

**Effect:** Target cannot have Benefit of Cover against attacks from this weapon.

### 11.18 Extra Attacks

**Effect:** When bearer fights, it attacks with:

- Each Extra Attacks weapon it has, AND
- One other melee weapon (without Extra Attacks)

**Note:** Number of attacks from Extra Attacks weapons cannot be modified by other rules (unless specifically named).

---

## 12. Unit Abilities

### 12.1 Feel No Pain

**[FEEL NO PAIN X+]**
**Effect:** Each time this model would lose a wound, roll D6. On X+, that wound is not lost.

**Rules:**

- Applies to all wound loss (normal damage, mortal wounds)
- Roll for each wound individually
- If model has multiple Feel No Pain abilities, only use one per wound

### 12.2 Deadly Demise

**[DEADLY DEMISE X]**
**Effect:** When model is destroyed, roll D6. On a 6, each unit within 6" suffers X mortal wounds.

**Timing:** Roll before removing model (for Transports: before embarked units disembark).

### 12.3 Deep Strike

**Effect:**

- Unit can be set up in Reserves instead of on battlefield during deployment
- During Reinforcements step, can be set up anywhere on battlefield **more than 9" horizontally** from all enemy models

### 12.4 Infiltrators

**Effect:** During deployment, if every model has this ability, unit can be set up:

- Anywhere on battlefield
- More than **9" horizontally** from enemy deployment zone
- More than **9" horizontally** from all enemy models

### 12.5 Scouts

**[SCOUTS X"]**
**Effect:** Before first turn begins, unit can make a Normal move of up to X".

**Rules:**

- If embarked in Dedicated Transport, the Transport can make this move instead
- Must end **more than 9" horizontally** from all enemy models

### 12.6 Lone Operative

**Effect:** Unless this unit is part of an Attached unit, this unit can only be selected as the target of a ranged attack if the attacking model is **within 12"**.

**Key Points:**

- Does NOT apply if the Lone Operative is attached to another unit (part of an Attached unit)
- Applies to the entire unit, not just individual models
- Enemy models must be within 12" to target this unit with ranged attacks

### 12.7 Stealth

**Effect:** If every model in unit has this ability, subtract **1 from Hit rolls** for ranged attacks against this unit.

### 12.8 Leader and Attached Units

**Leader Ability:**

- CHARACTER units with Leader can attach to specified Bodyguard units before battle
- Creates an **Attached unit**

**Attached Unit Rules:**

- Only one Leader per Attached unit
- Attacks **cannot** be allocated to CHARACTER models in Attached units (unless Precision)
- Starting Strength = combined Starting Strength of Leader + Bodyguard
- If Leader or Bodyguard destroyed, surviving unit reverts to its **original Starting Strength** (not the combined value)

**Destroyed Unit Triggers:**

- For rules triggered when a unit is destroyed, such rules are still triggered when one of the individual units that made up an Attached unit is destroyed
- Example: If a rule awards VP when an enemy unit is destroyed, destroying the Leader unit awards VP, and destroying the Bodyguard unit awards additional VP (for a total of 2VP if both are destroyed)

### 12.9 Fights First

**Effect:** Unit fights in the Fights First step of Fight phase (if eligible to fight).

**Requirement:** Every model in unit must have this ability.

### 12.10 Aura Abilities

Some abilities affect units within a certain range. These are known as Aura abilities.

**Identification:** Typically phrased as "while within X" of a model or unit.

**Rules:**

- Check range and conditions when the Aura ability is used/applied
- A model is **not** within range of its own Aura ability unless specifically stated
- Aura abilities that affect "friendly units" do not affect enemy units, and vice versa
- If a unit is within range of multiple identical Aura abilities, the effects do not stack unless stated

**Example:** An ability stating "While a friendly unit is within 6" of this model, add 1 to hit rolls" is an Aura ability. The model with the ability does not benefit from it unless stated.

### 12.11 Psychic Weapons and Abilities

**Psychic Weapons:**
Weapons with **[PSYCHIC]** in their profile are Psychic weapons.

**Rules:**

- Follow all normal weapon rules
- Typically can only be used by PSYKER models
- Some special rules specifically interact with Psychic weapons

**Psychic Abilities:**
Some abilities are noted as Psychic abilities on datasheets.

- These are used according to their specific rules
- May be affected by rules that interact with Psychic abilities

### 12.12 Leadership Tests

Leadership tests are required by various rules throughout the game.

**Procedure:**

1. Roll 2D6
2. Compare result to the unit's best **Leadership (Ld)** characteristic
3. If result **≥ Ld:** Test passed
4. If result **< Ld:** Test failed

**Note:** Battle-shock tests are a specific type of Leadership test with additional consequences (see Section 4.2).

---

## 13. Terrain

### 13.1 Benefit of Cover

**Effect:** Add **+1 to armour saving throws** against ranged attacks.

**Restrictions:**

- Does NOT apply to invulnerable saves
- Models with **Save 3+ or better** do not get Benefit of Cover against attacks with **AP 0**
- Multiple instances do not stack

### 13.2 Terrain Heights

| Height | Movement Rule                        |
| ------ | ------------------------------------ |
| â‰¤ 2" | Move over freely                     |
| > 2"   | Must climb (count vertical distance) |

---

## 14. Strategic Reserves

### 14.1 Placing Units in Reserves

During army mustering, units can be placed in Strategic Reserves (following mission rules for point limits).

### 14.2 Arriving from Reserves

**Arrival Timing by Battle Round:**

| Battle Round | Can Arrive? | Placement Restrictions            |
| ------------ | ----------- | --------------------------------- |
| 1            | No          | N/A                               |
| 2+           | Yes         | Within 6" of any battlefield edge |
| 3+           | Yes         | Within 6" of any battlefield edge |

**Placement Rules:**

- Set up wholly within 6" of battlefield edge
- More than 9" horizontally from enemy models
- If unit has Deep Strike, can choose to use that instead

**Destroyed:** Any Strategic Reserve unit not on battlefield at battle end counts as destroyed.

---

## 15. Stratagems

### 15.1 Using Stratagems

**Structure:**

- **CP Cost:** Command Point cost to use
- **When:** Timing for when Stratagem can be used
- **Target:** What unit(s) can be targeted
- **Effect:** What the Stratagem does
- **Restrictions:** Limitations on use

**General Rules:**

- Cannot use same Stratagem more than once per phase (unless stated)
- Cannot affect Battle-shocked units with friendly Stratagems

### 15.2 Core Stratagems

| Stratagem               | CP  | When                                              | Effect                                                               |
| ----------------------- | --- | ------------------------------------------------- | -------------------------------------------------------------------- |
| **Command Re-roll**     | 1   | Any phase, after a roll                           | Re-roll one dice roll, test, or saving throw                         |
| **Counter-Offensive**   | 2   | Fight phase, after enemy fights                   | One of your units fights next                                        |
| **Epic Challenge**      | 1   | Fight phase, CHARACTER selected to fight          | CHARACTER's melee attacks gain Precision                             |
| **Insane Bravery**      | 1   | Battle-shock step, before test                    | Unit auto-passes Battle-shock test (once per battle)                 |
| **Grenade**             | 1   | Shooting phase                                    | GRENADES unit: roll 6D6, each 4+ = 1 mortal wound to enemy within 8" |
| **Tank Shock**          | 1   | Charge phase, after VEHICLE charges               | Roll D6 equal to Toughness: each 5+ = 1 mortal wound (max 6)         |
| **Rapid Ingress**       | 1   | End of opponent's Movement phase                  | One Reserves unit arrives as if your Reinforcements step             |
| **Fire Overwatch**      | 1   | Opponent's Movement/Charge phase                  | Unit shoots enemy (requires 6 to hit, once per turn, no TITANIC)     |
| **Go to Ground**        | 1   | Opponent's Shooting phase, after targets selected | INFANTRY: 6+ invulnerable save + Benefit of Cover                    |
| **Smokescreen**         | 1   | Opponent's Shooting phase, after targets selected | SMOKE unit: Benefit of Cover + Stealth                               |
| **Heroic Intervention** | 1   | Opponent's Charge phase, after enemy charges      | Unit within 6" declares charge against that enemy (no Charge bonus)  |

---

## 16. Aircraft Rules

### 16.1 Aircraft Movement

**Deployment:** AIRCRAFT can be set up in Reserves without counting against Strategic Reserves limits.

**Movement Rules:**

- Must move minimum of **20"** in Movement phase
- Cannot Remain Stationary, Advance, or Fall Back
- Pivot value: **0"** (but separate pivoting rules apply)
- Can move over all models
- Can move within Engagement Range of enemy models
- Must end move **more than 9"** from all enemy models (if impossible, destroyed)

**Leaving Battlefield:**

- If movement would take AIRCRAFT off edge, remove from play
- Placed into Reserves
- Returns in later Reinforcements step

### 16.2 Aircraft in Combat

- **Charge Phase:** Cannot declare or be target of charges
- **Fight Phase:** Cannot make Pile-in or Consolidation moves; enemy units cannot end these moves within Engagement Range of AIRCRAFT
- **Can fight** if within Engagement Range (though rarely occurs)

---

## 17. Unit States and Conditions

### 17.1 Starting Strength

**Definition:** The number of models in a unit when added to the army.

**For Attached Units:** Combined Starting Strength of Leader + Bodyguard units.

### 17.2 Below Half-strength

**Calculation:**

| Unit Type         | Below Half-strength When...                    |
| ----------------- | ---------------------------------------------- |
| Single-model unit | Remaining wounds < Â½ of Wounds characteristic |
| Multi-model unit  | Remaining models < Â½ of Starting Strength     |

### 17.3 Destroyed

**Model Destroyed:** When a model's wounds reach 0 or below, it is destroyed and removed from play.

**Unit Destroyed:** When all models in a unit are destroyed, the unit is destroyed.

**Destroyed Triggers:** Rules that trigger "when destroyed" activate at this point.

---

## 18. Objective Markers

### 18.1 Objective Marker Basics

Objective markers represent key locations on the battlefield that armies fight to control.

**Placement:**

- Objective markers are placed according to the mission being played
- Typically circular markers approximately 40mm in diameter
- Models can move over and onto objective markers

### 18.2 Controlling Objectives

**Objective Control (OC):**
Each model has an Objective Control (OC) characteristic on its datasheet.

**Determining Control:**

1. Count the total OC of all models within range of the objective marker (typically 3")
2. The player with the **higher total OC** controls that objective
3. If totals are equal, the objective is **contested** (no one controls it)

**OC Modifiers:**

- Battle-shocked units have **OC 0** for all models
- Some abilities can modify OC values

**Range to Objectives:**

- Unless stated otherwise, a model is within range of an objective if it is within **3"** of the center of the objective marker
- Measure horizontally

### 18.3 Objective Markers and Terrain

- Objective markers can be placed on terrain features
- If an objective is on a terrain feature, models must be able to physically reach it to be within range
- Models on different levels of a terrain feature can both be within range if within 3" horizontally

### 18.4 Sticky Objectives

Some missions use "sticky" objectives with the following rule:

**Sticky Objective:** Once you control an objective marker, it remains under your control even if you have no models within range, until your opponent controls it.

---

## 19. Muster Your Army

### 19.1 Army Construction Basics

Before battle, players muster their armies following these steps:

**Points Limit:**

- Agree on a points limit with your opponent (e.g., 1000, 2000 points)
- Your army's total points cannot exceed this limit
- Each unit's points cost is listed on its datasheet

### 19.2 Detachments

**Detachment Rules:**

- Your army must be from a single Detachment
- Detachments provide special rules, Stratagems, and Enhancements
- All units must share a common Faction keyword (with some exceptions)

### 19.3 Army Composition

**Unit Categories:**
| Category | Description |
|----------|-------------|
| **Character** | Heroes and leaders (CHARACTER keyword) |
| **Battleline** | Core troops |
| **Dedicated Transport** | Vehicles that transport specific units |
| **Other** | All other unit types |

**Duplicate Units:**

- Maximum of **3 copies** of the same datasheet (same unit) in your army
- Maximum of **6 copies** for units with the BATTLELINE or DEDICATED TRANSPORT keywords

### 19.4 Warlord

**Designation:**

- One CHARACTER model in your army must be designated as your **Warlord**
- This is typically your army's leader
- Some missions have special rules for Warlords

### 19.5 Enhancements

**Enhancement Rules:**

- Enhancements are upgrades given to CHARACTER models
- Each Enhancement can only be included once in your army
- A CHARACTER model can only have one Enhancement
- Enhancements have points costs that add to your army total

---

## Appendix A: Quick Reference Tables

### Wound Roll Reference

| Strength vs Toughness | Roll Required |
| --------------------- | ------------- |
| S â‰¥ 2Ã—T            | 2+            |
| S > T                 | 3+            |
| S = T                 | 4+            |
| S < T                 | 5+            |
| S â‰¤ Â½T             | 6+            |

### Critical Rolls

| Roll Type  | Critical Value | Effect                         |
| ---------- | -------------- | ------------------------------ |
| Hit Roll   | Unmodified 6   | Always hits (Critical Hit)     |
| Wound Roll | Unmodified 6   | Always wounds (Critical Wound) |

### Modifier Caps

| Roll Type          | Cap |
| ------------------ | --- |
| Hit Roll           | Â±1 |
| Wound Roll         | Â±1 |
| Save (improvement) | +1  |

### Distance References

| Rule                          | Distance          |
| ----------------------------- | ----------------- |
| Engagement Range (horizontal) | 1"                |
| Engagement Range (vertical)   | 5"                |
| Unit Coherency (horizontal)   | 2"                |
| Unit Coherency (vertical)     | 5"                |
| Deep Strike minimum distance  | 9"                |
| Embark distance               | 3"                |
| Disembark distance            | 3" (6" emergency) |
| Pile In / Consolidate         | 3"                |
| Objective marker range        | 3"                |

### Objective Control Quick Reference

| Situation              | OC Value               |
| ---------------------- | ---------------------- |
| Normal model           | As listed on datasheet |
| Battle-shocked model   | 0                      |
| Model not within range | Does not contribute    |

**Determining Control:**

1. Sum OC of all models within range of objective
2. Player with higher total controls the objective
3. If tied, objective is contested (no one controls)

---

## Appendix B: Phase Summary

### Command Phase Checklist

1. â˜ Both players gain 1 CP
2. â˜ Resolve Command phase abilities
3. â˜ Take Battle-shock tests for Below Half-strength units

### Movement Phase Checklist

1. â˜ Select unit to move
2. â˜ Choose: Normal Move / Advance / Fall Back / Remain Stationary
3. â˜ Move models (respect Engagement Range, Coherency)
4. â˜ Repeat for all units
5. â˜ Set up Reinforcements (from Reserves)

### Shooting Phase Checklist

1. â˜ Select eligible unit
2. â˜ Declare all targets for all weapons
3. â˜ Resolve attacks (one target at a time)
4. â˜ Repeat for all eligible units

### Charge Phase Checklist

1. â˜ Select eligible unit
2. â˜ Declare charge targets (within 12")
3. â˜ Roll 2D6 for Charge distance
4. â˜ Make Charge move (or fail)
5. â˜ Repeat for all charging units

### Fight Phase Checklist

1. â˜ **Fights First Step:** Alternate fighting with Fights First units
2. â˜ **Remaining Combats:** Alternate fighting with remaining units
3. â˜ For each fighting unit:
   - â˜ Pile In (up to 3")
   - â˜ Select weapons and targets
   - â˜ Resolve attacks
   - â˜ Consolidate (up to 3")

---

_Document Version: 1.1_
_Source: Warhammer 40,000 10th Edition Core Rules (as of January 2026)_
_Adapted for game development reference_

**Changes in v1.1:**

- Added Redeployments (Section 1.4)
- Added Sequencing (Section 2.5)
- Added Random Characteristics (Section 2.6)
- Added Aura Abilities (Section 12.10)
- Added Psychic Weapons and Abilities (Section 12.11)
- Added Leadership Tests (Section 12.12)
- Added Objective Markers (Section 18)
- Added Muster Your Army (Section 19)
- Updated Lone Operative with full clarification
- Updated Desperate Escape with clarification
- Updated Leader and Attached Units with destroyed unit triggers
- Added visibility clarification for fully visible units

# Rule Coverage Matrix

Tracks implemented / partially implemented / not implemented rules.
Updated: 2026-03-17 from full audit.

## Core 40K Rules (40k_revised.md)

| Rule | Section | Status | Engine File | Notes |
|------|---------|--------|-------------|-------|
| Normal Move | §5.3 | PARTIAL | executor.rs, validator.rs | No path collision detection — models teleport through others |
| Advance Move | §5.4 | IMPL | executor.rs | M + D6 works, flag tracking works |
| Fall Back Move | §5.5 | PARTIAL | executor.rs | Desperate Escape count logic wrong (per enemy in ER, not per crossing) |
| Desperate Escape Tests | §5.5 | PARTIAL | executor.rs | Exists but trigger logic approximate |
| Pivot Costs | §5.6 | MISSING | — | No implementation |
| Terrain Climbing | §5.7 | MISSING | — | No vertical distance, no terrain height system |
| FLY Movement | §5.8 | MISSING | — | FLY keyword exists but not checked during movement |
| Surge Moves | §5.10 | MISSING | — | No implementation |
| Transports (full system) | §6.1-6.5 | MISSING | — | No embark/disembark/capacity/firing deck |
| Shooting Phase | §7.1-7.5 | IMPL | executor.rs, validator.rs, combat.rs | Core pipeline works |
| Locked in Combat | §7.4 | FIXED | validator.rs | Non-Pistol cannot target engaged enemies |
| Big Guns Never Tire | §7.5 | FIXED | combat.rs | Both shooter-in-ER and target-in-ER penalties |
| Hit/Wound/Save Pipeline | §8.1-8.7 | IMPL | combat.rs | Core pipeline works |
| Save Improvement Cap | §8.4 | FIXED | combat.rs | +1 max from all sources |
| Mortal Wounds | §8.7 | IMPL | combat.rs | Bypasses saves, FNP applies |
| Charge Declaration | §9.1-9.2 | IMPL | validator.rs | Targets visible (CP), range check |
| Charge Roll | §9.3 | IMPL | executor.rs | 2D6, distance check |
| Charge Move | §9.4 | FIXED | validator.rs | Must reach ALL declared targets |
| Fight Phase Alternation | §10.1 | FIXED | phase.rs | Non-active player picks first |
| FightsFirst → RemainingCombats | §10.1-10.3 | PARTIAL | phase.rs | Function exists, not auto-triggered |
| Pile-In | §10.4 | PARTIAL | validator.rs | Missing: b2b-only models, must-end-in-b2b |
| Melee Attacks | §10.5 | FIXED | executor.rs | Only ER-eligible models fight |
| Consolidation | §10.6 | PARTIAL | validator.rs | Same gaps as Pile-In |
| Deadly Demise | §12.2 | FIXED | executor.rs | Uses datasheet value (was always D3) |
| Infiltrators | §12.4 | MISSING | — | No deployment logic |
| Scouts | §12.5 | MISSING | — | No pre-game move |
| Lone Operative | §12.6 | IMPL | validator.rs | 12" range + closest target check |
| Stealth | §12.7 | IMPL | combat.rs | -1 to hit |
| Leader/Attached Units | §12.8 | PARTIAL | combat.rs | Bodyguard protection works, starting strength not combined |
| Aura Abilities | §12.10 | MISSING | — | No generic aura system |
| Psychic Weapons | §12.11 | MISSING | — | No PSYKER restriction |
| Extra Attacks | §11.18 | MISSING | — | Not enforced in fight sequence |
| Indirect Fire | §11.16 | IMPL | combat.rs, validator.rs | Works, Torrent prohibition added |
| Strategic Reserves | §14.2 | FIXED | validator.rs | Within 6" of edge enforced |
| Aircraft Movement | §16.1-16.2 | MISSING | — | Only charge prohibition enforced |
| Redeployments | §1.4 | MISSING | — | No mechanism |
| Unit Coherency | §1.7 | MISSING | — | No end-of-turn check, no movement check |
| Model on Model | §1.6 | MISSING | — | No "destroyed if forced into ER" |
| Model Stacking | §5.3 | MISSING | — | Units can stack on same position |
| Stratagems Per Phase | §15.1 | FIXED | validator.rs, phase.rs | Tracked and reset for both players |
| Battle-shocked Strat Block | §4.3 | IMPL | validator.rs | Implemented |
| Counter-Offensive | §15.2 | FIXED | stratagem.rs, validator.rs | AfterEnemyUnitFights timing |
| Epic Challenge | §15.2 | FIXED | executor.rs | Precision applied to melee |
| Simultaneous Rule Ordering | §2.5 | MISSING | — | No sequencing system |

## Combat Patrol Rules (CP_Rules.md)

| Rule | Section | Status | Engine File | Notes |
|------|---------|--------|-------------|-------|
| Board Size 44"x30" | §2 | IMPL | scenario.rs | Correct |
| 6 Missions | §13 | IMPL | scenario.rs, scoring.rs | All 6 present |
| Mission 1 Deployment | §13 | IMPL | scenario.rs | Search & Destroy |
| Missions 2-6 Deployment | §13 | STUB | scenario.rs | All use standard instead of per-mission layouts |
| Archeotech Recovery Timing | §13 | FIXED | phase.rs | Comments corrected (code was correct) |
| Scorched Earth Raze | §13 | PARTIAL | executor.rs | Command exists, phase doesn't offer decision |
| Display of Might VP Cap | §13 | MISSING | scoring.rs | No per-round 20VP cap |
| Plasma Pistol Supercharge | Faction | FIXED | scenario.rs | Both profiles present |
| Blessings of Khorne | Faction | FIXED | faction_runtime | Double 2+ no longer requires matching |
| Fearsome Presence | Faction | IMPL | unit.rs | OC 5 when not battle-shocked |
| Bane of the Craven | Faction | PARTIAL | scenario.rs | Effect text updated with exclusions |
| Martial Ka'tah Stacking | Faction | UNVERIFIED | — | Sustained Hits stacking needs testing |

## Boarding Actions Rules (boarding_actions_complete_v3.md)

| Rule | Section | Status | Engine File | Notes |
|------|---------|--------|-------------|-------|
| Walls Block Movement/LOS | §3.2 | IMPL | movement.rs, visibility.rs | Works |
| Hatchway States | §3.2 | IMPL | hatchway.rs | Open/Closed/Locked/OneWay |
| Operating Hatchways | §3.4 | IMPL | hatchway.rs | ER check, range, roll-off |
| FLY Suppression | §3.2 | FIXED | movement.rs | Function added |
| Move Cap 9" | §3.2 | IMPL | movement.rs | Correct |
| Deep Strike Rounds 2-3 | §3.1 | FIXED | movement.rs | Function added |
| Deep Strike Max 1/Round | §3.1 | FIXED | movement.rs | Function added |
| Returning Models Cap | §3.8 | FIXED | movement.rs | Constant added |
| Measurement Around Walls | §3.2 | MISSING | movement.rs | Uses straight-line, not pathfinding |
| ER Through Open Hatchway | §3.2 | IMPL | hatchway.rs | 2" horizontal |
| Objective Range 1" | §3.2 | FIXED | movement.rs | Function added |
| Secured Objectives | §3.4 | FIXED | tactical_manoeuvres.rs | State machine functions added |
| BA Enhancements Effects | §5 | FIXED | stratagems.rs | All 6 effects implemented |
| BA Stratagems | §4 | IMPL | stratagems.rs | All 5 universal |
| BA Deployment Sequence | §7.4 | MISSING | — | No alternating one-unit-per-zone |
| Reserves Via Entry Zones | §7.5 | MISSING | — | BA-specific reserve rules |
| Underdog Threshold | §6.6 | FIXED | scoring.rs | 30-point function |
| Pile-In/Consolidation BA | §3.6 | MISSING | — | BA visibility changes |
| Attack Allocation BA | §3.3 | MISSING | — | Unwounded target visible model |
| Prison Break Test | §4.4 | FIXED | mission_mechanics.rs | Logic corrected |
| Destroyed Points Table | §addendum | FIXED | scoring.rs | 0/15/30/45 VP |
| Furnace Burner MW Cap | §3.8 | FIXED | mission_mechanics.rs | 3 per unit per turn |
| Desperate Measures CP | §3.8 | FIXED | mission_mechanics.rs | 5+ on D6 |
| Inaccessible Area | §4.1 | FIXED | movement.rs | Function added |
| Prison Cells Restrictions | §4.4 | FIXED | mission_mechanics.rs | Functions added |
| Multi-Level Visibility | §4.5 | FIXED | visibility.rs | Function added |
| Corruption Consequences | §4.6 | FIXED | mission_mechanics.rs | Effects enum added |

## Perturabo Pipeline (AI_Primer.md)

| Component | Status | File | Notes |
|-----------|--------|------|-------|
| Self-play with ID search | FIXED | selfplay/src/lib.rs | Defaults to IterativeDeepening(4) |
| Configurable AI type | FIXED | trainer_bridge/src/lib.rs | ai_type param |
| Both game modes | FIXED | trainer_bridge/src/lib.rs | game_mode param |
| Model path for NNUE | FIXED | selfplay/src/lib.rs | model_path param |
| Feature normalization | FIXED | shard_loader.py | Divide by 127.0 |
| Training hyperparams | FIXED | train.py | Matches AI_Primer spec |
| AI_Primer constants | FIXED | AI_Primer.md | All 20+ refs updated to 1209/640/40 |
| Gating with proper search | PARTIAL | trainer_bridge/src/lib.rs | Uses ID(6) but only vs heuristic |

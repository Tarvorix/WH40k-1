# Rule Coverage Matrix

## Authoritative files
- `40k_revised.md` — Core 40K rules (19 sections, ~379 testable rules)
- `CP_Rules.md` — Combat Patrol overlay (~172 testable rules)
- `boarding_actions_complete_v3.md` — Boarding Actions rules (~118 testable rules)

## Source-Linked Rules Tests

All tests in `engine/crates/rules_tests/tests/`. Each test cites its rule source in a doc comment.

**Total: 626 source-linked rules tests across 22 test files.**

### Core Rules (40k_revised.md)

| Rule Section | Source | Test File | Tests | Status |
|-------------|--------|-----------|-------|--------|
| §1-3 Core Concepts, Dice, Battle Round | 40k_revised.md §1-3 | `core_dice_and_basics.rs` | 28 | Tested |
| §4 Command Phase | 40k_revised.md §4 | `core_command_phase.rs` | 23 | Tested |
| §5 Movement Phase | 40k_revised.md §5 | `core_movement.rs` | 57 | Tested |
| §6 Transport Rules | 40k_revised.md §6 | `core_transports.rs` | 31 | Tested |
| §7 Shooting Phase | 40k_revised.md §7 | `core_shooting.rs` | 22 | Tested |
| §8 Attack Resolution | 40k_revised.md §8 | `core_attack_resolution.rs` | 32 | Tested |
| §9 Charge Phase | 40k_revised.md §9 | `core_charge.rs` | 39 | Tested |
| §10 Fight Phase | 40k_revised.md §10 | `core_fight.rs` | 41 | Tested |
| §11 Weapon Abilities | 40k_revised.md §11 | `core_weapon_abilities.rs` | 51 | Tested |
| §12 Unit Abilities | 40k_revised.md §12 | `core_unit_abilities.rs` | 38 | Tested |
| §15 Stratagems | 40k_revised.md §15 | `core_stratagems.rs` | 37 | Tested |
| §18 Objectives | 40k_revised.md §18 | `core_objectives.rs` | 28 | Tested |

### Combat Patrol (CP_Rules.md)

| Rule Section | Source | Test File | Tests | Status |
|-------------|--------|-----------|-------|--------|
| §2 Pre-Battle Setup | CP_Rules.md §2 | `cp_setup.rs` | 21 | Tested |
| §13 Missions (all 6) | CP_Rules.md §13 | `cp_missions.rs` | 29 | Tested |

### Boarding Actions (boarding_actions_complete_v3.md)

| Rule Section | Source | Test File | Tests | Status |
|-------------|--------|-----------|-------|--------|
| §2.3, §3.2 Hatchways | BA §2.3, §3.2 | `ba_hatchways.rs` | 35 | Tested |
| §3.2 Movement | BA §3.2 | `ba_movement.rs` | 35 | Tested |
| §3.3 Visibility | BA §3.3 | `ba_visibility.rs` | 13 | Tested |
| §3.4 Tactical Manoeuvres | BA §3.4 | `ba_tactical_manoeuvres.rs` | 23 | Tested |
| §3.7 Leaders | BA §3.7 | `ba_leaders.rs` | 7 | Tested |
| §4 Stratagems | BA §4 | `ba_stratagems.rs` | 8 | Tested |
| §6 Mustering | BA §6 | `ba_mustering.rs` | 12 | Tested |
| §7-8 Scoring | BA §7-8 | `ba_scoring.rs` | 16 | Tested |

## Coverage Summary

| Document | Sections Covered | Test Files | Total Tests |
|----------|-----------------|------------|-------------|
| 40k_revised.md | 12 of 19 | 12 | 427 |
| CP_Rules.md | 2 key sections | 2 | 50 |
| boarding_actions_complete_v3.md | 8 sections | 8 | 149 |
| **TOTAL** | **22** | **22** | **626** |

### Sections Not Yet Covered
- §13 Terrain (4 rules) — partially covered via movement tests
- §14 Strategic Reserves (5 rules) — partially covered via movement/CP tests
- §16 Aircraft Rules (12 rules) — keyword checks in charge tests
- §17 Unit States (6 rules) — covered via command phase/unit ability tests
- §19 Muster Your Army (12 rules) — covered via BA mustering tests

## Requirement
Every implemented rule must record:
- source document
- source section or heading
- engine module
- automated tests
- replay test coverage
- unresolved gaps / TODOs

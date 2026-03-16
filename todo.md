# WH40K Engine - Implementation TODO

**Status: 9 real mission-specific BA map layouts implemented + full pipeline wired**
**All tests pass (0 failures) | 713 rules tests + 1,571 inline tests | Workspace compiles clean**

---

## Mission Map Layouts (2026-03-16) — DONE

### What was broken
Previous session claimed "Map Pipeline (fully wired)" but 8 of 9 symmetric missions
were using an identical generic placeholder layout. Only BA-11 had real geometry.
All missions looked the same in-game.

### What was fixed
All 9 symmetric missions now have unique, mission-specific map layouts transcribed
from official mission map images (Wahapedia source):

| Mission ID | Name | Objectives | Special Regions | Status |
|-----------|------|------------|-----------------|--------|
| BA-11 | Access Junction Primus | 3 | Central Junction | Real layout |
| BA-12 | Deck Sweepers | 3 | Underdog Entry Zone | Real layout |
| BA-13 | The Pipeline | 4 (2A+2B) | Power Line Network | Real layout |
| BA-21 | Power Struggle | 4 | Power Lines corridor | Real layout |
| BA-22 | Death in the Dark | 4 (2 per area) | 2 Lighting Areas | Real layout |
| BA-23 | Hull Breach | 3 datacores | 4 ventable compartments (1,2,4,5) | Real layout |
| BA-31 | Control Centre | 4 (A,B,C,D) | Control Centre room | Real layout |
| BA-32 | The Furnace | 3 | Furnace + 2 Furnace Control Zones | Real layout |
| BA-33 | Rad Leak | 3 (2 critical) | 4 radiation sectors (A,B,C,D) | Real layout |

Each mission has:
- [x] Unique compartment layout (8-13 compartments per mission)
- [x] Unique wall geometry (20-26 wall segments per mission)
- [x] Mission-specific hatchway count and initial states
- [x] Correct objective count and positions per source data
- [x] Player entry zones with correct roles (Main, Underdog, etc.)
- [x] Mission-specific special regions (Furnace, Power Lines, Lighting Areas, Sectors, etc.)
- [x] 14 unit tests verifying each mission's unique features

### Still NOT done (be honest)
- [ ] **Asymmetric missions (BA-1 through BA-6)** still use generic layout with minor entry zone tweaks — no mission images provided yet
- [ ] **Coordinate refinement** — layouts are structurally correct (right topology, right features) but exact wall coordinates are best-effort from small reference images, not pixel-perfect
- [ ] **Mission-specific mechanics** not wired to game flow (lighting rolls, venting, radiation, furnace burners, power line scoring)
- [ ] **Mission-specific scoring** only partially done in scoring.rs
- [ ] **WASM binary not rebuilt** — need `wasm-pack build` to see changes in browser
- [ ] **Not tested in actual browser** — only verified via Rust unit tests
- [ ] **Enhancements** not applied to UnitState (received but ignored)
- [ ] **Wargear options** not applied

---

## Boarding Actions Pipeline Status

### What IS wired end-to-end (verified)
1. **Unit pipeline**: Army builder → roster serialization → WASM → UnitState/ModelState → GameState
2. **Map pipeline**: Mission ID → load_mission_map() → Board::boarding_actions() → GameState.board
3. **View projection**: BoardView transmits walls, compartments, hatchways (with state), entry zones, objectives
4. **Frontend rendering**: WallRenderer.ts, HatchwayRenderer.ts draw real geometry with state colors
5. **Deployment validation**: Entry zone boundary checks during unit placement
6. **Hatchway operations**: Full state machine with range, roll-off, and split-unit logic

### What IS NOT wired
1. Mission-specific trigger mechanics (lighting, radiation, venting, corruption)
2. Full mission-specific scoring beyond basic progressive control
3. Asymmetric mission layouts (BA-1 through BA-6)
4. Enhancement/wargear application to UnitState
5. Browser-tested end-to-end flow

---

## Previous Completed Work

#### Source-Linked Rules Tests (2026-03-16) COMPLETE
- [x] 713 source-linked rules tests across 23 files

#### Wire BoardingFeatures into NNUE Pipeline (2026-03-15) COMPLETE
- [x] BA feature constants 1203-1208, FEATURE_SCHEMA_VERSION 2, TOTAL_FEATURES 1209

#### Phase 16: Boarding Actions (2026-03-15) COMPLETE
- [x] Hatchways, tactical manoeuvres, missions, 6 factions, army builder UI

#### Phase 14: Rules Compliance (2026-03-14) COMPLETE
- [x] Combat resolution, scoring, Perturabo pipeline

#### Phase 12: Free Click-to-Move (2026-03-13) COMPLETE
- [x] Free click-to-move/deploy, mobile responsive, per-model rendering

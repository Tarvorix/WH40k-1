## Core Behavior

1. **Listen to all instructions** - Read the user's request carefully. Understand exactly what they are asking before doing anything.
2. **Only do what is asked** - Do NOT modify code that wasn't explicitly requested. Do NOT "fix" things that aren't broken. Do NOT make improvements or refactors unless specifically asked.
3. **Explain before making changes** - Before editing any file, explain what you plan to change and why. Wait for confirmation if the change is significant.
4. **Double check all work** - After making changes, verify they are correct. Re-read the original request to confirm you addressed what was actually asked, not what you assumed.
5. **Complete all tasks fully** - Do not simplify, Do not use placeholders, Complete all tasks fully and completely.
6. **When commiting and pushing to Github** - Commiter and Author should be Tarvorix...no mention of Claude anywhere in any commit message.
7. **No Placeholder Code** - Absolutely no placeholder code everything must be implemented fully
8. **No Simplification** - Absolutely no simplification of code everything must be implemented fully
9. **Preserve all existing functionality** - Never remove functionality from code

## File Deletion Rules

- NEVER delete any file or directory without explicit user confirmation
- Before ANY rm, rm -rf, or delete operation: list exactly what will be deleted and ask "Should I delete these? (yes/no)"
- Wait for explicit "yes" before proceeding
- No exceptions

## Before Any Code Change

Ask yourself:
1. Did the user explicitly ask for this change?
2. Is this code actually broken, or am I assuming?
3. Will this change affect other working functionality?
4. Have I explained what I'm about to do?

## If Uncertain

ASK. Do not guess. Do not assume. Ask the user to clarify.

## Git Author Configuration

All commits must use Tarvorix as author and committer. No mention of "Claude" anywhere in git commits.

Author name: Tarvorix
Committer name: Tarvorix
Never use "Claude" or any variation in commit author, committer, or commit messages or email
Configure git before committing:
git config user.name "Tarvorix"
git config user.email "Tarvorix@users.noreply.github.com"

## Project Overview
- Develop WH40k Digital — rules-accurate Warhammer 40,000 engine with Stockfish-style search, NNUE evaluation, and TypeScript web frontend
- Use implementation_v3.md, architecture.md, mcp_api.md, repo_scaffold.md, rule_coverage_matrix.md as primary planning documents
- AI Primer: AI_Primer.md — complete AI subsystem reference (search, MCTS, NNUE, self-play)

## Rules Documents (Authoritative Sources)
- Primary Rules: 40k_revised.md (core 40K rules), CP_Rules.md (Combat Patrol overlay)
- Combat Patrol Factions: Frenzied_Reavers.md (World Eaters), Custodes.md (Adeptus Custodes)
- Boarding Actions Rules: boarding_actions_complete_v3.md, boarding_patrol_missions_complete_v3.md
- Boarding Actions Data: boarding_actions_maps_complete_v3.json, boarding_actions_objectives_complete_v3.json, boarding_actions_mission_tags_complete_v3.json
- Boarding Actions Integration: boarding_actions_integration_rust.md
- Boarding Actions Factions (6 detachments): Astra_Militarum_BA_Tempestus_Boarding_Regiment.md, World_Eaters_BA_Boarding_Butchers.md, World_Eaters_BA_Skullsworn.md, Chaos_Space_Marines_BA_Champions_of_Chaos.md, Chaos_Space_Marines_BA_Underdeck_Uprising.md, Space_Marines_BA_Terminator_Assault.md

## Project Structure (per implementation_v3.md)
- `engine/` — Rust workspace with 34 crates (game_core, eval_nnue, selfplay, wasm_api, etc.)
- `engine/crates/rules_tests/` — 713 source-linked rules tests across 23 files (also symlinked at `tests/rules/`)
- `web/` — TypeScript/React frontend with Zustand stores, WASM bridge, web worker
- `python/train_nnue/` — Perturabo NNUE training pipeline (model.py, train.py, shard_loader.py, export_weights.py)
- `python/train_policy_value/` — Alpharius policy/value MCTS training (policy_value_model.py, mcts_train.py)
- `python/selfplay_tools/` — selfplay utilities (planned)
- `python/analysis/` — analysis scripts (planned)
- `content/` — game data (faction JSON, boarding actions JSON)
- `tests/rules/` — symlink to engine/crates/rules_tests/tests/ (source-linked rules tests)

## AI Architecture
- **Perturabo** (Stockfish-style): Iterative deepening + NNUE evaluation. Training in python/train_nnue/
- **Alpharius** (AlphaZero-style): MCTS + policy/value dual-head network. Training in python/train_policy_value/
- TOTAL_FEATURES = 1209 (1203 base + 6 BA features at indices 1203-1208)
- ACTION_VOCAB_SIZE = 640 (40 intents x 16 unit slots)
- FEATURE_SCHEMA_VERSION = 2

## Game Modes
- **Combat Patrol**: 44"x30" board, 2 factions (Custodes, World Eaters), 6 missions, prebuilt rosters
- **Boarding Actions**: 48"x28" board (two 24"x28" boards side by side), 6 faction/detachments, 500pt army builder, hatchways, tactical manoeuvres, 15+ missions

## Test Coverage
- 2,284 total workspace tests (713 source-linked rules tests + 1,571 inline tests)
- Rules tests verify against 40k_revised.md, CP_Rules.md, and boarding_actions_complete_v3.md
- rule_coverage_matrix.md tracks test-to-rule traceability
- Run rules tests: `cargo test -p wh40k_rules_tests`
- Run all tests: `cargo test --workspace --exclude wh40k_trainer_bridge`

## Project Rules
- Always create plan
- Always write plan to todo.md
- Update todo.md after every change
- ALWAYS check implementation_v3.md for intended file locations before creating files
- ALWAYS read source docs before claiming something doesn't exist
- NEVER fabricate game content — read the authoritative rules documents first

# Repo Scaffold

## Workspace layout
- `/docs`
- `/engine`
- `/engine/crates/game_core`
- `/engine/crates/rules_content`
- `/engine/crates/combat_patrol_rules`
- `/engine/crates/search_core`
- `/engine/crates/eval_heuristic`
- `/engine/crates/eval_nnue`
- `/engine/crates/mcp_server`
- `/engine/crates/server_api`
- `/engine/crates/replay`
- `/engine/crates/wasm_api`
- `/web`
- `/python` (future training bridge)
- `/tests`

## First milestones
1. Headless runtime that can load Combat Patrol scenarios.
2. Legal action generation and replay logging.
3. TypeScript GUI connected to authoritative runtime.
4. MCP server exposing game sessions and legal actions.
5. Heuristic bot.
6. Stockfish-like search core.
7. NNUE runtime integration.
8. AlphaGo-style training bridge later.

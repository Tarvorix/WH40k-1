# Architecture

Authoritative sources:
- `40k_revised.md`
- `CP_Rules.md`

## Core principle
One authoritative headless runtime resolves all legality, randomness, effects, scoring, and observations. GUI, AI, tests, replays, and MCP all use the same runtime.

## Layers
1. Rules content layer: parsed rules, Combat Patrol overlays, factions, missions, traceability metadata.
2. Domain model: game state, players, units, models, weapons, abilities, objectives, turn/phase state, effects, reserves, transports.
3. Authoritative runtime: legal action generation, validation, resolution, triggered windows, scoring, replay log.
4. Search/eval layer: action abstraction, heuristic eval, NNUE eval runtime, transposition table, alpha-beta/PVS, tactical extensions, stochastic handling.
5. Interface layer: CLI, server API, WASM bridge, MCP server, replay tools.
6. Presentation layer: TypeScript sprite-capable GUI.

## Mandatory products
- Headless engine
- Browser/mobile-capable TypeScript GUI
- MCP server for external agents
- Replay/trace system
- Rule coverage matrix
- AI search engine with NNUE-ready evaluator path
- AlphaGo-style training bridge later

# MCP API

## Purpose
Expose the authoritative 40K engine to ChatGPT, Claude, Gemini, or other MCP-capable agents.

## Principles
- MCP never bypasses legality.
- Engine remains authoritative.
- Tools operate on public/private observations.
- All state mutations occur through validated actions.

## Required tools
- `game.create_session`
- `game.load_scenario`
- `game.get_observation`
- `game.list_legal_actions`
- `game.apply_action`
- `game.step_until_decision`
- `game.get_replay_log`
- `game.get_score`
- `game.reset`
- `game.end_session`

## Observation model
- Public observation: board state, public scoring, public effects, round/phase, legal choices for requesting side.
- Private observation: hidden/reserve/private handoff info if applicable.
- Debug/admin observation: tests only.

## Agent flow
1. Create or join session.
2. Request observation.
3. Request legal actions.
4. Choose action.
5. Apply action.
6. Engine resolves until next decision.
7. Repeat until game end.

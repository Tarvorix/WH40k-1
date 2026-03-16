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
- Develop WH40k Digital 
- Use implementation_v3.md, architecture.md, mcp_api.md, repo_scaffold.md, rules_coverage_matrix.md as primary planning documents.
- Primary Rules Documents: 40k_revised.md and CP_Rules.md
- Inital Combat Patrol Factions: Frenzied_Reavers.md and Custodes.md
- Boarding Actions Rules/Map Documents: boarding_actions_complete_v3.md, boarding_patrol_missions_complete_v3.md, boarding_actions_maps_complete_v3.json, boarding_actions_objectives_complete_v3.json and boarding_actions_mission_tags_complete_v3.json
- Boarding Actions Integration Document: boarding_actions_integration_rust.md
- Boarding Actions Factions/Detachment Rules and Datasheets: Astra_Militarum_BA_Tempestus_Boarding_Regiment.md, World_Eaters_BA_Boarding_Butchers.md, World_Eaters_BA_Skullsworn.md, Chaos_Space_Marines_BA_Champions_of_Chaos.md, Chaos_Space_Marines_BA_Underdeck_Uprising.md, Space_Marines_BA_Terminator_Assault.md

## Project Rules
- Always create plan
- Always write plan to todo.md
- Update todo.md after every change

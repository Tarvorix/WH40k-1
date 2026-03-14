# Lessons Learned — Engine Bug Patterns

Reference this document when adding new factions, phases, or game mechanics to avoid repeating past bugs.

---

## 1. ActionGenerator Must Track "Already Done" State

**Bug**: AI loops infinitely by generating the same action over and over.

**Root cause**: The ActionGenerator generates candidates without checking if the action was already performed this turn/phase.

**Examples hit**:
- `DeclareCharge` generated for a unit that already declared a charge (missing `declared_charge_targets` filter)
- `ChooseKaTahStance` generated for a unit that already chose a stance (missing `ka_tah_stances` check)
- `ChooseVaultswordsProfile` generated for a model that already chose a profile (missing `vaultswords_profiles` check)

**Rule**: Every action that records state in `TurnFlags` (or similar) must have a corresponding filter in the ActionGenerator. Before generating a candidate, check if the unit/model has already performed that action.

**Checklist for new faction abilities**:
- [ ] Does this ability store a choice in TurnFlags? → Add a filter in the generator
- [ ] Does this ability modify unit state (engagement, status)? → Check that state in the generator
- [ ] Can this ability be performed multiple times? If not → Filter already-performed units

---

## 2. ActionGenerator Candidates Must Pass Validation

**Bug**: AI picks an action that fails the validator, state doesn't change, same action generated next iteration → infinite loop.

**Root cause**: The ActionGenerator generates candidates based on approximate checks (e.g., reference position for ER), but the CommandValidator uses precise checks (e.g., individual model positions). Candidates that pass the generator's loose check fail the validator's strict check.

**Examples hit**:
- `MakeChargeMove` destination checked ER against target's reference position in generator, but validator checks against all individual target models
- `MakeChargeMove` generator didn't check non-target enemy proximity, but validator rejects moves ending in ER of non-target enemies
- Generator computed destinations that would move models off the board

**Fix implemented**: Generic pre-validation filter at the end of `ActionGenerator::generate()`:
```rust
candidates.candidates.retain(|action| {
    action.commands.iter().all(|cmd| {
        CommandValidator::validate(state, cmd).is_legal()
    })
});
```

**Rule**: Always keep the pre-validation filter. It catches mismatches between generator heuristics and validator rules. When adding new commands:
- [ ] Write the validator FIRST
- [ ] The generator can use approximate heuristics for candidate generation
- [ ] The pre-validation filter will catch any mismatches
- [ ] Don't remove the filter for performance — correctness > speed

---

## 3. Phase Sub-Flows Must Be Explicit

**Bug**: The charge phase requires a multi-step flow (Declare → Roll → Move) but the original ActionGenerator only generated DeclareCharge candidates, with no handling of the subsequent steps.

**Root cause**: The ActionGenerator treated each phase as a single decision point. Complex phases with mandatory sub-steps (declare targets, roll dice, move) need explicit sub-flow detection.

**Fix**: The `generate()` function now checks sub-phase state BEFORE falling through to phase-specific generation:
1. Check reaction windows → generate Decline/UseOverwatch
2. Check pending charge rolls → generate ResolveChargeRoll
3. Check pending charge moves → generate MakeChargeMove
4. Fall through to normal phase generation

**Rule**: When adding new multi-step mechanics:
- [ ] Define the sub-steps explicitly (e.g., DeclareCharge → ResolveChargeRoll → MakeChargeMove)
- [ ] Add state tracking for each step (declared_charge_targets, charge_roll_results, charged_this_turn)
- [ ] Add sub-flow detection at the TOP of `generate()`, before phase-specific generation
- [ ] Each sub-step should return early so the AI resolves it before generating new actions

---

## 4. Dice Rolls Must Be Random, Not AI Choices

**Bug**: The AI "chose" its own charge roll value from [3, 5, 7, 9, 11], always picking the lowest (safest) value. Charges never succeeded.

**Root cause**: Modeling dice rolls as AI decisions lets the AI game the system by always choosing favorable outcomes.

**Fix**: Use `roll: 0` as a sentinel value. The ActionGenerator generates a single "roll charge" candidate per unit. The executor detects `roll == 0` and uses `state.dice_roller.roll_2d6()` for actual random dice.

**Rule**: Dice-based mechanics should NEVER let the AI choose the roll value:
- [ ] Generate ONE candidate for the roll action (not multiple roll values)
- [ ] Use a sentinel (0) to signal "roll actual dice"
- [ ] The executor rolls dice using the state's DiceRoller
- [ ] Update the validator to accept the sentinel value
- [ ] For search evaluation: the search clones state and rolls on the clone, giving natural randomness

---

## 5. EndPhase Must Be Gated During Mandatory Phases

**Bug**: AI ended the PreBattle phase before deploying all units, leaving them permanently undeployed.

**Root cause**: `generate_phase_control_candidates()` unconditionally added `EndPhase` for every phase, including PreBattle where deployment is mandatory.

**Fix**: Block EndPhase during PreBattle if any player still has undeployed units:
```rust
if state.current_phase == Phase::PreBattle {
    let any_undeployed = state.units.iter().any(|u| {
        u.status == UnitStatus::Undeployed
    });
    if any_undeployed { return; }
}
```

**Rule**: When adding new mandatory phases or sub-phases:
- [ ] Determine the exit condition (when is it valid to end?)
- [ ] Gate EndPhase on that condition in `generate_phase_control_candidates`
- [ ] Also gate in the validator for defense-in-depth

---

## 6. Scoring Must Happen Once Per Player Per Round

**Bug**: Both players scored at every Command Phase ending, but there are two Command Phases per round (one per player). Result: 2x VP inflation.

**Root cause**: Scoring functions iterated over both players (`for player_idx in 0..2`) instead of scoring only the active player.

**Fix**: All scoring functions now use `state.active_player` to score only the player whose Command Phase is ending. The `players_to_score()` helper returns only the active player for BR2-4.

**Rule**: When adding new scoring triggers:
- [ ] Score only the active player at end of their Command Phase
- [ ] For BR5 split timing: 1st player at EndOfCommandPhase, 2nd player at EndOfTurn
- [ ] Never iterate both players in a per-round scoring function
- [ ] Test with both faction orderings (P0 first, P1 first)

---

## 7. Selfplay Loop Needs Safety Detection

**Bug**: Selfplay games ran to 10,000 commands when stuck in loops, wasting compute and producing garbage training data.

**Fix**: Three layers of defense:
1. **State hash detection**: Break if game state hash unchanged for 5 iterations
2. **Repeated action detection**: Break if same action label chosen 5 times consecutively
3. **Command limit**: Hard cap at 10,000 commands (MAX_COMMANDS_PER_GAME)

**Rule**: Any game loop (selfplay, benchmark, web UI) needs:
- [ ] Maximum iteration/command count
- [ ] Repeated-action detection (same action N times → break)
- [ ] State-hash stagnation detection (no state change → break)
- [ ] Error messages with phase/round context for debugging

---

## Quick Reference: Adding a New Faction

When implementing a new faction's Combat Patrol, verify:

1. **All unit abilities that store choices** → Filter in ActionGenerator + clear in TurnFlags
2. **All dice-based abilities** → Use DiceRoller, not AI choice
3. **All multi-step sequences** → Explicit sub-flow in ActionGenerator
4. **All phase-specific actions** → Pre-validated by generic filter
5. **All scoring mechanics** → Score active player only, test with both player orderings
6. **Run 50-game selfplay** → Must be 50/50 completion, 0 errors

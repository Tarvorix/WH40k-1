//! Effect state types for tracking active modifiers, abilities, and temporary effects.
//!
//! Source: implementation_v3.md Section 8.5
//! Source: 40k_revised.md - Abilities, Stratagems, Enhancements
//! Source: Custodes.md - Ka'tah, Enhancements
//! Source: Frenzied_Reavers.md - Blessings of Khorne

use serde::{Deserialize, Serialize};

use wh40k_core_types::{
    AbilityId, BattleRound, EffectDuration, EnhancementId, Phase, StackingBehavior, StratagemId,
    UnitId, ModelId, PlayerId,
};

// ---------------------------------------------------------------------------
// ActiveEffect
// ---------------------------------------------------------------------------

/// An active effect currently applied in the game.
///
/// Effects come from abilities, stratagems, enhancements, blessings, ka'tah stances,
/// and other sources. Each effect has a source, target, type, duration, and stacking
/// behavior.
///
/// Source: implementation_v3.md Section 8.5
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActiveEffect {
    /// Unique identifier for this effect instance.
    pub id: u32,

    /// What generated this effect.
    pub source: EffectSource,

    /// What this effect applies to.
    pub target: EffectTarget,

    /// What this effect does.
    pub effect_type: EffectType,

    /// How long this effect lasts.
    pub duration: EffectDuration,

    /// How this effect stacks with other instances.
    pub stacking: StackingBehavior,

    /// The battle round when this effect was applied.
    pub applied_round: BattleRound,

    /// The phase when this effect was applied.
    pub applied_phase: Phase,
}

impl ActiveEffect {
    /// Check if this effect has expired given the current round and phase.
    ///
    /// An effect is expired when:
    /// - UntilEndOfPhase: current phase is later than applied phase, or round advanced
    /// - UntilEndOfTurn: round advanced (player turn ended)
    /// - UntilEndOfBattleRound: battle round advanced
    /// - UntilStartOfNextCommandPhase: next command phase reached
    /// - ForBattleRounds(n): applied_round + n has passed
    /// - Persistent: never expires
    /// - UntilEndOfAttackSequence / UntilEndOfFightSequence: checked externally
    pub fn is_expired(&self, current_round: BattleRound, current_phase: Phase) -> bool {
        match self.duration {
            EffectDuration::UntilEndOfPhase => {
                // Expired if we're in a later phase or a later round
                if current_round > self.applied_round {
                    return true;
                }
                if current_round == self.applied_round && current_phase > self.applied_phase {
                    return true;
                }
                false
            }
            EffectDuration::UntilEndOfTurn => {
                // Expired when the round advances (turn has ended)
                current_round > self.applied_round
            }
            EffectDuration::UntilEndOfBattleRound => {
                // Expired when the battle round advances
                current_round > self.applied_round
            }
            EffectDuration::UntilStartOfNextCommandPhase => {
                // Expired when the next command phase starts
                if current_round > self.applied_round && current_phase >= Phase::Command {
                    return true;
                }
                false
            }
            EffectDuration::UntilEndOfAttackSequence => {
                // Managed externally by combat pipeline; consider not expired here
                false
            }
            EffectDuration::UntilEndOfFightSequence => {
                // Managed externally by combat pipeline; consider not expired here
                false
            }
            EffectDuration::Persistent => false,
            EffectDuration::ForBattleRounds(n) => {
                current_round.number() > self.applied_round.number() + n
            }
        }
    }
}

/// Remove expired effects from the active effects list.
///
/// Called during phase transitions to clean up effects that have expired.
pub fn tick_effects(
    effects: &mut Vec<ActiveEffect>,
    current_round: BattleRound,
    current_phase: Phase,
) {
    effects.retain(|e| !e.is_expired(current_round, current_phase));
}

// ---------------------------------------------------------------------------
// EffectSource
// ---------------------------------------------------------------------------

/// The origin of an active effect.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectSource {
    /// From a unit or model ability.
    Ability(AbilityId),

    /// From a stratagem.
    Stratagem(StratagemId),

    /// From an enhancement.
    Enhancement(EnhancementId),

    /// From Blessings of Khorne.
    Blessing(String),

    /// From a Ka'tah stance.
    KaTah(String),

    /// From a core rule (e.g., battle-shock, cover).
    CoreRule(String),

    /// From a mission rule.
    MissionRule(String),

    /// Custom source (escape hatch for irregular rules).
    Custom(String),
}

// ---------------------------------------------------------------------------
// EffectTarget
// ---------------------------------------------------------------------------

/// What an effect applies to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectTarget {
    /// Applies to a specific unit.
    Unit(UnitId),

    /// Applies to a specific model.
    Model(ModelId),

    /// Applies to all units of a player.
    Player(PlayerId),

    /// Applies to all units (global, both players).
    Global,
}

// ---------------------------------------------------------------------------
// EffectType
// ---------------------------------------------------------------------------

/// What an effect does. Comprehensive enum covering all game modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectType {
    // === Stat modifiers ===
    /// Modify the unit's movement characteristic by a delta (in mils).
    ModifyMovement(i32),

    /// Modify the unit's toughness by a delta.
    ModifyToughness(i8),

    /// Modify the unit's armor save by a delta (positive = worse, negative = better).
    ModifyArmorSave(i8),

    /// Modify the unit's leadership by a delta.
    ModifyLeadership(i8),

    /// Modify the unit's OC by a delta.
    ModifyOC(i8),

    /// Modify weapon strength by a delta.
    ModifyStrength(i8),

    /// Modify weapon AP by a delta.
    ModifyAP(i8),

    /// Modify weapon damage by a delta.
    ModifyDamage(i8),

    /// Modify number of attacks by a delta.
    ModifyAttacks(i8),

    /// Modify hit roll by a delta.
    ModifyHitRoll(i8),

    /// Modify wound roll by a delta.
    ModifyWoundRoll(i8),

    /// Modify save roll by a delta.
    ModifySaveRoll(i8),

    // === Ability grants ===
    /// Grant Feel No Pain at the given value.
    GrantFeelNoPain(u8),

    /// Grant an invulnerable save at the given value (e.g., 6 = 6++).
    /// Source: 40k_revised.md - Go to Ground grants 6++ invulnerable save
    GrantInvulnerableSave(u8),

    /// Grant the Fights First ability.
    GrantFightsFirst,

    /// Grant the ability to ignore cover.
    GrantIgnoresCover,

    /// Grant Benefit of Cover.
    GrantBenefitOfCover,

    /// Grant the ability to re-roll hit rolls (all or failed only).
    GrantRerollHits { all: bool },

    /// Grant the ability to re-roll wound rolls (all or failed only).
    GrantRerollWounds { all: bool },

    /// Grant the ability to re-roll charge rolls.
    GrantRerollCharge,

    /// Grant the ability to re-roll advance rolls.
    GrantRerollAdvance,

    /// Grant Sustained Hits N.
    GrantSustainedHits(u8),

    /// Grant Lethal Hits.
    GrantLethalHits,

    /// Grant critical hits on N+ (instead of 6+).
    GrantCriticalHitOn(u8),

    /// Grant critical wounds on N+ (instead of 6+).
    GrantCriticalWoundOn(u8),

    /// Grant Fight on Death (roll D6, succeed on N+).
    GrantFightOnDeath(u8),

    // === Restrictions ===
    /// Unit cannot use Overwatch.
    PreventOverwatch,

    /// Unit cannot Fall Back.
    PreventFallBack,

    /// Unit cannot be targeted by shooting unless closest.
    LoneOperative,

    /// Unit cannot be targeted unless within 12" (Stealth).
    Stealth,

    // === Scoring and mission ===
    /// Force a battle-shock test.
    ForceTest,

    /// Set battle-shocked status directly.
    SetBattleShocked(bool),

    /// Modify VP scored by a player.
    ModifyVP(i16),

    // === Re-rolls ===
    /// Command Re-roll: permission to re-roll one hit, wound, save, or damage roll.
    /// Source: 40k_revised.md §15.2 - "Command Re-roll"
    CommandReroll,

    // === Custom ===
    /// Custom effect (escape hatch for irregular rules).
    Custom(String),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_effect(duration: EffectDuration, round: u8, phase: Phase) -> ActiveEffect {
        ActiveEffect {
            id: 1,
            source: EffectSource::Stratagem(StratagemId::new(1)),
            target: EffectTarget::Unit(UnitId::new(0)),
            effect_type: EffectType::ModifyHitRoll(1),
            duration,
            stacking: StackingBehavior::Additive,
            applied_round: BattleRound::new(round),
            applied_phase: phase,
        }
    }

    #[test]
    fn test_effect_until_end_of_phase_not_expired() {
        let effect = make_test_effect(EffectDuration::UntilEndOfPhase, 1, Phase::Shooting);
        assert!(!effect.is_expired(BattleRound::new(1), Phase::Shooting));
    }

    #[test]
    fn test_effect_until_end_of_phase_expired_next_phase() {
        let effect = make_test_effect(EffectDuration::UntilEndOfPhase, 1, Phase::Shooting);
        assert!(effect.is_expired(BattleRound::new(1), Phase::Charge));
    }

    #[test]
    fn test_effect_until_end_of_phase_expired_next_round() {
        let effect = make_test_effect(EffectDuration::UntilEndOfPhase, 1, Phase::Shooting);
        assert!(effect.is_expired(BattleRound::new(2), Phase::Command));
    }

    #[test]
    fn test_effect_until_end_of_turn_not_expired() {
        let effect = make_test_effect(EffectDuration::UntilEndOfTurn, 1, Phase::Movement);
        assert!(!effect.is_expired(BattleRound::new(1), Phase::Fight));
    }

    #[test]
    fn test_effect_until_end_of_turn_expired() {
        let effect = make_test_effect(EffectDuration::UntilEndOfTurn, 1, Phase::Movement);
        assert!(effect.is_expired(BattleRound::new(2), Phase::Command));
    }

    #[test]
    fn test_effect_until_end_of_battle_round() {
        let effect = make_test_effect(EffectDuration::UntilEndOfBattleRound, 1, Phase::Command);
        assert!(!effect.is_expired(BattleRound::new(1), Phase::Fight));
        assert!(effect.is_expired(BattleRound::new(2), Phase::Command));
    }

    #[test]
    fn test_effect_until_start_of_next_command_phase() {
        let effect = make_test_effect(
            EffectDuration::UntilStartOfNextCommandPhase,
            1,
            Phase::Shooting,
        );
        // Same round, later phase - not expired
        assert!(!effect.is_expired(BattleRound::new(1), Phase::Fight));
        // Next round, before command - not expired
        assert!(!effect.is_expired(BattleRound::new(2), Phase::PreBattle));
        // Next round, at command - expired
        assert!(effect.is_expired(BattleRound::new(2), Phase::Command));
    }

    #[test]
    fn test_effect_persistent_never_expires() {
        let effect = make_test_effect(EffectDuration::Persistent, 1, Phase::Command);
        assert!(!effect.is_expired(BattleRound::new(1), Phase::Fight));
        assert!(!effect.is_expired(BattleRound::new(5), Phase::Fight));
    }

    #[test]
    fn test_effect_for_battle_rounds() {
        let effect = make_test_effect(EffectDuration::ForBattleRounds(2), 1, Phase::Command);
        assert!(!effect.is_expired(BattleRound::new(1), Phase::Fight));
        assert!(!effect.is_expired(BattleRound::new(2), Phase::Fight));
        assert!(!effect.is_expired(BattleRound::new(3), Phase::Fight));
        assert!(effect.is_expired(BattleRound::new(4), Phase::Command));
    }

    #[test]
    fn test_tick_effects_removes_expired() {
        let mut effects = vec![
            make_test_effect(EffectDuration::UntilEndOfPhase, 1, Phase::Shooting),
            make_test_effect(EffectDuration::Persistent, 1, Phase::Command),
            make_test_effect(EffectDuration::UntilEndOfTurn, 1, Phase::Movement),
        ];

        // At round 1, Charge phase: the shooting-phase effect should be removed
        tick_effects(&mut effects, BattleRound::new(1), Phase::Charge);
        assert_eq!(effects.len(), 2);

        // At round 2: the turn effect should also be removed
        tick_effects(&mut effects, BattleRound::new(2), Phase::Command);
        assert_eq!(effects.len(), 1);

        // Persistent effect stays
        assert_eq!(effects[0].duration, EffectDuration::Persistent);
    }

    #[test]
    fn test_effect_attack_sequence_managed_externally() {
        let effect = make_test_effect(
            EffectDuration::UntilEndOfAttackSequence,
            1,
            Phase::Shooting,
        );
        // Never expires through tick - managed externally
        assert!(!effect.is_expired(BattleRound::new(5), Phase::Fight));
    }

    #[test]
    fn test_effect_source_variants() {
        let _ability = EffectSource::Ability(AbilityId::new(1));
        let _strat = EffectSource::Stratagem(StratagemId::new(1));
        let _enh = EffectSource::Enhancement(EnhancementId::new(1));
        let _blessing = EffectSource::Blessing("Total Carnage".to_string());
        let _katah = EffectSource::KaTah("Dacatarai".to_string());
        let _core = EffectSource::CoreRule("battle-shock".to_string());
        let _mission = EffectSource::MissionRule("hold objective".to_string());
        let _custom = EffectSource::Custom("special".to_string());
    }

    #[test]
    fn test_effect_type_variants() {
        let _mv = EffectType::ModifyMovement(2000); // +2 inches in mils
        let _t = EffectType::ModifyToughness(1);
        let _fnp = EffectType::GrantFeelNoPain(5);
        let _ff = EffectType::GrantFightsFirst;
        let _rh = EffectType::GrantRerollHits { all: false };
        let _rw = EffectType::GrantRerollWounds { all: true };
        let _sh = EffectType::GrantSustainedHits(1);
        let _ch = EffectType::GrantCriticalHitOn(5);
        let _fod = EffectType::GrantFightOnDeath(4);
        let _custom = EffectType::Custom("special rule".to_string());
    }

    #[test]
    fn test_effect_target_variants() {
        let _unit = EffectTarget::Unit(UnitId::new(1));
        let _model = EffectTarget::Model(ModelId::new(1));
        let _player = EffectTarget::Player(PlayerId::new(0));
        let _global = EffectTarget::Global;
    }

    #[test]
    fn test_effect_serialization() {
        let effect = make_test_effect(EffectDuration::UntilEndOfPhase, 1, Phase::Shooting);
        let json = serde_json::to_string(&effect).unwrap();
        let back: ActiveEffect = serde_json::from_str(&json).unwrap();
        assert_eq!(effect, back);
    }
}

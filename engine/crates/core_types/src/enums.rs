//! Game state enums for phase progression, unit status, and game flow.
//!
//! Source: implementation_v3.md Section 6.1, 8.1
//! Source: 40k_revised.md - Phase structure
//! Source: CP_Rules.md - Combat Patrol format

use serde::{Deserialize, Serialize};

/// The five phases of a 40K battle round, plus pre-battle and end states.
/// Source: 40k_revised.md - "THE BATTLE ROUND" section
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Phase {
    /// Pre-battle setup (deployment, declarations)
    PreBattle,
    /// Command Phase: gain CP, test battle-shock
    Command,
    /// Movement Phase: move units, reinforcements arrive
    Movement,
    /// Shooting Phase: ranged attacks
    Shooting,
    /// Charge Phase: declare and resolve charges
    Charge,
    /// Fight Phase: melee combat
    Fight,
    /// Game has ended
    GameEnd,
}

impl Phase {
    /// Returns the next phase in sequence within a turn.
    /// Returns None if at GameEnd.
    pub fn next(self) -> Option<Phase> {
        match self {
            Phase::PreBattle => Some(Phase::Command),
            Phase::Command => Some(Phase::Movement),
            Phase::Movement => Some(Phase::Shooting),
            Phase::Shooting => Some(Phase::Charge),
            Phase::Charge => Some(Phase::Fight),
            Phase::Fight => None, // End of player turn, next player starts at Command
            Phase::GameEnd => None,
        }
    }

    /// Whether this phase involves active gameplay (not setup or end).
    pub fn is_active(self) -> bool {
        matches!(
            self,
            Phase::Command
                | Phase::Movement
                | Phase::Shooting
                | Phase::Charge
                | Phase::Fight
        )
    }
}

/// Sub-phases within each main phase for fine-grained timing.
/// Source: implementation_v3.md Section 8.1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubPhase {
    // Pre-battle sub-phases
    DetermineAttackerDefender,
    SelectEnhancements,
    SelectSecondaries,
    SelectPatrolSquad,
    Deployment,
    DetermineFirstTurn,

    // Command Phase sub-phases
    CommandPhaseStart,
    GainCommandPoints,
    BattleShockTests,
    CommandPhaseEnd,

    // Movement Phase sub-phases
    MovementPhaseStart,
    SelectUnitToMove,
    ResolveMovement,
    Reinforcements,
    MovementPhaseEnd,

    // Shooting Phase sub-phases
    ShootingPhaseStart,
    SelectUnitToShoot,
    SelectTargets,
    ResolveAttacks,
    ShootingPhaseEnd,

    // Charge Phase sub-phases
    ChargePhaseStart,
    SelectUnitToCharge,
    DeclareChargeTargets,
    ResolveOverwatch,
    RollChargeDistance,
    MakeChargeMove,
    ChargePhaseEnd,

    // Fight Phase sub-phases
    FightPhaseStart,
    FightsFirst,
    RemainingCombats,
    PileIn,
    ResolveMeleeAttacks,
    Consolidate,
    FightPhaseEnd,

    // Generic
    ReactionWindow,
    StratagemWindow,
    EffectResolution,
}

/// Battle round number (1-5 for Combat Patrol).
/// Source: CP_Rules.md - "The battle lasts five battle rounds"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BattleRound(pub u8);

impl BattleRound {
    pub const MIN: BattleRound = BattleRound(1);
    pub const MAX: BattleRound = BattleRound(5);

    pub const fn new(round: u8) -> Self {
        Self(round)
    }

    pub const fn number(self) -> u8 {
        self.0
    }

    pub fn next(self) -> Option<BattleRound> {
        if self.0 < 5 {
            Some(BattleRound(self.0 + 1))
        } else {
            None
        }
    }

    pub fn is_valid(self) -> bool {
        self.0 >= 1 && self.0 <= 5
    }
}

/// Which player's turn is currently active.
/// Source: 40k_revised.md - "each battle round, both players have a turn"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayerRole {
    /// The player who goes first (Attacker in CP deployment)
    Attacker,
    /// The player who goes second (Defender in CP deployment)
    Defender,
}

impl PlayerRole {
    pub fn opponent(self) -> PlayerRole {
        match self {
            PlayerRole::Attacker => PlayerRole::Defender,
            PlayerRole::Defender => PlayerRole::Attacker,
        }
    }
}

/// Movement actions a unit can take.
/// Source: 40k_revised.md - Movement Phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MovementAction {
    /// Normal move up to M characteristic
    NormalMove,
    /// Advance: move up to M + D6
    Advance,
    /// Remain Stationary (don't move)
    RemainStationary,
    /// Fall Back from engagement range
    FallBack,
    /// Arrive from Reserves/Deep Strike
    ArriveFromReserves,
}

/// Current status of a unit on the battlefield.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitStatus {
    /// On the battlefield, operational
    OnBattlefield,
    /// In strategic reserves
    InStrategicReserves,
    /// Destroyed and removed from play
    Destroyed,
    /// Set up but not yet deployed (pre-battle)
    Undeployed,
}

/// Outcome of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameOutcome {
    /// Game is still in progress
    InProgress,
    /// A player has won (by VP or concession)
    Victory(super::PlayerId),
    /// The game ended in a draw
    Draw,
}

/// How the game ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameEndReason {
    /// All 5 battle rounds completed
    RoundsComplete,
    /// One player has been tabled (all units destroyed)
    Tabled,
    /// A player conceded
    Concession,
}

/// Duration types for temporary effects.
/// Source: implementation_v3.md Section 8.5
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectDuration {
    /// Lasts until end of the current phase
    UntilEndOfPhase,
    /// Lasts until end of the current turn (player turn)
    UntilEndOfTurn,
    /// Lasts until end of the current battle round
    UntilEndOfBattleRound,
    /// Lasts until start of next Command Phase
    UntilStartOfNextCommandPhase,
    /// Lasts for the duration of an attack sequence
    UntilEndOfAttackSequence,
    /// Lasts until the unit finishes making its attacks
    UntilEndOfFightSequence,
    /// Permanent effect for the rest of the game
    Persistent,
    /// Lasts for a specific number of battle rounds
    ForBattleRounds(u8),
}

/// What kind of reaction window is currently open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReactionWindowType {
    /// Stratagem can be played
    Stratagem,
    /// Overwatch can be fired
    Overwatch,
    /// Heroic Intervention
    HeroicIntervention,
    /// Counter-Offensive fight
    CounterOffensive,
    /// Generic decision point
    Decision,
}

/// The side of the battlefield (for deployment, objectives, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BattlefieldEdge {
    /// Player's own deployment zone edge
    Own,
    /// Opponent's deployment zone edge
    Opponent,
    /// Left flank
    Left,
    /// Right flank
    Right,
}

/// Terrain cover types.
/// Source: 40k_revised.md - Terrain Features
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoverType {
    /// No cover
    None,
    /// Benefit of Cover (+1 to saving throw)
    BenefitOfCover,
}

/// Line of sight result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Visibility {
    /// Target is visible
    Visible,
    /// Target is not visible (LOS blocked)
    NotVisible,
}

/// Engagement status of a unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EngagementStatus {
    /// Not within engagement range of any enemy
    NotEngaged,
    /// Within engagement range of at least one enemy unit
    Engaged,
}

/// Whether a model/unit is alive or destroyed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AliveStatus {
    Alive,
    Destroyed,
}

/// Wound allocation priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AllocationStatus {
    /// No wounds allocated
    Unwounded,
    /// Has been allocated wounds but not destroyed
    WoundedAllocated,
}

/// Result of a saving throw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SaveResult {
    /// Save passed, no damage
    Saved,
    /// Save failed, damage applied
    Failed,
}

/// Type of save being used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SaveType {
    /// Normal armour save (modified by AP)
    Armour,
    /// Invulnerable save (not modified by AP)
    Invulnerable,
}

/// Result of a battle-shock test.
/// Source: 40k_revised.md - Battle-shock
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BattleShockResult {
    /// Test passed, unit is not battle-shocked
    Passed,
    /// Test failed, unit is battle-shocked
    Failed,
}

/// Stacking behavior for effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StackingBehavior {
    /// Multiple instances stack additively
    Additive,
    /// Only the best instance applies
    BestOnly,
    /// Does not stack, only one instance allowed
    Unique,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_sequence() {
        assert_eq!(Phase::PreBattle.next(), Some(Phase::Command));
        assert_eq!(Phase::Command.next(), Some(Phase::Movement));
        assert_eq!(Phase::Movement.next(), Some(Phase::Shooting));
        assert_eq!(Phase::Shooting.next(), Some(Phase::Charge));
        assert_eq!(Phase::Charge.next(), Some(Phase::Fight));
        assert_eq!(Phase::Fight.next(), None);
        assert_eq!(Phase::GameEnd.next(), None);
    }

    #[test]
    fn test_phase_is_active() {
        assert!(!Phase::PreBattle.is_active());
        assert!(Phase::Command.is_active());
        assert!(Phase::Movement.is_active());
        assert!(Phase::Shooting.is_active());
        assert!(Phase::Charge.is_active());
        assert!(Phase::Fight.is_active());
        assert!(!Phase::GameEnd.is_active());
    }

    #[test]
    fn test_battle_round() {
        let r1 = BattleRound::new(1);
        assert_eq!(r1.number(), 1);
        assert!(r1.is_valid());
        assert_eq!(r1.next(), Some(BattleRound::new(2)));

        let r5 = BattleRound::new(5);
        assert!(r5.is_valid());
        assert_eq!(r5.next(), None);

        let r0 = BattleRound::new(0);
        assert!(!r0.is_valid());

        let r6 = BattleRound::new(6);
        assert!(!r6.is_valid());
    }

    #[test]
    fn test_player_role_opponent() {
        assert_eq!(PlayerRole::Attacker.opponent(), PlayerRole::Defender);
        assert_eq!(PlayerRole::Defender.opponent(), PlayerRole::Attacker);
    }

    #[test]
    fn test_enum_serialization() {
        let phase = Phase::Shooting;
        let json = serde_json::to_string(&phase).unwrap();
        let deserialized: Phase = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, phase);
    }

    #[test]
    fn test_battle_round_ordering() {
        assert!(BattleRound::new(1) < BattleRound::new(2));
        assert!(BattleRound::new(3) < BattleRound::new(5));
    }
}

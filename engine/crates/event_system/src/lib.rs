//! WH40K Engine - Event System crate
//!
//! Provides the event bus for the 40K engine. Events are emitted during
//! game state transitions and can be subscribed to by rules, abilities,
//! stratagems, and other systems.
//!
//! Source: implementation_v3.md Section 6.5 (Layer E - Event system)
//!
//! ## Architecture
//!
//! - [`GameEvent`] - ~40 variant enum covering all game events
//! - [`EventBus`] - Central event handling: emit, subscribe, query, drain
//! - [`ReactionWindow`] - Pause point for player decisions
//! - [`EventFilter`] - Predicate for matching events
//! - [`EventHandler`] - Registered handler with ID, filter, and priority

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;
use thiserror::Error;

use wh40k_core_types::{
    BattleRound, BattleShockResult, EffectDuration, EventId, GameOutcome, IdGenerator,
    ModelId, MovementAction, ObjectiveId, Phase, PlayerId, Position, ReactionWindowType,
    SaveType, StratagemId, UnitId, WeaponId,
};

// ============================================================================
// Local ID and helper types not present in core_types
// ============================================================================

/// Identifies a registered event handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HandlerId(pub u32);

impl HandlerId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl From<u32> for HandlerId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl fmt::Display for HandlerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Handler({})", self.0)
    }
}

/// Identifies an active effect in the effect system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EffectId(pub u32);

impl EffectId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl From<u32> for EffectId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl fmt::Display for EffectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Effect({})", self.0)
    }
}

/// Ka'tah martial stance for Adeptus Custodes.
/// Source: Custodes.md - Martial Ka'tah
///
/// Each time a unit with this ability is selected to fight, choose one stance.
/// The selected stance is active until that unit finishes making its attacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KaTahStance {
    /// Dacatarai: Melee weapons have [SUSTAINED HITS 1]
    Dacatarai,
    /// Rendax: Melee weapons have [LETHAL HITS]
    Rendax,
}

impl fmt::Display for KaTahStance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KaTahStance::Dacatarai => write!(f, "Dacatarai"),
            KaTahStance::Rendax => write!(f, "Rendax"),
        }
    }
}

/// Blessings of Khorne active blessings bitfield.
/// Source: Frenzied_Reavers.md - Blessings of Khorne
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActiveBlessings {
    /// Which blessings are currently active (indices into the blessing table).
    pub active: Vec<BlessingOfKhorne>,
}

impl ActiveBlessings {
    pub fn new() -> Self {
        Self { active: Vec::new() }
    }

    pub fn from_blessings(blessings: Vec<BlessingOfKhorne>) -> Self {
        Self { active: blessings }
    }

    pub fn has(&self, blessing: &BlessingOfKhorne) -> bool {
        self.active.contains(blessing)
    }

    pub fn add(&mut self, blessing: BlessingOfKhorne) {
        if !self.active.contains(&blessing) {
            self.active.push(blessing);
        }
    }

    pub fn clear(&mut self) {
        self.active.clear();
    }
}

impl Default for ActiveBlessings {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual Blessings of Khorne options.
///
/// Source: Frenzied_Reavers.md - Blessings of Khorne
///
/// At the start of each battle round, roll 5D6. Allocate dice to activate up
/// to two Blessings. Each Blessing can only be activated once per round. Active
/// Blessings apply to all units with this ability until the end of the battle round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlessingOfKhorne {
    /// Rage-fuelled Invigoration (Double 2+):
    /// Each time a model makes a Pile-in or Consolidation move, it can move
    /// up to 6" instead of up to 3".
    RageFuelledInvigoration,
    /// Total Carnage (Double 2+):
    /// Each time a model is destroyed by a melee attack, if it has not fought
    /// this phase, roll one D6: on a 4+, do not remove it from play. The
    /// destroyed model can fight after the attacking unit has finished making
    /// its attacks, and is then removed from play.
    TotalCarnage,
    /// Martial Excellence (Double 4+ or Triple any):
    /// Melee weapons equipped by models in this unit have the
    /// [SUSTAINED HITS 1] ability.
    MartialExcellence,
}

impl fmt::Display for BlessingOfKhorne {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlessingOfKhorne::RageFuelledInvigoration => write!(f, "Rage-fuelled Invigoration"),
            BlessingOfKhorne::TotalCarnage => write!(f, "Total Carnage"),
            BlessingOfKhorne::MartialExcellence => write!(f, "Martial Excellence"),
        }
    }
}

/// Reason for gaining or spending command points.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CpReason {
    /// Gained at start of Command Phase
    CommandPhaseGain,
    /// Spent on a stratagem
    StratagemCost(StratagemId),
    /// Refunded from an ability
    AbilityRefund,
    /// Custom reason (escape hatch)
    Custom(String),
}

impl fmt::Display for CpReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CpReason::CommandPhaseGain => write!(f, "Command Phase gain"),
            CpReason::StratagemCost(id) => write!(f, "Stratagem {}", id),
            CpReason::AbilityRefund => write!(f, "Ability refund"),
            CpReason::Custom(desc) => write!(f, "{}", desc),
        }
    }
}

/// The cause of a model or unit being destroyed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DestroyCause {
    /// Destroyed by an attack from a weapon
    Attack {
        attacker: UnitId,
        weapon: WeaponId,
    },
    /// Destroyed by mortal wounds
    MortalWounds {
        source: String,
    },
    /// Destroyed by a hazardous weapon test
    HazardousTest {
        weapon: WeaponId,
    },
    /// Destroyed by a game rule (e.g., fleeing off-table)
    GameRule {
        reason: String,
    },
}

impl fmt::Display for DestroyCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DestroyCause::Attack { attacker, weapon } => {
                write!(f, "Attack from unit {} with weapon {}", attacker, weapon)
            }
            DestroyCause::MortalWounds { source } => write!(f, "Mortal wounds: {}", source),
            DestroyCause::HazardousTest { weapon } => {
                write!(f, "Hazardous test for weapon {}", weapon)
            }
            DestroyCause::GameRule { reason } => write!(f, "Game rule: {}", reason),
        }
    }
}

/// Source of victory points scored.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VpSource {
    /// Points from holding a primary objective
    PrimaryObjective(ObjectiveId),
    /// Points from completing a secondary objective
    SecondaryObjective(String),
    /// Points from the mission rule (e.g., "Take and Hold")
    MissionRule(String),
    /// Custom VP source (escape hatch)
    Custom(String),
}

impl fmt::Display for VpSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VpSource::PrimaryObjective(id) => write!(f, "Primary objective {}", id),
            VpSource::SecondaryObjective(name) => write!(f, "Secondary: {}", name),
            VpSource::MissionRule(name) => write!(f, "Mission: {}", name),
            VpSource::Custom(desc) => write!(f, "{}", desc),
        }
    }
}

/// The result of a hit or wound roll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RollResult {
    /// The roll succeeded (hit/wounded)
    Success,
    /// The roll failed (missed/failed to wound)
    Failure,
}

impl fmt::Display for RollResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RollResult::Success => write!(f, "Success"),
            RollResult::Failure => write!(f, "Failure"),
        }
    }
}

/// The result of a feel-no-pain roll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FnpResult {
    /// The wound was ignored
    Ignored,
    /// The wound was suffered
    Suffered,
}

impl fmt::Display for FnpResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FnpResult::Ignored => write!(f, "Ignored"),
            FnpResult::Suffered => write!(f, "Suffered"),
        }
    }
}

/// The target of an effect (unit, model, or player-wide).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectTarget {
    /// Applies to a specific unit
    Unit(UnitId),
    /// Applies to a specific model
    Model(ModelId),
    /// Applies to all units of a player
    Player(PlayerId),
    /// Applies to all units on the battlefield (global)
    Global,
}

impl fmt::Display for EffectTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EffectTarget::Unit(id) => write!(f, "Unit({})", id),
            EffectTarget::Model(id) => write!(f, "Model({})", id),
            EffectTarget::Player(id) => write!(f, "Player({})", id),
            EffectTarget::Global => write!(f, "Global"),
        }
    }
}

/// Represents a legal action a player can take in a reaction window.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReactionAction {
    /// Use a stratagem
    UseStratagem(StratagemId),
    /// Fire overwatch
    FireOverwatch(UnitId),
    /// Make a heroic intervention
    HeroicIntervention(UnitId),
    /// Counter-offensive: fight with a unit out of normal order
    CounterOffensive(UnitId),
    /// Decline (pass, do nothing)
    Decline,
    /// Custom action (escape hatch for faction-specific reactions)
    Custom(String),
}

impl fmt::Display for ReactionAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReactionAction::UseStratagem(id) => write!(f, "Use stratagem {}", id),
            ReactionAction::FireOverwatch(id) => write!(f, "Overwatch with unit {}", id),
            ReactionAction::HeroicIntervention(id) => write!(f, "Heroic intervention unit {}", id),
            ReactionAction::CounterOffensive(id) => write!(f, "Counter-offensive unit {}", id),
            ReactionAction::Decline => write!(f, "Decline"),
            ReactionAction::Custom(desc) => write!(f, "{}", desc),
        }
    }
}

// ============================================================================
// GameEvent - ~40 variant enum covering all game events
// ============================================================================

/// All possible game events emitted during a 40K game.
///
/// Events are emitted by the command system and rules runtime as game state
/// transitions occur. They form an append-only log that can be queried for
/// replay, AI evaluation, rules triggers, and UI display.
///
/// Source: implementation_v3.md Section 6.5
/// Source: 40k_revised.md - Full rules reference
/// Source: Custodes.md, Frenzied_Reavers.md - Faction-specific events
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameEvent {
    // === Round and Turn Structure ===

    /// A new battle round has begun.
    BattleRoundStarted {
        round: BattleRound,
    },

    /// A player's turn has started within a battle round.
    TurnStarted {
        player: PlayerId,
        round: BattleRound,
    },

    /// A player's turn has ended.
    TurnEnded {
        player: PlayerId,
        round: BattleRound,
    },

    /// A phase has started for the active player.
    PhaseStarted {
        phase: Phase,
        player: PlayerId,
    },

    /// A phase has ended for the active player.
    PhaseEnded {
        phase: Phase,
        player: PlayerId,
    },

    // === Command Points ===

    /// A player gained command points.
    CommandPointsGained {
        player: PlayerId,
        amount: u8,
        reason: CpReason,
    },

    /// A player spent command points.
    CommandPointsSpent {
        player: PlayerId,
        amount: u8,
        reason: CpReason,
    },

    // === Battle-shock ===

    /// A unit is required to take a battle-shock test.
    BattleShockTestRequired {
        unit: UnitId,
    },

    /// A battle-shock test has been resolved.
    BattleShockTestResolved {
        unit: UnitId,
        result: BattleShockResult,
        roll: u8,
    },

    // === Movement ===

    /// A unit has been selected to move.
    UnitSelectedToMove {
        unit: UnitId,
    },

    /// A unit's movement action has started.
    MoveStarted {
        unit: UnitId,
        action: MovementAction,
    },

    /// A unit's movement action has completed.
    MoveCompleted {
        unit: UnitId,
        action: MovementAction,
        from: Position,
        to: Position,
    },

    /// An advance roll has been made for a unit.
    AdvanceRolled {
        unit: UnitId,
        roll: u8,
        total_move: u16,
    },

    /// A unit has arrived from reserves (Strategic Reserves or Deep Strike).
    UnitArrivedFromReserves {
        unit: UnitId,
        position: Position,
    },

    // === Shooting ===

    /// A unit has been selected to shoot.
    UnitSelectedToShoot {
        unit: UnitId,
    },

    /// An attack sequence has begun (attacker fires a weapon at a defender).
    AttackSequenceStarted {
        attacker: UnitId,
        defender: UnitId,
        weapon: WeaponId,
    },

    /// A hit roll has been made.
    HitRollMade {
        attacker: UnitId,
        roll: u8,
        modified: i8,
        is_critical: bool,
        result: RollResult,
    },

    /// A wound roll has been made.
    WoundRollMade {
        attacker: UnitId,
        roll: u8,
        modified: i8,
        is_critical: bool,
        result: RollResult,
    },

    /// A saving throw has been made.
    SaveRollMade {
        defender: UnitId,
        roll: u8,
        modified: i8,
        save_type: SaveType,
        result: wh40k_core_types::SaveResult,
    },

    /// Damage has been inflicted on a model.
    DamageInflicted {
        target_model: ModelId,
        wounds: u8,
        source: WeaponId,
    },

    /// Mortal wounds have been inflicted on a unit.
    MortalWoundsInflicted {
        target_unit: UnitId,
        wounds: u8,
        source: String,
    },

    /// A feel-no-pain roll has been made.
    FeelNoPainRollMade {
        model: ModelId,
        roll: u8,
        result: FnpResult,
    },

    /// A model has been destroyed.
    ModelDestroyed {
        model: ModelId,
        unit: UnitId,
        cause: DestroyCause,
    },

    /// A unit has been completely destroyed (all models removed).
    UnitDestroyed {
        unit: UnitId,
        cause: DestroyCause,
    },

    /// An attack sequence has ended.
    AttackSequenceEnded {
        attacker: UnitId,
        defender: UnitId,
    },

    // === Charge ===

    /// A unit has declared a charge against one or more targets.
    ChargeDeclared {
        unit: UnitId,
        targets: Vec<UnitId>,
    },

    /// A charge roll has been made.
    ChargeRollMade {
        unit: UnitId,
        roll: u8,
        distance: u8,
        success: bool,
    },

    /// A charge has been completed (unit moved into engagement range).
    ChargeCompleted {
        unit: UnitId,
    },

    /// A Heroic Intervention move has been completed.
    /// Source: 40k_revised.md - "Heroic Intervention"
    HeroicInterventionCompleted {
        unit: UnitId,
    },

    /// A charge has failed (insufficient roll to reach targets).
    ChargeFailed {
        unit: UnitId,
    },

    /// Overwatch has been fired at an incoming unit.
    OverwatchFired {
        unit: UnitId,
        target: UnitId,
    },

    // === Fight ===

    /// A unit has been selected to fight in the Fight Phase.
    UnitSelectedToFight {
        unit: UnitId,
    },

    /// A pile-in move has been made.
    PileInMade {
        unit: UnitId,
        distance: u16,
    },

    /// A fight (melee attack sequence) has been resolved.
    FightResolved {
        attacker: UnitId,
        defender: UnitId,
    },

    /// A consolidation move has been made.
    ConsolidationMade {
        unit: UnitId,
        distance: u16,
    },

    // === Objectives and Scoring ===

    /// Control of an objective has changed.
    ObjectiveControlChanged {
        objective: ObjectiveId,
        old_controller: Option<PlayerId>,
        new_controller: Option<PlayerId>,
    },

    /// An objective has been secured by a player.
    ObjectiveSecured {
        objective: ObjectiveId,
        player: PlayerId,
    },

    /// Victory points have been scored.
    VictoryPointsScored {
        player: PlayerId,
        amount: u16,
        source: VpSource,
    },

    // === Stratagems and Effects ===

    /// A stratagem has been used.
    StratagemUsed {
        stratagem: StratagemId,
        player: PlayerId,
        target: EffectTarget,
    },

    /// A temporary effect has been applied.
    EffectApplied {
        effect_id: EffectId,
        target: EffectTarget,
        duration: EffectDuration,
    },

    /// A temporary effect has expired.
    EffectExpired {
        effect_id: EffectId,
        target: EffectTarget,
    },

    // === Faction-specific: World Eaters (Frenzied Reavers) ===

    /// Blessings of Khorne dice have been rolled at the start of battle round.
    /// Source: Frenzied_Reavers.md - Blessings of Khorne
    BlessingsOfKhorneRolled {
        dice: Vec<u8>,
        active_blessings: ActiveBlessings,
    },

    /// Fight on Death has been triggered for a destroyed model.
    /// Source: Frenzied_Reavers.md - Unbridled Bloodlust blessing
    FightOnDeathTriggered {
        model: ModelId,
        roll: u8,
        success: bool,
    },

    // === Faction-specific: Adeptus Custodes ===

    /// A Ka'tah stance has been chosen for a unit.
    /// Source: Custodes.md - Ka'tah Stances
    KaTahStanceChosen {
        unit: UnitId,
        stance: KaTahStance,
    },

    // === Stratagem Effects ===

    /// A stratagem's specific effect was applied.
    /// Source: 40k_revised.md - Core Stratagems
    /// Source: Custodes.md, Frenzied_Reavers.md - Faction Stratagems
    StratagemEffectApplied {
        stratagem: StratagemId,
        target: UnitId,
        description: String,
    },

    /// Command Re-roll stratagem was used to re-roll a die.
    /// Source: 40k_revised.md §15.2 - "Command Re-roll"
    CommandRerollUsed {
        player: PlayerId,
        roll_type: String,
        original_roll: u8,
        new_roll: u8,
    },

    // === Game End ===

    /// The battle has ended.
    BattleEnded {
        outcome: GameOutcome,
    },
}

impl GameEvent {
    /// Returns the discriminant name of this event variant as a string.
    pub fn kind_name(&self) -> &'static str {
        match self {
            GameEvent::BattleRoundStarted { .. } => "BattleRoundStarted",
            GameEvent::TurnStarted { .. } => "TurnStarted",
            GameEvent::TurnEnded { .. } => "TurnEnded",
            GameEvent::PhaseStarted { .. } => "PhaseStarted",
            GameEvent::PhaseEnded { .. } => "PhaseEnded",
            GameEvent::CommandPointsGained { .. } => "CommandPointsGained",
            GameEvent::CommandPointsSpent { .. } => "CommandPointsSpent",
            GameEvent::BattleShockTestRequired { .. } => "BattleShockTestRequired",
            GameEvent::BattleShockTestResolved { .. } => "BattleShockTestResolved",
            GameEvent::UnitSelectedToMove { .. } => "UnitSelectedToMove",
            GameEvent::MoveStarted { .. } => "MoveStarted",
            GameEvent::MoveCompleted { .. } => "MoveCompleted",
            GameEvent::AdvanceRolled { .. } => "AdvanceRolled",
            GameEvent::UnitArrivedFromReserves { .. } => "UnitArrivedFromReserves",
            GameEvent::UnitSelectedToShoot { .. } => "UnitSelectedToShoot",
            GameEvent::AttackSequenceStarted { .. } => "AttackSequenceStarted",
            GameEvent::HitRollMade { .. } => "HitRollMade",
            GameEvent::WoundRollMade { .. } => "WoundRollMade",
            GameEvent::SaveRollMade { .. } => "SaveRollMade",
            GameEvent::DamageInflicted { .. } => "DamageInflicted",
            GameEvent::MortalWoundsInflicted { .. } => "MortalWoundsInflicted",
            GameEvent::FeelNoPainRollMade { .. } => "FeelNoPainRollMade",
            GameEvent::ModelDestroyed { .. } => "ModelDestroyed",
            GameEvent::UnitDestroyed { .. } => "UnitDestroyed",
            GameEvent::AttackSequenceEnded { .. } => "AttackSequenceEnded",
            GameEvent::ChargeDeclared { .. } => "ChargeDeclared",
            GameEvent::ChargeRollMade { .. } => "ChargeRollMade",
            GameEvent::ChargeCompleted { .. } => "ChargeCompleted",
            GameEvent::HeroicInterventionCompleted { .. } => "HeroicInterventionCompleted",
            GameEvent::ChargeFailed { .. } => "ChargeFailed",
            GameEvent::OverwatchFired { .. } => "OverwatchFired",
            GameEvent::UnitSelectedToFight { .. } => "UnitSelectedToFight",
            GameEvent::PileInMade { .. } => "PileInMade",
            GameEvent::FightResolved { .. } => "FightResolved",
            GameEvent::ConsolidationMade { .. } => "ConsolidationMade",
            GameEvent::ObjectiveControlChanged { .. } => "ObjectiveControlChanged",
            GameEvent::ObjectiveSecured { .. } => "ObjectiveSecured",
            GameEvent::VictoryPointsScored { .. } => "VictoryPointsScored",
            GameEvent::StratagemUsed { .. } => "StratagemUsed",
            GameEvent::EffectApplied { .. } => "EffectApplied",
            GameEvent::EffectExpired { .. } => "EffectExpired",
            GameEvent::BlessingsOfKhorneRolled { .. } => "BlessingsOfKhorneRolled",
            GameEvent::FightOnDeathTriggered { .. } => "FightOnDeathTriggered",
            GameEvent::KaTahStanceChosen { .. } => "KaTahStanceChosen",
            GameEvent::StratagemEffectApplied { .. } => "StratagemEffectApplied",
            GameEvent::CommandRerollUsed { .. } => "CommandRerollUsed",
            GameEvent::BattleEnded { .. } => "BattleEnded",
        }
    }

    /// Extract the UnitId most relevant to this event, if any.
    /// For events involving multiple units, returns the "primary" unit
    /// (attacker for combat events, the acting unit for movement, etc.).
    pub fn primary_unit(&self) -> Option<UnitId> {
        match self {
            GameEvent::BattleShockTestRequired { unit } => Some(*unit),
            GameEvent::BattleShockTestResolved { unit, .. } => Some(*unit),
            GameEvent::UnitSelectedToMove { unit } => Some(*unit),
            GameEvent::MoveStarted { unit, .. } => Some(*unit),
            GameEvent::MoveCompleted { unit, .. } => Some(*unit),
            GameEvent::AdvanceRolled { unit, .. } => Some(*unit),
            GameEvent::UnitArrivedFromReserves { unit, .. } => Some(*unit),
            GameEvent::UnitSelectedToShoot { unit } => Some(*unit),
            GameEvent::AttackSequenceStarted { attacker, .. } => Some(*attacker),
            GameEvent::HitRollMade { attacker, .. } => Some(*attacker),
            GameEvent::WoundRollMade { attacker, .. } => Some(*attacker),
            GameEvent::SaveRollMade { defender, .. } => Some(*defender),
            GameEvent::UnitDestroyed { unit, .. } => Some(*unit),
            GameEvent::AttackSequenceEnded { attacker, .. } => Some(*attacker),
            GameEvent::ChargeDeclared { unit, .. } => Some(*unit),
            GameEvent::ChargeRollMade { unit, .. } => Some(*unit),
            GameEvent::ChargeCompleted { unit } => Some(*unit),
            GameEvent::HeroicInterventionCompleted { unit } => Some(*unit),
            GameEvent::ChargeFailed { unit } => Some(*unit),
            GameEvent::OverwatchFired { unit, .. } => Some(*unit),
            GameEvent::UnitSelectedToFight { unit } => Some(*unit),
            GameEvent::PileInMade { unit, .. } => Some(*unit),
            GameEvent::FightResolved { attacker, .. } => Some(*attacker),
            GameEvent::ConsolidationMade { unit, .. } => Some(*unit),
            GameEvent::KaTahStanceChosen { unit, .. } => Some(*unit),
            GameEvent::StratagemEffectApplied { target, .. } => Some(*target),
            _ => None,
        }
    }

    /// Extract the PlayerId most relevant to this event, if any.
    pub fn primary_player(&self) -> Option<PlayerId> {
        match self {
            GameEvent::TurnStarted { player, .. } => Some(*player),
            GameEvent::TurnEnded { player, .. } => Some(*player),
            GameEvent::PhaseStarted { player, .. } => Some(*player),
            GameEvent::PhaseEnded { player, .. } => Some(*player),
            GameEvent::CommandPointsGained { player, .. } => Some(*player),
            GameEvent::CommandPointsSpent { player, .. } => Some(*player),
            GameEvent::ObjectiveSecured { player, .. } => Some(*player),
            GameEvent::VictoryPointsScored { player, .. } => Some(*player),
            GameEvent::StratagemUsed { player, .. } => Some(*player),
            GameEvent::CommandRerollUsed { player, .. } => Some(*player),
            _ => None,
        }
    }

    /// Extract the Phase this event relates to, if any.
    pub fn related_phase(&self) -> Option<Phase> {
        match self {
            GameEvent::PhaseStarted { phase, .. } => Some(*phase),
            GameEvent::PhaseEnded { phase, .. } => Some(*phase),
            _ => None,
        }
    }

    /// Returns true if this is a combat-related event (attack sequence, rolls, damage).
    pub fn is_combat_event(&self) -> bool {
        matches!(
            self,
            GameEvent::AttackSequenceStarted { .. }
                | GameEvent::HitRollMade { .. }
                | GameEvent::WoundRollMade { .. }
                | GameEvent::SaveRollMade { .. }
                | GameEvent::DamageInflicted { .. }
                | GameEvent::MortalWoundsInflicted { .. }
                | GameEvent::FeelNoPainRollMade { .. }
                | GameEvent::ModelDestroyed { .. }
                | GameEvent::UnitDestroyed { .. }
                | GameEvent::AttackSequenceEnded { .. }
                | GameEvent::FightResolved { .. }
                | GameEvent::CommandRerollUsed { .. }
        )
    }

    /// Returns true if this is a movement-related event.
    pub fn is_movement_event(&self) -> bool {
        matches!(
            self,
            GameEvent::UnitSelectedToMove { .. }
                | GameEvent::MoveStarted { .. }
                | GameEvent::MoveCompleted { .. }
                | GameEvent::AdvanceRolled { .. }
                | GameEvent::UnitArrivedFromReserves { .. }
        )
    }

    /// Returns true if this is a charge-related event.
    pub fn is_charge_event(&self) -> bool {
        matches!(
            self,
            GameEvent::ChargeDeclared { .. }
                | GameEvent::ChargeRollMade { .. }
                | GameEvent::ChargeCompleted { .. }
                | GameEvent::HeroicInterventionCompleted { .. }
                | GameEvent::ChargeFailed { .. }
                | GameEvent::OverwatchFired { .. }
        )
    }

    /// Returns true if this is a phase-structure event.
    pub fn is_phase_event(&self) -> bool {
        matches!(
            self,
            GameEvent::BattleRoundStarted { .. }
                | GameEvent::TurnStarted { .. }
                | GameEvent::TurnEnded { .. }
                | GameEvent::PhaseStarted { .. }
                | GameEvent::PhaseEnded { .. }
                | GameEvent::BattleEnded { .. }
        )
    }
}

impl fmt::Display for GameEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameEvent::BattleRoundStarted { round } => {
                write!(f, "Battle Round {} started", round.number())
            }
            GameEvent::TurnStarted { player, round } => {
                write!(f, "Player {} turn started (round {})", player, round.number())
            }
            GameEvent::TurnEnded { player, round } => {
                write!(f, "Player {} turn ended (round {})", player, round.number())
            }
            GameEvent::PhaseStarted { phase, player } => {
                write!(f, "{:?} Phase started for Player {}", phase, player)
            }
            GameEvent::PhaseEnded { phase, player } => {
                write!(f, "{:?} Phase ended for Player {}", phase, player)
            }
            GameEvent::CommandPointsGained { player, amount, reason } => {
                write!(f, "Player {} gained {}CP ({})", player, amount, reason)
            }
            GameEvent::CommandPointsSpent { player, amount, reason } => {
                write!(f, "Player {} spent {}CP ({})", player, amount, reason)
            }
            GameEvent::BattleShockTestRequired { unit } => {
                write!(f, "Unit {} must take Battle-shock test", unit)
            }
            GameEvent::BattleShockTestResolved { unit, result, roll } => {
                write!(f, "Unit {} Battle-shock: {:?} (rolled {})", unit, result, roll)
            }
            GameEvent::UnitSelectedToMove { unit } => {
                write!(f, "Unit {} selected to move", unit)
            }
            GameEvent::MoveStarted { unit, action } => {
                write!(f, "Unit {} {:?} started", unit, action)
            }
            GameEvent::MoveCompleted { unit, action, from, to } => {
                write!(f, "Unit {} {:?} from {} to {}", unit, action, from, to)
            }
            GameEvent::AdvanceRolled { unit, roll, total_move } => {
                write!(f, "Unit {} advance roll: {} (total {}\")", unit, roll, total_move)
            }
            GameEvent::UnitArrivedFromReserves { unit, position } => {
                write!(f, "Unit {} arrived from reserves at {}", unit, position)
            }
            GameEvent::UnitSelectedToShoot { unit } => {
                write!(f, "Unit {} selected to shoot", unit)
            }
            GameEvent::AttackSequenceStarted { attacker, defender, weapon } => {
                write!(
                    f,
                    "Attack: Unit {} -> Unit {} with weapon {}",
                    attacker, defender, weapon
                )
            }
            GameEvent::HitRollMade { attacker, roll, modified, is_critical, result } => {
                write!(
                    f,
                    "Unit {} hit roll: {} (mod {}, crit={}, {:?})",
                    attacker, roll, modified, is_critical, result
                )
            }
            GameEvent::WoundRollMade { attacker, roll, modified, is_critical, result } => {
                write!(
                    f,
                    "Unit {} wound roll: {} (mod {}, crit={}, {:?})",
                    attacker, roll, modified, is_critical, result
                )
            }
            GameEvent::SaveRollMade { defender, roll, modified, save_type, result } => {
                write!(
                    f,
                    "Unit {} {:?} save: {} (mod {}, {:?})",
                    defender, save_type, roll, modified, result
                )
            }
            GameEvent::DamageInflicted { target_model, wounds, source } => {
                write!(
                    f,
                    "Model {} took {} wounds from weapon {}",
                    target_model, wounds, source
                )
            }
            GameEvent::MortalWoundsInflicted { target_unit, wounds, source } => {
                write!(
                    f,
                    "Unit {} took {} mortal wounds from {}",
                    target_unit, wounds, source
                )
            }
            GameEvent::FeelNoPainRollMade { model, roll, result } => {
                write!(f, "Model {} FNP roll: {} ({:?})", model, roll, result)
            }
            GameEvent::ModelDestroyed { model, unit, cause } => {
                write!(f, "Model {} (unit {}) destroyed: {}", model, unit, cause)
            }
            GameEvent::UnitDestroyed { unit, cause } => {
                write!(f, "Unit {} destroyed: {}", unit, cause)
            }
            GameEvent::AttackSequenceEnded { attacker, defender } => {
                write!(f, "Attack ended: Unit {} -> Unit {}", attacker, defender)
            }
            GameEvent::ChargeDeclared { unit, targets } => {
                write!(f, "Unit {} declares charge at {:?}", unit, targets)
            }
            GameEvent::ChargeRollMade { unit, roll, distance, success } => {
                write!(
                    f,
                    "Unit {} charge roll: {} (dist {}\", {})",
                    unit,
                    roll,
                    distance,
                    if *success { "success" } else { "failed" }
                )
            }
            GameEvent::ChargeCompleted { unit } => {
                write!(f, "Unit {} charge completed", unit)
            }
            GameEvent::HeroicInterventionCompleted { unit } => {
                write!(f, "Unit {} heroic intervention completed", unit)
            }
            GameEvent::ChargeFailed { unit } => {
                write!(f, "Unit {} charge failed", unit)
            }
            GameEvent::OverwatchFired { unit, target } => {
                write!(f, "Unit {} fires Overwatch at unit {}", unit, target)
            }
            GameEvent::UnitSelectedToFight { unit } => {
                write!(f, "Unit {} selected to fight", unit)
            }
            GameEvent::PileInMade { unit, distance } => {
                write!(f, "Unit {} piled in {}\"", unit, distance)
            }
            GameEvent::FightResolved { attacker, defender } => {
                write!(f, "Fight resolved: Unit {} vs Unit {}", attacker, defender)
            }
            GameEvent::ConsolidationMade { unit, distance } => {
                write!(f, "Unit {} consolidated {}\"", unit, distance)
            }
            GameEvent::ObjectiveControlChanged { objective, old_controller, new_controller } => {
                write!(
                    f,
                    "Objective {} control: {:?} -> {:?}",
                    objective, old_controller, new_controller
                )
            }
            GameEvent::ObjectiveSecured { objective, player } => {
                write!(f, "Objective {} secured by Player {}", objective, player)
            }
            GameEvent::VictoryPointsScored { player, amount, source } => {
                write!(f, "Player {} scored {}VP ({})", player, amount, source)
            }
            GameEvent::StratagemUsed { stratagem, player, target } => {
                write!(
                    f,
                    "Player {} used stratagem {} on {}",
                    player, stratagem, target
                )
            }
            GameEvent::EffectApplied { effect_id, target, duration } => {
                write!(
                    f,
                    "Effect {} applied to {} (duration: {:?})",
                    effect_id, target, duration
                )
            }
            GameEvent::EffectExpired { effect_id, target } => {
                write!(f, "Effect {} expired on {}", effect_id, target)
            }
            GameEvent::BlessingsOfKhorneRolled { dice, active_blessings } => {
                write!(
                    f,
                    "Blessings of Khorne rolled {:?}, active: {:?}",
                    dice, active_blessings.active
                )
            }
            GameEvent::FightOnDeathTriggered { model, roll, success } => {
                write!(
                    f,
                    "Model {} Fight on Death: rolled {} ({})",
                    model,
                    roll,
                    if *success { "success" } else { "failed" }
                )
            }
            GameEvent::KaTahStanceChosen { unit, stance } => {
                write!(f, "Unit {} chose Ka'tah stance: {}", unit, stance)
            }
            GameEvent::StratagemEffectApplied { stratagem, target, description } => {
                write!(
                    f,
                    "Stratagem {} effect applied to unit {}: {}",
                    stratagem, target, description
                )
            }
            GameEvent::CommandRerollUsed { player, roll_type, original_roll, new_roll } => {
                write!(
                    f,
                    "Player {} used Command Re-roll on {}: {} -> {}",
                    player, roll_type, original_roll, new_roll
                )
            }
            GameEvent::BattleEnded { outcome } => {
                write!(f, "Battle ended: {:?}", outcome)
            }
        }
    }
}

// ============================================================================
// EventFilter - Predicate for matching events
// ============================================================================

/// A predicate for matching game events. Used by event handlers to specify
/// which events they care about.
#[derive(Clone, Serialize, Deserialize)]
pub enum EventFilter {
    /// Matches all events.
    All,
    /// Matches events that occur during a specific phase.
    ByPhase(Phase),
    /// Matches events by their discriminant name (e.g., "HitRollMade").
    ByEventKind(String),
    /// Matches events involving a specific unit.
    ByUnit(UnitId),
    /// Matches events involving a specific player.
    ByPlayer(PlayerId),
    /// Matches events that satisfy all of the inner filters (logical AND).
    And(Vec<EventFilter>),
    /// Matches events that satisfy any of the inner filters (logical OR).
    Or(Vec<EventFilter>),
    /// Negates an inner filter.
    Not(Box<EventFilter>),
}

impl EventFilter {
    /// Test whether a game event matches this filter.
    pub fn matches(&self, event: &GameEvent) -> bool {
        match self {
            EventFilter::All => true,

            EventFilter::ByPhase(phase) => {
                event.related_phase() == Some(*phase)
            }

            EventFilter::ByEventKind(kind) => event.kind_name() == kind.as_str(),

            EventFilter::ByUnit(unit_id) => {
                event.primary_unit() == Some(*unit_id)
            }

            EventFilter::ByPlayer(player_id) => {
                event.primary_player() == Some(*player_id)
            }

            EventFilter::And(filters) => filters.iter().all(|f| f.matches(event)),

            EventFilter::Or(filters) => filters.iter().any(|f| f.matches(event)),

            EventFilter::Not(inner) => !inner.matches(event),
        }
    }

    /// Create a filter that matches a specific event kind by name.
    pub fn kind(name: &str) -> Self {
        EventFilter::ByEventKind(name.to_string())
    }

    /// Create a filter that matches events for a specific unit.
    pub fn unit(unit_id: UnitId) -> Self {
        EventFilter::ByUnit(unit_id)
    }

    /// Create a filter that matches events for a specific player.
    pub fn player(player_id: PlayerId) -> Self {
        EventFilter::ByPlayer(player_id)
    }

    /// Create a filter that matches events in a specific phase.
    pub fn phase(phase: Phase) -> Self {
        EventFilter::ByPhase(phase)
    }

    /// Combine this filter with another using AND.
    pub fn and(self, other: EventFilter) -> Self {
        match self {
            EventFilter::And(mut filters) => {
                filters.push(other);
                EventFilter::And(filters)
            }
            _ => EventFilter::And(vec![self, other]),
        }
    }

    /// Combine this filter with another using OR.
    pub fn or(self, other: EventFilter) -> Self {
        match self {
            EventFilter::Or(mut filters) => {
                filters.push(other);
                EventFilter::Or(filters)
            }
            _ => EventFilter::Or(vec![self, other]),
        }
    }

    /// Negate this filter.
    pub fn negate(self) -> Self {
        EventFilter::Not(Box::new(self))
    }
}

impl fmt::Debug for EventFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventFilter::All => write!(f, "All"),
            EventFilter::ByPhase(phase) => write!(f, "ByPhase({:?})", phase),
            EventFilter::ByEventKind(kind) => write!(f, "ByEventKind(\"{}\")", kind),
            EventFilter::ByUnit(unit) => write!(f, "ByUnit({:?})", unit),
            EventFilter::ByPlayer(player) => write!(f, "ByPlayer({:?})", player),
            EventFilter::And(filters) => write!(f, "And({:?})", filters),
            EventFilter::Or(filters) => write!(f, "Or({:?})", filters),
            EventFilter::Not(inner) => write!(f, "Not({:?})", inner),
        }
    }
}

// ============================================================================
// EventHandler - A registered handler with an ID and filter
// ============================================================================

/// A registered event handler with an ID, filter, and priority.
///
/// Handlers are registered on the [`EventBus`] and matched against incoming
/// events. Higher priority handlers are checked first.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventHandler {
    /// Unique identifier for this handler.
    pub handler_id: HandlerId,
    /// The filter that determines which events this handler matches.
    pub filter: EventFilter,
    /// Priority for ordering. Higher values are processed first.
    /// Default priority is 0.
    pub priority: i32,
    /// Human-readable description for debugging and logging.
    pub description: String,
    /// Whether this handler is currently active.
    pub active: bool,
}

impl EventHandler {
    /// Create a new event handler.
    pub fn new(handler_id: HandlerId, filter: EventFilter, description: &str) -> Self {
        Self {
            handler_id,
            filter,
            priority: 0,
            description: description.to_string(),
            active: true,
        }
    }

    /// Create a new event handler with a specific priority.
    pub fn with_priority(
        handler_id: HandlerId,
        filter: EventFilter,
        priority: i32,
        description: &str,
    ) -> Self {
        Self {
            handler_id,
            filter,
            priority,
            description: description.to_string(),
            active: true,
        }
    }

    /// Test whether this handler matches a given event.
    pub fn matches(&self, event: &GameEvent) -> bool {
        self.active && self.filter.matches(event)
    }

    /// Deactivate this handler (it will no longer match events).
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Reactivate this handler.
    pub fn activate(&mut self) {
        self.active = true;
    }
}

// ============================================================================
// ReactionWindow - Represents a pause point for player decisions
// ============================================================================

/// Represents a pause point in the game where a player can make a decision.
///
/// Reaction windows are created when certain events occur that allow the
/// non-active player (or sometimes the active player) to respond with
/// stratagems, overwatch, heroic interventions, etc.
///
/// Source: 40k_revised.md - Stratagems, Overwatch, Heroic Intervention
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReactionWindow {
    /// The type of reaction window.
    pub window_type: ReactionWindowType,
    /// The phase during which this window opened.
    pub phase: Phase,
    /// Timing within the phase (e.g., "after charge declared").
    pub timing: ReactionTiming,
    /// The player who can respond.
    pub owner: PlayerId,
    /// List of legal actions the player can take.
    pub legal_actions: Vec<ReactionAction>,
    /// The event that triggered this reaction window.
    pub triggering_event_id: Option<EventId>,
    /// Whether the window auto-declines if no response is given.
    pub auto_decline: bool,
    /// Maximum time in seconds before auto-decline (0 = no limit).
    pub deadline_seconds: u32,
}

impl ReactionWindow {
    /// Create a new reaction window.
    pub fn new(
        window_type: ReactionWindowType,
        phase: Phase,
        timing: ReactionTiming,
        owner: PlayerId,
    ) -> Self {
        Self {
            window_type,
            phase,
            timing,
            owner,
            legal_actions: Vec::new(),
            triggering_event_id: None,
            auto_decline: true,
            deadline_seconds: 0,
        }
    }

    /// Add a legal action to this window.
    pub fn add_action(&mut self, action: ReactionAction) {
        self.legal_actions.push(action);
    }

    /// Set the triggering event.
    pub fn with_trigger(mut self, event_id: EventId) -> Self {
        self.triggering_event_id = Some(event_id);
        self
    }

    /// Set the deadline in seconds.
    pub fn with_deadline(mut self, seconds: u32) -> Self {
        self.deadline_seconds = seconds;
        self.auto_decline = true;
        self
    }

    /// Disable auto-decline (window stays open until explicitly resolved).
    pub fn without_auto_decline(mut self) -> Self {
        self.auto_decline = false;
        self
    }

    /// Whether this window has any legal actions available.
    pub fn has_actions(&self) -> bool {
        !self.legal_actions.is_empty()
    }

    /// Whether the only legal action is to decline.
    pub fn must_decline(&self) -> bool {
        self.legal_actions.is_empty()
            || (self.legal_actions.len() == 1
                && matches!(self.legal_actions[0], ReactionAction::Decline))
    }
}

/// When within a phase a reaction window opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReactionTiming {
    /// Before a specific action occurs.
    Before,
    /// After a specific action occurs.
    After,
    /// During an action (e.g., during charge move, overwatch can fire).
    During,
    /// At the start of a phase, before any actions.
    PhaseStart,
    /// At the end of a phase, after all actions.
    PhaseEnd,
}

// ============================================================================
// Timestamped event log entry
// ============================================================================

/// An event log entry that pairs a GameEvent with metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventLogEntry {
    /// Unique sequential ID for this event.
    pub event_id: EventId,
    /// The game event itself.
    pub event: GameEvent,
    /// The battle round when this event occurred.
    pub round: BattleRound,
    /// The phase when this event occurred.
    pub phase: Phase,
    /// The active player when this event occurred.
    pub active_player: PlayerId,
    /// Sequential order within the game.
    pub sequence: u64,
}

// ============================================================================
// EventBus - Central event handling
// ============================================================================

/// Errors that can occur in the event system.
#[derive(Debug, Error)]
pub enum EventSystemError {
    #[error("Handler with id {0} not found")]
    HandlerNotFound(HandlerId),

    #[error("Handler with id {0} already exists")]
    HandlerAlreadyExists(HandlerId),

    #[error("Event queue overflow: max depth {0} reached")]
    QueueOverflow(usize),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Central event bus for the 40K engine.
///
/// The EventBus is the core communication mechanism between engine subsystems.
/// Events are emitted during game state transitions, logged to an append-only
/// history, and matched against registered handlers.
///
/// ## Usage
///
/// ```rust
/// use wh40k_event_system::*;
/// use wh40k_core_types::*;
///
/// let mut bus = EventBus::new();
///
/// // Register a handler
/// let handler = EventHandler::new(
///     HandlerId::new(1),
///     EventFilter::kind("BattleRoundStarted"),
///     "Track round starts",
/// );
/// bus.subscribe(handler).unwrap();
///
/// // Emit an event
/// bus.emit(
///     GameEvent::BattleRoundStarted { round: BattleRound::new(1) },
///     BattleRound::new(1),
///     Phase::Command,
///     PlayerId::new(0),
/// );
///
/// // Query events
/// assert_eq!(bus.event_count(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBus {
    /// Append-only event history log.
    log: Vec<EventLogEntry>,
    /// Registered event handlers, sorted by priority (descending).
    handlers: Vec<EventHandler>,
    /// Queue for nested event emission. When emitting an event triggers
    /// another event, the nested event is queued for processing after
    /// the current batch completes.
    #[serde(skip)]
    queue: VecDeque<QueuedEvent>,
    /// Counter for generating sequential event IDs.
    id_generator: IdGenerator,
    /// Global sequence counter for ordering events.
    sequence_counter: u64,
    /// Maximum queue depth to prevent infinite loops.
    max_queue_depth: usize,
    /// Whether we are currently draining the queue (to detect nested drain).
    #[serde(skip)]
    draining: bool,
    /// Handler ID generator for auto-assigning IDs.
    next_handler_id: u32,
}

/// An event waiting in the queue for deferred processing.
#[derive(Debug, Clone)]
struct QueuedEvent {
    event: GameEvent,
    round: BattleRound,
    phase: Phase,
    active_player: PlayerId,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// Create a new, empty EventBus.
    pub fn new() -> Self {
        Self {
            log: Vec::new(),
            handlers: Vec::new(),
            queue: VecDeque::new(),
            id_generator: IdGenerator::new(),
            sequence_counter: 0,
            max_queue_depth: 1000,
            draining: false,
            next_handler_id: 0,
        }
    }

    /// Create a new EventBus with a custom max queue depth.
    pub fn with_max_queue_depth(max_depth: usize) -> Self {
        Self {
            max_queue_depth: max_depth,
            ..Self::new()
        }
    }

    // ========================================================================
    // Emitting events
    // ========================================================================

    /// Emit a game event. The event is added to the log and all matching
    /// handlers are identified.
    ///
    /// If the bus is currently draining the queue, the event is queued
    /// for processing after the current drain completes.
    ///
    /// Returns the EventId assigned to this event.
    pub fn emit(
        &mut self,
        event: GameEvent,
        round: BattleRound,
        phase: Phase,
        active_player: PlayerId,
    ) -> EventId {
        // Always log immediately. The event queue (enqueue/drain_queue) is a
        // separate mechanism for deferred event processing, not for emit.
        self.log_event(event, round, phase, active_player)
    }

    /// Internal: log an event to the history and return its EventId.
    fn log_event(
        &mut self,
        event: GameEvent,
        round: BattleRound,
        phase: Phase,
        active_player: PlayerId,
    ) -> EventId {
        let event_id: EventId = self.id_generator.next_id();
        let sequence = self.sequence_counter;
        self.sequence_counter += 1;

        let entry = EventLogEntry {
            event_id,
            event,
            round,
            phase,
            active_player,
            sequence,
        };

        self.log.push(entry);
        event_id
    }

    /// Queue an event for deferred processing.
    /// This is useful when an event handler needs to emit additional events.
    ///
    /// Returns Ok(()) or an error if the queue is full.
    pub fn enqueue(
        &mut self,
        event: GameEvent,
        round: BattleRound,
        phase: Phase,
        active_player: PlayerId,
    ) -> Result<(), EventSystemError> {
        if self.queue.len() >= self.max_queue_depth {
            return Err(EventSystemError::QueueOverflow(self.max_queue_depth));
        }

        self.queue.push_back(QueuedEvent {
            event,
            round,
            phase,
            active_player,
        });

        Ok(())
    }

    /// Drain the event queue, processing all queued events.
    ///
    /// Each queued event is logged to the history. If processing a queued
    /// event causes more events to be queued (via enqueue), those are
    /// processed in the same drain cycle.
    ///
    /// Returns the number of events that were drained from the queue.
    pub fn drain_queue(&mut self) -> Result<usize, EventSystemError> {
        if self.draining {
            // Nested drain - events are already being processed
            return Ok(0);
        }

        self.draining = true;
        let mut count = 0;

        while let Some(queued) = self.queue.pop_front() {
            self.log_event(queued.event, queued.round, queued.phase, queued.active_player);
            count += 1;

            if count > self.max_queue_depth {
                self.draining = false;
                return Err(EventSystemError::QueueOverflow(self.max_queue_depth));
            }
        }

        self.draining = false;
        Ok(count)
    }

    /// Returns true if the event queue has pending events.
    pub fn has_queued_events(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Returns the number of events in the queue.
    pub fn queued_count(&self) -> usize {
        self.queue.len()
    }

    // ========================================================================
    // Subscribing / Unsubscribing handlers
    // ========================================================================

    /// Register an event handler. The handler's filter determines which
    /// events it matches.
    ///
    /// Returns an error if a handler with the same ID already exists.
    pub fn subscribe(&mut self, handler: EventHandler) -> Result<(), EventSystemError> {
        if self.handlers.iter().any(|h| h.handler_id == handler.handler_id) {
            return Err(EventSystemError::HandlerAlreadyExists(handler.handler_id));
        }

        self.handlers.push(handler);
        // Sort handlers by priority descending (highest priority first)
        self.handlers.sort_by(|a, b| b.priority.cmp(&a.priority));
        Ok(())
    }

    /// Register an event handler with an auto-generated ID.
    /// Returns the HandlerId that was assigned.
    pub fn subscribe_auto(
        &mut self,
        filter: EventFilter,
        priority: i32,
        description: &str,
    ) -> HandlerId {
        let handler_id = HandlerId::new(self.next_handler_id);
        self.next_handler_id += 1;

        let handler = EventHandler::with_priority(handler_id, filter, priority, description);
        self.handlers.push(handler);
        self.handlers.sort_by(|a, b| b.priority.cmp(&a.priority));
        handler_id
    }

    /// Remove a handler by its ID.
    ///
    /// Returns an error if no handler with that ID exists.
    pub fn unsubscribe(&mut self, handler_id: HandlerId) -> Result<(), EventSystemError> {
        let original_len = self.handlers.len();
        self.handlers.retain(|h| h.handler_id != handler_id);
        if self.handlers.len() == original_len {
            Err(EventSystemError::HandlerNotFound(handler_id))
        } else {
            Ok(())
        }
    }

    /// Deactivate a handler without removing it. The handler can be
    /// reactivated later.
    pub fn deactivate_handler(&mut self, handler_id: HandlerId) -> Result<(), EventSystemError> {
        let handler = self
            .handlers
            .iter_mut()
            .find(|h| h.handler_id == handler_id)
            .ok_or(EventSystemError::HandlerNotFound(handler_id))?;
        handler.deactivate();
        Ok(())
    }

    /// Reactivate a previously deactivated handler.
    pub fn activate_handler(&mut self, handler_id: HandlerId) -> Result<(), EventSystemError> {
        let handler = self
            .handlers
            .iter_mut()
            .find(|h| h.handler_id == handler_id)
            .ok_or(EventSystemError::HandlerNotFound(handler_id))?;
        handler.activate();
        Ok(())
    }

    /// Get a reference to a handler by ID.
    pub fn get_handler(&self, handler_id: HandlerId) -> Option<&EventHandler> {
        self.handlers.iter().find(|h| h.handler_id == handler_id)
    }

    /// Get all registered handlers.
    pub fn handlers(&self) -> &[EventHandler] {
        &self.handlers
    }

    /// Get all handler IDs that match a given event.
    /// Returned in priority order (highest first).
    pub fn matching_handlers(&self, event: &GameEvent) -> Vec<HandlerId> {
        self.handlers
            .iter()
            .filter(|h| h.matches(event))
            .map(|h| h.handler_id)
            .collect()
    }

    /// Returns the number of registered handlers.
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }

    // ========================================================================
    // Querying the event log
    // ========================================================================

    /// Get the total number of events in the log.
    pub fn event_count(&self) -> usize {
        self.log.len()
    }

    /// Get the full event log.
    pub fn log(&self) -> &[EventLogEntry] {
        &self.log
    }

    /// Get a specific event by its EventId.
    pub fn get_event(&self, event_id: EventId) -> Option<&EventLogEntry> {
        self.log.iter().find(|e| e.event_id == event_id)
    }

    /// Get the most recently emitted event.
    pub fn latest_event(&self) -> Option<&EventLogEntry> {
        self.log.last()
    }

    /// Get the N most recently emitted events.
    pub fn latest_events(&self, n: usize) -> &[EventLogEntry] {
        let start = self.log.len().saturating_sub(n);
        &self.log[start..]
    }

    /// Get all events matching a filter.
    pub fn events_matching(&self, filter: &EventFilter) -> Vec<&EventLogEntry> {
        self.log
            .iter()
            .filter(|entry| filter.matches(&entry.event))
            .collect()
    }

    /// Get all events of a specific kind (by discriminant name).
    pub fn events_by_kind(&self, kind: &str) -> Vec<&EventLogEntry> {
        self.log
            .iter()
            .filter(|entry| entry.event.kind_name() == kind)
            .collect()
    }

    /// Get all events involving a specific unit.
    pub fn events_by_unit(&self, unit_id: UnitId) -> Vec<&EventLogEntry> {
        self.log
            .iter()
            .filter(|entry| entry.event.primary_unit() == Some(unit_id))
            .collect()
    }

    /// Get all events involving a specific player.
    pub fn events_by_player(&self, player_id: PlayerId) -> Vec<&EventLogEntry> {
        self.log
            .iter()
            .filter(|entry| entry.event.primary_player() == Some(player_id))
            .collect()
    }

    /// Get all events from a specific battle round.
    pub fn events_in_round(&self, round: BattleRound) -> Vec<&EventLogEntry> {
        self.log.iter().filter(|entry| entry.round == round).collect()
    }

    /// Get all events from a specific phase.
    pub fn events_in_phase(&self, phase: Phase) -> Vec<&EventLogEntry> {
        self.log.iter().filter(|entry| entry.phase == phase).collect()
    }

    /// Get all events from a specific round and phase combination.
    pub fn events_in_round_phase(
        &self,
        round: BattleRound,
        phase: Phase,
    ) -> Vec<&EventLogEntry> {
        self.log
            .iter()
            .filter(|entry| entry.round == round && entry.phase == phase)
            .collect()
    }

    /// Get all events that occurred after a given sequence number.
    pub fn events_since(&self, sequence: u64) -> Vec<&EventLogEntry> {
        self.log
            .iter()
            .filter(|entry| entry.sequence > sequence)
            .collect()
    }

    /// Get the most recent event of a specific kind.
    pub fn latest_event_of_kind(&self, kind: &str) -> Option<&EventLogEntry> {
        self.log
            .iter()
            .rev()
            .find(|entry| entry.event.kind_name() == kind)
    }

    /// Check if any event of a given kind has been emitted.
    pub fn has_event_of_kind(&self, kind: &str) -> bool {
        self.log.iter().any(|entry| entry.event.kind_name() == kind)
    }

    /// Count events matching a filter.
    pub fn count_matching(&self, filter: &EventFilter) -> usize {
        self.log
            .iter()
            .filter(|entry| filter.matches(&entry.event))
            .count()
    }

    /// Count events of a specific kind.
    pub fn count_by_kind(&self, kind: &str) -> usize {
        self.log
            .iter()
            .filter(|entry| entry.event.kind_name() == kind)
            .count()
    }

    /// Check if a unit has been destroyed (UnitDestroyed event exists for it).
    pub fn is_unit_destroyed(&self, unit_id: UnitId) -> bool {
        self.log.iter().any(|entry| {
            matches!(&entry.event, GameEvent::UnitDestroyed { unit, .. } if *unit == unit_id)
        })
    }

    /// Get the current battle round based on the most recent BattleRoundStarted event.
    pub fn current_round(&self) -> Option<BattleRound> {
        self.log
            .iter()
            .rev()
            .find_map(|entry| {
                if let GameEvent::BattleRoundStarted { round } = &entry.event {
                    Some(*round)
                } else {
                    None
                }
            })
    }

    /// Get the current phase based on the most recent PhaseStarted event.
    pub fn current_phase(&self) -> Option<Phase> {
        self.log
            .iter()
            .rev()
            .find_map(|entry| {
                if let GameEvent::PhaseStarted { phase, .. } = &entry.event {
                    Some(*phase)
                } else {
                    None
                }
            })
    }

    /// Get total victory points scored by a player.
    pub fn total_vp(&self, player_id: PlayerId) -> u16 {
        self.log
            .iter()
            .filter_map(|entry| {
                if let GameEvent::VictoryPointsScored { player, amount, .. } = &entry.event {
                    if *player == player_id {
                        return Some(*amount);
                    }
                }
                None
            })
            .sum()
    }

    /// Get total command points gained by a player.
    pub fn total_cp_gained(&self, player_id: PlayerId) -> u32 {
        self.log
            .iter()
            .filter_map(|entry| {
                if let GameEvent::CommandPointsGained { player, amount, .. } = &entry.event {
                    if *player == player_id {
                        return Some(*amount as u32);
                    }
                }
                None
            })
            .sum()
    }

    /// Get total command points spent by a player.
    pub fn total_cp_spent(&self, player_id: PlayerId) -> u32 {
        self.log
            .iter()
            .filter_map(|entry| {
                if let GameEvent::CommandPointsSpent { player, amount, .. } = &entry.event {
                    if *player == player_id {
                        return Some(*amount as u32);
                    }
                }
                None
            })
            .sum()
    }

    /// Get all units that have been destroyed.
    pub fn destroyed_units(&self) -> Vec<UnitId> {
        self.log
            .iter()
            .filter_map(|entry| {
                if let GameEvent::UnitDestroyed { unit, .. } = &entry.event {
                    Some(*unit)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all models that have been destroyed.
    pub fn destroyed_models(&self) -> Vec<(ModelId, UnitId)> {
        self.log
            .iter()
            .filter_map(|entry| {
                if let GameEvent::ModelDestroyed { model, unit, .. } = &entry.event {
                    Some((*model, *unit))
                } else {
                    None
                }
            })
            .collect()
    }

    // ========================================================================
    // Serialization
    // ========================================================================

    /// Serialize the event log to JSON.
    pub fn log_to_json(&self) -> Result<String, EventSystemError> {
        serde_json::to_string(&self.log)
            .map_err(|e| EventSystemError::SerializationError(e.to_string()))
    }

    /// Serialize the event log to pretty-printed JSON.
    pub fn log_to_json_pretty(&self) -> Result<String, EventSystemError> {
        serde_json::to_string_pretty(&self.log)
            .map_err(|e| EventSystemError::SerializationError(e.to_string()))
    }

    /// Deserialize an event log from JSON and replace the current log.
    pub fn load_log_from_json(&mut self, json: &str) -> Result<(), EventSystemError> {
        let log: Vec<EventLogEntry> = serde_json::from_str(json)
            .map_err(|e| EventSystemError::SerializationError(e.to_string()))?;

        // Update the sequence counter and ID generator to continue from where
        // the loaded log left off.
        if let Some(last) = log.last() {
            self.sequence_counter = last.sequence + 1;
            // The ID generator needs to be past the highest event ID in the log.
            let max_id = log.iter().map(|e| e.event_id.raw()).max().unwrap_or(0);
            self.id_generator = IdGenerator::starting_at(max_id + 1);
        }

        self.log = log;
        Ok(())
    }

    /// Clear the event log and reset all counters.
    /// This is primarily useful for testing.
    pub fn clear(&mut self) {
        self.log.clear();
        self.queue.clear();
        self.id_generator = IdGenerator::new();
        self.sequence_counter = 0;
        self.draining = false;
    }

    /// Clear only the handlers, keeping the event log intact.
    pub fn clear_handlers(&mut self) {
        self.handlers.clear();
        self.next_handler_id = 0;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_core_types::*;

    // Helper to create a basic event bus with some events
    fn test_bus() -> EventBus {
        let mut bus = EventBus::new();
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);
        let r1 = BattleRound::new(1);

        bus.emit(
            GameEvent::BattleRoundStarted { round: r1 },
            r1,
            Phase::Command,
            p0,
        );
        bus.emit(
            GameEvent::TurnStarted { player: p0, round: r1 },
            r1,
            Phase::Command,
            p0,
        );
        bus.emit(
            GameEvent::PhaseStarted { phase: Phase::Command, player: p0 },
            r1,
            Phase::Command,
            p0,
        );
        bus.emit(
            GameEvent::CommandPointsGained {
                player: p0,
                amount: 1,
                reason: CpReason::CommandPhaseGain,
            },
            r1,
            Phase::Command,
            p0,
        );
        bus.emit(
            GameEvent::PhaseEnded { phase: Phase::Command, player: p0 },
            r1,
            Phase::Command,
            p0,
        );
        bus.emit(
            GameEvent::PhaseStarted { phase: Phase::Movement, player: p0 },
            r1,
            Phase::Movement,
            p0,
        );
        bus.emit(
            GameEvent::UnitSelectedToMove { unit: UnitId::new(1) },
            r1,
            Phase::Movement,
            p0,
        );
        bus.emit(
            GameEvent::MoveCompleted {
                unit: UnitId::new(1),
                action: MovementAction::NormalMove,
                from: Position::from_inches(10, 10),
                to: Position::from_inches(16, 10),
            },
            r1,
            Phase::Movement,
            p0,
        );

        // Player 1 events
        bus.emit(
            GameEvent::TurnStarted { player: p1, round: r1 },
            r1,
            Phase::Command,
            p1,
        );
        bus.emit(
            GameEvent::CommandPointsGained {
                player: p1,
                amount: 1,
                reason: CpReason::CommandPhaseGain,
            },
            r1,
            Phase::Command,
            p1,
        );

        bus
    }

    // ========================================================================
    // GameEvent tests
    // ========================================================================

    #[test]
    fn test_game_event_kind_name() {
        let event = GameEvent::BattleRoundStarted {
            round: BattleRound::new(1),
        };
        assert_eq!(event.kind_name(), "BattleRoundStarted");

        let event = GameEvent::HitRollMade {
            attacker: UnitId::new(1),
            roll: 4,
            modified: 4,
            is_critical: false,
            result: RollResult::Success,
        };
        assert_eq!(event.kind_name(), "HitRollMade");
    }

    #[test]
    fn test_game_event_primary_unit() {
        let event = GameEvent::UnitSelectedToMove { unit: UnitId::new(5) };
        assert_eq!(event.primary_unit(), Some(UnitId::new(5)));

        let event = GameEvent::AttackSequenceStarted {
            attacker: UnitId::new(1),
            defender: UnitId::new(2),
            weapon: WeaponId::new(10),
        };
        assert_eq!(event.primary_unit(), Some(UnitId::new(1)));

        let event = GameEvent::BattleRoundStarted {
            round: BattleRound::new(1),
        };
        assert_eq!(event.primary_unit(), None);
    }

    #[test]
    fn test_game_event_primary_player() {
        let event = GameEvent::TurnStarted {
            player: PlayerId::new(1),
            round: BattleRound::new(1),
        };
        assert_eq!(event.primary_player(), Some(PlayerId::new(1)));

        let event = GameEvent::CommandPointsGained {
            player: PlayerId::new(0),
            amount: 1,
            reason: CpReason::CommandPhaseGain,
        };
        assert_eq!(event.primary_player(), Some(PlayerId::new(0)));

        let event = GameEvent::UnitSelectedToMove { unit: UnitId::new(1) };
        assert_eq!(event.primary_player(), None);
    }

    #[test]
    fn test_game_event_related_phase() {
        let event = GameEvent::PhaseStarted {
            phase: Phase::Shooting,
            player: PlayerId::new(0),
        };
        assert_eq!(event.related_phase(), Some(Phase::Shooting));

        let event = GameEvent::BattleRoundStarted {
            round: BattleRound::new(1),
        };
        assert_eq!(event.related_phase(), None);
    }

    #[test]
    fn test_game_event_is_combat_event() {
        let hit = GameEvent::HitRollMade {
            attacker: UnitId::new(1),
            roll: 4,
            modified: 4,
            is_critical: false,
            result: RollResult::Success,
        };
        assert!(hit.is_combat_event());

        let move_event = GameEvent::MoveCompleted {
            unit: UnitId::new(1),
            action: MovementAction::NormalMove,
            from: Position::from_inches(0, 0),
            to: Position::from_inches(6, 0),
        };
        assert!(!move_event.is_combat_event());
    }

    #[test]
    fn test_game_event_is_movement_event() {
        let event = GameEvent::MoveStarted {
            unit: UnitId::new(1),
            action: MovementAction::Advance,
        };
        assert!(event.is_movement_event());

        let event = GameEvent::HitRollMade {
            attacker: UnitId::new(1),
            roll: 3,
            modified: 3,
            is_critical: false,
            result: RollResult::Failure,
        };
        assert!(!event.is_movement_event());
    }

    #[test]
    fn test_game_event_is_charge_event() {
        let event = GameEvent::ChargeDeclared {
            unit: UnitId::new(1),
            targets: vec![UnitId::new(2), UnitId::new(3)],
        };
        assert!(event.is_charge_event());

        let event = GameEvent::ChargeRollMade {
            unit: UnitId::new(1),
            roll: 7,
            distance: 7,
            success: true,
        };
        assert!(event.is_charge_event());
    }

    #[test]
    fn test_game_event_is_phase_event() {
        let event = GameEvent::BattleRoundStarted {
            round: BattleRound::new(1),
        };
        assert!(event.is_phase_event());

        let event = GameEvent::BattleEnded {
            outcome: GameOutcome::Draw,
        };
        assert!(event.is_phase_event());
    }

    #[test]
    fn test_game_event_display() {
        let event = GameEvent::BattleRoundStarted {
            round: BattleRound::new(3),
        };
        let display = format!("{}", event);
        assert!(display.contains("Battle Round 3"));

        let event = GameEvent::ModelDestroyed {
            model: ModelId::new(5),
            unit: UnitId::new(2),
            cause: DestroyCause::Attack {
                attacker: UnitId::new(1),
                weapon: WeaponId::new(10),
            },
        };
        let display = format!("{}", event);
        assert!(display.contains("Model 5"));
        assert!(display.contains("unit 2"));
        assert!(display.contains("destroyed"));
    }

    #[test]
    fn test_game_event_serialization() {
        let event = GameEvent::AttackSequenceStarted {
            attacker: UnitId::new(1),
            defender: UnitId::new(2),
            weapon: WeaponId::new(10),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: GameEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn test_game_event_serialization_all_variants() {
        let p0 = PlayerId::new(0);
        let u1 = UnitId::new(1);
        let u2 = UnitId::new(2);
        let m1 = ModelId::new(1);
        let w1 = WeaponId::new(1);
        let r1 = BattleRound::new(1);

        let events = vec![
            GameEvent::BattleRoundStarted { round: r1 },
            GameEvent::TurnStarted { player: p0, round: r1 },
            GameEvent::TurnEnded { player: p0, round: r1 },
            GameEvent::PhaseStarted { phase: Phase::Command, player: p0 },
            GameEvent::PhaseEnded { phase: Phase::Command, player: p0 },
            GameEvent::CommandPointsGained {
                player: p0,
                amount: 1,
                reason: CpReason::CommandPhaseGain,
            },
            GameEvent::CommandPointsSpent {
                player: p0,
                amount: 1,
                reason: CpReason::StratagemCost(StratagemId::new(1)),
            },
            GameEvent::BattleShockTestRequired { unit: u1 },
            GameEvent::BattleShockTestResolved {
                unit: u1,
                result: BattleShockResult::Passed,
                roll: 7,
            },
            GameEvent::UnitSelectedToMove { unit: u1 },
            GameEvent::MoveStarted { unit: u1, action: MovementAction::NormalMove },
            GameEvent::MoveCompleted {
                unit: u1,
                action: MovementAction::NormalMove,
                from: Position::from_inches(0, 0),
                to: Position::from_inches(6, 0),
            },
            GameEvent::AdvanceRolled { unit: u1, roll: 4, total_move: 10 },
            GameEvent::UnitArrivedFromReserves {
                unit: u1,
                position: Position::from_inches(20, 20),
            },
            GameEvent::UnitSelectedToShoot { unit: u1 },
            GameEvent::AttackSequenceStarted {
                attacker: u1,
                defender: u2,
                weapon: w1,
            },
            GameEvent::HitRollMade {
                attacker: u1,
                roll: 4,
                modified: 3,
                is_critical: false,
                result: RollResult::Success,
            },
            GameEvent::WoundRollMade {
                attacker: u1,
                roll: 5,
                modified: 5,
                is_critical: false,
                result: RollResult::Success,
            },
            GameEvent::SaveRollMade {
                defender: u2,
                roll: 3,
                modified: 5,
                save_type: SaveType::Armour,
                result: SaveResult::Failed,
            },
            GameEvent::DamageInflicted {
                target_model: m1,
                wounds: 2,
                source: w1,
            },
            GameEvent::MortalWoundsInflicted {
                target_unit: u2,
                wounds: 3,
                source: "Devastating Wounds".to_string(),
            },
            GameEvent::FeelNoPainRollMade {
                model: m1,
                roll: 5,
                result: FnpResult::Ignored,
            },
            GameEvent::ModelDestroyed {
                model: m1,
                unit: u2,
                cause: DestroyCause::Attack { attacker: u1, weapon: w1 },
            },
            GameEvent::UnitDestroyed {
                unit: u2,
                cause: DestroyCause::Attack { attacker: u1, weapon: w1 },
            },
            GameEvent::AttackSequenceEnded { attacker: u1, defender: u2 },
            GameEvent::ChargeDeclared {
                unit: u1,
                targets: vec![u2],
            },
            GameEvent::ChargeRollMade {
                unit: u1,
                roll: 9,
                distance: 9,
                success: true,
            },
            GameEvent::ChargeCompleted { unit: u1 },
            GameEvent::ChargeFailed { unit: u1 },
            GameEvent::OverwatchFired { unit: u2, target: u1 },
            GameEvent::UnitSelectedToFight { unit: u1 },
            GameEvent::PileInMade { unit: u1, distance: 3 },
            GameEvent::FightResolved { attacker: u1, defender: u2 },
            GameEvent::ConsolidationMade { unit: u1, distance: 3 },
            GameEvent::ObjectiveControlChanged {
                objective: ObjectiveId::new(1),
                old_controller: None,
                new_controller: Some(p0),
            },
            GameEvent::ObjectiveSecured {
                objective: ObjectiveId::new(1),
                player: p0,
            },
            GameEvent::VictoryPointsScored {
                player: p0,
                amount: 5,
                source: VpSource::PrimaryObjective(ObjectiveId::new(1)),
            },
            GameEvent::StratagemUsed {
                stratagem: StratagemId::new(1),
                player: p0,
                target: EffectTarget::Unit(u1),
            },
            GameEvent::EffectApplied {
                effect_id: EffectId::new(1),
                target: EffectTarget::Unit(u1),
                duration: EffectDuration::UntilEndOfPhase,
            },
            GameEvent::EffectExpired {
                effect_id: EffectId::new(1),
                target: EffectTarget::Unit(u1),
            },
            GameEvent::BlessingsOfKhorneRolled {
                dice: vec![1, 3, 4, 5, 6],
                active_blessings: ActiveBlessings::from_blessings(vec![
                    BlessingOfKhorne::RageFuelledInvigoration,
                    BlessingOfKhorne::TotalCarnage,
                ]),
            },
            GameEvent::FightOnDeathTriggered {
                model: m1,
                roll: 4,
                success: true,
            },
            GameEvent::KaTahStanceChosen {
                unit: u1,
                stance: KaTahStance::Dacatarai,
            },
            GameEvent::BattleEnded {
                outcome: GameOutcome::Victory(p0),
            },
        ];

        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let back: GameEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(*event, back, "Failed round-trip for {:?}", event.kind_name());
        }
    }

    // ========================================================================
    // EventFilter tests
    // ========================================================================

    #[test]
    fn test_filter_all() {
        let filter = EventFilter::All;
        let event = GameEvent::BattleRoundStarted {
            round: BattleRound::new(1),
        };
        assert!(filter.matches(&event));
    }

    #[test]
    fn test_filter_by_phase() {
        let filter = EventFilter::ByPhase(Phase::Shooting);

        let shooting_start = GameEvent::PhaseStarted {
            phase: Phase::Shooting,
            player: PlayerId::new(0),
        };
        assert!(filter.matches(&shooting_start));

        let movement_start = GameEvent::PhaseStarted {
            phase: Phase::Movement,
            player: PlayerId::new(0),
        };
        assert!(!filter.matches(&movement_start));

        // Non-phase events don't match ByPhase
        let move_event = GameEvent::UnitSelectedToMove { unit: UnitId::new(1) };
        assert!(!filter.matches(&move_event));
    }

    #[test]
    fn test_filter_by_event_kind() {
        let filter = EventFilter::ByEventKind("HitRollMade".to_string());

        let hit = GameEvent::HitRollMade {
            attacker: UnitId::new(1),
            roll: 4,
            modified: 4,
            is_critical: false,
            result: RollResult::Success,
        };
        assert!(filter.matches(&hit));

        let wound = GameEvent::WoundRollMade {
            attacker: UnitId::new(1),
            roll: 3,
            modified: 3,
            is_critical: false,
            result: RollResult::Failure,
        };
        assert!(!filter.matches(&wound));
    }

    #[test]
    fn test_filter_by_unit() {
        let filter = EventFilter::ByUnit(UnitId::new(5));

        let event = GameEvent::UnitSelectedToMove { unit: UnitId::new(5) };
        assert!(filter.matches(&event));

        let event = GameEvent::UnitSelectedToMove { unit: UnitId::new(3) };
        assert!(!filter.matches(&event));
    }

    #[test]
    fn test_filter_by_player() {
        let filter = EventFilter::ByPlayer(PlayerId::new(0));

        let event = GameEvent::TurnStarted {
            player: PlayerId::new(0),
            round: BattleRound::new(1),
        };
        assert!(filter.matches(&event));

        let event = GameEvent::TurnStarted {
            player: PlayerId::new(1),
            round: BattleRound::new(1),
        };
        assert!(!filter.matches(&event));
    }

    #[test]
    fn test_filter_and() {
        let filter = EventFilter::And(vec![
            EventFilter::ByEventKind("PhaseStarted".to_string()),
            EventFilter::ByPhase(Phase::Shooting),
        ]);

        let shooting_start = GameEvent::PhaseStarted {
            phase: Phase::Shooting,
            player: PlayerId::new(0),
        };
        assert!(filter.matches(&shooting_start));

        let movement_start = GameEvent::PhaseStarted {
            phase: Phase::Movement,
            player: PlayerId::new(0),
        };
        assert!(!movement_start.related_phase().map_or(false, |p| p == Phase::Shooting));
        assert!(!filter.matches(&movement_start));
    }

    #[test]
    fn test_filter_or() {
        let filter = EventFilter::Or(vec![
            EventFilter::ByEventKind("HitRollMade".to_string()),
            EventFilter::ByEventKind("WoundRollMade".to_string()),
        ]);

        let hit = GameEvent::HitRollMade {
            attacker: UnitId::new(1),
            roll: 4,
            modified: 4,
            is_critical: false,
            result: RollResult::Success,
        };
        assert!(filter.matches(&hit));

        let wound = GameEvent::WoundRollMade {
            attacker: UnitId::new(1),
            roll: 5,
            modified: 5,
            is_critical: false,
            result: RollResult::Success,
        };
        assert!(filter.matches(&wound));

        let save = GameEvent::SaveRollMade {
            defender: UnitId::new(2),
            roll: 3,
            modified: 5,
            save_type: SaveType::Armour,
            result: SaveResult::Failed,
        };
        assert!(!filter.matches(&save));
    }

    #[test]
    fn test_filter_not() {
        let filter = EventFilter::Not(Box::new(EventFilter::ByEventKind(
            "BattleRoundStarted".to_string(),
        )));

        let round_start = GameEvent::BattleRoundStarted {
            round: BattleRound::new(1),
        };
        assert!(!filter.matches(&round_start));

        let turn_start = GameEvent::TurnStarted {
            player: PlayerId::new(0),
            round: BattleRound::new(1),
        };
        assert!(filter.matches(&turn_start));
    }

    #[test]
    fn test_filter_builder_and() {
        let filter = EventFilter::kind("PhaseStarted")
            .and(EventFilter::phase(Phase::Shooting));

        let shooting_start = GameEvent::PhaseStarted {
            phase: Phase::Shooting,
            player: PlayerId::new(0),
        };
        assert!(filter.matches(&shooting_start));
    }

    #[test]
    fn test_filter_builder_or() {
        let filter = EventFilter::kind("HitRollMade")
            .or(EventFilter::kind("WoundRollMade"));

        let hit = GameEvent::HitRollMade {
            attacker: UnitId::new(1),
            roll: 4,
            modified: 4,
            is_critical: false,
            result: RollResult::Success,
        };
        assert!(filter.matches(&hit));
    }

    #[test]
    fn test_filter_builder_negate() {
        let filter = EventFilter::kind("BattleRoundStarted").negate();

        let event = GameEvent::TurnStarted {
            player: PlayerId::new(0),
            round: BattleRound::new(1),
        };
        assert!(filter.matches(&event));
    }

    #[test]
    fn test_filter_serialization() {
        let filter = EventFilter::And(vec![
            EventFilter::ByEventKind("HitRollMade".to_string()),
            EventFilter::ByUnit(UnitId::new(5)),
        ]);
        let json = serde_json::to_string(&filter).unwrap();
        let back: EventFilter = serde_json::from_str(&json).unwrap();
        // Verify it still works
        let hit = GameEvent::HitRollMade {
            attacker: UnitId::new(5),
            roll: 4,
            modified: 4,
            is_critical: false,
            result: RollResult::Success,
        };
        assert!(back.matches(&hit));
    }

    // ========================================================================
    // EventHandler tests
    // ========================================================================

    #[test]
    fn test_handler_creation() {
        let handler = EventHandler::new(
            HandlerId::new(1),
            EventFilter::All,
            "Test handler",
        );
        assert_eq!(handler.handler_id, HandlerId::new(1));
        assert_eq!(handler.priority, 0);
        assert!(handler.active);
        assert_eq!(handler.description, "Test handler");
    }

    #[test]
    fn test_handler_with_priority() {
        let handler = EventHandler::with_priority(
            HandlerId::new(1),
            EventFilter::All,
            10,
            "High priority",
        );
        assert_eq!(handler.priority, 10);
    }

    #[test]
    fn test_handler_matches() {
        let handler = EventHandler::new(
            HandlerId::new(1),
            EventFilter::kind("HitRollMade"),
            "Hit handler",
        );

        let hit = GameEvent::HitRollMade {
            attacker: UnitId::new(1),
            roll: 4,
            modified: 4,
            is_critical: false,
            result: RollResult::Success,
        };
        assert!(handler.matches(&hit));

        let wound = GameEvent::WoundRollMade {
            attacker: UnitId::new(1),
            roll: 5,
            modified: 5,
            is_critical: false,
            result: RollResult::Success,
        };
        assert!(!handler.matches(&wound));
    }

    #[test]
    fn test_handler_deactivate() {
        let mut handler = EventHandler::new(
            HandlerId::new(1),
            EventFilter::All,
            "Test",
        );

        let event = GameEvent::BattleRoundStarted {
            round: BattleRound::new(1),
        };
        assert!(handler.matches(&event));

        handler.deactivate();
        assert!(!handler.matches(&event));

        handler.activate();
        assert!(handler.matches(&event));
    }

    // ========================================================================
    // EventBus tests
    // ========================================================================

    #[test]
    fn test_event_bus_emit() {
        let mut bus = EventBus::new();
        let event_id = bus.emit(
            GameEvent::BattleRoundStarted { round: BattleRound::new(1) },
            BattleRound::new(1),
            Phase::Command,
            PlayerId::new(0),
        );

        assert_eq!(bus.event_count(), 1);
        let entry = bus.get_event(event_id).unwrap();
        assert_eq!(entry.event.kind_name(), "BattleRoundStarted");
        assert_eq!(entry.sequence, 0);
    }

    #[test]
    fn test_event_bus_emit_multiple() {
        let bus = test_bus();
        assert_eq!(bus.event_count(), 10);
    }

    #[test]
    fn test_event_bus_latest_event() {
        let bus = test_bus();
        let latest = bus.latest_event().unwrap();
        assert_eq!(latest.event.kind_name(), "CommandPointsGained");
    }

    #[test]
    fn test_event_bus_latest_events() {
        let bus = test_bus();
        let latest_3 = bus.latest_events(3);
        assert_eq!(latest_3.len(), 3);
    }

    #[test]
    fn test_event_bus_events_by_kind() {
        let bus = test_bus();
        let turn_starts = bus.events_by_kind("TurnStarted");
        assert_eq!(turn_starts.len(), 2);
    }

    #[test]
    fn test_event_bus_events_by_unit() {
        let bus = test_bus();
        let unit1_events = bus.events_by_unit(UnitId::new(1));
        assert_eq!(unit1_events.len(), 2); // UnitSelectedToMove + MoveCompleted
    }

    #[test]
    fn test_event_bus_events_by_player() {
        let bus = test_bus();
        let p0_events = bus.events_by_player(PlayerId::new(0));
        // TurnStarted(p0), PhaseStarted(Command,p0), CP gained(p0), PhaseEnded(Command,p0), PhaseStarted(Movement,p0)
        assert_eq!(p0_events.len(), 5);
    }

    #[test]
    fn test_event_bus_events_in_round() {
        let bus = test_bus();
        let r1_events = bus.events_in_round(BattleRound::new(1));
        assert_eq!(r1_events.len(), 10);

        let r2_events = bus.events_in_round(BattleRound::new(2));
        assert_eq!(r2_events.len(), 0);
    }

    #[test]
    fn test_event_bus_events_in_phase() {
        let bus = test_bus();
        let command_events = bus.events_in_phase(Phase::Command);
        // Events 1-5 (round start, turn start p0, phase start/end command, cp gained p0)
        // plus events 9-10 (turn start p1, cp gained p1) = 7 total
        assert_eq!(command_events.len(), 7);
    }

    #[test]
    fn test_event_bus_events_matching() {
        let bus = test_bus();
        let filter = EventFilter::kind("CommandPointsGained");
        let cp_events = bus.events_matching(&filter);
        assert_eq!(cp_events.len(), 2);
    }

    #[test]
    fn test_event_bus_events_since() {
        let bus = test_bus();
        let since_5 = bus.events_since(5);
        assert_eq!(since_5.len(), 4); // sequences 6, 7, 8, 9
    }

    #[test]
    fn test_event_bus_latest_event_of_kind() {
        let bus = test_bus();
        let latest_cp = bus.latest_event_of_kind("CommandPointsGained");
        assert!(latest_cp.is_some());
        if let GameEvent::CommandPointsGained { player, .. } = &latest_cp.unwrap().event {
            assert_eq!(*player, PlayerId::new(1));
        } else {
            panic!("Expected CommandPointsGained");
        }
    }

    #[test]
    fn test_event_bus_has_event_of_kind() {
        let bus = test_bus();
        assert!(bus.has_event_of_kind("BattleRoundStarted"));
        assert!(!bus.has_event_of_kind("BattleEnded"));
    }

    #[test]
    fn test_event_bus_count_matching() {
        let bus = test_bus();
        let count = bus.count_matching(&EventFilter::kind("TurnStarted"));
        assert_eq!(count, 2);
    }

    #[test]
    fn test_event_bus_count_by_kind() {
        let bus = test_bus();
        assert_eq!(bus.count_by_kind("PhaseStarted"), 2);
        assert_eq!(bus.count_by_kind("BattleEnded"), 0);
    }

    #[test]
    fn test_event_bus_is_unit_destroyed() {
        let mut bus = EventBus::new();
        let u1 = UnitId::new(1);

        assert!(!bus.is_unit_destroyed(u1));

        bus.emit(
            GameEvent::UnitDestroyed {
                unit: u1,
                cause: DestroyCause::Attack {
                    attacker: UnitId::new(2),
                    weapon: WeaponId::new(1),
                },
            },
            BattleRound::new(1),
            Phase::Shooting,
            PlayerId::new(0),
        );

        assert!(bus.is_unit_destroyed(u1));
        assert!(!bus.is_unit_destroyed(UnitId::new(2)));
    }

    #[test]
    fn test_event_bus_current_round() {
        let bus = test_bus();
        assert_eq!(bus.current_round(), Some(BattleRound::new(1)));
    }

    #[test]
    fn test_event_bus_current_phase() {
        let bus = test_bus();
        assert_eq!(bus.current_phase(), Some(Phase::Movement));
    }

    #[test]
    fn test_event_bus_total_vp() {
        let mut bus = EventBus::new();
        let p0 = PlayerId::new(0);
        let r1 = BattleRound::new(1);

        bus.emit(
            GameEvent::VictoryPointsScored {
                player: p0,
                amount: 5,
                source: VpSource::PrimaryObjective(ObjectiveId::new(1)),
            },
            r1,
            Phase::Command,
            p0,
        );
        bus.emit(
            GameEvent::VictoryPointsScored {
                player: p0,
                amount: 3,
                source: VpSource::SecondaryObjective("Slay the Warlord".to_string()),
            },
            r1,
            Phase::Command,
            p0,
        );

        assert_eq!(bus.total_vp(p0), 8);
        assert_eq!(bus.total_vp(PlayerId::new(1)), 0);
    }

    #[test]
    fn test_event_bus_total_cp_gained_spent() {
        let bus = test_bus();
        assert_eq!(bus.total_cp_gained(PlayerId::new(0)), 1);
        assert_eq!(bus.total_cp_gained(PlayerId::new(1)), 1);
        assert_eq!(bus.total_cp_spent(PlayerId::new(0)), 0);
    }

    #[test]
    fn test_event_bus_destroyed_units() {
        let mut bus = EventBus::new();
        let r1 = BattleRound::new(1);
        let p0 = PlayerId::new(0);

        bus.emit(
            GameEvent::UnitDestroyed {
                unit: UnitId::new(3),
                cause: DestroyCause::MortalWounds { source: "Smite".to_string() },
            },
            r1,
            Phase::Shooting,
            p0,
        );
        bus.emit(
            GameEvent::UnitDestroyed {
                unit: UnitId::new(7),
                cause: DestroyCause::Attack {
                    attacker: UnitId::new(1),
                    weapon: WeaponId::new(1),
                },
            },
            r1,
            Phase::Fight,
            p0,
        );

        let destroyed = bus.destroyed_units();
        assert_eq!(destroyed.len(), 2);
        assert!(destroyed.contains(&UnitId::new(3)));
        assert!(destroyed.contains(&UnitId::new(7)));
    }

    #[test]
    fn test_event_bus_destroyed_models() {
        let mut bus = EventBus::new();
        let r1 = BattleRound::new(1);
        let p0 = PlayerId::new(0);

        bus.emit(
            GameEvent::ModelDestroyed {
                model: ModelId::new(10),
                unit: UnitId::new(2),
                cause: DestroyCause::Attack {
                    attacker: UnitId::new(1),
                    weapon: WeaponId::new(1),
                },
            },
            r1,
            Phase::Shooting,
            p0,
        );

        let destroyed = bus.destroyed_models();
        assert_eq!(destroyed.len(), 1);
        assert_eq!(destroyed[0], (ModelId::new(10), UnitId::new(2)));
    }

    // ========================================================================
    // EventBus handler tests
    // ========================================================================

    #[test]
    fn test_event_bus_subscribe() {
        let mut bus = EventBus::new();
        let handler = EventHandler::new(
            HandlerId::new(1),
            EventFilter::All,
            "Test",
        );
        bus.subscribe(handler).unwrap();
        assert_eq!(bus.handler_count(), 1);
    }

    #[test]
    fn test_event_bus_subscribe_duplicate() {
        let mut bus = EventBus::new();
        let handler1 = EventHandler::new(HandlerId::new(1), EventFilter::All, "Test 1");
        let handler2 = EventHandler::new(HandlerId::new(1), EventFilter::All, "Test 2");

        bus.subscribe(handler1).unwrap();
        let result = bus.subscribe(handler2);
        assert!(result.is_err());
    }

    #[test]
    fn test_event_bus_subscribe_auto() {
        let mut bus = EventBus::new();
        let id1 = bus.subscribe_auto(EventFilter::All, 0, "Handler 1");
        let id2 = bus.subscribe_auto(EventFilter::All, 0, "Handler 2");

        assert_ne!(id1, id2);
        assert_eq!(bus.handler_count(), 2);
    }

    #[test]
    fn test_event_bus_unsubscribe() {
        let mut bus = EventBus::new();
        let handler = EventHandler::new(HandlerId::new(1), EventFilter::All, "Test");
        bus.subscribe(handler).unwrap();
        assert_eq!(bus.handler_count(), 1);

        bus.unsubscribe(HandlerId::new(1)).unwrap();
        assert_eq!(bus.handler_count(), 0);
    }

    #[test]
    fn test_event_bus_unsubscribe_not_found() {
        let mut bus = EventBus::new();
        let result = bus.unsubscribe(HandlerId::new(999));
        assert!(result.is_err());
    }

    #[test]
    fn test_event_bus_deactivate_handler() {
        let mut bus = EventBus::new();
        let handler = EventHandler::new(HandlerId::new(1), EventFilter::All, "Test");
        bus.subscribe(handler).unwrap();

        let event = GameEvent::BattleRoundStarted {
            round: BattleRound::new(1),
        };
        assert_eq!(bus.matching_handlers(&event).len(), 1);

        bus.deactivate_handler(HandlerId::new(1)).unwrap();
        assert_eq!(bus.matching_handlers(&event).len(), 0);

        bus.activate_handler(HandlerId::new(1)).unwrap();
        assert_eq!(bus.matching_handlers(&event).len(), 1);
    }

    #[test]
    fn test_event_bus_matching_handlers() {
        let mut bus = EventBus::new();

        let h1 = EventHandler::new(
            HandlerId::new(1),
            EventFilter::kind("HitRollMade"),
            "Hit handler",
        );
        let h2 = EventHandler::new(
            HandlerId::new(2),
            EventFilter::kind("WoundRollMade"),
            "Wound handler",
        );
        let h3 = EventHandler::new(
            HandlerId::new(3),
            EventFilter::All,
            "All handler",
        );

        bus.subscribe(h1).unwrap();
        bus.subscribe(h2).unwrap();
        bus.subscribe(h3).unwrap();

        let hit = GameEvent::HitRollMade {
            attacker: UnitId::new(1),
            roll: 4,
            modified: 4,
            is_critical: false,
            result: RollResult::Success,
        };

        let matching = bus.matching_handlers(&hit);
        assert_eq!(matching.len(), 2); // h1 (HitRollMade) and h3 (All)
        assert!(matching.contains(&HandlerId::new(1)));
        assert!(matching.contains(&HandlerId::new(3)));
        assert!(!matching.contains(&HandlerId::new(2)));
    }

    #[test]
    fn test_event_bus_handler_priority_ordering() {
        let mut bus = EventBus::new();

        let h_low = EventHandler::with_priority(
            HandlerId::new(1),
            EventFilter::All,
            0,
            "Low",
        );
        let h_high = EventHandler::with_priority(
            HandlerId::new(2),
            EventFilter::All,
            100,
            "High",
        );
        let h_mid = EventHandler::with_priority(
            HandlerId::new(3),
            EventFilter::All,
            50,
            "Mid",
        );

        bus.subscribe(h_low).unwrap();
        bus.subscribe(h_high).unwrap();
        bus.subscribe(h_mid).unwrap();

        let event = GameEvent::BattleRoundStarted {
            round: BattleRound::new(1),
        };

        let matching = bus.matching_handlers(&event);
        assert_eq!(matching.len(), 3);
        // Highest priority first
        assert_eq!(matching[0], HandlerId::new(2)); // priority 100
        assert_eq!(matching[1], HandlerId::new(3)); // priority 50
        assert_eq!(matching[2], HandlerId::new(1)); // priority 0
    }

    #[test]
    fn test_event_bus_get_handler() {
        let mut bus = EventBus::new();
        let handler = EventHandler::new(HandlerId::new(42), EventFilter::All, "The answer");
        bus.subscribe(handler).unwrap();

        let retrieved = bus.get_handler(HandlerId::new(42));
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().description, "The answer");

        assert!(bus.get_handler(HandlerId::new(99)).is_none());
    }

    // ========================================================================
    // Event queue tests
    // ========================================================================

    #[test]
    fn test_event_bus_enqueue_and_drain() {
        let mut bus = EventBus::new();
        let r1 = BattleRound::new(1);
        let p0 = PlayerId::new(0);

        bus.enqueue(
            GameEvent::BattleRoundStarted { round: r1 },
            r1,
            Phase::Command,
            p0,
        )
        .unwrap();
        bus.enqueue(
            GameEvent::TurnStarted { player: p0, round: r1 },
            r1,
            Phase::Command,
            p0,
        )
        .unwrap();

        assert!(bus.has_queued_events());
        assert_eq!(bus.queued_count(), 2);
        assert_eq!(bus.event_count(), 0); // not yet logged

        let drained = bus.drain_queue().unwrap();
        assert_eq!(drained, 2);
        assert_eq!(bus.event_count(), 2);
        assert!(!bus.has_queued_events());
    }

    #[test]
    fn test_event_bus_queue_overflow() {
        let mut bus = EventBus::with_max_queue_depth(5);
        let r1 = BattleRound::new(1);
        let p0 = PlayerId::new(0);

        for i in 0..5 {
            bus.enqueue(
                GameEvent::BattleRoundStarted {
                    round: BattleRound::new(i as u8 + 1),
                },
                r1,
                Phase::Command,
                p0,
            )
            .unwrap();
        }

        // 6th enqueue should fail
        let result = bus.enqueue(
            GameEvent::BattleRoundStarted {
                round: BattleRound::new(6),
            },
            r1,
            Phase::Command,
            p0,
        );
        assert!(result.is_err());
    }

    // ========================================================================
    // ReactionWindow tests
    // ========================================================================

    #[test]
    fn test_reaction_window_creation() {
        let window = ReactionWindow::new(
            ReactionWindowType::Overwatch,
            Phase::Charge,
            ReactionTiming::During,
            PlayerId::new(1),
        );

        assert_eq!(window.window_type, ReactionWindowType::Overwatch);
        assert_eq!(window.phase, Phase::Charge);
        assert_eq!(window.owner, PlayerId::new(1));
        assert!(window.legal_actions.is_empty());
        assert!(!window.has_actions());
        assert!(window.must_decline());
    }

    #[test]
    fn test_reaction_window_with_actions() {
        let mut window = ReactionWindow::new(
            ReactionWindowType::Overwatch,
            Phase::Charge,
            ReactionTiming::During,
            PlayerId::new(1),
        );

        window.add_action(ReactionAction::FireOverwatch(UnitId::new(3)));
        window.add_action(ReactionAction::Decline);

        assert!(window.has_actions());
        assert!(!window.must_decline());
        assert_eq!(window.legal_actions.len(), 2);
    }

    #[test]
    fn test_reaction_window_must_decline() {
        let mut window = ReactionWindow::new(
            ReactionWindowType::Stratagem,
            Phase::Shooting,
            ReactionTiming::Before,
            PlayerId::new(0),
        );

        // Empty actions -> must decline
        assert!(window.must_decline());

        // Only decline action -> must decline
        window.add_action(ReactionAction::Decline);
        assert!(window.must_decline());

        // Real action -> not forced to decline
        window.add_action(ReactionAction::UseStratagem(StratagemId::new(1)));
        assert!(!window.must_decline());
    }

    #[test]
    fn test_reaction_window_builder() {
        let window = ReactionWindow::new(
            ReactionWindowType::Stratagem,
            Phase::Shooting,
            ReactionTiming::After,
            PlayerId::new(0),
        )
        .with_trigger(EventId::new(5))
        .with_deadline(30);

        assert_eq!(window.triggering_event_id, Some(EventId::new(5)));
        assert_eq!(window.deadline_seconds, 30);
        assert!(window.auto_decline);
    }

    #[test]
    fn test_reaction_window_without_auto_decline() {
        let window = ReactionWindow::new(
            ReactionWindowType::Decision,
            Phase::Movement,
            ReactionTiming::PhaseStart,
            PlayerId::new(0),
        )
        .without_auto_decline();

        assert!(!window.auto_decline);
    }

    #[test]
    fn test_reaction_window_serialization() {
        let mut window = ReactionWindow::new(
            ReactionWindowType::Overwatch,
            Phase::Charge,
            ReactionTiming::During,
            PlayerId::new(1),
        );
        window.add_action(ReactionAction::FireOverwatch(UnitId::new(3)));
        window.add_action(ReactionAction::Decline);

        let json = serde_json::to_string(&window).unwrap();
        let back: ReactionWindow = serde_json::from_str(&json).unwrap();
        assert_eq!(window, back);
    }

    // ========================================================================
    // Serialization / log persistence tests
    // ========================================================================

    #[test]
    fn test_event_bus_log_to_json() {
        let bus = test_bus();
        let json = bus.log_to_json().unwrap();
        assert!(!json.is_empty());

        // Should be valid JSON
        let parsed: Vec<EventLogEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 10);
    }

    #[test]
    fn test_event_bus_log_to_json_pretty() {
        let bus = test_bus();
        let json = bus.log_to_json_pretty().unwrap();
        assert!(json.contains('\n')); // Pretty-printed has newlines
    }

    #[test]
    fn test_event_bus_load_log_from_json() {
        let bus = test_bus();
        let json = bus.log_to_json().unwrap();

        let mut bus2 = EventBus::new();
        bus2.load_log_from_json(&json).unwrap();
        assert_eq!(bus2.event_count(), 10);

        // New events should get IDs after the loaded ones
        let new_id = bus2.emit(
            GameEvent::BattleEnded { outcome: GameOutcome::Draw },
            BattleRound::new(1),
            Phase::GameEnd,
            PlayerId::new(0),
        );
        assert_eq!(bus2.event_count(), 11);
        // The new event should have a higher ID than any loaded event
        let max_loaded_id = bus.log().last().unwrap().event_id.raw();
        assert!(new_id.raw() > max_loaded_id);
    }

    #[test]
    fn test_event_bus_clear() {
        let mut bus = test_bus();
        assert!(bus.event_count() > 0);
        bus.clear();
        assert_eq!(bus.event_count(), 0);
        assert!(!bus.has_queued_events());
    }

    #[test]
    fn test_event_bus_clear_handlers() {
        let mut bus = EventBus::new();
        bus.subscribe_auto(EventFilter::All, 0, "Handler 1");
        bus.subscribe_auto(EventFilter::All, 0, "Handler 2");
        assert_eq!(bus.handler_count(), 2);

        bus.clear_handlers();
        assert_eq!(bus.handler_count(), 0);
    }

    // ========================================================================
    // EventLogEntry tests
    // ========================================================================

    #[test]
    fn test_event_log_entry_serialization() {
        let entry = EventLogEntry {
            event_id: EventId::new(42),
            event: GameEvent::BattleRoundStarted {
                round: BattleRound::new(1),
            },
            round: BattleRound::new(1),
            phase: Phase::Command,
            active_player: PlayerId::new(0),
            sequence: 0,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let back: EventLogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, back);
    }

    // ========================================================================
    // Faction-specific event tests
    // ========================================================================

    #[test]
    fn test_blessings_of_khorne_event() {
        let mut bus = EventBus::new();
        let r1 = BattleRound::new(1);
        let p0 = PlayerId::new(0);

        bus.emit(
            GameEvent::BlessingsOfKhorneRolled {
                dice: vec![1, 2, 3, 5, 6],
                active_blessings: ActiveBlessings::from_blessings(vec![
                    BlessingOfKhorne::RageFuelledInvigoration,
                    BlessingOfKhorne::MartialExcellence,
                ]),
            },
            r1,
            Phase::Command,
            p0,
        );

        let events = bus.events_by_kind("BlessingsOfKhorneRolled");
        assert_eq!(events.len(), 1);

        if let GameEvent::BlessingsOfKhorneRolled { dice, active_blessings } =
            &events[0].event
        {
            assert_eq!(dice.len(), 5);
            assert!(active_blessings.has(&BlessingOfKhorne::RageFuelledInvigoration));
            assert!(active_blessings.has(&BlessingOfKhorne::MartialExcellence));
            assert!(!active_blessings.has(&BlessingOfKhorne::TotalCarnage));
        } else {
            panic!("Expected BlessingsOfKhorneRolled");
        }
    }

    #[test]
    fn test_ka_tah_stance_event() {
        let mut bus = EventBus::new();
        let r1 = BattleRound::new(1);
        let p0 = PlayerId::new(0);

        bus.emit(
            GameEvent::KaTahStanceChosen {
                unit: UnitId::new(1),
                stance: KaTahStance::Dacatarai,
            },
            r1,
            Phase::Command,
            p0,
        );

        let events = bus.events_by_kind("KaTahStanceChosen");
        assert_eq!(events.len(), 1);

        if let GameEvent::KaTahStanceChosen { unit, stance } = &events[0].event {
            assert_eq!(*unit, UnitId::new(1));
            assert_eq!(*stance, KaTahStance::Dacatarai);
        } else {
            panic!("Expected KaTahStanceChosen");
        }
    }

    #[test]
    fn test_fight_on_death_event() {
        let mut bus = EventBus::new();
        let r1 = BattleRound::new(1);
        let p0 = PlayerId::new(0);

        bus.emit(
            GameEvent::FightOnDeathTriggered {
                model: ModelId::new(5),
                roll: 4,
                success: true,
            },
            r1,
            Phase::Fight,
            p0,
        );

        let events = bus.events_by_kind("FightOnDeathTriggered");
        assert_eq!(events.len(), 1);

        if let GameEvent::FightOnDeathTriggered { model, roll, success } = &events[0].event {
            assert_eq!(*model, ModelId::new(5));
            assert_eq!(*roll, 4);
            assert!(*success);
        } else {
            panic!("Expected FightOnDeathTriggered");
        }
    }

    // ========================================================================
    // Helper type tests
    // ========================================================================

    #[test]
    fn test_handler_id() {
        let id = HandlerId::new(42);
        assert_eq!(id.raw(), 42);
        assert_eq!(format!("{}", id), "Handler(42)");
        assert_eq!(HandlerId::from(42u32), id);
    }

    #[test]
    fn test_effect_id() {
        let id = EffectId::new(10);
        assert_eq!(id.raw(), 10);
        assert_eq!(format!("{}", id), "Effect(10)");
        assert_eq!(EffectId::from(10u32), id);
    }

    #[test]
    fn test_ka_tah_stance_display() {
        // Source: Custodes.md - Martial Ka'tah (only Dacatarai and Rendax)
        assert_eq!(format!("{}", KaTahStance::Dacatarai), "Dacatarai");
        assert_eq!(format!("{}", KaTahStance::Rendax), "Rendax");
    }

    #[test]
    fn test_active_blessings() {
        let mut blessings = ActiveBlessings::new();
        assert!(blessings.active.is_empty());

        blessings.add(BlessingOfKhorne::TotalCarnage);
        blessings.add(BlessingOfKhorne::MartialExcellence);
        assert!(blessings.has(&BlessingOfKhorne::TotalCarnage));
        assert!(blessings.has(&BlessingOfKhorne::MartialExcellence));
        assert!(!blessings.has(&BlessingOfKhorne::RageFuelledInvigoration));

        // No duplicates
        blessings.add(BlessingOfKhorne::TotalCarnage);
        assert_eq!(blessings.active.len(), 2);

        blessings.clear();
        assert!(blessings.active.is_empty());
    }

    #[test]
    fn test_cp_reason_display() {
        assert_eq!(
            format!("{}", CpReason::CommandPhaseGain),
            "Command Phase gain"
        );
        assert_eq!(
            format!("{}", CpReason::StratagemCost(StratagemId::new(5))),
            "Stratagem 5"
        );
        assert_eq!(format!("{}", CpReason::AbilityRefund), "Ability refund");
        assert_eq!(
            format!("{}", CpReason::Custom("Test".to_string())),
            "Test"
        );
    }

    #[test]
    fn test_destroy_cause_display() {
        let cause = DestroyCause::Attack {
            attacker: UnitId::new(1),
            weapon: WeaponId::new(5),
        };
        let display = format!("{}", cause);
        assert!(display.contains("unit 1"));
        assert!(display.contains("weapon 5"));

        let cause = DestroyCause::MortalWounds {
            source: "Devastating Wounds".to_string(),
        };
        assert!(format!("{}", cause).contains("Devastating Wounds"));
    }

    #[test]
    fn test_vp_source_display() {
        let source = VpSource::PrimaryObjective(ObjectiveId::new(2));
        assert!(format!("{}", source).contains("Primary objective 2"));

        let source = VpSource::SecondaryObjective("Slay the Warlord".to_string());
        assert!(format!("{}", source).contains("Slay the Warlord"));
    }

    #[test]
    fn test_effect_target_display() {
        assert_eq!(
            format!("{}", EffectTarget::Unit(UnitId::new(1))),
            "Unit(1)"
        );
        assert_eq!(
            format!("{}", EffectTarget::Player(PlayerId::new(0))),
            "Player(0)"
        );
        assert_eq!(format!("{}", EffectTarget::Global), "Global");
    }

    #[test]
    fn test_reaction_action_display() {
        assert_eq!(
            format!("{}", ReactionAction::UseStratagem(StratagemId::new(1))),
            "Use stratagem 1"
        );
        assert_eq!(
            format!("{}", ReactionAction::Decline),
            "Decline"
        );
    }

    #[test]
    fn test_roll_result() {
        assert_eq!(format!("{}", RollResult::Success), "Success");
        assert_eq!(format!("{}", RollResult::Failure), "Failure");
    }

    #[test]
    fn test_fnp_result() {
        assert_eq!(format!("{}", FnpResult::Ignored), "Ignored");
        assert_eq!(format!("{}", FnpResult::Suffered), "Suffered");
    }

    #[test]
    fn test_blessing_of_khorne_display() {
        // Source: Frenzied_Reavers.md - Blessings of Khorne
        assert_eq!(
            format!("{}", BlessingOfKhorne::RageFuelledInvigoration),
            "Rage-fuelled Invigoration"
        );
        assert_eq!(
            format!("{}", BlessingOfKhorne::TotalCarnage),
            "Total Carnage"
        );
        assert_eq!(
            format!("{}", BlessingOfKhorne::MartialExcellence),
            "Martial Excellence"
        );
    }

    // ========================================================================
    // Integration-style tests
    // ========================================================================

    #[test]
    fn test_full_attack_sequence() {
        let mut bus = EventBus::new();
        let r1 = BattleRound::new(1);
        let p0 = PlayerId::new(0);
        let attacker = UnitId::new(1);
        let defender = UnitId::new(2);
        let weapon = WeaponId::new(10);
        let target_model = ModelId::new(5);

        // Subscribe a handler for combat events
        let combat_handler = EventHandler::new(
            HandlerId::new(1),
            EventFilter::Or(vec![
                EventFilter::kind("AttackSequenceStarted"),
                EventFilter::kind("HitRollMade"),
                EventFilter::kind("WoundRollMade"),
                EventFilter::kind("SaveRollMade"),
                EventFilter::kind("DamageInflicted"),
                EventFilter::kind("AttackSequenceEnded"),
            ]),
            "Combat tracker",
        );
        bus.subscribe(combat_handler).unwrap();

        // Full attack sequence
        bus.emit(
            GameEvent::UnitSelectedToShoot { unit: attacker },
            r1,
            Phase::Shooting,
            p0,
        );
        bus.emit(
            GameEvent::AttackSequenceStarted { attacker, defender, weapon },
            r1,
            Phase::Shooting,
            p0,
        );
        bus.emit(
            GameEvent::HitRollMade {
                attacker,
                roll: 4,
                modified: 4,
                is_critical: false,
                result: RollResult::Success,
            },
            r1,
            Phase::Shooting,
            p0,
        );
        bus.emit(
            GameEvent::WoundRollMade {
                attacker,
                roll: 5,
                modified: 5,
                is_critical: false,
                result: RollResult::Success,
            },
            r1,
            Phase::Shooting,
            p0,
        );
        bus.emit(
            GameEvent::SaveRollMade {
                defender,
                roll: 2,
                modified: 4,
                save_type: SaveType::Armour,
                result: SaveResult::Failed,
            },
            r1,
            Phase::Shooting,
            p0,
        );
        bus.emit(
            GameEvent::DamageInflicted {
                target_model,
                wounds: 2,
                source: weapon,
            },
            r1,
            Phase::Shooting,
            p0,
        );
        bus.emit(
            GameEvent::ModelDestroyed {
                model: target_model,
                unit: defender,
                cause: DestroyCause::Attack { attacker, weapon },
            },
            r1,
            Phase::Shooting,
            p0,
        );
        bus.emit(
            GameEvent::AttackSequenceEnded { attacker, defender },
            r1,
            Phase::Shooting,
            p0,
        );

        // Verify
        assert_eq!(bus.event_count(), 8);

        // All combat events should match the handler
        let combat_events: Vec<_> = bus
            .log()
            .iter()
            .filter(|e| {
                bus.matching_handlers(&e.event)
                    .contains(&HandlerId::new(1))
            })
            .collect();
        // AttackSequenceStarted, HitRollMade, WoundRollMade, SaveRollMade, DamageInflicted, AttackSequenceEnded
        assert_eq!(combat_events.len(), 6);

        // Check attacker's events
        let attacker_events = bus.events_by_unit(attacker);
        // UnitSelectedToShoot, AttackSequenceStarted, HitRollMade, WoundRollMade, AttackSequenceEnded
        assert_eq!(attacker_events.len(), 5);

        // Check destroyed models
        assert_eq!(bus.destroyed_models().len(), 1);
    }

    #[test]
    fn test_full_charge_sequence() {
        let mut bus = EventBus::new();
        let r1 = BattleRound::new(1);
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);
        let charger = UnitId::new(1);
        let target = UnitId::new(2);

        bus.emit(
            GameEvent::PhaseStarted { phase: Phase::Charge, player: p0 },
            r1,
            Phase::Charge,
            p0,
        );
        bus.emit(
            GameEvent::ChargeDeclared {
                unit: charger,
                targets: vec![target],
            },
            r1,
            Phase::Charge,
            p0,
        );
        bus.emit(
            GameEvent::OverwatchFired { unit: target, target: charger },
            r1,
            Phase::Charge,
            p1,
        );
        bus.emit(
            GameEvent::ChargeRollMade {
                unit: charger,
                roll: 9,
                distance: 9,
                success: true,
            },
            r1,
            Phase::Charge,
            p0,
        );
        bus.emit(
            GameEvent::ChargeCompleted { unit: charger },
            r1,
            Phase::Charge,
            p0,
        );

        // Verify charge events
        let charge_events: Vec<_> = bus
            .log()
            .iter()
            .filter(|e| e.event.is_charge_event())
            .collect();
        assert_eq!(charge_events.len(), 4); // Declared, Overwatch, Roll, Completed

        // Check the charger's events
        let charger_events = bus.events_by_unit(charger);
        assert_eq!(charger_events.len(), 3); // Declared, Roll, Completed
    }

    #[test]
    fn test_events_in_round_phase() {
        let bus = test_bus();
        let events = bus.events_in_round_phase(BattleRound::new(1), Phase::Movement);
        // PhaseStarted(Movement, p0), UnitSelectedToMove, MoveCompleted = 3
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_default_event_bus() {
        let bus = EventBus::default();
        assert_eq!(bus.event_count(), 0);
        assert_eq!(bus.handler_count(), 0);
    }

    #[test]
    fn test_event_bus_sequence_monotonic() {
        let mut bus = EventBus::new();
        let r1 = BattleRound::new(1);
        let p0 = PlayerId::new(0);

        for _ in 0..10 {
            bus.emit(
                GameEvent::BattleRoundStarted { round: r1 },
                r1,
                Phase::Command,
                p0,
            );
        }

        // Sequences should be monotonically increasing
        for i in 1..bus.log().len() {
            assert!(bus.log()[i].sequence > bus.log()[i - 1].sequence);
        }
    }

    #[test]
    fn test_event_bus_event_ids_unique() {
        let mut bus = EventBus::new();
        let r1 = BattleRound::new(1);
        let p0 = PlayerId::new(0);

        let mut ids = Vec::new();
        for _ in 0..20 {
            let id = bus.emit(
                GameEvent::BattleRoundStarted { round: r1 },
                r1,
                Phase::Command,
                p0,
            );
            ids.push(id);
        }

        // All IDs should be unique
        let unique_count = {
            let mut sorted = ids.clone();
            sorted.sort();
            sorted.dedup();
            sorted.len()
        };
        assert_eq!(unique_count, 20);
    }

    #[test]
    fn test_event_bus_handlers_list() {
        let mut bus = EventBus::new();
        bus.subscribe_auto(EventFilter::All, 10, "First");
        bus.subscribe_auto(EventFilter::All, 20, "Second");
        bus.subscribe_auto(EventFilter::All, 5, "Third");

        let handlers = bus.handlers();
        assert_eq!(handlers.len(), 3);
        // Should be sorted by priority descending
        assert_eq!(handlers[0].priority, 20);
        assert_eq!(handlers[1].priority, 10);
        assert_eq!(handlers[2].priority, 5);
    }
}

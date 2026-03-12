//! WH40K Engine - ContentSchema crate
//!
//! Defines the data schemas for faction content. These types are the templates
//! that faction YAML/JSON files deserialize into. They describe datasheets,
//! weapon profiles, abilities, stratagems, enhancements, secondary objectives,
//! missions, and complete faction definitions.
//!
//! The key design principle is the RulePrimitive DSL: a set of composable
//! building blocks that faction content files assemble declaratively to express
//! rules without custom code.
//!
//! Source: implementation_v3.md Section 6.3 (Layer B - Content Schema)
//! Source: Custodes.md, Frenzied_Reavers.md - Faction data

use serde::{Deserialize, Serialize};
use wh40k_core_types::{
    ArmorPenetration, ArmorSave, AttackCount, BaseSize, ContentPackId, Damage,
    DatasheetId, EffectDuration, EnhancementId, FactionId, Inches, InvulnerableSave, Keyword,
    KeywordSet, Leadership, MissionId, MoveCharacteristic, ObjectiveControl, Phase, PlayerId,
    SecondaryObjectiveId, Skill, Strength, SubPhase, Toughness, WeaponAbility, WeaponType,
    Wounds,
};

// ─── DeploymentMapId ────────────────────────────────────────────────────────

/// Identifies a deployment map layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeploymentMapId(pub u32);

impl DeploymentMapId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl From<u32> for DeploymentMapId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<DeploymentMapId> for u32 {
    fn from(id: DeploymentMapId) -> u32 {
        id.0
    }
}

// ─── UnitSizeSpec ───────────────────────────────────────────────────────────

/// Specifies the number of models in a unit.
/// Can be a fixed count or a range (for variable-size units).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitSizeSpec {
    /// Fixed number of models (e.g., Tristraen = Fixed(1))
    Fixed(u8),
    /// Variable unit size with min and max (e.g., Berzerkers min=5, max=10)
    Range { min: u8, max: u8 },
}

impl UnitSizeSpec {
    pub fn min_models(self) -> u8 {
        match self {
            UnitSizeSpec::Fixed(n) => n,
            UnitSizeSpec::Range { min, .. } => min,
        }
    }

    pub fn max_models(self) -> u8 {
        match self {
            UnitSizeSpec::Fixed(n) => n,
            UnitSizeSpec::Range { max, .. } => max,
        }
    }
}

// ─── WeaponAbilityDef ───────────────────────────────────────────────────────

/// Serializable weapon ability specification for content authoring.
/// Mirrors WeaponAbility from core_types but designed for YAML/JSON authoring
/// with string-based keyword references for Anti- abilities.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeaponAbilityDef {
    /// Can shoot after Advancing. Hit roll modified by -1.
    Assault,
    /// +1 to Hit rolls if bearer Remained Stationary.
    Heavy,
    /// +N attacks when within half range.
    RapidFire(u8),
    /// Can be used in Engagement Range.
    Pistol,
    /// +1 Attack per 5 models in target unit.
    Blast,
    /// Automatically hits (no Hit roll).
    Torrent,
    /// Re-roll failed Wound rolls.
    TwinLinked,
    /// Critical Hit auto-wounds (skip Wound roll).
    LethalHits,
    /// Critical Hit generates N extra hits.
    SustainedHits(u8),
    /// Critical Wound deals mortal wounds instead of normal damage.
    DevastatingWounds,
    /// After attacking, roll D6 per model with Hazardous weapon. On 1, suffer mortal wounds.
    Hazardous,
    /// Can target CHARACTERs in Attached units directly.
    Precision,
    /// Critical Wound on threshold+ when targeting units with specified keyword.
    Anti { keyword: Keyword, threshold: u8 },
    /// Can target non-visible units. -1 to Hit, +1 to enemy save.
    IndirectFire,
    /// Target cannot benefit from cover.
    IgnoresCover,
    /// +N to Damage when within half range.
    Melta(u8),
    /// +1 to Wound roll if bearer Charged this turn.
    Lance,
    /// Extra attacks don't prevent attacking with other weapons.
    ExtraAttacks,
}

impl WeaponAbilityDef {
    /// Convert to the runtime WeaponAbility type.
    pub fn to_runtime(&self) -> WeaponAbility {
        match self {
            WeaponAbilityDef::Assault => WeaponAbility::Assault,
            WeaponAbilityDef::Heavy => WeaponAbility::Heavy,
            WeaponAbilityDef::RapidFire(n) => WeaponAbility::RapidFire(*n),
            WeaponAbilityDef::Pistol => WeaponAbility::Pistol,
            WeaponAbilityDef::Blast => WeaponAbility::Blast,
            WeaponAbilityDef::Torrent => WeaponAbility::Torrent,
            WeaponAbilityDef::TwinLinked => WeaponAbility::TwinLinked,
            WeaponAbilityDef::LethalHits => WeaponAbility::LethalHits,
            WeaponAbilityDef::SustainedHits(n) => WeaponAbility::SustainedHits(*n),
            WeaponAbilityDef::DevastatingWounds => WeaponAbility::DevastatingWounds,
            WeaponAbilityDef::Hazardous => WeaponAbility::Hazardous,
            WeaponAbilityDef::Precision => WeaponAbility::Precision,
            WeaponAbilityDef::Anti { keyword, threshold } => {
                WeaponAbility::Anti(*keyword, *threshold)
            }
            WeaponAbilityDef::IndirectFire => WeaponAbility::IndirectFire,
            WeaponAbilityDef::IgnoresCover => WeaponAbility::IgnoresCover,
            WeaponAbilityDef::Melta(n) => WeaponAbility::Melta(*n),
            WeaponAbilityDef::Lance => WeaponAbility::Lance,
            WeaponAbilityDef::ExtraAttacks => WeaponAbility::ExtraAttacks,
        }
    }

    /// Create from a runtime WeaponAbility.
    pub fn from_runtime(ability: &WeaponAbility) -> Self {
        match ability {
            WeaponAbility::Assault => WeaponAbilityDef::Assault,
            WeaponAbility::Heavy => WeaponAbilityDef::Heavy,
            WeaponAbility::RapidFire(n) => WeaponAbilityDef::RapidFire(*n),
            WeaponAbility::Pistol => WeaponAbilityDef::Pistol,
            WeaponAbility::Blast => WeaponAbilityDef::Blast,
            WeaponAbility::Torrent => WeaponAbilityDef::Torrent,
            WeaponAbility::TwinLinked => WeaponAbilityDef::TwinLinked,
            WeaponAbility::LethalHits => WeaponAbilityDef::LethalHits,
            WeaponAbility::SustainedHits(n) => WeaponAbilityDef::SustainedHits(*n),
            WeaponAbility::DevastatingWounds => WeaponAbilityDef::DevastatingWounds,
            WeaponAbility::Hazardous => WeaponAbilityDef::Hazardous,
            WeaponAbility::Precision => WeaponAbilityDef::Precision,
            WeaponAbility::Anti(keyword, threshold) => WeaponAbilityDef::Anti {
                keyword: *keyword,
                threshold: *threshold,
            },
            WeaponAbility::IndirectFire => WeaponAbilityDef::IndirectFire,
            WeaponAbility::IgnoresCover => WeaponAbilityDef::IgnoresCover,
            WeaponAbility::Melta(n) => WeaponAbilityDef::Melta(*n),
            WeaponAbility::Lance => WeaponAbilityDef::Lance,
            WeaponAbility::ExtraAttacks => WeaponAbilityDef::ExtraAttacks,
        }
    }
}

// ─── WeaponProfileSchema ────────────────────────────────────────────────────

/// Weapon template for content authoring.
/// Defines a weapon profile as it appears in a datasheet.
///
/// Source: 40k_revised.md - Weapon characteristics
/// Source: Custodes.md, Frenzied_Reavers.md - Weapon profiles
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WeaponProfileSchema {
    /// Display name (e.g., "Sentinel blade", "Vaultswords - Behemor")
    pub name: String,
    /// Ranged or Melee
    pub weapon_type: WeaponType,
    /// Range in inches (0 for melee weapons)
    pub range: Inches,
    /// Number of attacks (can be variable, e.g., D6)
    pub attacks: AttackCount,
    /// Ballistic Skill (ranged) or Weapon Skill (melee)
    pub skill: Skill,
    /// Strength characteristic
    pub strength: Strength,
    /// Armor Penetration
    pub ap: ArmorPenetration,
    /// Damage characteristic (can be variable, e.g., D3)
    pub damage: Damage,
    /// Weapon abilities (Assault, Pistol, Sustained Hits, etc.)
    pub abilities: Vec<WeaponAbilityDef>,
}

// ─── ModelLoadout ───────────────────────────────────────────────────────────

/// Describes which models in a unit carry which weapons.
/// Used to define the fixed loadout for Combat Patrol units.
///
/// Example: Custodian Wardens - 1 model with Castellan axe, 2 with Guardian spear
/// Example: Berzerkers - 3 with plasma pistol, 2 with eviscerator, 5 with chainblade
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelLoadout {
    /// Human-readable label for this model type within the unit
    /// e.g., "Berzerker Champion", "Standard Custodian", "Dishonoured"
    pub model_label: String,
    /// How many models in the unit have this loadout
    pub count: u8,
    /// Names of the ranged weapon profiles this model carries
    /// (references WeaponProfileSchema.name in the parent datasheet)
    pub ranged_weapons: Vec<String>,
    /// Names of the melee weapon profiles this model carries
    pub melee_weapons: Vec<String>,
}

// ─── WargearAbility ─────────────────────────────────────────────────────────

/// A wargear ability that modifies the bearer's characteristics.
///
/// Example: Praesidium Shield - "Add 1 to the bearer's Wounds characteristic."
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WargearAbility {
    /// Display name (e.g., "Praesidium Shield")
    pub name: String,
    /// Description of what the wargear does
    pub description: String,
    /// The rules effects of this wargear
    pub effects: Vec<RulePrimitive>,
    /// Which models in the unit carry this wargear (label from ModelLoadout)
    /// If empty, all models in the unit have it.
    pub model_labels: Vec<String>,
}

// ─── AbilityTrigger ─────────────────────────────────────────────────────────

/// When an ability activates.
///
/// Source: implementation_v3.md Section 6.3
/// Source: Custodes.md - Martial Ka'tah triggers on "selected to fight"
/// Source: Frenzied_Reavers.md - Blessings of Khorne triggers at "start of battle round"
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AbilityTrigger {
    /// Triggers when the unit is selected to fight (e.g., Martial Ka'tah stance choice)
    OnSelectedToFight,
    /// Triggers when the unit is selected to shoot
    OnSelectedToShoot,
    /// Triggers when the unit is selected to move
    OnSelectedToMove,
    /// Triggers when the unit makes an attack (per-attack trigger)
    OnAttack,
    /// Triggers when the unit is targeted by an attack
    OnTargeted,
    /// Triggers when a model in the unit is damaged
    OnDamaged,
    /// Triggers when a model in the unit is destroyed
    OnDestroyed,
    /// Triggers when the unit destroys an enemy unit
    OnEnemyUnitDestroyed,
    /// Triggers when the unit destroys an enemy CHARACTER model
    OnEnemyCharacterDestroyed,
    /// Triggers at the start of a specific phase
    StartOfPhase(Phase),
    /// Triggers at the end of a specific phase
    EndOfPhase(Phase),
    /// Triggers at the start of each battle round (e.g., Blessings of Khorne)
    StartOfBattleRound,
    /// Triggers at the end of each battle round
    EndOfBattleRound,
    /// Triggers when the unit declares a charge
    OnDeclareCharge,
    /// Triggers when the unit is charged (target of a charge declaration)
    OnCharged,
    /// Triggers when the unit falls back
    OnFallBack,
    /// Triggers when an enemy unit within engagement range falls back
    OnEnemyFallBack,
    /// Triggers when the unit advances
    OnAdvance,
    /// Triggers when a Battle-shock test is taken
    OnBattleShockTest,
    /// Triggers on a Leadership test
    OnLeadershipTest,
    /// Triggers when the unit piles in
    OnPileIn,
    /// Triggers when the unit consolidates
    OnConsolidate,
    /// Always active (passive ability, e.g., invulnerable save)
    Passive,
    /// Triggers at a specific sub-phase
    AtSubPhase(SubPhase),
    /// Triggers when the bearer is within Engagement Range of an enemy
    WhileInEngagementRange,
    /// Triggers when the bearer is not Battle-shocked
    WhileNotBattleShocked,
    /// Triggers when a specific condition is met
    WhenCondition(Condition),
}

// ─── AbilityType ────────────────────────────────────────────────────────────

/// Classification of ability source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AbilityType {
    /// Faction-wide ability (e.g., Martial Ka'tah, Blessings of Khorne)
    FactionAbility,
    /// Unit-specific ability (e.g., A Worthy Skull on Master of Executions)
    UnitAbility,
    /// Wargear ability (e.g., Praesidium Shield)
    WargearAbility,
    /// Enhancement ability (e.g., Watchman of Terra, Fearsome Presence)
    EnhancementAbility,
    /// Core rule ability (e.g., Leader, Deadly Demise)
    CoreAbility,
}

// ─── Condition ──────────────────────────────────────────────────────────────

/// Conditions that can gate ability activation, targeting, or scoring.
///
/// These are composable and used throughout the schema for conditional logic
/// in RulePrimitives, AbilityTriggers, TargetRestrictions, and ScoringRules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Condition {
    /// Unit has a specific keyword
    HasKeyword(Keyword),
    /// Unit has all keywords in the set
    HasAllKeywords(Vec<Keyword>),
    /// Unit has any keyword in the set
    HasAnyKeyword(Vec<Keyword>),
    /// Unit is within Engagement Range of an enemy
    InEngagementRange,
    /// Unit is Battle-shocked
    BattleShocked,
    /// Unit is not Battle-shocked
    NotBattleShocked,
    /// Unit is below half strength (models remaining < starting / 2)
    BelowHalfStrength,
    /// Unit is at or above half strength
    AboveHalfStrength,
    /// Unit/model is within a certain range of a target
    WithinRange(Inches),
    /// Unit is on (controlling) an objective marker
    OnObjective,
    /// Unit controls an objective in No Man's Land
    OnObjectiveInNoMansLand,
    /// The model made a charge move this turn
    ChargedThisTurn,
    /// The unit Remained Stationary this turn
    RemainedStationary,
    /// The unit Advanced this turn
    AdvancedThisTurn,
    /// The unit Fell Back this turn
    FellBackThisTurn,
    /// The model is a CHARACTER
    IsCharacter,
    /// The model is a MONSTER
    IsMonster,
    /// The model is a VEHICLE
    IsVehicle,
    /// The model is INFANTRY
    IsInfantry,
    /// The target is a CHARACTER unit
    TargetIsCharacter,
    /// The target is not a MONSTER or VEHICLE
    TargetNotMonsterOrVehicle,
    /// Specific battle round number or later
    BattleRoundOrLater(u8),
    /// Specific battle round number
    ExactBattleRound(u8),
    /// During the player's own turn
    IsPlayerTurn,
    /// During the opponent's turn
    IsOpponentTurn,
    /// Logical AND of multiple conditions
    All(Vec<Condition>),
    /// Logical OR of multiple conditions
    Any(Vec<Condition>),
    /// Logical NOT of a condition
    Not(Box<Condition>),
    /// A specific Blessing of Khorne is active
    BlessingActive(String),
    /// The unit has not yet fought this phase
    HasNotFoughtThisPhase,
    /// The unit has not been selected to move this phase
    HasNotMovedThisPhase,
    /// The unit has not been selected to shoot this phase
    HasNotShotThisPhase,
    /// The unit was selected as a target of a charge
    WasChargeTarget,
    /// The model was destroyed by a melee attack
    DestroyedByMeleeAttack,
    /// Dice roll meets threshold (for conditional effects like Warrior Exemplar's 3+)
    DiceRollMeetsThreshold(u8),
    /// The unit owns/controls the closest objective to a specific edge
    ControlsClosestObjective(ObjectiveProximity),
    /// The model/unit destroyed one or more enemy units this phase
    DestroyedEnemyUnitThisPhase,
    /// Number of objectives matching a sub-condition
    ObjectiveCount { condition: Box<Condition>, count: u8 },
}

/// Which battlefield edge an objective is closest to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectiveProximity {
    /// Closest to your own battlefield edge
    OwnEdge,
    /// Closest to opponent's battlefield edge
    OpponentEdge,
    /// Any in No Man's Land
    NoMansLand,
}

// ─── TargetRestriction ──────────────────────────────────────────────────────

/// Restrictions on what a stratagem or ability can target.
///
/// Source: Custodes.md - stratagems target "ADEPTUS CUSTODES unit from your army"
/// Source: Frenzied_Reavers.md - Horrifying Butchery targets "KHORNE BERZERKERS unit"
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TargetRestriction {
    /// Required keywords (all must be present)
    pub required_keywords: Vec<Keyword>,
    /// Excluded keywords (none may be present)
    pub excluded_keywords: Vec<Keyword>,
    /// Must be owned by the player using the ability
    pub owner: TargetOwner,
    /// Additional conditions
    pub conditions: Vec<Condition>,
}

impl Default for TargetRestriction {
    fn default() -> Self {
        Self {
            required_keywords: Vec::new(),
            excluded_keywords: Vec::new(),
            owner: TargetOwner::Friendly,
            conditions: Vec::new(),
        }
    }
}

/// Who the target belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetOwner {
    /// Must be a friendly unit
    Friendly,
    /// Must be an enemy unit
    Enemy,
    /// Can be either
    Any,
}

// ─── RulePrimitive ──────────────────────────────────────────────────────────

/// The composable rule primitives DSL.
///
/// These are the building blocks that faction YAML files assemble declaratively
/// to express game rules. The engine interprets these at runtime without
/// needing faction-specific code.
///
/// Source: implementation_v3.md Section 6.3 - Rule Primitives
/// Source: Custodes.md - Ka'tah stances, enhancements, stratagems
/// Source: Frenzied_Reavers.md - Blessings of Khorne, abilities, stratagems
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RulePrimitive {
    /// Add a flat modifier to a stat.
    /// Example: Praesidium Shield adds +1 to Wounds.
    /// Example: Inescapable Vengeance adds +2 to Advance rolls.
    AddModifier {
        target: ModifierTarget,
        stat: StatTarget,
        value: i16,
    },

    /// Set a status flag on a target.
    /// Example: Mark a unit as Battle-shocked.
    SetStatus {
        target: ModifierTarget,
        status: StatusEffect,
    },

    /// Grant a weapon ability to weapons of a certain type.
    /// Example: Dacatarai stance grants Sustained Hits 1 to melee weapons.
    /// Example: Rendax stance grants Lethal Hits to melee weapons.
    GrantWeaponAbility {
        weapon_ability: WeaponAbilityDef,
        weapon_filter: WeaponFilter,
    },

    /// Apply re-roll logic.
    /// Example: A Worthy Skull - re-roll Hit and Wound vs CHARACTER.
    ApplyReroll {
        scope: RerollScope,
        condition: Option<Condition>,
    },

    /// Modify a characteristic.
    /// Example: Watchman of Terra sets OC to 4 while in Engagement Range.
    /// Example: Fearsome Presence sets OC to 5 while not Battle-shocked.
    ModifyCharacteristic {
        characteristic: CharacteristicTarget,
        modifier: CharacteristicModifier,
    },

    /// Restrict what the target can do or who can be targeted.
    /// Example: Blast weapons can't target units in Engagement Range.
    RestrictTarget {
        condition: Condition,
    },

    /// Require a keyword for the effect to apply.
    RequireKeyword {
        keyword: Keyword,
    },

    /// Trigger a movement reaction.
    /// Example: Bloodlust - Jakhals move D6" after being shot.
    /// Example: Rage-fuelled Invigoration - 6" pile-in/consolidate.
    TriggerMoveReaction {
        move_type: ReactionMoveType,
        distance: MoveDistance,
    },

    /// Trigger a fight reaction (fight on death).
    /// Example: Total Carnage - destroyed model fights back on 4+.
    /// Example: Berserk Resilience - CHARACTER fights back on 4+ (2+ with Total Carnage).
    TriggerFightReaction {
        fight_type: FightReactionType,
    },

    /// Trigger an effect when a model/unit is destroyed.
    /// Example: Warrior Exemplar - roll D6, on 3+ gain 1CP.
    TriggerOnDestroyed {
        effect: Box<RulePrimitive>,
    },

    /// Score Victory Points.
    /// Example: Consecrated Ground - +3VP per enemy unit destroyed.
    ScoreVP {
        amount: i16,
        condition: Option<Condition>,
    },

    /// Score VP with timing restrictions.
    /// Example: Raise the Vexillas - 3VP at end of your turn from round 3.
    /// Example: Champions of Khorne - 2VP/3VP at end of Command phase from round 2.
    ConditionalScoreVP {
        amount: i16,
        condition: Condition,
        timing: ScoringTiming,
    },

    /// Alter a unit's Objective Control characteristic.
    /// Example: Watchman of Terra - OC becomes 4 in Engagement Range.
    /// Example: Fearsome Presence - OC becomes 5 while not Battle-shocked.
    AlterOC {
        target: ModifierTarget,
        new_oc: OCModifier,
    },

    /// Alter Command Points.
    /// Example: A Worthy Skull - gain 1CP when destroying enemy CHARACTER.
    /// Example: Warrior Exemplar - gain 1CP on 3+ when destroying enemy unit.
    AlterCP {
        amount: i8,
    },

    /// Dice pool allocation mechanic (Blessings of Khorne).
    /// Roll N dice, then allocate pairs/triples to activate blessings.
    DicePoolAllocation {
        num_dice: u8,
        max_blessings: u8,
        blessings: Vec<DicePoolBlessingDef>,
    },

    /// Stance choice mechanic (Martial Ka'tah).
    /// Choose one stance from a list; its effects apply until end of attacks.
    StanceChoice {
        stances: Vec<StanceDef>,
    },

    /// Fight on death mechanic.
    /// Roll D6 (with optional modifier), on threshold+, the destroyed model
    /// fights back before being removed.
    FightOnDeath {
        threshold: u8,
        modifier_source: Option<String>,
    },

    /// Apply an effect for a specific duration.
    /// Example: Stratagems that last "until end of phase".
    ApplyForDuration {
        effect: Box<RulePrimitive>,
        duration: EffectDuration,
    },

    /// Force a Leadership test on a target.
    /// Example: Overawing Magnificence - enemy must take Leadership test.
    ForceLeadershipTest {
        target: ModifierTarget,
        on_fail: Box<RulePrimitive>,
    },

    /// Force a Battle-shock test on a target.
    /// Example: Horrifying Butchery - enemy takes Battle-shock test.
    ForceBattleShockTest {
        target: ModifierTarget,
    },

    /// Desperate Escape test (Fall Back through Engagement Range).
    /// Example: Bane of the Craven - all models must take Desperate Escape.
    ForceDespairEscapeTest {
        modifier: i8,
        condition: Option<Condition>,
    },

    /// Conditional effect: if condition is met, apply inner effect.
    Conditional {
        condition: Condition,
        effect: Box<RulePrimitive>,
        else_effect: Option<Box<RulePrimitive>>,
    },

    /// Apply multiple effects as a group.
    Composite {
        effects: Vec<RulePrimitive>,
    },

    /// Roll a D6 and apply an effect based on result meeting a threshold.
    DiceCheck {
        threshold: u8,
        on_success: Box<RulePrimitive>,
        on_failure: Option<Box<RulePrimitive>>,
        modifier: i8,
    },

    /// Grant Feel No Pain save.
    /// Example: Go to Ground grants 6++ FNP.
    GrantFeelNoPain {
        value: u8,
    },

    /// Grant cover benefit.
    /// Example: Go to Ground grants Benefit of Cover.
    GrantCover,

    /// Make an attack gain Precision.
    /// Example: Epic Challenge stratagem.
    GrantPrecision,

    /// Overwatch shooting (hit only on 6s).
    /// Example: Fire Overwatch core stratagem.
    Overwatch,

    /// Tank Shock: roll T dice, each 5+ deals 1 MW (max cap).
    TankShock {
        dice_count_stat: CharacteristicTarget,
        threshold: u8,
        max_mortal_wounds: u8,
    },

    /// No operation (placeholder for effects that are handled elsewhere).
    Noop,
}

// ─── Supporting types for RulePrimitive ─────────────────────────────────────

/// What the modifier applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierTarget {
    /// The unit using the ability
    Self_,
    /// The target of the ability/attack
    Target,
    /// A specific player
    Player(PlayerId),
    /// All friendly units
    AllFriendly,
    /// All enemy units
    AllEnemy,
    /// The bearer (specific model)
    Bearer,
    /// Selected enemy unit (for targeted stratagems)
    SelectedEnemy,
}

/// Which stat is being modified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatTarget {
    /// Wounds characteristic
    Wounds,
    /// Attacks characteristic
    Attacks,
    /// Strength
    Strength,
    /// Toughness
    Toughness,
    /// Armor Save
    ArmorSave,
    /// Invulnerable Save
    InvulnerableSave,
    /// Ballistic Skill / Weapon Skill
    Skill,
    /// Armor Penetration
    ArmorPenetration,
    /// Damage
    Damage,
    /// Move characteristic
    Movement,
    /// Leadership
    Leadership,
    /// Objective Control
    ObjectiveControl,
    /// Advance roll
    AdvanceRoll,
    /// Charge roll
    ChargeRoll,
    /// Hit roll
    HitRoll,
    /// Wound roll
    WoundRoll,
    /// Save roll
    SaveRoll,
}

/// Status effects that can be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatusEffect {
    /// Unit is Battle-shocked
    BattleShocked,
    /// Unit has Fights First
    FightsFirst,
    /// Unit has Fights Last
    FightsLast,
    /// Unit cannot shoot this turn
    CannotShoot,
    /// Unit cannot charge this turn
    CannotCharge,
    /// Unit cannot Fall Back
    CannotFallBack,
    /// Unit is immune to Battle-shock
    ImmuneToBattleShock,
    /// Unit has Stealth
    Stealth,
    /// Unit counts as having Remained Stationary
    CountsAsStationary,
}

/// Which weapons the ability applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeaponFilter {
    /// All weapons
    All,
    /// Only melee weapons
    MeleeOnly,
    /// Only ranged weapons
    RangedOnly,
    // For named weapon filters, use WeaponFilterExt which supports String.
}

/// Extended weapon filter that can reference a specific weapon by name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeaponFilterExt {
    /// All weapons
    All,
    /// Only melee weapons
    MeleeOnly,
    /// Only ranged weapons
    RangedOnly,
    /// A specific named weapon
    Named(String),
}

/// What scope of re-roll to grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RerollScope {
    /// Re-roll hit rolls
    HitRolls,
    /// Re-roll wound rolls
    WoundRolls,
    /// Re-roll both hit and wound rolls
    HitAndWoundRolls,
    /// Re-roll a single dice roll (Command Re-roll)
    SingleDice,
    /// Re-roll charge rolls
    ChargeRolls,
    /// Re-roll advance rolls
    AdvanceRolls,
    /// Re-roll save rolls
    SaveRolls,
    /// Re-roll damage rolls
    DamageRolls,
    /// Re-roll failed hit rolls only
    FailedHitRolls,
    /// Re-roll failed wound rolls only
    FailedWoundRolls,
}

/// Which characteristic is being targeted for modification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CharacteristicTarget {
    /// Movement distance
    Movement,
    /// Toughness
    Toughness,
    /// Armor Save
    ArmorSave,
    /// Wounds per model
    Wounds,
    /// Leadership
    Leadership,
    /// Objective Control
    ObjectiveControl,
    /// Ballistic Skill
    BallisticSkill,
    /// Weapon Skill
    WeaponSkill,
    /// Attacks
    Attacks,
    /// Strength
    Strength,
    /// Armor Penetration
    ArmorPenetration,
    /// Damage
    Damage,
}

/// How a characteristic is modified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CharacteristicModifier {
    /// Set to an absolute value
    Set(i16),
    /// Add a value (can be negative for subtraction)
    Add(i16),
    /// Multiply by a value
    Multiply(i16),
    /// Set to the worse of current and the given value (e.g., degraded profiles)
    WorseOf(i16),
    /// Set to the better of current and the given value
    BetterOf(i16),
}

/// Types of reactive movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReactionMoveType {
    /// Normal Move reaction (e.g., Bloodlust)
    NormalMove,
    /// Pile-in move (extended range)
    PileIn,
    /// Consolidation move (extended range)
    Consolidation,
    /// Pile-in AND consolidation (e.g., Rage-fuelled Invigoration, The Gilded Spear)
    PileInAndConsolidation,
    /// Fall Back move
    FallBack,
    /// Heroic Intervention
    HeroicIntervention,
}

/// Distance for a reactive move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MoveDistance {
    /// Fixed distance in inches
    Fixed(Inches),
    /// Roll D6 inches
    D6,
    /// Roll D3 inches
    D3,
    /// Roll D6 + bonus inches
    D6Plus(Inches),
}

/// Types of fight-back reactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FightReactionType {
    /// Fight on death: model fights before being removed
    FightOnDeath,
    /// Counter-offensive: unit fights next in the Fight phase
    CounterOffensive,
    /// Heroic Intervention fight
    HeroicInterventionFight,
}

/// How OC is modified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OCModifier {
    /// Set OC to a specific value
    SetTo(u8),
    /// Add to current OC
    Add(i8),
    /// Set OC to 0 (Battle-shock)
    Zero,
}

/// Timing for scoring VP.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScoringTiming {
    /// Which phase the scoring occurs at the end of
    pub phase: Phase,
    /// Whose turn (your turn, opponent's turn, either)
    pub whose_turn: TurnOwner,
    /// From which battle round onwards
    pub from_round: u8,
}

/// Whose turn scoring applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TurnOwner {
    /// Your turn only
    Active,
    /// Opponent's turn only
    Opponent,
    /// Either turn
    Either,
}

// ─── DicePoolBlessingDef ────────────────────────────────────────────────────

/// Definition for a blessing in a dice pool mechanic (Blessings of Khorne).
///
/// Source: Frenzied_Reavers.md - Blessings of Khorne
///
/// Each blessing requires a specific dice pattern to activate and grants
/// effects to all units with the faction ability for the rest of the battle round.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DicePoolBlessingDef {
    /// Display name (e.g., "Rage-fuelled Invigoration", "Total Carnage")
    pub name: String,
    /// What dice pattern is needed to activate
    pub requirement: DiceRequirement,
    /// Effects applied when activated
    pub effects: Vec<RulePrimitive>,
    /// Human-readable description
    pub description: String,
}

/// What dice pattern is required to activate a blessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiceRequirement {
    /// Two dice showing the same value, both 2 or higher
    /// (e.g., two 3s, two 5s)
    DoubleTwoPlus,
    /// Two dice showing the same value, both 4 or higher
    /// (e.g., two 4s, two 5s, two 6s)
    DoubleFourPlus,
    /// Three dice showing the same value (any value)
    TripleAny,
    /// Either DoubleFourPlus OR TripleAny
    DoubleFourPlusOrTripleAny,
}

// ─── StanceDef ──────────────────────────────────────────────────────────────

/// Definition for a stance in a stance-choice mechanic (Martial Ka'tah).
///
/// Source: Custodes.md - Martial Ka'tah stances (Dacatarai, Rendax)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StanceDef {
    /// Display name (e.g., "Dacatarai", "Rendax")
    pub name: String,
    /// Effects applied while the stance is active
    pub effects: Vec<RulePrimitive>,
    /// Human-readable description
    pub description: String,
}

// ─── AbilitySchema ──────────────────────────────────────────────────────────

/// Unit or faction ability definition.
///
/// Source: Custodes.md - Martial Ka'tah, Praesidium Shield, etc.
/// Source: Frenzied_Reavers.md - Blessings of Khorne, A Worthy Skull, etc.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbilitySchema {
    /// Display name (e.g., "Martial Ka'tah", "A Worthy Skull")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Classification of the ability
    pub ability_type: AbilityType,
    /// The rules effects of this ability, expressed as primitives
    pub effects: Vec<RulePrimitive>,
    /// When this ability triggers (None = always active / passive)
    pub trigger: Option<AbilityTrigger>,
}

// ─── DatasheetSchema ────────────────────────────────────────────────────────

/// Template for a unit type (datasheet).
///
/// Contains all the information from a unit's datasheet: stats, weapons,
/// abilities, keywords, and model composition.
///
/// Source: Custodes.md - Unit Datasheets section
/// Source: Frenzied_Reavers.md - Unit Datasheets section
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasheetSchema {
    /// Unique identifier for this datasheet
    pub id: DatasheetId,
    /// Display name (e.g., "Tristraen", "Custodian Guard", "Vorrakh")
    pub name: String,
    /// All keywords for this unit
    pub keywords: KeywordSet,
    /// Movement characteristic
    pub movement: MoveCharacteristic,
    /// Toughness characteristic
    pub toughness: Toughness,
    /// Armor Save characteristic
    pub armor_save: ArmorSave,
    /// Invulnerable Save (None if the unit has no invuln)
    pub invulnerable_save: Option<InvulnerableSave>,
    /// Wounds per model
    pub wounds: Wounds,
    /// Leadership characteristic
    pub leadership: Leadership,
    /// Objective Control characteristic
    pub objective_control: ObjectiveControl,
    /// Model base size in mm
    pub base_size: BaseSize,
    /// Ranged weapon profiles
    pub ranged_weapons: Vec<WeaponProfileSchema>,
    /// Melee weapon profiles
    pub melee_weapons: Vec<WeaponProfileSchema>,
    /// Unit abilities (faction, unit, core)
    pub abilities: Vec<AbilitySchema>,
    /// Unit size specification (fixed or variable)
    pub unit_size: UnitSizeSpec,
    /// Which models carry which weapons
    pub model_loadouts: Vec<ModelLoadout>,
    /// Wargear abilities (e.g., Praesidium Shield)
    pub wargear_abilities: Vec<WargearAbility>,
}

// ─── StratagemSchema ────────────────────────────────────────────────────────

/// Stratagem template for content authoring.
///
/// Source: Custodes.md - Faction Stratagems, Core Stratagems
/// Source: Frenzied_Reavers.md - Faction Stratagems, Core Stratagems
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StratagemSchema {
    /// Display name (e.g., "The Gilded Spear", "Horrifying Butchery")
    pub name: String,
    /// CP cost to use
    pub cp_cost: u8,
    /// Which phase the stratagem can be used in
    pub phase: Phase,
    /// When exactly within the phase
    pub timing: StratagemTiming,
    /// Classification of the stratagem
    pub stratagem_type: StratagemType,
    /// What can be targeted
    pub target_restriction: TargetRestriction,
    /// Effects applied when used
    pub effects: Vec<RulePrimitive>,
    /// Can only be used once per battle
    pub once_per_battle: bool,
    /// Can only be used once per turn (the default restriction)
    pub once_per_turn: bool,
    /// Human-readable description
    pub description: String,
}

/// When within a phase a stratagem can be used.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StratagemTiming {
    /// At the start of the phase
    StartOfPhase,
    /// During the phase (general)
    DuringPhase,
    /// At the end of the phase
    EndOfPhase,
    /// When a specific event occurs
    OnEvent(String),
    /// Just after an enemy unit has shot (Bloodlust)
    AfterEnemyShoots,
    /// Just after an enemy unit declares a charge (Overawing Magnificence)
    AfterChargeDeclaration,
    /// Just after an enemy unit has selected targets (Berserk Resilience)
    AfterTargetSelection,
    /// Before selecting a unit to fight
    BeforeFight,
    /// Any time during the phase
    AnyTime,
}

/// Classification of stratagem type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StratagemType {
    /// Battle Tactic - aggressive/offensive stratagems
    BattleTactic,
    /// Strategic Ploy - positioning/defensive stratagems
    StrategicPloy,
    /// Epic Deed - heroic actions by Characters
    EpicDeed,
    /// Wargear - equipment-related stratagems
    Wargear,
}

// ─── EnhancementSchema ──────────────────────────────────────────────────────

/// Enhancement (warlord trait) template for content authoring.
///
/// Source: Custodes.md - Watchman of Terra, Warrior Exemplar
/// Source: Frenzied_Reavers.md - Fearsome Presence, Bane of the Craven
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EnhancementSchema {
    /// Display name (e.g., "Watchman of Terra")
    pub name: String,
    /// Human-readable description / flavor text
    pub description: String,
    /// Whether this is the default enhancement (pre-selected)
    pub is_default: bool,
    /// Effects applied when this enhancement is active
    pub effects: Vec<RulePrimitive>,
    /// Conditions under which the enhancement is active
    pub conditions: Vec<Condition>,
}

// ─── SecondaryObjectiveSchema ───────────────────────────────────────────────

/// Secondary objective template for content authoring.
///
/// Source: Custodes.md - Raise the Vexillas, Consecrated Ground
/// Source: Frenzied_Reavers.md - Champions of Khorne, Skull Takers
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SecondaryObjectiveSchema {
    /// Display name (e.g., "Raise the Vexillas")
    pub name: String,
    /// Human-readable description / flavor text
    pub description: String,
    /// Whether this is the default secondary objective
    pub is_default: bool,
    /// Scoring rules
    pub scoring: Vec<ScoringRule>,
    /// Maximum VP that can be scored from this objective (None = unlimited)
    pub max_vp: Option<i16>,
}

/// A scoring rule for objectives.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScoringRule {
    /// When to check for scoring
    pub timing: ScoringTiming,
    /// What condition must be met to score
    pub condition: Condition,
    /// How many VP to score when condition is met
    pub vp_amount: i16,
    /// Optional description of this scoring component
    pub description: Option<String>,
}

// ─── ObjectiveDef ───────────────────────────────────────────────────────────

/// An objective marker definition within a mission.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectiveDef {
    /// Objective marker label (e.g., "A", "B", "C", "D")
    pub label: String,
    /// Position on the battlefield (x, y in inches)
    pub position_x: Inches,
    pub position_y: Inches,
    /// Which zone this objective is in
    pub zone: ObjectiveZone,
    /// Control range (default 3")
    pub control_range: Inches,
}

/// Which zone of the battlefield an objective is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectiveZone {
    /// In the attacker's deployment zone
    AttackerZone,
    /// In the defender's deployment zone
    DefenderZone,
    /// In No Man's Land
    NoMansLand,
}

// ─── MissionSchema ──────────────────────────────────────────────────────────

/// Mission definition for content authoring.
///
/// Source: CP_Rules.md - Combat Patrol missions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionSchema {
    /// Unique identifier
    pub id: MissionId,
    /// Display name
    pub name: String,
    /// Which deployment map to use
    pub deployment_map: DeploymentMapId,
    /// Objective marker positions and rules
    pub objectives: Vec<ObjectiveDef>,
    /// Primary objective scoring rules
    pub primary_scoring: Vec<ScoringRule>,
    /// Special rules for this mission
    pub special_rules: Vec<RulePrimitive>,
    /// Number of battle rounds (always 5 for Combat Patrol)
    pub rounds: u8,
    /// Human-readable description
    pub description: String,
}

// ─── PatrolSquadOption ──────────────────────────────────────────────────────

/// An optional unit choice in a Combat Patrol.
///
/// Source: Custodes.md - "Choose One: Custodian Wardens OR Allarus Custodians"
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PatrolSquadOption {
    /// Display label for this option (e.g., "Option A - Higher Model Count")
    pub label: String,
    /// The datasheet ID of the unit
    pub datasheet_id: DatasheetId,
    /// How many models in this option
    pub model_count: u8,
}

// ─── FactionSchema ──────────────────────────────────────────────────────────

/// Complete faction definition.
///
/// Contains everything needed to play a faction: datasheets, stratagems,
/// enhancements, secondary objectives, and the faction ability.
///
/// Source: Custodes.md - Tristraen's Gilded Blades
/// Source: Frenzied_Reavers.md - Frenzied Reavers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactionSchema {
    /// Unique identifier
    pub id: FactionId,
    /// Display name (e.g., "Tristraen's Gilded Blades", "Frenzied Reavers")
    pub name: String,
    /// Faction keywords shared by all units
    pub faction_keywords: KeywordSet,
    /// The faction-wide ability
    pub faction_ability: AbilitySchema,
    /// All unit datasheets in this faction
    pub datasheets: Vec<DatasheetSchema>,
    /// Faction-specific stratagems
    pub stratagems: Vec<StratagemSchema>,
    /// Enhancements available to the warlord
    pub enhancements: Vec<EnhancementSchema>,
    /// Secondary objectives available
    pub secondary_objectives: Vec<SecondaryObjectiveSchema>,
}

// ─── CombatPatrolPackSchema ─────────────────────────────────────────────────

/// Ties a faction to its units for a specific Combat Patrol configuration.
///
/// Source: Custodes.md - Force Composition
/// Source: Frenzied_Reavers.md - Force Composition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatPatrolPackSchema {
    /// Display name (e.g., "Tristraen's Gilded Blades")
    pub name: String,
    /// Which faction this patrol belongs to
    pub faction_id: FactionId,
    /// Units that are always included
    pub fixed_units: Vec<DatasheetId>,
    /// Optional unit choices (pick one group)
    pub patrol_squads: Vec<PatrolSquadOption>,
    /// The default enhancement ID
    pub default_enhancement: EnhancementId,
    /// The optional (alternative) enhancement ID
    pub optional_enhancement: EnhancementId,
    /// The default secondary objective ID
    pub default_secondary: SecondaryObjectiveId,
    /// The optional (alternative) secondary objective ID
    pub optional_secondary: SecondaryObjectiveId,
}

// ─── ContentPack ────────────────────────────────────────────────────────────

/// Compiled content ready for runtime use.
///
/// This is the top-level container that the content compiler produces.
/// It contains all factions and missions needed for a game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentPack {
    /// Unique identifier for this content pack
    pub pack_id: ContentPackId,
    /// Semantic version string
    pub version: String,
    /// Hash of the content for integrity checking
    pub content_hash: u64,
    /// All factions in this pack
    pub factions: Vec<FactionSchema>,
    /// All missions in this pack
    pub missions: Vec<MissionSchema>,
}

// ─── Error types ────────────────────────────────────────────────────────────

/// Errors that can occur during content schema operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ContentSchemaError {
    #[error("Invalid datasheet: {0}")]
    InvalidDatasheet(String),

    #[error("Invalid weapon profile: {0}")]
    InvalidWeaponProfile(String),

    #[error("Invalid ability: {0}")]
    InvalidAbility(String),

    #[error("Invalid stratagem: {0}")]
    InvalidStratagem(String),

    #[error("Invalid enhancement: {0}")]
    InvalidEnhancement(String),

    #[error("Invalid secondary objective: {0}")]
    InvalidSecondaryObjective(String),

    #[error("Invalid mission: {0}")]
    InvalidMission(String),

    #[error("Invalid faction: {0}")]
    InvalidFaction(String),

    #[error("Missing reference: {entity_type} '{name}' not found")]
    MissingReference {
        entity_type: String,
        name: String,
    },

    #[error("Duplicate ID: {entity_type} with ID {id}")]
    DuplicateId {
        entity_type: String,
        id: u32,
    },

    #[error("YAML parse error: {0}")]
    YamlParseError(String),

    #[error("JSON parse error: {0}")]
    JsonParseError(String),
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use serde_yaml;

    // ── WeaponProfileSchema ─────────────────────────────────────────────

    #[test]
    fn test_weapon_profile_schema_sentinel_blade_ranged() {
        let weapon = WeaponProfileSchema {
            name: "Sentinel blade".to_string(),
            weapon_type: WeaponType::Ranged,
            range: Inches::from_inches(12),
            attacks: AttackCount::Fixed(2),
            skill: Skill::TWO_PLUS,
            strength: Strength::new(4),
            ap: ArmorPenetration::MINUS_1,
            damage: Damage::Fixed(2),
            abilities: vec![WeaponAbilityDef::Assault, WeaponAbilityDef::Pistol],
        };
        let json = serde_json::to_string(&weapon).unwrap();
        let back: WeaponProfileSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(weapon, back);
    }

    #[test]
    fn test_weapon_profile_schema_sentinel_blade_melee() {
        let weapon = WeaponProfileSchema {
            name: "Sentinel blade".to_string(),
            weapon_type: WeaponType::Melee,
            range: Inches::ZERO,
            attacks: AttackCount::Fixed(5),
            skill: Skill::TWO_PLUS,
            strength: Strength::new(6),
            ap: ArmorPenetration::MINUS_2,
            damage: Damage::Fixed(1),
            abilities: vec![],
        };
        let json = serde_json::to_string(&weapon).unwrap();
        let back: WeaponProfileSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(weapon, back);
    }

    #[test]
    fn test_weapon_profile_schema_vaultswords_behemor() {
        let weapon = WeaponProfileSchema {
            name: "Vaultswords - Behemor".to_string(),
            weapon_type: WeaponType::Melee,
            range: Inches::ZERO,
            attacks: AttackCount::Fixed(6),
            skill: Skill::TWO_PLUS,
            strength: Strength::new(7),
            ap: ArmorPenetration::MINUS_2,
            damage: Damage::Fixed(2),
            abilities: vec![WeaponAbilityDef::Precision],
        };
        assert_eq!(weapon.name, "Vaultswords - Behemor");
        assert_eq!(weapon.abilities.len(), 1);
    }

    #[test]
    fn test_weapon_profile_schema_hellforged_weapons() {
        let weapon = WeaponProfileSchema {
            name: "Hellforged weapons".to_string(),
            weapon_type: WeaponType::Melee,
            range: Inches::ZERO,
            attacks: AttackCount::Fixed(16),
            skill: Skill::TWO_PLUS,
            strength: Strength::new(6),
            ap: ArmorPenetration::MINUS_1,
            damage: Damage::Fixed(1),
            abilities: vec![],
        };
        assert_eq!(weapon.attacks, AttackCount::Fixed(16));
    }

    #[test]
    fn test_weapon_profile_schema_balistus_grenade_launcher() {
        let weapon = WeaponProfileSchema {
            name: "Balistus grenade launcher".to_string(),
            weapon_type: WeaponType::Ranged,
            range: Inches::from_inches(18),
            attacks: AttackCount::D6,
            skill: Skill::TWO_PLUS,
            strength: Strength::new(4),
            ap: ArmorPenetration::MINUS_1,
            damage: Damage::Fixed(1),
            abilities: vec![WeaponAbilityDef::Blast],
        };
        assert_eq!(weapon.attacks.min_value(), 1);
        assert_eq!(weapon.attacks.max_value(), 6);
    }

    #[test]
    fn test_weapon_profile_schema_plasma_pistol_supercharge() {
        let weapon = WeaponProfileSchema {
            name: "Plasma pistol - supercharge".to_string(),
            weapon_type: WeaponType::Ranged,
            range: Inches::from_inches(12),
            attacks: AttackCount::Fixed(1),
            skill: Skill::FOUR_PLUS,
            strength: Strength::new(8),
            ap: ArmorPenetration::MINUS_3,
            damage: Damage::Fixed(2),
            abilities: vec![WeaponAbilityDef::Hazardous, WeaponAbilityDef::Pistol],
        };
        assert_eq!(weapon.abilities.len(), 2);
    }

    #[test]
    fn test_weapon_profile_schema_infernal_cannon() {
        let weapon = WeaponProfileSchema {
            name: "Infernal cannon".to_string(),
            weapon_type: WeaponType::Ranged,
            range: Inches::from_inches(24),
            attacks: AttackCount::Fixed(3),
            skill: Skill::THREE_PLUS,
            strength: Strength::new(5),
            ap: ArmorPenetration::MINUS_1,
            damage: Damage::Fixed(2),
            abilities: vec![WeaponAbilityDef::RapidFire(1)],
        };
        let yaml = serde_yaml::to_string(&weapon).unwrap();
        let back: WeaponProfileSchema = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(weapon, back);
    }

    #[test]
    fn test_weapon_profile_schema_axe_of_dismemberment() {
        let weapon = WeaponProfileSchema {
            name: "Axe of dismemberment".to_string(),
            weapon_type: WeaponType::Melee,
            range: Inches::ZERO,
            attacks: AttackCount::Fixed(5),
            skill: Skill::TWO_PLUS,
            strength: Strength::new(7),
            ap: ArmorPenetration::MINUS_2,
            damage: Damage::Fixed(2),
            abilities: vec![
                WeaponAbilityDef::DevastatingWounds,
                WeaponAbilityDef::Precision,
            ],
        };
        assert_eq!(weapon.abilities.len(), 2);
    }

    // ── WeaponAbilityDef ────────────────────────────────────────────────

    #[test]
    fn test_weapon_ability_def_roundtrip() {
        let abilities = vec![
            WeaponAbilityDef::Assault,
            WeaponAbilityDef::Heavy,
            WeaponAbilityDef::RapidFire(1),
            WeaponAbilityDef::Pistol,
            WeaponAbilityDef::Blast,
            WeaponAbilityDef::Torrent,
            WeaponAbilityDef::TwinLinked,
            WeaponAbilityDef::LethalHits,
            WeaponAbilityDef::SustainedHits(1),
            WeaponAbilityDef::DevastatingWounds,
            WeaponAbilityDef::Hazardous,
            WeaponAbilityDef::Precision,
            WeaponAbilityDef::Anti {
                keyword: Keyword::Infantry,
                threshold: 4,
            },
            WeaponAbilityDef::IndirectFire,
            WeaponAbilityDef::IgnoresCover,
            WeaponAbilityDef::Melta(2),
            WeaponAbilityDef::Lance,
            WeaponAbilityDef::ExtraAttacks,
        ];

        for ability in &abilities {
            let json = serde_json::to_string(ability).unwrap();
            let back: WeaponAbilityDef = serde_json::from_str(&json).unwrap();
            assert_eq!(*ability, back);

            // Verify runtime conversion roundtrip
            let runtime = ability.to_runtime();
            let back_def = WeaponAbilityDef::from_runtime(&runtime);
            assert_eq!(*ability, back_def);
        }
    }

    // ── UnitSizeSpec ────────────────────────────────────────────────────

    #[test]
    fn test_unit_size_spec() {
        let fixed = UnitSizeSpec::Fixed(1);
        assert_eq!(fixed.min_models(), 1);
        assert_eq!(fixed.max_models(), 1);

        let range = UnitSizeSpec::Range { min: 5, max: 10 };
        assert_eq!(range.min_models(), 5);
        assert_eq!(range.max_models(), 10);

        let json = serde_json::to_string(&fixed).unwrap();
        let back: UnitSizeSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(fixed, back);
    }

    // ── ModelLoadout ────────────────────────────────────────────────────

    #[test]
    fn test_model_loadout_serialization() {
        let loadout = ModelLoadout {
            model_label: "Berzerker Champion".to_string(),
            count: 1,
            ranged_weapons: vec!["Plasma pistol - standard".to_string()],
            melee_weapons: vec!["Chainblade".to_string()],
        };
        let json = serde_json::to_string(&loadout).unwrap();
        let back: ModelLoadout = serde_json::from_str(&json).unwrap();
        assert_eq!(loadout, back);
    }

    #[test]
    fn test_model_loadout_berzerkers() {
        let loadouts = vec![
            ModelLoadout {
                model_label: "Berzerker Champion".to_string(),
                count: 1,
                ranged_weapons: vec!["Plasma pistol - standard".to_string()],
                melee_weapons: vec!["Chainblade".to_string()],
            },
            ModelLoadout {
                model_label: "Plasma Berzerker".to_string(),
                count: 2,
                ranged_weapons: vec!["Plasma pistol - standard".to_string()],
                melee_weapons: vec!["Chainblade".to_string()],
            },
            ModelLoadout {
                model_label: "Eviscerator Berzerker".to_string(),
                count: 2,
                ranged_weapons: vec!["Bolt pistol".to_string()],
                melee_weapons: vec!["Khornate eviscerator".to_string()],
            },
            ModelLoadout {
                model_label: "Standard Berzerker".to_string(),
                count: 5,
                ranged_weapons: vec!["Bolt pistol".to_string()],
                melee_weapons: vec!["Chainblade".to_string()],
            },
        ];
        let total_models: u8 = loadouts.iter().map(|l| l.count).sum();
        assert_eq!(total_models, 10);
    }

    // ── WargearAbility ──────────────────────────────────────────────────

    #[test]
    fn test_wargear_ability_praesidium_shield() {
        let shield = WargearAbility {
            name: "Praesidium Shield".to_string(),
            description: "Add 1 to the bearer's Wounds characteristic.".to_string(),
            effects: vec![RulePrimitive::AddModifier {
                target: ModifierTarget::Bearer,
                stat: StatTarget::Wounds,
                value: 1,
            }],
            model_labels: vec![], // All models in Custodian Guard have it
        };
        let json = serde_json::to_string(&shield).unwrap();
        let back: WargearAbility = serde_json::from_str(&json).unwrap();
        assert_eq!(shield, back);
    }

    // ── AbilitySchema ───────────────────────────────────────────────────

    #[test]
    fn test_ability_schema_martial_katah() {
        let katah = AbilitySchema {
            name: "Martial Ka'tah".to_string(),
            description: "Each time this unit is selected to fight, choose one stance.".to_string(),
            ability_type: AbilityType::FactionAbility,
            effects: vec![RulePrimitive::StanceChoice {
                stances: vec![
                    StanceDef {
                        name: "Dacatarai".to_string(),
                        effects: vec![RulePrimitive::GrantWeaponAbility {
                            weapon_ability: WeaponAbilityDef::SustainedHits(1),
                            weapon_filter: WeaponFilter::MeleeOnly,
                        }],
                        description: "Melee weapons have [SUSTAINED HITS 1]".to_string(),
                    },
                    StanceDef {
                        name: "Rendax".to_string(),
                        effects: vec![RulePrimitive::GrantWeaponAbility {
                            weapon_ability: WeaponAbilityDef::LethalHits,
                            weapon_filter: WeaponFilter::MeleeOnly,
                        }],
                        description: "Melee weapons have [LETHAL HITS]".to_string(),
                    },
                ],
            }],
            trigger: Some(AbilityTrigger::OnSelectedToFight),
        };
        let json = serde_json::to_string(&katah).unwrap();
        let back: AbilitySchema = serde_json::from_str(&json).unwrap();
        assert_eq!(katah, back);
    }

    #[test]
    fn test_ability_schema_blessings_of_khorne() {
        let blessings = AbilitySchema {
            name: "Blessings of Khorne".to_string(),
            description: "Roll 5D6 at the start of each battle round, activate up to two Blessings.".to_string(),
            ability_type: AbilityType::FactionAbility,
            effects: vec![RulePrimitive::DicePoolAllocation {
                num_dice: 5,
                max_blessings: 2,
                blessings: vec![
                    DicePoolBlessingDef {
                        name: "Rage-fuelled Invigoration".to_string(),
                        requirement: DiceRequirement::DoubleTwoPlus,
                        effects: vec![RulePrimitive::TriggerMoveReaction {
                            move_type: ReactionMoveType::PileInAndConsolidation,
                            distance: MoveDistance::Fixed(Inches::from_inches(6)),
                        }],
                        description: "Pile-in and Consolidation moves up to 6\" instead of 3\".".to_string(),
                    },
                    DicePoolBlessingDef {
                        name: "Total Carnage".to_string(),
                        requirement: DiceRequirement::DoubleTwoPlus,
                        effects: vec![RulePrimitive::FightOnDeath {
                            threshold: 4,
                            modifier_source: None,
                        }],
                        description: "Destroyed models fight on 4+ before removal.".to_string(),
                    },
                    DicePoolBlessingDef {
                        name: "Martial Excellence".to_string(),
                        requirement: DiceRequirement::DoubleFourPlusOrTripleAny,
                        effects: vec![RulePrimitive::GrantWeaponAbility {
                            weapon_ability: WeaponAbilityDef::SustainedHits(1),
                            weapon_filter: WeaponFilter::MeleeOnly,
                        }],
                        description: "Melee weapons have [SUSTAINED HITS 1].".to_string(),
                    },
                ],
            }],
            trigger: Some(AbilityTrigger::StartOfBattleRound),
        };
        let yaml = serde_yaml::to_string(&blessings).unwrap();
        let back: AbilitySchema = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(blessings, back);
    }

    #[test]
    fn test_ability_schema_a_worthy_skull() {
        let worthy_skull = AbilitySchema {
            name: "A Worthy Skull".to_string(),
            description: "Re-roll Hit and Wound vs CHARACTER. Gain 1CP on CHARACTER kill.".to_string(),
            ability_type: AbilityType::UnitAbility,
            effects: vec![
                RulePrimitive::Conditional {
                    condition: Condition::TargetIsCharacter,
                    effect: Box::new(RulePrimitive::ApplyReroll {
                        scope: RerollScope::HitAndWoundRolls,
                        condition: None,
                    }),
                    else_effect: None,
                },
                RulePrimitive::Conditional {
                    condition: Condition::All(vec![
                        Condition::TargetIsCharacter,
                        Condition::DestroyedEnemyUnitThisPhase,
                    ]),
                    effect: Box::new(RulePrimitive::AlterCP { amount: 1 }),
                    else_effect: None,
                },
            ],
            trigger: Some(AbilityTrigger::OnAttack),
        };
        let json = serde_json::to_string(&worthy_skull).unwrap();
        let back: AbilitySchema = serde_json::from_str(&json).unwrap();
        assert_eq!(worthy_skull, back);
    }

    // ── RulePrimitive ───────────────────────────────────────────────────

    #[test]
    fn test_rule_primitive_add_modifier() {
        let prim = RulePrimitive::AddModifier {
            target: ModifierTarget::Bearer,
            stat: StatTarget::Wounds,
            value: 1,
        };
        let json = serde_json::to_string(&prim).unwrap();
        let back: RulePrimitive = serde_json::from_str(&json).unwrap();
        assert_eq!(prim, back);
    }

    #[test]
    fn test_rule_primitive_grant_weapon_ability() {
        let prim = RulePrimitive::GrantWeaponAbility {
            weapon_ability: WeaponAbilityDef::SustainedHits(1),
            weapon_filter: WeaponFilter::MeleeOnly,
        };
        let json = serde_json::to_string(&prim).unwrap();
        let back: RulePrimitive = serde_json::from_str(&json).unwrap();
        assert_eq!(prim, back);
    }

    #[test]
    fn test_rule_primitive_alter_oc() {
        let prim = RulePrimitive::AlterOC {
            target: ModifierTarget::Bearer,
            new_oc: OCModifier::SetTo(4),
        };
        let json = serde_json::to_string(&prim).unwrap();
        let back: RulePrimitive = serde_json::from_str(&json).unwrap();
        assert_eq!(prim, back);
    }

    #[test]
    fn test_rule_primitive_dice_check() {
        // Warrior Exemplar: roll D6, on 3+ gain 1CP
        let prim = RulePrimitive::DiceCheck {
            threshold: 3,
            on_success: Box::new(RulePrimitive::AlterCP { amount: 1 }),
            on_failure: None,
            modifier: 0,
        };
        let json = serde_json::to_string(&prim).unwrap();
        let back: RulePrimitive = serde_json::from_str(&json).unwrap();
        assert_eq!(prim, back);
    }

    #[test]
    fn test_rule_primitive_apply_for_duration() {
        let prim = RulePrimitive::ApplyForDuration {
            effect: Box::new(RulePrimitive::AddModifier {
                target: ModifierTarget::Self_,
                stat: StatTarget::AdvanceRoll,
                value: 2,
            }),
            duration: EffectDuration::UntilEndOfPhase,
        };
        let json = serde_json::to_string(&prim).unwrap();
        let back: RulePrimitive = serde_json::from_str(&json).unwrap();
        assert_eq!(prim, back);
    }

    #[test]
    fn test_rule_primitive_composite() {
        let prim = RulePrimitive::Composite {
            effects: vec![
                RulePrimitive::GrantCover,
                RulePrimitive::GrantFeelNoPain { value: 6 },
            ],
        };
        let json = serde_json::to_string(&prim).unwrap();
        let back: RulePrimitive = serde_json::from_str(&json).unwrap();
        assert_eq!(prim, back);
    }

    #[test]
    fn test_rule_primitive_fight_on_death_with_modifier() {
        // Berserk Resilience: 4+ (2+ with Total Carnage)
        let prim = RulePrimitive::FightOnDeath {
            threshold: 4,
            modifier_source: Some("Total Carnage".to_string()),
        };
        let json = serde_json::to_string(&prim).unwrap();
        let back: RulePrimitive = serde_json::from_str(&json).unwrap();
        assert_eq!(prim, back);
    }

    #[test]
    fn test_rule_primitive_tank_shock() {
        let prim = RulePrimitive::TankShock {
            dice_count_stat: CharacteristicTarget::Toughness,
            threshold: 5,
            max_mortal_wounds: 6,
        };
        let json = serde_json::to_string(&prim).unwrap();
        let back: RulePrimitive = serde_json::from_str(&json).unwrap();
        assert_eq!(prim, back);
    }

    // ── DicePoolBlessingDef ─────────────────────────────────────────────

    #[test]
    fn test_dice_pool_blessing_def() {
        let blessing = DicePoolBlessingDef {
            name: "Martial Excellence".to_string(),
            requirement: DiceRequirement::DoubleFourPlusOrTripleAny,
            effects: vec![RulePrimitive::GrantWeaponAbility {
                weapon_ability: WeaponAbilityDef::SustainedHits(1),
                weapon_filter: WeaponFilter::MeleeOnly,
            }],
            description: "Melee weapons have [SUSTAINED HITS 1].".to_string(),
        };
        let json = serde_json::to_string(&blessing).unwrap();
        let back: DicePoolBlessingDef = serde_json::from_str(&json).unwrap();
        assert_eq!(blessing, back);
    }

    // ── StanceDef ───────────────────────────────────────────────────────

    #[test]
    fn test_stance_def() {
        let stance = StanceDef {
            name: "Dacatarai".to_string(),
            effects: vec![RulePrimitive::GrantWeaponAbility {
                weapon_ability: WeaponAbilityDef::SustainedHits(1),
                weapon_filter: WeaponFilter::MeleeOnly,
            }],
            description: "Melee weapons have [SUSTAINED HITS 1]".to_string(),
        };
        let yaml = serde_yaml::to_string(&stance).unwrap();
        let back: StanceDef = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(stance, back);
    }

    // ── Condition ───────────────────────────────────────────────────────

    #[test]
    fn test_condition_serialization() {
        let conditions = vec![
            Condition::HasKeyword(Keyword::Infantry),
            Condition::InEngagementRange,
            Condition::BattleShocked,
            Condition::NotBattleShocked,
            Condition::BelowHalfStrength,
            Condition::WithinRange(Inches::from_inches(6)),
            Condition::OnObjective,
            Condition::ChargedThisTurn,
            Condition::IsCharacter,
            Condition::TargetIsCharacter,
            Condition::BattleRoundOrLater(3),
            Condition::All(vec![
                Condition::OnObjectiveInNoMansLand,
                Condition::IsCharacter,
                Condition::NotBattleShocked,
            ]),
            Condition::Any(vec![
                Condition::IsMonster,
                Condition::IsVehicle,
            ]),
            Condition::Not(Box::new(Condition::BattleShocked)),
            Condition::BlessingActive("Total Carnage".to_string()),
            Condition::DiceRollMeetsThreshold(3),
            Condition::ControlsClosestObjective(ObjectiveProximity::OwnEdge),
        ];

        for cond in &conditions {
            let json = serde_json::to_string(cond).unwrap();
            let back: Condition = serde_json::from_str(&json).unwrap();
            assert_eq!(*cond, back);
        }
    }

    // ── TargetRestriction ───────────────────────────────────────────────

    #[test]
    fn test_target_restriction_custodes() {
        // "One ADEPTUS CUSTODES unit from your army"
        let restriction = TargetRestriction {
            required_keywords: vec![Keyword::AdeptusCustodes],
            excluded_keywords: vec![],
            owner: TargetOwner::Friendly,
            conditions: vec![],
        };
        let json = serde_json::to_string(&restriction).unwrap();
        let back: TargetRestriction = serde_json::from_str(&json).unwrap();
        assert_eq!(restriction, back);
    }

    #[test]
    fn test_target_restriction_berzerkers() {
        // "One KHORNE BERZERKERS unit from your army"
        let restriction = TargetRestriction {
            required_keywords: vec![Keyword::Berzerkers],
            excluded_keywords: vec![],
            owner: TargetOwner::Friendly,
            conditions: vec![],
        };
        assert_eq!(restriction.required_keywords.len(), 1);
    }

    // ── StratagemSchema ─────────────────────────────────────────────────

    #[test]
    fn test_stratagem_schema_gilded_spear() {
        let stratagem = StratagemSchema {
            name: "The Gilded Spear".to_string(),
            cp_cost: 1,
            phase: Phase::Fight,
            timing: StratagemTiming::DuringPhase,
            stratagem_type: StratagemType::BattleTactic,
            target_restriction: TargetRestriction {
                required_keywords: vec![Keyword::AdeptusCustodes],
                excluded_keywords: vec![],
                owner: TargetOwner::Friendly,
                conditions: vec![Condition::HasNotFoughtThisPhase],
            },
            effects: vec![RulePrimitive::ApplyForDuration {
                effect: Box::new(RulePrimitive::TriggerMoveReaction {
                    move_type: ReactionMoveType::PileIn,
                    distance: MoveDistance::Fixed(Inches::from_inches(6)),
                }),
                duration: EffectDuration::UntilEndOfPhase,
            }],
            once_per_battle: false,
            once_per_turn: true,
            description: "Pile-in moves up to 6\" instead of 3\".".to_string(),
        };
        let json = serde_json::to_string(&stratagem).unwrap();
        let back: StratagemSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(stratagem, back);
    }

    #[test]
    fn test_stratagem_schema_horrifying_butchery() {
        let stratagem = StratagemSchema {
            name: "Horrifying Butchery".to_string(),
            cp_cost: 1,
            phase: Phase::Fight,
            timing: StratagemTiming::StartOfPhase,
            stratagem_type: StratagemType::BattleTactic,
            target_restriction: TargetRestriction {
                required_keywords: vec![Keyword::Berzerkers],
                excluded_keywords: vec![],
                owner: TargetOwner::Friendly,
                conditions: vec![],
            },
            effects: vec![RulePrimitive::ForceBattleShockTest {
                target: ModifierTarget::SelectedEnemy,
            }],
            once_per_battle: false,
            once_per_turn: true,
            description: "Select enemy unit in ER; it must take a Battle-shock test.".to_string(),
        };
        assert_eq!(stratagem.cp_cost, 1);
        assert_eq!(stratagem.phase, Phase::Fight);
    }

    #[test]
    fn test_stratagem_schema_bloodlust() {
        let stratagem = StratagemSchema {
            name: "Bloodlust".to_string(),
            cp_cost: 1,
            phase: Phase::Shooting,
            timing: StratagemTiming::AfterEnemyShoots,
            stratagem_type: StratagemType::StrategicPloy,
            target_restriction: TargetRestriction {
                required_keywords: vec![Keyword::Jakhals],
                excluded_keywords: vec![],
                owner: TargetOwner::Friendly,
                conditions: vec![],
            },
            effects: vec![RulePrimitive::TriggerMoveReaction {
                move_type: ReactionMoveType::NormalMove,
                distance: MoveDistance::D6,
            }],
            once_per_battle: false,
            once_per_turn: true,
            description: "Jakhals move D6\" after being shot.".to_string(),
        };
        let yaml = serde_yaml::to_string(&stratagem).unwrap();
        let back: StratagemSchema = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(stratagem, back);
    }

    #[test]
    fn test_stratagem_schema_overawing_magnificence() {
        let stratagem = StratagemSchema {
            name: "Overawing Magnificence".to_string(),
            cp_cost: 1,
            phase: Phase::Charge,
            timing: StratagemTiming::AfterChargeDeclaration,
            stratagem_type: StratagemType::BattleTactic,
            target_restriction: TargetRestriction {
                required_keywords: vec![Keyword::AdeptusCustodes],
                excluded_keywords: vec![],
                owner: TargetOwner::Friendly,
                conditions: vec![Condition::WasChargeTarget],
            },
            effects: vec![RulePrimitive::ForceLeadershipTest {
                target: ModifierTarget::SelectedEnemy,
                on_fail: Box::new(RulePrimitive::ApplyForDuration {
                    effect: Box::new(RulePrimitive::AddModifier {
                        target: ModifierTarget::SelectedEnemy,
                        stat: StatTarget::ChargeRoll,
                        value: -2,
                    }),
                    duration: EffectDuration::UntilEndOfPhase,
                }),
            }],
            once_per_battle: false,
            once_per_turn: true,
            description: "Enemy takes Ld test; on fail, -2 to Charge rolls.".to_string(),
        };
        let json = serde_json::to_string(&stratagem).unwrap();
        let back: StratagemSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(stratagem, back);
    }

    #[test]
    fn test_stratagem_schema_berserk_resilience() {
        let stratagem = StratagemSchema {
            name: "Berserk Resilience".to_string(),
            cp_cost: 1,
            phase: Phase::Fight,
            timing: StratagemTiming::AfterTargetSelection,
            stratagem_type: StratagemType::BattleTactic,
            target_restriction: TargetRestriction {
                required_keywords: vec![Keyword::WorldEaters, Keyword::Character],
                excluded_keywords: vec![],
                owner: TargetOwner::Friendly,
                conditions: vec![],
            },
            effects: vec![RulePrimitive::ApplyForDuration {
                effect: Box::new(RulePrimitive::FightOnDeath {
                    threshold: 4,
                    modifier_source: Some("Total Carnage".to_string()),
                }),
                duration: EffectDuration::UntilEndOfPhase,
            }],
            once_per_battle: false,
            once_per_turn: true,
            description: "CHARACTER fights on death (4+, 2+ with Total Carnage).".to_string(),
        };
        assert_eq!(stratagem.cp_cost, 1);
    }

    // ── EnhancementSchema ───────────────────────────────────────────────

    #[test]
    fn test_enhancement_schema_watchman_of_terra() {
        let enhancement = EnhancementSchema {
            name: "Watchman of Terra".to_string(),
            description: "While within Engagement Range of enemies, OC becomes 4.".to_string(),
            is_default: true,
            effects: vec![RulePrimitive::Conditional {
                condition: Condition::InEngagementRange,
                effect: Box::new(RulePrimitive::AlterOC {
                    target: ModifierTarget::Bearer,
                    new_oc: OCModifier::SetTo(4),
                }),
                else_effect: None,
            }],
            conditions: vec![Condition::InEngagementRange],
        };
        let json = serde_json::to_string(&enhancement).unwrap();
        let back: EnhancementSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(enhancement, back);
    }

    #[test]
    fn test_enhancement_schema_warrior_exemplar() {
        let enhancement = EnhancementSchema {
            name: "Warrior Exemplar".to_string(),
            description: "Each time Tristraen destroys an enemy unit, roll D6; on 3+, gain 1CP.".to_string(),
            is_default: false,
            effects: vec![RulePrimitive::TriggerOnDestroyed {
                effect: Box::new(RulePrimitive::DiceCheck {
                    threshold: 3,
                    on_success: Box::new(RulePrimitive::AlterCP { amount: 1 }),
                    on_failure: None,
                    modifier: 0,
                }),
            }],
            conditions: vec![],
        };
        assert!(!enhancement.is_default);
    }

    #[test]
    fn test_enhancement_schema_fearsome_presence() {
        let enhancement = EnhancementSchema {
            name: "Fearsome Presence".to_string(),
            description: "While not Battle-shocked, OC becomes 5.".to_string(),
            is_default: true,
            effects: vec![RulePrimitive::Conditional {
                condition: Condition::NotBattleShocked,
                effect: Box::new(RulePrimitive::AlterOC {
                    target: ModifierTarget::Bearer,
                    new_oc: OCModifier::SetTo(5),
                }),
                else_effect: None,
            }],
            conditions: vec![Condition::NotBattleShocked],
        };
        assert!(enhancement.is_default);
    }

    #[test]
    fn test_enhancement_schema_bane_of_the_craven() {
        let enhancement = EnhancementSchema {
            name: "Bane of the Craven".to_string(),
            description: "Enemies that Fall Back from bearer must take Desperate Escape tests.".to_string(),
            is_default: false,
            effects: vec![RulePrimitive::Conditional {
                condition: Condition::All(vec![
                    Condition::TargetNotMonsterOrVehicle,
                ]),
                effect: Box::new(RulePrimitive::ForceDespairEscapeTest {
                    modifier: 0,
                    condition: Some(Condition::BattleShocked),
                }),
                else_effect: None,
            }],
            conditions: vec![],
        };
        assert!(!enhancement.is_default);
    }

    // ── SecondaryObjectiveSchema ────────────────────────────────────────

    #[test]
    fn test_secondary_objective_raise_the_vexillas() {
        let secondary = SecondaryObjectiveSchema {
            name: "Raise the Vexillas".to_string(),
            description: "Score 3VP for controlling both closest objectives from round 3.".to_string(),
            is_default: true,
            scoring: vec![ScoringRule {
                timing: ScoringTiming {
                    phase: Phase::Fight, // End of turn (after Fight)
                    whose_turn: TurnOwner::Active,
                    from_round: 3,
                },
                condition: Condition::All(vec![
                    Condition::ControlsClosestObjective(ObjectiveProximity::OwnEdge),
                    Condition::ControlsClosestObjective(ObjectiveProximity::OpponentEdge),
                ]),
                vp_amount: 3,
                description: Some("Control both closest objectives".to_string()),
            }],
            max_vp: Some(9), // Rounds 3, 4, 5 = max 9VP
        };
        let json = serde_json::to_string(&secondary).unwrap();
        let back: SecondaryObjectiveSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(secondary, back);
    }

    #[test]
    fn test_secondary_objective_consecrated_ground() {
        let secondary = SecondaryObjectiveSchema {
            name: "Consecrated Ground".to_string(),
            description: "+3VP per enemy unit destroyed, -1VP per Custodes model lost.".to_string(),
            is_default: false,
            scoring: vec![
                ScoringRule {
                    timing: ScoringTiming {
                        phase: Phase::Fight,
                        whose_turn: TurnOwner::Either,
                        from_round: 1,
                    },
                    condition: Condition::DestroyedEnemyUnitThisPhase,
                    vp_amount: 3,
                    description: Some("Enemy unit destroyed".to_string()),
                },
                ScoringRule {
                    timing: ScoringTiming {
                        phase: Phase::Fight,
                        whose_turn: TurnOwner::Either,
                        from_round: 1,
                    },
                    condition: Condition::HasKeyword(Keyword::AdeptusCustodes),
                    vp_amount: -1,
                    description: Some("Custodes model destroyed".to_string()),
                },
            ],
            max_vp: None, // Unlimited (but practically bounded)
        };
        assert!(!secondary.is_default);
        assert_eq!(secondary.scoring.len(), 2);
    }

    #[test]
    fn test_secondary_objective_champions_of_khorne() {
        let secondary = SecondaryObjectiveSchema {
            name: "Champions of Khorne".to_string(),
            description: "Score VP for CHARACTER controlling objectives in NML from round 2.".to_string(),
            is_default: true,
            scoring: vec![
                ScoringRule {
                    timing: ScoringTiming {
                        phase: Phase::Command,
                        whose_turn: TurnOwner::Active,
                        from_round: 2,
                    },
                    condition: Condition::All(vec![
                        Condition::ObjectiveCount {
                            condition: Box::new(Condition::All(vec![
                                Condition::OnObjectiveInNoMansLand,
                                Condition::IsCharacter,
                                Condition::NotBattleShocked,
                            ])),
                            count: 1,
                        },
                    ]),
                    vp_amount: 2,
                    description: Some("One CHARACTER on NML objective".to_string()),
                },
                ScoringRule {
                    timing: ScoringTiming {
                        phase: Phase::Command,
                        whose_turn: TurnOwner::Active,
                        from_round: 2,
                    },
                    condition: Condition::ObjectiveCount {
                        condition: Box::new(Condition::All(vec![
                            Condition::OnObjectiveInNoMansLand,
                            Condition::IsCharacter,
                            Condition::NotBattleShocked,
                        ])),
                        count: 2,
                    },
                    vp_amount: 3,
                    description: Some("Two CHARACTERs on NML objectives".to_string()),
                },
            ],
            max_vp: Some(12),
        };
        assert!(secondary.is_default);
    }

    #[test]
    fn test_secondary_objective_skull_takers() {
        let secondary = SecondaryObjectiveSchema {
            name: "Skull Takers".to_string(),
            description: "Score 3VP for each CHARACTER that destroys an enemy unit in Fight.".to_string(),
            is_default: false,
            scoring: vec![ScoringRule {
                timing: ScoringTiming {
                    phase: Phase::Fight,
                    whose_turn: TurnOwner::Either,
                    from_round: 1,
                },
                condition: Condition::All(vec![
                    Condition::IsCharacter,
                    Condition::DestroyedEnemyUnitThisPhase,
                ]),
                vp_amount: 3,
                description: Some("CHARACTER destroyed enemy unit in Fight".to_string()),
            }],
            max_vp: Some(12),
        };
        assert!(!secondary.is_default);
    }

    // ── DatasheetSchema ─────────────────────────────────────────────────

    #[test]
    fn test_datasheet_schema_tristraen() {
        let tristraen = DatasheetSchema {
            id: DatasheetId::new(1),
            name: "Tristraen".to_string(),
            keywords: KeywordSet::TRISTRAEN_KEYWORDS,
            movement: MoveCharacteristic::from_inches(6),
            toughness: Toughness::new(6),
            armor_save: ArmorSave::TWO_PLUS,
            invulnerable_save: Some(InvulnerableSave::FOUR_PLUS),
            wounds: Wounds::new(6),
            leadership: Leadership::new(6),
            objective_control: ObjectiveControl::new(2),
            base_size: BaseSize::MM40,
            ranged_weapons: vec![],
            melee_weapons: vec![
                WeaponProfileSchema {
                    name: "Vaultswords - Behemor".to_string(),
                    weapon_type: WeaponType::Melee,
                    range: Inches::ZERO,
                    attacks: AttackCount::Fixed(6),
                    skill: Skill::TWO_PLUS,
                    strength: Strength::new(7),
                    ap: ArmorPenetration::MINUS_2,
                    damage: Damage::Fixed(2),
                    abilities: vec![WeaponAbilityDef::Precision],
                },
                WeaponProfileSchema {
                    name: "Vaultswords - Hurricanus".to_string(),
                    weapon_type: WeaponType::Melee,
                    range: Inches::ZERO,
                    attacks: AttackCount::Fixed(9),
                    skill: Skill::TWO_PLUS,
                    strength: Strength::new(5),
                    ap: ArmorPenetration::MINUS_1,
                    damage: Damage::Fixed(1),
                    abilities: vec![WeaponAbilityDef::SustainedHits(1)],
                },
                WeaponProfileSchema {
                    name: "Vaultswords - Victus".to_string(),
                    weapon_type: WeaponType::Melee,
                    range: Inches::ZERO,
                    attacks: AttackCount::Fixed(5),
                    skill: Skill::TWO_PLUS,
                    strength: Strength::new(6),
                    ap: ArmorPenetration::MINUS_3,
                    damage: Damage::Fixed(3),
                    abilities: vec![WeaponAbilityDef::DevastatingWounds],
                },
            ],
            abilities: vec![AbilitySchema {
                name: "Martial Ka'tah".to_string(),
                description: "Choose stance when selected to fight.".to_string(),
                ability_type: AbilityType::FactionAbility,
                effects: vec![RulePrimitive::StanceChoice {
                    stances: vec![
                        StanceDef {
                            name: "Dacatarai".to_string(),
                            effects: vec![RulePrimitive::GrantWeaponAbility {
                                weapon_ability: WeaponAbilityDef::SustainedHits(1),
                                weapon_filter: WeaponFilter::MeleeOnly,
                            }],
                            description: "Melee: Sustained Hits 1".to_string(),
                        },
                        StanceDef {
                            name: "Rendax".to_string(),
                            effects: vec![RulePrimitive::GrantWeaponAbility {
                                weapon_ability: WeaponAbilityDef::LethalHits,
                                weapon_filter: WeaponFilter::MeleeOnly,
                            }],
                            description: "Melee: Lethal Hits".to_string(),
                        },
                    ],
                }],
                trigger: Some(AbilityTrigger::OnSelectedToFight),
            }],
            unit_size: UnitSizeSpec::Fixed(1),
            model_loadouts: vec![ModelLoadout {
                model_label: "Tristraen".to_string(),
                count: 1,
                ranged_weapons: vec![],
                melee_weapons: vec![
                    "Vaultswords - Behemor".to_string(),
                    "Vaultswords - Hurricanus".to_string(),
                    "Vaultswords - Victus".to_string(),
                ],
            }],
            wargear_abilities: vec![],
        };

        let json = serde_json::to_string(&tristraen).unwrap();
        let back: DatasheetSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(tristraen.name, back.name);
        assert_eq!(tristraen.melee_weapons.len(), 3);
        assert_eq!(tristraen.wounds, Wounds::new(6));
    }

    #[test]
    fn test_datasheet_schema_custodian_guard() {
        let guard = DatasheetSchema {
            id: DatasheetId::new(2),
            name: "Custodian Guard".to_string(),
            keywords: KeywordSet::CUSTODIAN_GUARD_KEYWORDS,
            movement: MoveCharacteristic::from_inches(6),
            toughness: Toughness::new(6),
            armor_save: ArmorSave::TWO_PLUS,
            invulnerable_save: Some(InvulnerableSave::FOUR_PLUS),
            wounds: Wounds::new(3),
            leadership: Leadership::new(6),
            objective_control: ObjectiveControl::new(2),
            base_size: BaseSize::MM40,
            ranged_weapons: vec![WeaponProfileSchema {
                name: "Sentinel blade".to_string(),
                weapon_type: WeaponType::Ranged,
                range: Inches::from_inches(12),
                attacks: AttackCount::Fixed(2),
                skill: Skill::TWO_PLUS,
                strength: Strength::new(4),
                ap: ArmorPenetration::MINUS_1,
                damage: Damage::Fixed(2),
                abilities: vec![WeaponAbilityDef::Assault, WeaponAbilityDef::Pistol],
            }],
            melee_weapons: vec![WeaponProfileSchema {
                name: "Sentinel blade".to_string(),
                weapon_type: WeaponType::Melee,
                range: Inches::ZERO,
                attacks: AttackCount::Fixed(5),
                skill: Skill::TWO_PLUS,
                strength: Strength::new(6),
                ap: ArmorPenetration::MINUS_2,
                damage: Damage::Fixed(1),
                abilities: vec![],
            }],
            abilities: vec![],
            unit_size: UnitSizeSpec::Fixed(3),
            model_loadouts: vec![ModelLoadout {
                model_label: "Custodian Guard".to_string(),
                count: 3,
                ranged_weapons: vec!["Sentinel blade".to_string()],
                melee_weapons: vec!["Sentinel blade".to_string()],
            }],
            wargear_abilities: vec![WargearAbility {
                name: "Praesidium Shield".to_string(),
                description: "Add 1 to the bearer's Wounds characteristic.".to_string(),
                effects: vec![RulePrimitive::AddModifier {
                    target: ModifierTarget::Bearer,
                    stat: StatTarget::Wounds,
                    value: 1,
                }],
                model_labels: vec![], // All models
            }],
        };

        assert_eq!(guard.unit_size.min_models(), 3);
        assert_eq!(guard.wargear_abilities.len(), 1);
        // With Praesidium Shield, effective wounds should be 4 per model
    }

    #[test]
    fn test_datasheet_schema_vorrakh() {
        let vorrakh = DatasheetSchema {
            id: DatasheetId::new(10),
            name: "Vorrakh".to_string(),
            keywords: KeywordSet::VORRAKH_KEYWORDS,
            movement: MoveCharacteristic::from_inches(10),
            toughness: Toughness::new(10),
            armor_save: ArmorSave::TWO_PLUS,
            invulnerable_save: Some(InvulnerableSave::FOUR_PLUS),
            wounds: Wounds::new(10),
            leadership: Leadership::new(6),
            objective_control: ObjectiveControl::new(3),
            base_size: BaseSize::MM60,
            ranged_weapons: vec![WeaponProfileSchema {
                name: "Infernal cannon".to_string(),
                weapon_type: WeaponType::Ranged,
                range: Inches::from_inches(24),
                attacks: AttackCount::Fixed(3),
                skill: Skill::THREE_PLUS,
                strength: Strength::new(5),
                ap: ArmorPenetration::MINUS_1,
                damage: Damage::Fixed(2),
                abilities: vec![WeaponAbilityDef::RapidFire(1)],
            }],
            melee_weapons: vec![WeaponProfileSchema {
                name: "Hellforged weapons".to_string(),
                weapon_type: WeaponType::Melee,
                range: Inches::ZERO,
                attacks: AttackCount::Fixed(16),
                skill: Skill::TWO_PLUS,
                strength: Strength::new(6),
                ap: ArmorPenetration::MINUS_1,
                damage: Damage::Fixed(1),
                abilities: vec![],
            }],
            abilities: vec![],
            unit_size: UnitSizeSpec::Fixed(1),
            model_loadouts: vec![ModelLoadout {
                model_label: "Vorrakh".to_string(),
                count: 1,
                ranged_weapons: vec!["Infernal cannon".to_string()],
                melee_weapons: vec!["Hellforged weapons".to_string()],
            }],
            wargear_abilities: vec![],
        };

        assert_eq!(vorrakh.toughness, Toughness::new(10));
        assert_eq!(vorrakh.wounds, Wounds::new(10));
        assert_eq!(vorrakh.objective_control, ObjectiveControl::new(3));
    }

    #[test]
    fn test_datasheet_schema_master_of_executions() {
        let moe = DatasheetSchema {
            id: DatasheetId::new(11),
            name: "Master of Executions".to_string(),
            keywords: KeywordSet::MASTER_OF_EXECUTIONS_KEYWORDS,
            movement: MoveCharacteristic::from_inches(8),
            toughness: Toughness::new(4),
            armor_save: ArmorSave::THREE_PLUS,
            invulnerable_save: None,
            wounds: Wounds::new(4),
            leadership: Leadership::new(6),
            objective_control: ObjectiveControl::new(1),
            base_size: BaseSize::MM40,
            ranged_weapons: vec![WeaponProfileSchema {
                name: "Bolt pistol".to_string(),
                weapon_type: WeaponType::Ranged,
                range: Inches::from_inches(12),
                attacks: AttackCount::Fixed(1),
                skill: Skill::THREE_PLUS,
                strength: Strength::new(4),
                ap: ArmorPenetration::ZERO,
                damage: Damage::Fixed(1),
                abilities: vec![WeaponAbilityDef::Pistol],
            }],
            melee_weapons: vec![WeaponProfileSchema {
                name: "Axe of dismemberment".to_string(),
                weapon_type: WeaponType::Melee,
                range: Inches::ZERO,
                attacks: AttackCount::Fixed(5),
                skill: Skill::TWO_PLUS,
                strength: Strength::new(7),
                ap: ArmorPenetration::MINUS_2,
                damage: Damage::Fixed(2),
                abilities: vec![
                    WeaponAbilityDef::DevastatingWounds,
                    WeaponAbilityDef::Precision,
                ],
            }],
            abilities: vec![AbilitySchema {
                name: "A Worthy Skull".to_string(),
                description: "Re-roll Hit and Wound vs CHARACTER. Gain 1CP on CHARACTER kill.".to_string(),
                ability_type: AbilityType::UnitAbility,
                effects: vec![
                    RulePrimitive::Conditional {
                        condition: Condition::TargetIsCharacter,
                        effect: Box::new(RulePrimitive::ApplyReroll {
                            scope: RerollScope::HitAndWoundRolls,
                            condition: None,
                        }),
                        else_effect: None,
                    },
                    RulePrimitive::Conditional {
                        condition: Condition::All(vec![
                            Condition::TargetIsCharacter,
                            Condition::DestroyedEnemyUnitThisPhase,
                        ]),
                        effect: Box::new(RulePrimitive::AlterCP { amount: 1 }),
                        else_effect: None,
                    },
                ],
                trigger: Some(AbilityTrigger::OnAttack),
            }],
            unit_size: UnitSizeSpec::Fixed(1),
            model_loadouts: vec![ModelLoadout {
                model_label: "Master of Executions".to_string(),
                count: 1,
                ranged_weapons: vec!["Bolt pistol".to_string()],
                melee_weapons: vec!["Axe of dismemberment".to_string()],
            }],
            wargear_abilities: vec![],
        };

        assert_eq!(moe.invulnerable_save, None);
        assert_eq!(moe.abilities.len(), 1);
        assert_eq!(moe.abilities[0].name, "A Worthy Skull");
    }

    #[test]
    fn test_datasheet_schema_jakhals() {
        let jakhals = DatasheetSchema {
            id: DatasheetId::new(13),
            name: "Jakhals".to_string(),
            keywords: KeywordSet::JAKHALS_KEYWORDS,
            movement: MoveCharacteristic::from_inches(7),
            toughness: Toughness::new(4),
            armor_save: ArmorSave::SIX_PLUS,
            invulnerable_save: None,
            wounds: Wounds::new(1),
            leadership: Leadership::new(7),
            objective_control: ObjectiveControl::new(1),
            base_size: BaseSize::MM28,
            ranged_weapons: vec![WeaponProfileSchema {
                name: "Autopistol".to_string(),
                weapon_type: WeaponType::Ranged,
                range: Inches::from_inches(12),
                attacks: AttackCount::Fixed(1),
                skill: Skill::FOUR_PLUS,
                strength: Strength::new(3),
                ap: ArmorPenetration::ZERO,
                damage: Damage::Fixed(1),
                abilities: vec![WeaponAbilityDef::Pistol],
            }],
            melee_weapons: vec![
                WeaponProfileSchema {
                    name: "Chainblades".to_string(),
                    weapon_type: WeaponType::Melee,
                    range: Inches::ZERO,
                    attacks: AttackCount::Fixed(3),
                    skill: Skill::FOUR_PLUS,
                    strength: Strength::new(3),
                    ap: ArmorPenetration::ZERO,
                    damage: Damage::Fixed(1),
                    abilities: vec![],
                },
                WeaponProfileSchema {
                    name: "Mauler chainblade".to_string(),
                    weapon_type: WeaponType::Melee,
                    range: Inches::ZERO,
                    attacks: AttackCount::Fixed(3),
                    skill: Skill::FIVE_PLUS,
                    strength: Strength::new(4),
                    ap: ArmorPenetration::MINUS_1,
                    damage: Damage::Fixed(2),
                    abilities: vec![],
                },
                WeaponProfileSchema {
                    name: "Skullsmasher and mangler".to_string(),
                    weapon_type: WeaponType::Melee,
                    range: Inches::ZERO,
                    attacks: AttackCount::Fixed(2),
                    skill: Skill::FOUR_PLUS,
                    strength: Strength::new(4),
                    ap: ArmorPenetration::MINUS_1,
                    damage: Damage::Fixed(2),
                    abilities: vec![],
                },
            ],
            abilities: vec![],
            unit_size: UnitSizeSpec::Fixed(10),
            model_loadouts: vec![
                ModelLoadout {
                    model_label: "Jakhal Pack Leader".to_string(),
                    count: 1,
                    ranged_weapons: vec!["Autopistol".to_string()],
                    melee_weapons: vec!["Chainblades".to_string()],
                },
                ModelLoadout {
                    model_label: "Dishonoured".to_string(),
                    count: 1,
                    ranged_weapons: vec![],
                    melee_weapons: vec!["Skullsmasher and mangler".to_string()],
                },
                ModelLoadout {
                    model_label: "Mauler Jakhal".to_string(),
                    count: 1,
                    ranged_weapons: vec!["Autopistol".to_string()],
                    melee_weapons: vec!["Mauler chainblade".to_string()],
                },
                ModelLoadout {
                    model_label: "Standard Jakhal".to_string(),
                    count: 7,
                    ranged_weapons: vec!["Autopistol".to_string()],
                    melee_weapons: vec!["Chainblades".to_string()],
                },
            ],
            wargear_abilities: vec![],
        };

        let total_models: u8 = jakhals.model_loadouts.iter().map(|l| l.count).sum();
        assert_eq!(total_models, 10);
        assert_eq!(jakhals.leadership, Leadership::new(7));
        assert_eq!(jakhals.armor_save, ArmorSave::SIX_PLUS);
    }

    // ── FactionSchema ───────────────────────────────────────────────────

    #[test]
    fn test_faction_schema_minimal() {
        let faction = FactionSchema {
            id: FactionId::new(1),
            name: "Test Faction".to_string(),
            faction_keywords: KeywordSet::from_keywords(&[Keyword::Imperium, Keyword::AdeptusCustodes]),
            faction_ability: AbilitySchema {
                name: "Test Ability".to_string(),
                description: "A test ability.".to_string(),
                ability_type: AbilityType::FactionAbility,
                effects: vec![],
                trigger: None,
            },
            datasheets: vec![],
            stratagems: vec![],
            enhancements: vec![],
            secondary_objectives: vec![],
        };

        let json = serde_json::to_string(&faction).unwrap();
        let back: FactionSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(faction, back);
    }

    // ── MissionSchema ───────────────────────────────────────────────────

    #[test]
    fn test_mission_schema() {
        let mission = MissionSchema {
            id: MissionId::new(1),
            name: "Supply Drop".to_string(),
            deployment_map: DeploymentMapId::new(1),
            objectives: vec![
                ObjectiveDef {
                    label: "A".to_string(),
                    position_x: Inches::from_inches(22),
                    position_y: Inches::from_inches(5),
                    zone: ObjectiveZone::AttackerZone,
                    control_range: Inches::from_inches(3),
                },
                ObjectiveDef {
                    label: "B".to_string(),
                    position_x: Inches::from_inches(22),
                    position_y: Inches::from_inches(15),
                    zone: ObjectiveZone::NoMansLand,
                    control_range: Inches::from_inches(3),
                },
                ObjectiveDef {
                    label: "C".to_string(),
                    position_x: Inches::from_inches(22),
                    position_y: Inches::from_inches(25),
                    zone: ObjectiveZone::DefenderZone,
                    control_range: Inches::from_inches(3),
                },
            ],
            primary_scoring: vec![ScoringRule {
                timing: ScoringTiming {
                    phase: Phase::Command,
                    whose_turn: TurnOwner::Active,
                    from_round: 2,
                },
                condition: Condition::OnObjective,
                vp_amount: 5,
                description: Some("Control objectives".to_string()),
            }],
            special_rules: vec![],
            rounds: 5,
            description: "A standard Combat Patrol mission.".to_string(),
        };

        let json = serde_json::to_string(&mission).unwrap();
        let back: MissionSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(mission, back);
        assert_eq!(mission.rounds, 5);
        assert_eq!(mission.objectives.len(), 3);
    }

    // ── CombatPatrolPackSchema ──────────────────────────────────────────

    #[test]
    fn test_combat_patrol_pack_schema_custodes() {
        let pack = CombatPatrolPackSchema {
            name: "Tristraen's Gilded Blades".to_string(),
            faction_id: FactionId::new(1),
            fixed_units: vec![DatasheetId::new(1), DatasheetId::new(2)],
            patrol_squads: vec![
                PatrolSquadOption {
                    label: "Option A - Higher Model Count".to_string(),
                    datasheet_id: DatasheetId::new(3),
                    model_count: 3,
                },
                PatrolSquadOption {
                    label: "Option B - Maximum Durability".to_string(),
                    datasheet_id: DatasheetId::new(4),
                    model_count: 2,
                },
            ],
            default_enhancement: EnhancementId::new(1),
            optional_enhancement: EnhancementId::new(2),
            default_secondary: SecondaryObjectiveId::new(1),
            optional_secondary: SecondaryObjectiveId::new(2),
        };

        let json = serde_json::to_string(&pack).unwrap();
        let back: CombatPatrolPackSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(pack, back);
        assert_eq!(pack.fixed_units.len(), 2);
        assert_eq!(pack.patrol_squads.len(), 2);
    }

    #[test]
    fn test_combat_patrol_pack_schema_frenzied_reavers() {
        let pack = CombatPatrolPackSchema {
            name: "Frenzied Reavers".to_string(),
            faction_id: FactionId::new(2),
            fixed_units: vec![
                DatasheetId::new(10),
                DatasheetId::new(11),
                DatasheetId::new(12),
                DatasheetId::new(13),
            ],
            patrol_squads: vec![], // All units are fixed for Frenzied Reavers
            default_enhancement: EnhancementId::new(3),
            optional_enhancement: EnhancementId::new(4),
            default_secondary: SecondaryObjectiveId::new(3),
            optional_secondary: SecondaryObjectiveId::new(4),
        };

        assert_eq!(pack.fixed_units.len(), 4);
        assert_eq!(pack.patrol_squads.len(), 0); // No choices
    }

    // ── ContentPack ─────────────────────────────────────────────────────

    #[test]
    fn test_content_pack() {
        let pack = ContentPack {
            pack_id: ContentPackId::new(1),
            version: "0.1.0".to_string(),
            content_hash: 0xDEADBEEF,
            factions: vec![],
            missions: vec![],
        };

        let json = serde_json::to_string(&pack).unwrap();
        let back: ContentPack = serde_json::from_str(&json).unwrap();
        assert_eq!(pack, back);
        assert_eq!(pack.content_hash, 0xDEADBEEF);
    }

    // ── DeploymentMapId ─────────────────────────────────────────────────

    #[test]
    fn test_deployment_map_id() {
        let id = DeploymentMapId::new(42);
        assert_eq!(id.raw(), 42);

        let json = serde_json::to_string(&id).unwrap();
        let back: DeploymentMapId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    // ── YAML round-trip tests ───────────────────────────────────────────

    #[test]
    fn test_yaml_roundtrip_weapon_profile() {
        let weapon = WeaponProfileSchema {
            name: "Guardian spear".to_string(),
            weapon_type: WeaponType::Ranged,
            range: Inches::from_inches(24),
            attacks: AttackCount::Fixed(2),
            skill: Skill::TWO_PLUS,
            strength: Strength::new(4),
            ap: ArmorPenetration::MINUS_1,
            damage: Damage::Fixed(2),
            abilities: vec![WeaponAbilityDef::Assault],
        };
        let yaml = serde_yaml::to_string(&weapon).unwrap();
        let back: WeaponProfileSchema = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(weapon, back);
    }

    #[test]
    fn test_yaml_roundtrip_ability() {
        let ability = AbilitySchema {
            name: "Martial Ka'tah".to_string(),
            description: "Choose stance when selected to fight.".to_string(),
            ability_type: AbilityType::FactionAbility,
            effects: vec![RulePrimitive::StanceChoice {
                stances: vec![
                    StanceDef {
                        name: "Dacatarai".to_string(),
                        effects: vec![RulePrimitive::GrantWeaponAbility {
                            weapon_ability: WeaponAbilityDef::SustainedHits(1),
                            weapon_filter: WeaponFilter::MeleeOnly,
                        }],
                        description: "Sustained Hits 1".to_string(),
                    },
                    StanceDef {
                        name: "Rendax".to_string(),
                        effects: vec![RulePrimitive::GrantWeaponAbility {
                            weapon_ability: WeaponAbilityDef::LethalHits,
                            weapon_filter: WeaponFilter::MeleeOnly,
                        }],
                        description: "Lethal Hits".to_string(),
                    },
                ],
            }],
            trigger: Some(AbilityTrigger::OnSelectedToFight),
        };
        let yaml = serde_yaml::to_string(&ability).unwrap();
        let back: AbilitySchema = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(ability, back);
    }

    #[test]
    fn test_yaml_roundtrip_enhancement() {
        let enhancement = EnhancementSchema {
            name: "Fearsome Presence".to_string(),
            description: "OC becomes 5 while not Battle-shocked.".to_string(),
            is_default: true,
            effects: vec![RulePrimitive::Conditional {
                condition: Condition::NotBattleShocked,
                effect: Box::new(RulePrimitive::AlterOC {
                    target: ModifierTarget::Bearer,
                    new_oc: OCModifier::SetTo(5),
                }),
                else_effect: None,
            }],
            conditions: vec![Condition::NotBattleShocked],
        };
        let yaml = serde_yaml::to_string(&enhancement).unwrap();
        let back: EnhancementSchema = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(enhancement, back);
    }

    #[test]
    fn test_yaml_roundtrip_scoring_rule() {
        let rule = ScoringRule {
            timing: ScoringTiming {
                phase: Phase::Command,
                whose_turn: TurnOwner::Active,
                from_round: 2,
            },
            condition: Condition::All(vec![
                Condition::OnObjectiveInNoMansLand,
                Condition::IsCharacter,
            ]),
            vp_amount: 3,
            description: Some("CHARACTER on NML objective".to_string()),
        };
        let yaml = serde_yaml::to_string(&rule).unwrap();
        let back: ScoringRule = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(rule, back);
    }

    // ── Error type tests ────────────────────────────────────────────────

    #[test]
    fn test_error_display() {
        let err = ContentSchemaError::InvalidDatasheet("missing name".to_string());
        assert_eq!(err.to_string(), "Invalid datasheet: missing name");

        let err = ContentSchemaError::MissingReference {
            entity_type: "weapon".to_string(),
            name: "Sentinel blade".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Missing reference: weapon 'Sentinel blade' not found"
        );

        let err = ContentSchemaError::DuplicateId {
            entity_type: "datasheet".to_string(),
            id: 42,
        };
        assert_eq!(err.to_string(), "Duplicate ID: datasheet with ID 42");
    }

    // ── Complex integration tests ───────────────────────────────────────

    #[test]
    fn test_full_custodes_faction_structure() {
        // Verify the overall shape of a Custodes faction can be constructed
        let custodes = FactionSchema {
            id: FactionId::new(1),
            name: "Tristraen's Gilded Blades".to_string(),
            faction_keywords: KeywordSet::from_keywords(&[
                Keyword::Imperium,
                Keyword::AdeptusCustodes,
            ]),
            faction_ability: AbilitySchema {
                name: "Martial Ka'tah".to_string(),
                description: "Choose stance when selected to fight.".to_string(),
                ability_type: AbilityType::FactionAbility,
                effects: vec![RulePrimitive::StanceChoice {
                    stances: vec![
                        StanceDef {
                            name: "Dacatarai".to_string(),
                            effects: vec![RulePrimitive::GrantWeaponAbility {
                                weapon_ability: WeaponAbilityDef::SustainedHits(1),
                                weapon_filter: WeaponFilter::MeleeOnly,
                            }],
                            description: "Melee: Sustained Hits 1".to_string(),
                        },
                        StanceDef {
                            name: "Rendax".to_string(),
                            effects: vec![RulePrimitive::GrantWeaponAbility {
                                weapon_ability: WeaponAbilityDef::LethalHits,
                                weapon_filter: WeaponFilter::MeleeOnly,
                            }],
                            description: "Melee: Lethal Hits".to_string(),
                        },
                    ],
                }],
                trigger: Some(AbilityTrigger::OnSelectedToFight),
            },
            datasheets: vec![], // Would contain Tristraen, Guard, Wardens, Allarus
            stratagems: vec![], // Would contain Gilded Spear, Inescapable Vengeance, etc.
            enhancements: vec![
                EnhancementSchema {
                    name: "Watchman of Terra".to_string(),
                    description: "OC 4 in Engagement Range.".to_string(),
                    is_default: true,
                    effects: vec![RulePrimitive::Conditional {
                        condition: Condition::InEngagementRange,
                        effect: Box::new(RulePrimitive::AlterOC {
                            target: ModifierTarget::Bearer,
                            new_oc: OCModifier::SetTo(4),
                        }),
                        else_effect: None,
                    }],
                    conditions: vec![Condition::InEngagementRange],
                },
                EnhancementSchema {
                    name: "Warrior Exemplar".to_string(),
                    description: "D6 3+ = 1CP on unit destruction.".to_string(),
                    is_default: false,
                    effects: vec![RulePrimitive::TriggerOnDestroyed {
                        effect: Box::new(RulePrimitive::DiceCheck {
                            threshold: 3,
                            on_success: Box::new(RulePrimitive::AlterCP { amount: 1 }),
                            on_failure: None,
                            modifier: 0,
                        }),
                    }],
                    conditions: vec![],
                },
            ],
            secondary_objectives: vec![],
        };

        // Verify it serializes and deserializes
        let json = serde_json::to_string(&custodes).unwrap();
        let back: FactionSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(custodes.name, back.name);
        assert_eq!(custodes.enhancements.len(), 2);
        assert!(custodes.enhancements[0].is_default);
        assert!(!custodes.enhancements[1].is_default);
    }

    #[test]
    fn test_full_frenzied_reavers_faction_structure() {
        // Verify the overall shape of a Frenzied Reavers faction can be constructed
        let reavers = FactionSchema {
            id: FactionId::new(2),
            name: "Frenzied Reavers".to_string(),
            faction_keywords: KeywordSet::from_keywords(&[
                Keyword::Chaos,
                Keyword::Khorne,
                Keyword::WorldEaters,
            ]),
            faction_ability: AbilitySchema {
                name: "Blessings of Khorne".to_string(),
                description: "Roll 5D6, activate blessings.".to_string(),
                ability_type: AbilityType::FactionAbility,
                effects: vec![RulePrimitive::DicePoolAllocation {
                    num_dice: 5,
                    max_blessings: 2,
                    blessings: vec![
                        DicePoolBlessingDef {
                            name: "Rage-fuelled Invigoration".to_string(),
                            requirement: DiceRequirement::DoubleTwoPlus,
                            effects: vec![RulePrimitive::TriggerMoveReaction {
                                move_type: ReactionMoveType::PileInAndConsolidation,
                                distance: MoveDistance::Fixed(Inches::from_inches(6)),
                            }],
                            description: "6\" pile-in/consolidation.".to_string(),
                        },
                        DicePoolBlessingDef {
                            name: "Total Carnage".to_string(),
                            requirement: DiceRequirement::DoubleTwoPlus,
                            effects: vec![RulePrimitive::FightOnDeath {
                                threshold: 4,
                                modifier_source: None,
                            }],
                            description: "Fight on death (4+).".to_string(),
                        },
                        DicePoolBlessingDef {
                            name: "Martial Excellence".to_string(),
                            requirement: DiceRequirement::DoubleFourPlusOrTripleAny,
                            effects: vec![RulePrimitive::GrantWeaponAbility {
                                weapon_ability: WeaponAbilityDef::SustainedHits(1),
                                weapon_filter: WeaponFilter::MeleeOnly,
                            }],
                            description: "Melee: Sustained Hits 1.".to_string(),
                        },
                    ],
                }],
                trigger: Some(AbilityTrigger::StartOfBattleRound),
            },
            datasheets: vec![], // Would contain Vorrakh, MoE, Berzerkers, Jakhals
            stratagems: vec![],
            enhancements: vec![
                EnhancementSchema {
                    name: "Fearsome Presence".to_string(),
                    description: "OC 5 while not Battle-shocked.".to_string(),
                    is_default: true,
                    effects: vec![RulePrimitive::Conditional {
                        condition: Condition::NotBattleShocked,
                        effect: Box::new(RulePrimitive::AlterOC {
                            target: ModifierTarget::Bearer,
                            new_oc: OCModifier::SetTo(5),
                        }),
                        else_effect: None,
                    }],
                    conditions: vec![Condition::NotBattleShocked],
                },
                EnhancementSchema {
                    name: "Bane of the Craven".to_string(),
                    description: "Desperate Escape on Fall Back.".to_string(),
                    is_default: false,
                    effects: vec![RulePrimitive::Conditional {
                        condition: Condition::TargetNotMonsterOrVehicle,
                        effect: Box::new(RulePrimitive::ForceDespairEscapeTest {
                            modifier: 0,
                            condition: Some(Condition::BattleShocked),
                        }),
                        else_effect: None,
                    }],
                    conditions: vec![],
                },
            ],
            secondary_objectives: vec![],
        };

        let json = serde_json::to_string(&reavers).unwrap();
        let back: FactionSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(reavers.name, back.name);
        assert_eq!(reavers.enhancements.len(), 2);

        // Verify the Blessings of Khorne structure
        if let RulePrimitive::DicePoolAllocation {
            num_dice,
            max_blessings,
            blessings,
        } = &reavers.faction_ability.effects[0]
        {
            assert_eq!(*num_dice, 5);
            assert_eq!(*max_blessings, 2);
            assert_eq!(blessings.len(), 3);
            assert_eq!(blessings[0].name, "Rage-fuelled Invigoration");
            assert_eq!(blessings[1].name, "Total Carnage");
            assert_eq!(blessings[2].name, "Martial Excellence");
        } else {
            panic!("Expected DicePoolAllocation");
        }
    }
}

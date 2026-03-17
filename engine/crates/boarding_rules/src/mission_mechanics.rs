//! Mission-Specific Mechanics for Boarding Actions.
//!
//! Implements the unique mechanics for each of the 16 Boarding Actions missions.
//! Each mission's mechanics are triggered by events or phase transitions.
//!
//! Source: boarding_actions_complete_v3.md
//! Source: boarding_actions_mission_tags_complete_v3.json
//! Source: boarding_actions_objectives_complete_v3.json

use serde::{Deserialize, Serialize};

// Re-export from scoring to avoid DRY violation (BA-17).
pub use crate::scoring::{destroyed_points_to_vp, STANDARD_DESTROYED_POINTS_THRESHOLDS};

// ---------------------------------------------------------------------------
// MissionMechanic enum
// ---------------------------------------------------------------------------

/// Identifies which mission-specific mechanic is active.
/// A mission can have multiple mechanics simultaneously.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MissionMechanic {
    /// BA-12: Underdog entry zone (deploy-only, disabled for reserves after round 1)
    UnderdogEntryZone,
    /// BA-13, BA-21: Linked power line objectives (4th scoring condition)
    LinkedPowerLines,
    /// BA-22: Lighting area toggle (roll 1-3 = lights off)
    LightingAreas,
    /// BA-23: Compartment venting (D3 mortals, -1" Move, close hatchways)
    CompartmentVenting,
    /// BA-23: Data download (one-shot 15VP per download)
    DataDownload,
    /// BA-31: Unlock all hatchways
    UnlockAllHatchways,
    /// BA-32: Turn on burners (tactical manoeuvre from furnace control zone)
    TurnOnBurners,
    /// BA-33: Radiation progression (4 sectors, escalating penalties)
    RadiationProgression,
    /// BA-01: Attacker-only airlocks (one-way opening)
    AttackerAirlocks,
    /// BA-02: VP transfer mechanic (attacker starts at 60VP)
    VpTransfer,
    /// BA-03: Defender predeployment in strongrooms
    StrongroomPredeployment,
    /// BA-04: Imprisoned unit, prison break, silent infiltration
    PrisonBreak,
    /// BA-05: Multi-level (2 boards), access zone transfer, generator->terminal power
    MultiLevel,
    /// BA-06: Corruption attempts (D6 2+), 4 unique consequences
    CorruptMachineSpirit,
    /// BA-32: Any unit can Secure Site (override BATTLELINE restriction)
    SecureSiteOverride,
    /// Underdog +1 CP bonus (most symmetric missions)
    UnderdogCpBonus,
    /// BA-31: Control objective A as 4th progressive condition
    ControlObjectiveA,
    /// BA-32: Control furnace objective as 4th progressive condition
    ControlFurnaceObjective,
    /// BA-01, BA-04: Patrol entry zone restrictions
    PatrolEntryZone,
    /// BA-04: Silent infiltration chain (attacker pregame 6" moves)
    SilentInfiltration,
    /// BA-04: Backup entry zone with delayed enablement
    BackupEntryZone,
    /// BA-01, BA-21, BA-31: Warlord kill end-game objective
    WarlordKill,
    /// BA-32: Desperate Measures CP gain (5+ on D6 if controlling furnace objective)
    DesperateMeasuresCpGain,
}

/// Return which mission mechanics apply to a given mission.
///
/// This is the authoritative mapping from mission ID to active mechanics.
pub fn get_mission_mechanics(mission_id: &str) -> Vec<MissionMechanic> {
    // Normalize the ID for matching
    let normalized = normalize_for_match(mission_id);

    match normalized.as_str() {
        "ba11" => vec![
            MissionMechanic::UnderdogCpBonus,
        ],

        "ba12" => vec![
            MissionMechanic::UnderdogEntryZone,
        ],

        "ba13" => vec![
            MissionMechanic::LinkedPowerLines,
            MissionMechanic::UnderdogCpBonus,
        ],

        "ba21" => vec![
            MissionMechanic::LinkedPowerLines,
            MissionMechanic::WarlordKill,
            MissionMechanic::UnderdogCpBonus,
        ],

        "ba22" => vec![
            MissionMechanic::LightingAreas,
            MissionMechanic::UnderdogCpBonus,
        ],

        "ba23" => vec![
            MissionMechanic::CompartmentVenting,
            MissionMechanic::DataDownload,
        ],

        "ba31" => vec![
            MissionMechanic::UnlockAllHatchways,
            MissionMechanic::ControlObjectiveA,
            MissionMechanic::WarlordKill,
            MissionMechanic::UnderdogCpBonus,
        ],

        "ba32" => vec![
            MissionMechanic::TurnOnBurners,
            MissionMechanic::SecureSiteOverride,
            MissionMechanic::ControlFurnaceObjective,
            MissionMechanic::DesperateMeasuresCpGain,
        ],

        "ba33" => vec![
            MissionMechanic::RadiationProgression,
            MissionMechanic::WarlordKill,
            MissionMechanic::UnderdogCpBonus,
        ],

        "ba1" | "ba01" => vec![
            MissionMechanic::AttackerAirlocks,
            MissionMechanic::PatrolEntryZone,
            MissionMechanic::WarlordKill,
        ],

        "ba2" | "ba02" => vec![
            MissionMechanic::VpTransfer,
            MissionMechanic::SecureSiteOverride,
        ],

        "ba3" | "ba03" => vec![
            MissionMechanic::StrongroomPredeployment,
            MissionMechanic::UnderdogCpBonus,
        ],

        "ba4" | "ba04" => vec![
            MissionMechanic::PrisonBreak,
            MissionMechanic::SilentInfiltration,
            MissionMechanic::PatrolEntryZone,
            MissionMechanic::BackupEntryZone,
        ],

        "ba5" | "ba05" => vec![
            MissionMechanic::MultiLevel,
        ],

        "ba6" | "ba06" => vec![
            MissionMechanic::CorruptMachineSpirit,
        ],

        _ => vec![],
    }
}

/// Normalize a mission ID for matching: lowercase, remove hyphens and leading zeros.
fn normalize_for_match(id: &str) -> String {
    let lower = id.to_lowercase().replace('-', "");
    // Remove leading zeros in the numeric part
    // e.g., "ba01" -> "ba1"
    if lower.starts_with("ba0") && lower.len() == 4 {
        format!("ba{}", &lower[3..])
    } else {
        lower
    }
}

// ---------------------------------------------------------------------------
// Radiation mechanics (BA-33)
// ---------------------------------------------------------------------------

/// Radiation severity level for a sector in a given battle round.
///
/// Source: boarding_actions_mission_tags_complete_v3.json (BA-33 triggers)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RadiationLevel {
    /// No radiation exposure
    None,
    /// Mild radiation: Leadership worsens by 1
    Mild,
    /// High radiation: subtract 1" Move + Mild effects
    High,
    /// Extreme radiation: subtract 1 Toughness + High effects + Mild effects
    Extreme,
}

impl RadiationLevel {
    /// Leadership penalty at this radiation level.
    /// Mild/High/Extreme all worsen Leadership by 1.
    pub fn leadership_penalty(&self) -> i8 {
        match self {
            RadiationLevel::None => 0,
            RadiationLevel::Mild | RadiationLevel::High | RadiationLevel::Extreme => 1,
        }
    }

    /// Movement penalty in mils (thousandths of an inch) at this radiation level.
    /// High and Extreme subtract 1" (1000 mils) from Move characteristic.
    pub fn movement_penalty(&self) -> i32 {
        match self {
            RadiationLevel::None | RadiationLevel::Mild => 0,
            RadiationLevel::High | RadiationLevel::Extreme => 1000, // 1 inch = 1000 mils
        }
    }

    /// Toughness penalty at this radiation level.
    /// Only Extreme radiation reduces Toughness by 1.
    pub fn toughness_penalty(&self) -> i8 {
        match self {
            RadiationLevel::Extreme => 1,
            _ => 0,
        }
    }

    /// Whether this level has any effect at all.
    pub fn is_active(&self) -> bool {
        !matches!(self, RadiationLevel::None)
    }
}

/// Get the radiation level for a given sector at a given battle round.
///
/// Radiation progression table (BA-33 Rad Leak):
///
/// | Sector | Round 1 | Round 2 | Round 3 | Round 4 | Round 5 |
/// |--------|---------|---------|---------|---------|---------|
/// | A      | Mild    | High    | Extreme | Extreme | Extreme |
/// | B      | None    | Mild    | High    | Extreme | Extreme |
/// | C      | None    | None    | Mild    | High    | Extreme |
/// | D      | None    | None    | None    | Mild    | High    |
///
/// Source: boarding_actions_mission_tags_complete_v3.json (BA-33 triggers)
pub fn get_radiation_level(sector: &str, battle_round: u8) -> RadiationLevel {
    let sector_upper = sector.trim().to_uppercase();
    // Extract the sector letter. If the input is just a single letter (A/B/C/D),
    // use it directly. Otherwise, look for "SECTOR X" pattern or take the last character.
    let sector_char = if sector_upper.len() == 1 {
        sector_upper.chars().next().unwrap_or(' ')
    } else if let Some(rest) = sector_upper.strip_prefix("SECTOR ") {
        rest.trim().chars().next().unwrap_or(' ')
    } else {
        // Fall back to last non-whitespace character
        sector_upper.chars().rev().find(|c| !c.is_whitespace()).unwrap_or(' ')
    };
    if !matches!(sector_char, 'A' | 'B' | 'C' | 'D') {
        return RadiationLevel::None;
    }

    match (sector_char, battle_round) {
        // Sector A: starts Mild at round 1, progresses to Extreme by round 3
        ('A', 1) => RadiationLevel::Mild,
        ('A', 2) => RadiationLevel::High,
        ('A', r) if r >= 3 => RadiationLevel::Extreme,

        // Sector B: starts Mild at round 2
        ('B', 1) => RadiationLevel::None,
        ('B', 2) => RadiationLevel::Mild,
        ('B', 3) => RadiationLevel::High,
        ('B', r) if r >= 4 => RadiationLevel::Extreme,

        // Sector C: starts Mild at round 3
        ('C', r) if r <= 2 => RadiationLevel::None,
        ('C', 3) => RadiationLevel::Mild,
        ('C', 4) => RadiationLevel::High,
        ('C', 5) => RadiationLevel::Extreme,

        // Sector D: starts Mild at round 4
        ('D', r) if r <= 3 => RadiationLevel::None,
        ('D', 4) => RadiationLevel::Mild,
        ('D', 5) => RadiationLevel::High,

        _ => RadiationLevel::None,
    }
}

/// Get all radiation levels for all sectors at a given battle round.
pub fn get_all_radiation_levels(battle_round: u8) -> Vec<(&'static str, RadiationLevel)> {
    vec![
        ("Sector A", get_radiation_level("A", battle_round)),
        ("Sector B", get_radiation_level("B", battle_round)),
        ("Sector C", get_radiation_level("C", battle_round)),
        ("Sector D", get_radiation_level("D", battle_round)),
    ]
}

// ---------------------------------------------------------------------------
// Corruption mechanics (BA-06)
// ---------------------------------------------------------------------------

/// A corruption consequence for BA-06 Corrupt the Machine Spirit.
///
/// When the Attacker successfully corrupts an objective (D6 roll of 2+),
/// they select one unique consequence from this list. Each can only be
/// selected once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorruptionConsequence {
    /// Unique identifier for this consequence
    pub id: String,
    /// Display name
    pub name: String,
    /// Full description of the consequence's effect
    pub description: String,
}

/// Get the 4 unique corruption consequences for BA-06.
///
/// Source: boarding_actions_mission_tags_complete_v3.json (BA-06)
/// Source: boarding_actions_complete_v3.md
pub fn get_corruption_consequences() -> Vec<CorruptionConsequence> {
    vec![
        CorruptionConsequence {
            id: "door_controls".to_string(),
            name: "Door Controls".to_string(),
            description: "The Attacker can open or close up to 3 Hatchways on the battlefield."
                .to_string(),
        },
        CorruptionConsequence {
            id: "steam_venting_overrides".to_string(),
            name: "Steam Venting Overrides".to_string(),
            description:
                "Until the start of the next Command phase, all ranged attacks on the battlefield are limited to a maximum range of 6 inches."
                    .to_string(),
        },
        CorruptionConsequence {
            id: "gravitational_systems".to_string(),
            name: "Gravitational Systems".to_string(),
            description:
                "Until the start of the next Command phase, no unit on the battlefield can use the Set to Defend or Set Overwatch Tactical Manoeuvres."
                    .to_string(),
        },
        CorruptionConsequence {
            id: "communications_network".to_string(),
            name: "Communications Network".to_string(),
            description:
                "The Defender cannot gain Command points until the end of the Attacker's next Command phase."
                    .to_string(),
        },
    ]
}

// ---------------------------------------------------------------------------
// Destroyed points threshold scoring (canonical definition in scoring.rs)
// ---------------------------------------------------------------------------
// `destroyed_points_to_vp` and `STANDARD_DESTROYED_POINTS_THRESHOLDS` are
// re-exported from `crate::scoring` at the top of this file (BA-17 DRY fix).

// ---------------------------------------------------------------------------
// Lighting mechanics (BA-22)
// ---------------------------------------------------------------------------

/// Result of a lighting area roll for BA-22 Death in the Dark.
///
/// At the end of each player's Movement phase, roll a D6 for each Lighting Area:
/// - 1-3: lights off (darkness effects apply until end of turn)
/// - 4-6: lights on (no effect)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LightingRollResult {
    /// Roll 1-3: lights off, darkness penalties apply
    LightsOff,
    /// Roll 4-6: lights on, no darkness penalties
    LightsOn,
}

impl LightingRollResult {
    /// Interpret a D6 roll as a lighting result.
    pub fn from_roll(roll: u8) -> LightingRollResult {
        if roll <= 3 {
            LightingRollResult::LightsOff
        } else {
            LightingRollResult::LightsOn
        }
    }

    /// Whether darkness penalties apply.
    pub fn is_dark(&self) -> bool {
        matches!(self, LightingRollResult::LightsOff)
    }
}

/// Darkness effects when a Lighting Area has its lights off (BA-22).
///
/// Source: boarding_actions_mission_tags_complete_v3.json (BA-22 triggers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarknessEffects {
    /// Maximum ranged attack range while in a dark area: 9 inches
    pub max_ranged_range_inches: u8,
    /// Ranged hit roll modifier within 9 inches: -1
    pub ranged_hit_modifier: i8,
    /// Maximum charge target range while in a dark area: 9 inches
    pub max_charge_range_inches: u8,
    /// Charge roll modifier within 9 inches: -1
    pub charge_roll_modifier: i8,
}

impl Default for DarknessEffects {
    fn default() -> Self {
        Self {
            max_ranged_range_inches: 9,
            ranged_hit_modifier: -1,
            max_charge_range_inches: 9,
            charge_roll_modifier: -1,
        }
    }
}

/// Get the darkness effects for a Lighting Area with lights off.
pub fn get_darkness_effects() -> DarknessEffects {
    DarknessEffects::default()
}

// ---------------------------------------------------------------------------
// Compartment venting mechanics (BA-23)
// ---------------------------------------------------------------------------

/// Effects of a vented compartment in BA-23 Hull Breach.
///
/// Source: boarding_actions_mission_tags_complete_v3.json (BA-23 triggers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VentingEffect {
    /// D3 mortal wounds to each unit within the vented compartment
    pub mortal_wounds_d3: bool,
    /// Movement penalty: -1 inch for Normal and Advance moves until end of turn
    pub move_penalty_inches: u8,
    /// Close all open hatchways within the vented compartment
    pub close_hatchways: bool,
}

impl Default for VentingEffect {
    fn default() -> Self {
        Self {
            mortal_wounds_d3: true,
            move_penalty_inches: 1,
            close_hatchways: true,
        }
    }
}

/// Get the venting effects for a compartment vent event.
pub fn get_venting_effects() -> VentingEffect {
    VentingEffect::default()
}

/// Check if a vent roll succeeds (BA-23: roll a D6, on 3+ the compartment is vented).
pub fn vent_roll_succeeds(roll: u8) -> bool {
    roll >= 3
}

// ---------------------------------------------------------------------------
// Prison break mechanics (BA-04)
// ---------------------------------------------------------------------------

/// Outcome of a prison break attempt (BA-04 Jailbreak).
///
/// The imprisoned unit can attempt to open the Prison Cells hatchway by
/// rolling a D6 against the highest Toughness characteristic in the unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrisonBreakResult {
    /// Roll met or exceeded the Toughness test: hatchway opens
    Success,
    /// Roll failed: hatchway remains closed
    Failure,
}

/// Check if a prison break attempt succeeds.
///
/// The imprisoned unit rolls a D6; if the result is >= Toughness, the attempt
/// FAILS (the unit is too tough to break free). If the result is less than
/// Toughness, the Prison Cells hatchway opens (and can never be closed).
pub fn prison_break_test(roll: u8, unit_toughness: u8) -> PrisonBreakResult {
    if roll >= unit_toughness {
        PrisonBreakResult::Failure
    } else {
        PrisonBreakResult::Success
    }
}

// ---------------------------------------------------------------------------
// Silent infiltration mechanics (BA-04)
// ---------------------------------------------------------------------------

/// Check if a silent infiltration move succeeds (BA-04).
///
/// The Attacker selects units one at a time; each must roll a D6 with a
/// cumulative penalty (-1 for each subsequent unit). On 2+ (adjusted),
/// the unit makes a Normal Move up to 6 inches.
///
/// `unit_index`: 0-based index of which unit in the chain (0 = first, 1 = second, etc.)
pub fn silent_infiltration_test(roll: u8, unit_index: u8) -> bool {
    let target = 2u8.saturating_add(unit_index);
    roll >= target
}

/// Maximum move distance for a successful silent infiltration: 6 inches.
pub const SILENT_INFILTRATION_MOVE_INCHES: u8 = 6;

// ---------------------------------------------------------------------------
// Corruption attempt mechanics (BA-06)
// ---------------------------------------------------------------------------

/// Check if a corruption attempt succeeds (BA-06).
///
/// At the end of the Attacker's Command phase, for each objective marker
/// the Attacker controls, roll a D6. On a 2+, the objective is corrupted
/// and removed from the battlefield.
pub fn corruption_attempt_succeeds(roll: u8) -> bool {
    roll >= 2
}

// ---------------------------------------------------------------------------
// Furnace burner mechanics (BA-32)
// ---------------------------------------------------------------------------

/// Interpret a burner damage roll (BA-32 The Furnace).
///
/// When burners are turned on, roll a D6 for each model in each unit
/// within the Furnace. On a result of (threshold)+, the model suffers
/// a mortal wound.
///
/// The threshold is typically 4+ (suffer a mortal wound on 4+).
pub fn burner_roll_hits(roll: u8) -> bool {
    roll >= 4
}

/// Maximum mortal wounds from burners per unit per turn.
/// Source: boarding_actions_missions_complete_v3.md §3.8
pub const FURNACE_BURNER_MW_CAP: u8 = 3;

/// Calculate total mortal wounds from burner rolls, capped at 3 per unit.
pub fn capped_burner_mortal_wounds(hits: u8) -> u8 {
    hits.min(FURNACE_BURNER_MW_CAP)
}

/// Check if Desperate Measures CP roll succeeds (5+ on D6).
/// If player controls one or more objectives within the Furnace, roll D6.
/// On 5+, gain 1 CP.
/// Source: boarding_actions_missions_complete_v3.md §3.8
pub fn desperate_measures_cp_check(roll: u8) -> bool {
    roll >= 5
}

// ---------------------------------------------------------------------------
// VP transfer mechanics (BA-02)
// ---------------------------------------------------------------------------

/// Initial VP state for BA-02 Pull Their Teeth.
///
/// The Attacker starts with 60 VP; the Defender starts with 0 VP.
/// VP is transferred from Attacker to Defender based on Loader objective control.
pub const VP_TRANSFER_INITIAL_ATTACKER: i16 = 60;
pub const VP_TRANSFER_INITIAL_DEFENDER: i16 = 0;

/// Calculate VP transfer amount per Loader (BA-02).
///
/// At start of Defender's Command phase:
/// - If Defender controls the Control Node: transfer 10 VP per Loader
/// - If Defender does not control the Control Node: transfer 5 VP per Loader
pub fn vp_transfer_per_loader(defender_controls_control_node: bool) -> i16 {
    if defender_controls_control_node {
        10
    } else {
        5
    }
}

/// Bonus VP for Attacker controlling the Control Node (BA-02).
///
/// At start of Attacker's Command phase, if Attacker controls the Control Node,
/// score 5 VP.
pub const ATTACKER_CONTROL_NODE_BONUS: i16 = 5;

// ---------------------------------------------------------------------------
// Multi-level mechanics (BA-05)
// ---------------------------------------------------------------------------

/// Check if a unit can change levels (BA-05 Power the Generators).
///
/// At the end of each player's Movement phase, if every model in a unit
/// is wholly within an Access Zone, the unit can be removed and set up
/// within the corresponding Access Zone on the other game board.
///
/// This is a prerequisite check; the actual mechanics are:
/// 1. All models must be within the Access Zone
/// 2. Remove the unit from the current board
/// 3. Set up wholly within the corresponding Access Zone on the other board
pub fn can_change_level(all_models_in_access_zone: bool) -> bool {
    all_models_in_access_zone
}

// ---------------------------------------------------------------------------
// Exclusive outcome scoring (BA-04 Jailbreak)
// ---------------------------------------------------------------------------

/// The 4 exclusive outcomes for BA-04 Jailbreak end-game scoring.
///
/// | Outcome | Attacker VP | Defender VP |
/// |---------|-------------|-------------|
/// | Imprisoned unit still in prison | 0 | 90 |
/// | Imprisoned unit out of prison but destroyed | 30 | 60 |
/// | Imprisoned unit out, alive, engaged by defender | 60 | 30 |
/// | Imprisoned unit out, alive, not engaged | 90 | 0 |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JailbreakOutcome {
    /// Imprisoned unit still within Prison Cells
    StillImprisoned,
    /// Imprisoned unit escaped but was destroyed
    EscapedButDestroyed,
    /// Imprisoned unit escaped, alive, engaged by defender
    EscapedEngaged,
    /// Imprisoned unit escaped, alive, free
    EscapedFree,
}

impl JailbreakOutcome {
    /// VP awarded to the Attacker for this outcome.
    pub fn attacker_vp(&self) -> i16 {
        match self {
            JailbreakOutcome::StillImprisoned => 0,
            JailbreakOutcome::EscapedButDestroyed => 30,
            JailbreakOutcome::EscapedEngaged => 60,
            JailbreakOutcome::EscapedFree => 90,
        }
    }

    /// VP awarded to the Defender for this outcome.
    pub fn defender_vp(&self) -> i16 {
        match self {
            JailbreakOutcome::StillImprisoned => 90,
            JailbreakOutcome::EscapedButDestroyed => 60,
            JailbreakOutcome::EscapedEngaged => 30,
            JailbreakOutcome::EscapedFree => 0,
        }
    }

    /// Determine the outcome from the game state.
    pub fn from_state(
        in_prison: bool,
        destroyed: bool,
        engaged_by_defender: bool,
    ) -> JailbreakOutcome {
        if in_prison {
            JailbreakOutcome::StillImprisoned
        } else if destroyed {
            JailbreakOutcome::EscapedButDestroyed
        } else if engaged_by_defender {
            JailbreakOutcome::EscapedEngaged
        } else {
            JailbreakOutcome::EscapedFree
        }
    }
}

// ---------------------------------------------------------------------------
// Unlock all hatchways mechanics (BA-31)
// ---------------------------------------------------------------------------

/// Check if the "Unlock Overrides" trigger fires (BA-31 Control Centre).
///
/// Fires during each player's Command phase if:
/// 1. Player controls objective B
/// 2. With a non-Battle-shocked unit in range
/// 3. At least one such unit is not within Engagement Range of an enemy
///
/// Effect: open every Hatchway on the battlefield.
pub fn unlock_overrides_fires(
    controls_objective_b: bool,
    has_non_battleshocked_in_range: bool,
    has_non_engaged_in_range: bool,
) -> bool {
    controls_objective_b && has_non_battleshocked_in_range && has_non_engaged_in_range
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Mission mechanics mapping tests ---

    #[test]
    fn test_get_mission_mechanics_ba_11() {
        let mechanics = get_mission_mechanics("BA-11");
        assert!(mechanics.contains(&MissionMechanic::UnderdogCpBonus));
        assert!(!mechanics.contains(&MissionMechanic::RadiationProgression));
    }

    #[test]
    fn test_get_mission_mechanics_ba_22() {
        let mechanics = get_mission_mechanics("BA-22");
        assert!(mechanics.contains(&MissionMechanic::LightingAreas));
        assert!(mechanics.contains(&MissionMechanic::UnderdogCpBonus));
        assert!(!mechanics.contains(&MissionMechanic::RadiationProgression));
    }

    #[test]
    fn test_get_mission_mechanics_ba_33() {
        let mechanics = get_mission_mechanics("BA-33");
        assert!(mechanics.contains(&MissionMechanic::RadiationProgression));
        assert!(mechanics.contains(&MissionMechanic::WarlordKill));
        assert!(mechanics.contains(&MissionMechanic::UnderdogCpBonus));
    }

    #[test]
    fn test_get_mission_mechanics_ba_04() {
        let mechanics = get_mission_mechanics("BA-04");
        assert!(mechanics.contains(&MissionMechanic::PrisonBreak));
        assert!(mechanics.contains(&MissionMechanic::SilentInfiltration));
        assert!(mechanics.contains(&MissionMechanic::PatrolEntryZone));
        assert!(mechanics.contains(&MissionMechanic::BackupEntryZone));
    }

    #[test]
    fn test_get_mission_mechanics_ba_06() {
        let mechanics = get_mission_mechanics("BA-06");
        assert!(mechanics.contains(&MissionMechanic::CorruptMachineSpirit));
    }

    #[test]
    fn test_get_mission_mechanics_ba_01() {
        let mechanics = get_mission_mechanics("BA-01");
        assert!(mechanics.contains(&MissionMechanic::AttackerAirlocks));
        assert!(mechanics.contains(&MissionMechanic::PatrolEntryZone));
        assert!(mechanics.contains(&MissionMechanic::WarlordKill));
    }

    #[test]
    fn test_get_mission_mechanics_ba_02() {
        let mechanics = get_mission_mechanics("BA-02");
        assert!(mechanics.contains(&MissionMechanic::VpTransfer));
        assert!(mechanics.contains(&MissionMechanic::SecureSiteOverride));
    }

    #[test]
    fn test_get_mission_mechanics_ba_05() {
        let mechanics = get_mission_mechanics("BA-05");
        assert!(mechanics.contains(&MissionMechanic::MultiLevel));
    }

    #[test]
    fn test_get_mission_mechanics_ba_23() {
        let mechanics = get_mission_mechanics("BA-23");
        assert!(mechanics.contains(&MissionMechanic::CompartmentVenting));
        assert!(mechanics.contains(&MissionMechanic::DataDownload));
    }

    #[test]
    fn test_get_mission_mechanics_ba_31() {
        let mechanics = get_mission_mechanics("BA-31");
        assert!(mechanics.contains(&MissionMechanic::UnlockAllHatchways));
        assert!(mechanics.contains(&MissionMechanic::ControlObjectiveA));
        assert!(mechanics.contains(&MissionMechanic::WarlordKill));
    }

    #[test]
    fn test_get_mission_mechanics_ba_32() {
        let mechanics = get_mission_mechanics("BA-32");
        assert!(mechanics.contains(&MissionMechanic::TurnOnBurners));
        assert!(mechanics.contains(&MissionMechanic::SecureSiteOverride));
        assert!(mechanics.contains(&MissionMechanic::ControlFurnaceObjective));
        assert!(mechanics.contains(&MissionMechanic::DesperateMeasuresCpGain));
    }

    #[test]
    fn test_get_mission_mechanics_unknown() {
        let mechanics = get_mission_mechanics("BA-99");
        assert!(mechanics.is_empty());
    }

    #[test]
    fn test_get_mission_mechanics_normalization() {
        // Test that various ID formats all work
        assert_eq!(
            get_mission_mechanics("BA-01"),
            get_mission_mechanics("BA-1")
        );
        assert_eq!(
            get_mission_mechanics("BA-06"),
            get_mission_mechanics("BA-6")
        );
        // BA-11 should work as-is since it doesn't have a leading zero
        assert!(!get_mission_mechanics("BA-11").is_empty());
    }

    // --- Radiation level tests ---

    #[test]
    fn test_radiation_sector_a_all_rounds() {
        assert_eq!(get_radiation_level("A", 1), RadiationLevel::Mild);
        assert_eq!(get_radiation_level("A", 2), RadiationLevel::High);
        assert_eq!(get_radiation_level("A", 3), RadiationLevel::Extreme);
        assert_eq!(get_radiation_level("A", 4), RadiationLevel::Extreme);
        assert_eq!(get_radiation_level("A", 5), RadiationLevel::Extreme);
    }

    #[test]
    fn test_radiation_sector_b_all_rounds() {
        assert_eq!(get_radiation_level("B", 1), RadiationLevel::None);
        assert_eq!(get_radiation_level("B", 2), RadiationLevel::Mild);
        assert_eq!(get_radiation_level("B", 3), RadiationLevel::High);
        assert_eq!(get_radiation_level("B", 4), RadiationLevel::Extreme);
        assert_eq!(get_radiation_level("B", 5), RadiationLevel::Extreme);
    }

    #[test]
    fn test_radiation_sector_c_all_rounds() {
        assert_eq!(get_radiation_level("C", 1), RadiationLevel::None);
        assert_eq!(get_radiation_level("C", 2), RadiationLevel::None);
        assert_eq!(get_radiation_level("C", 3), RadiationLevel::Mild);
        assert_eq!(get_radiation_level("C", 4), RadiationLevel::High);
        assert_eq!(get_radiation_level("C", 5), RadiationLevel::Extreme);
    }

    #[test]
    fn test_radiation_sector_d_all_rounds() {
        assert_eq!(get_radiation_level("D", 1), RadiationLevel::None);
        assert_eq!(get_radiation_level("D", 2), RadiationLevel::None);
        assert_eq!(get_radiation_level("D", 3), RadiationLevel::None);
        assert_eq!(get_radiation_level("D", 4), RadiationLevel::Mild);
        assert_eq!(get_radiation_level("D", 5), RadiationLevel::High);
    }

    #[test]
    fn test_radiation_full_sector_name() {
        // Should work with full "Sector A" names too
        assert_eq!(get_radiation_level("Sector A", 1), RadiationLevel::Mild);
        assert_eq!(get_radiation_level("Sector B", 2), RadiationLevel::Mild);
        assert_eq!(get_radiation_level("Sector C", 3), RadiationLevel::Mild);
        assert_eq!(get_radiation_level("Sector D", 4), RadiationLevel::Mild);
    }

    #[test]
    fn test_radiation_unknown_sector() {
        assert_eq!(get_radiation_level("E", 1), RadiationLevel::None);
        assert_eq!(get_radiation_level("unknown", 3), RadiationLevel::None);
    }

    #[test]
    fn test_radiation_penalties() {
        assert_eq!(RadiationLevel::None.leadership_penalty(), 0);
        assert_eq!(RadiationLevel::None.movement_penalty(), 0);
        assert_eq!(RadiationLevel::None.toughness_penalty(), 0);

        assert_eq!(RadiationLevel::Mild.leadership_penalty(), 1);
        assert_eq!(RadiationLevel::Mild.movement_penalty(), 0);
        assert_eq!(RadiationLevel::Mild.toughness_penalty(), 0);

        assert_eq!(RadiationLevel::High.leadership_penalty(), 1);
        assert_eq!(RadiationLevel::High.movement_penalty(), 1000);
        assert_eq!(RadiationLevel::High.toughness_penalty(), 0);

        assert_eq!(RadiationLevel::Extreme.leadership_penalty(), 1);
        assert_eq!(RadiationLevel::Extreme.movement_penalty(), 1000);
        assert_eq!(RadiationLevel::Extreme.toughness_penalty(), 1);
    }

    #[test]
    fn test_radiation_is_active() {
        assert!(!RadiationLevel::None.is_active());
        assert!(RadiationLevel::Mild.is_active());
        assert!(RadiationLevel::High.is_active());
        assert!(RadiationLevel::Extreme.is_active());
    }

    #[test]
    fn test_get_all_radiation_levels() {
        let levels = get_all_radiation_levels(3);
        assert_eq!(levels.len(), 4);
        assert_eq!(levels[0], ("Sector A", RadiationLevel::Extreme));
        assert_eq!(levels[1], ("Sector B", RadiationLevel::High));
        assert_eq!(levels[2], ("Sector C", RadiationLevel::Mild));
        assert_eq!(levels[3], ("Sector D", RadiationLevel::None));
    }

    // --- Corruption consequence tests ---

    #[test]
    fn test_corruption_consequences() {
        let consequences = get_corruption_consequences();
        assert_eq!(consequences.len(), 4);

        // Check all 4 are present
        let ids: Vec<&str> = consequences.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&"door_controls"));
        assert!(ids.contains(&"steam_venting_overrides"));
        assert!(ids.contains(&"gravitational_systems"));
        assert!(ids.contains(&"communications_network"));

        // Check names
        let door = consequences.iter().find(|c| c.id == "door_controls").unwrap();
        assert_eq!(door.name, "Door Controls");
        assert!(!door.description.is_empty());

        let steam = consequences
            .iter()
            .find(|c| c.id == "steam_venting_overrides")
            .unwrap();
        assert!(steam.description.contains("6 inches"));

        let grav = consequences
            .iter()
            .find(|c| c.id == "gravitational_systems")
            .unwrap();
        assert!(grav.description.contains("Set to Defend"));
        assert!(grav.description.contains("Overwatch"));

        let comms = consequences
            .iter()
            .find(|c| c.id == "communications_network")
            .unwrap();
        assert!(comms.description.contains("Command points"));
    }

    // --- Destroyed points threshold tests ---

    #[test]
    fn test_destroyed_points_thresholds() {
        assert_eq!(
            destroyed_points_to_vp(0, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            0
        );
        assert_eq!(
            destroyed_points_to_vp(124, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            0
        );
        assert_eq!(
            destroyed_points_to_vp(125, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            15
        );
        assert_eq!(
            destroyed_points_to_vp(249, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            15
        );
        assert_eq!(
            destroyed_points_to_vp(250, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            30
        );
        assert_eq!(
            destroyed_points_to_vp(374, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            30
        );
        assert_eq!(
            destroyed_points_to_vp(375, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            45
        );
        assert_eq!(
            destroyed_points_to_vp(499, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            45
        );
        assert_eq!(
            destroyed_points_to_vp(500, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            45
        );
        assert_eq!(
            destroyed_points_to_vp(1000, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            45
        );
    }

    #[test]
    fn test_destroyed_points_boundary_values() {
        // Test exact boundary values (re-exported from scoring.rs)
        let thresholds = &STANDARD_DESTROYED_POINTS_THRESHOLDS;
        assert_eq!(destroyed_points_to_vp(0, thresholds), 0);
        assert_eq!(destroyed_points_to_vp(124, thresholds), 0);
        assert_eq!(destroyed_points_to_vp(125, thresholds), 15);
        assert_eq!(destroyed_points_to_vp(249, thresholds), 15);
        assert_eq!(destroyed_points_to_vp(250, thresholds), 30);
    }

    // --- Lighting roll tests ---

    #[test]
    fn test_lighting_roll_result() {
        assert_eq!(LightingRollResult::from_roll(1), LightingRollResult::LightsOff);
        assert_eq!(LightingRollResult::from_roll(2), LightingRollResult::LightsOff);
        assert_eq!(LightingRollResult::from_roll(3), LightingRollResult::LightsOff);
        assert_eq!(LightingRollResult::from_roll(4), LightingRollResult::LightsOn);
        assert_eq!(LightingRollResult::from_roll(5), LightingRollResult::LightsOn);
        assert_eq!(LightingRollResult::from_roll(6), LightingRollResult::LightsOn);
    }

    #[test]
    fn test_lighting_is_dark() {
        assert!(LightingRollResult::LightsOff.is_dark());
        assert!(!LightingRollResult::LightsOn.is_dark());
    }

    #[test]
    fn test_darkness_effects() {
        let effects = get_darkness_effects();
        assert_eq!(effects.max_ranged_range_inches, 9);
        assert_eq!(effects.ranged_hit_modifier, -1);
        assert_eq!(effects.max_charge_range_inches, 9);
        assert_eq!(effects.charge_roll_modifier, -1);
    }

    // --- Venting tests ---

    #[test]
    fn test_venting_effects() {
        let effects = get_venting_effects();
        assert!(effects.mortal_wounds_d3);
        assert_eq!(effects.move_penalty_inches, 1);
        assert!(effects.close_hatchways);
    }

    #[test]
    fn test_vent_roll() {
        assert!(!vent_roll_succeeds(1));
        assert!(!vent_roll_succeeds(2));
        assert!(vent_roll_succeeds(3));
        assert!(vent_roll_succeeds(4));
        assert!(vent_roll_succeeds(5));
        assert!(vent_roll_succeeds(6));
    }

    // --- Prison break tests ---

    #[test]
    fn test_prison_break_test() {
        // Toughness 4 unit: roll >= T fails, roll < T succeeds
        assert_eq!(prison_break_test(4, 4), PrisonBreakResult::Failure);
        assert_eq!(prison_break_test(5, 4), PrisonBreakResult::Failure);
        assert_eq!(prison_break_test(6, 4), PrisonBreakResult::Failure);
        assert_eq!(prison_break_test(3, 4), PrisonBreakResult::Success);
        assert_eq!(prison_break_test(1, 4), PrisonBreakResult::Success);

        // Toughness 5 unit
        assert_eq!(prison_break_test(5, 5), PrisonBreakResult::Failure);
        assert_eq!(prison_break_test(4, 5), PrisonBreakResult::Success);
    }

    // --- Silent infiltration tests ---

    #[test]
    fn test_silent_infiltration() {
        // First unit: needs 2+
        assert!(silent_infiltration_test(2, 0));
        assert!(silent_infiltration_test(6, 0));
        assert!(!silent_infiltration_test(1, 0));

        // Second unit: needs 3+
        assert!(silent_infiltration_test(3, 1));
        assert!(!silent_infiltration_test(2, 1));

        // Third unit: needs 4+
        assert!(silent_infiltration_test(4, 2));
        assert!(!silent_infiltration_test(3, 2));

        // Fourth unit: needs 5+
        assert!(silent_infiltration_test(5, 3));
        assert!(!silent_infiltration_test(4, 3));

        // Fifth unit: needs 6+
        assert!(silent_infiltration_test(6, 4));
        assert!(!silent_infiltration_test(5, 4));
    }

    // --- Corruption attempt tests ---

    #[test]
    fn test_corruption_attempt() {
        assert!(!corruption_attempt_succeeds(1));
        assert!(corruption_attempt_succeeds(2));
        assert!(corruption_attempt_succeeds(3));
        assert!(corruption_attempt_succeeds(6));
    }

    // --- Burner roll tests ---

    #[test]
    fn test_burner_roll() {
        assert!(!burner_roll_hits(1));
        assert!(!burner_roll_hits(2));
        assert!(!burner_roll_hits(3));
        assert!(burner_roll_hits(4));
        assert!(burner_roll_hits(5));
        assert!(burner_roll_hits(6));
    }

    #[test]
    fn test_furnace_burner_mw_cap() {
        assert_eq!(FURNACE_BURNER_MW_CAP, 3);
        assert_eq!(capped_burner_mortal_wounds(0), 0);
        assert_eq!(capped_burner_mortal_wounds(1), 1);
        assert_eq!(capped_burner_mortal_wounds(2), 2);
        assert_eq!(capped_burner_mortal_wounds(3), 3);
        assert_eq!(capped_burner_mortal_wounds(4), 3);
        assert_eq!(capped_burner_mortal_wounds(10), 3);
        assert_eq!(capped_burner_mortal_wounds(255), 3);
    }

    #[test]
    fn test_desperate_measures_cp_check() {
        assert!(!desperate_measures_cp_check(1));
        assert!(!desperate_measures_cp_check(2));
        assert!(!desperate_measures_cp_check(3));
        assert!(!desperate_measures_cp_check(4));
        assert!(desperate_measures_cp_check(5));
        assert!(desperate_measures_cp_check(6));
    }

    // --- VP transfer tests ---

    #[test]
    fn test_vp_transfer_per_loader() {
        // Defender controls Control Node: 10 VP per loader
        assert_eq!(vp_transfer_per_loader(true), 10);
        // Defender does not control: 5 VP per loader
        assert_eq!(vp_transfer_per_loader(false), 5);
    }

    #[test]
    fn test_vp_transfer_initial_state() {
        assert_eq!(VP_TRANSFER_INITIAL_ATTACKER, 60);
        assert_eq!(VP_TRANSFER_INITIAL_DEFENDER, 0);
    }

    // --- Jailbreak outcome tests ---

    #[test]
    fn test_jailbreak_outcomes() {
        assert_eq!(JailbreakOutcome::StillImprisoned.attacker_vp(), 0);
        assert_eq!(JailbreakOutcome::StillImprisoned.defender_vp(), 90);

        assert_eq!(JailbreakOutcome::EscapedButDestroyed.attacker_vp(), 30);
        assert_eq!(JailbreakOutcome::EscapedButDestroyed.defender_vp(), 60);

        assert_eq!(JailbreakOutcome::EscapedEngaged.attacker_vp(), 60);
        assert_eq!(JailbreakOutcome::EscapedEngaged.defender_vp(), 30);

        assert_eq!(JailbreakOutcome::EscapedFree.attacker_vp(), 90);
        assert_eq!(JailbreakOutcome::EscapedFree.defender_vp(), 0);
    }

    #[test]
    fn test_jailbreak_outcome_from_state() {
        assert_eq!(
            JailbreakOutcome::from_state(true, false, false),
            JailbreakOutcome::StillImprisoned
        );
        assert_eq!(
            JailbreakOutcome::from_state(false, true, false),
            JailbreakOutcome::EscapedButDestroyed
        );
        assert_eq!(
            JailbreakOutcome::from_state(false, false, true),
            JailbreakOutcome::EscapedEngaged
        );
        assert_eq!(
            JailbreakOutcome::from_state(false, false, false),
            JailbreakOutcome::EscapedFree
        );
    }

    #[test]
    fn test_jailbreak_vp_symmetry() {
        // Attacker VP + Defender VP should always equal 90
        for outcome in [
            JailbreakOutcome::StillImprisoned,
            JailbreakOutcome::EscapedButDestroyed,
            JailbreakOutcome::EscapedEngaged,
            JailbreakOutcome::EscapedFree,
        ] {
            assert_eq!(
                outcome.attacker_vp() + outcome.defender_vp(),
                90,
                "VP should sum to 90 for {:?}",
                outcome
            );
        }
    }

    // --- Unlock overrides tests ---

    #[test]
    fn test_unlock_overrides() {
        assert!(unlock_overrides_fires(true, true, true));
        assert!(!unlock_overrides_fires(false, true, true));
        assert!(!unlock_overrides_fires(true, false, true));
        assert!(!unlock_overrides_fires(true, true, false));
    }

    // --- Multi-level tests ---

    #[test]
    fn test_can_change_level() {
        assert!(can_change_level(true));
        assert!(!can_change_level(false));
    }

    // --- Serialization tests ---

    #[test]
    fn test_radiation_level_serialization() {
        let level = RadiationLevel::Extreme;
        let json = serde_json::to_string(&level).unwrap();
        let deserialized: RadiationLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, level);
    }

    #[test]
    fn test_mission_mechanic_serialization() {
        let mechanic = MissionMechanic::RadiationProgression;
        let json = serde_json::to_string(&mechanic).unwrap();
        let deserialized: MissionMechanic = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, mechanic);
    }

    #[test]
    fn test_lighting_roll_serialization() {
        let result = LightingRollResult::LightsOff;
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: LightingRollResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, result);
    }

    #[test]
    fn test_jailbreak_outcome_serialization() {
        let outcome = JailbreakOutcome::EscapedEngaged;
        let json = serde_json::to_string(&outcome).unwrap();
        let deserialized: JailbreakOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, outcome);
    }

    #[test]
    fn test_corruption_consequence_serialization() {
        let consequences = get_corruption_consequences();
        let json = serde_json::to_string(&consequences).unwrap();
        let deserialized: Vec<CorruptionConsequence> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 4);
        assert_eq!(deserialized[0].id, consequences[0].id);
    }
}

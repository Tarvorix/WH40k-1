use serde::{Deserialize, Serialize};

/// A complete faction definition for Boarding Actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardingFactionDef {
    pub faction_name: String,
    pub faction_keyword: String,
    pub army_rule_name: String,
    pub army_rule_description: String,
    pub detachments: Vec<BoardingDetachmentDef>,
    #[serde(default)]
    pub datasheets: Vec<BoardingUnitDatasheet>,
}

/// A Boarding Actions detachment with its unique rules, stratagems, enhancements, and allowed units.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardingDetachmentDef {
    pub detachment_name: String,
    pub detachment_rule_name: String,
    pub detachment_rule_description: String,
    pub enhancements: Vec<BoardingEnhancementDef>,
    pub stratagems: Vec<BoardingStratagemDef>,
    pub allowed_units: Vec<BoardingUnitRef>,
    pub mustering_rules: Vec<MusteringRule>,
}

/// An enhancement available in a Boarding Actions detachment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardingEnhancementDef {
    pub name: String,
    pub description: String,
}

/// A stratagem available in a Boarding Actions detachment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardingStratagemDef {
    pub name: String,
    pub cp_cost: u8,
    pub timing: String,
    pub target: String,
    pub effect: String,
}

/// A reference to a unit allowed in a Boarding Actions detachment, with any restrictions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardingUnitRef {
    pub datasheet_name: String,
    #[serde(default)]
    pub max_count: Option<u8>,
    #[serde(default)]
    pub allowed_sizes: Vec<u8>,
    #[serde(default)]
    pub conditional_limit: Option<String>,
}

/// A mustering rule that applies to army construction for a Boarding Actions detachment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusteringRule {
    pub rule_type: String,
    pub description: String,
}

/// A complete unit datasheet for Boarding Actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardingUnitDatasheet {
    pub name: String,
    pub points: Vec<PointsCost>,
    pub is_epic_hero: bool,
    pub is_character: bool,
    pub is_battleline: bool,
    pub profile: UnitProfile,
    pub ranged_weapons: Vec<WeaponProfile>,
    pub melee_weapons: Vec<WeaponProfile>,
    pub abilities: Vec<AbilityDef>,
    #[serde(default)]
    pub leader_info: Option<LeaderInfo>,
    pub keywords: Vec<String>,
    pub faction_keywords: Vec<String>,
    #[serde(default)]
    pub wargear_options: Vec<WargearOption>,
    pub unit_composition: Vec<CompositionEntry>,
}

/// Points cost for a specific model count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointsCost {
    pub model_count: u8,
    pub points: u16,
}

/// A unit's stat profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitProfile {
    /// e.g., "6\"" or "5\""
    pub movement: String,
    pub toughness: u8,
    /// e.g., "3+" or "2+"
    pub save: String,
    pub wounds: u8,
    /// e.g., "6+"
    pub leadership: String,
    pub oc: u8,
}

/// A weapon profile (ranged or melee).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponProfile {
    pub name: String,
    /// e.g., "24\"" for ranged, "Melee" for melee
    pub range: String,
    /// e.g., "4" or "D6+1"
    pub attacks: String,
    /// e.g., "3+" or "N/A"
    pub skill: String,
    pub strength: u8,
    /// e.g., "-1" or "0"
    pub ap: String,
    /// e.g., "2" or "D3"
    pub damage: String,
    /// e.g., ["Lethal Hits", "Anti-Infantry 4+"]
    #[serde(default)]
    pub abilities: Vec<String>,
}

/// An ability definition for a unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityDef {
    pub name: String,
    pub description: String,
    /// One of: "core", "faction", "unit", "invulnerable_save", "leader"
    pub ability_type: String,
}

/// Leader attachment information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderInfo {
    /// Unit names this leader can join.
    pub can_lead: Vec<String>,
    pub leader_abilities: Vec<String>,
}

/// A wargear option for a unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WargearOption {
    pub description: String,
}

/// A composition entry describing model names and counts in a unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionEntry {
    pub model_name: String,
    /// e.g., "1" or "1 Chaos Champion + 4 Chaos Terminators"
    pub count: String,
}

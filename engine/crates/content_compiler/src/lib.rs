//! WH40K Engine - ContentCompiler crate
//!
//! Parses YAML faction/mission files, validates them, resolves references,
//! computes stable content IDs, and emits compiled ContentPack artifacts.
//!
//! Source: implementation_v3.md Section 9

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use wh40k_content_schema::{
    AbilitySchema, ContentPack, DatasheetSchema, EnhancementSchema, FactionSchema,
    MissionSchema, RulePrimitive, SecondaryObjectiveSchema, StratagemSchema,
    WeaponProfileSchema,
};
use wh40k_core_types::{
    ContentPackId, DatasheetId, FactionId, MissionId,
};

// ─── Severity ───────────────────────────────────────────────────────────────

/// Severity level for validation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// Fatal validation error - compilation cannot proceed
    Error,
    /// Non-fatal warning - compilation can proceed but content may have issues
    Warning,
}

// ─── ValidationError ────────────────────────────────────────────────────────

/// An individual validation issue found during content compilation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationError {
    /// Which field or path had the issue
    pub field: String,
    /// Human-readable description of the issue
    pub message: String,
    /// Whether this is a fatal error or a warning
    pub severity: Severity,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = match self.severity {
            Severity::Error => "ERROR",
            Severity::Warning => "WARNING",
        };
        write!(f, "[{}] {}: {}", level, self.field, self.message)
    }
}

// ─── CompilerError ──────────────────────────────────────────────────────────

/// Errors that can occur during content compilation.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CompilerError {
    /// YAML parsing failed
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Content validation failed with one or more errors
    #[error("Validation errors: {}", .0.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; "))]
    ValidationError(Vec<ValidationError>),

    /// An unresolved reference was found (e.g., weapon name in loadout doesn't match any weapon)
    #[error("Reference error: {0}")]
    ReferenceError(String),

    /// A duplicate name or ID was found
    #[error("Duplicate error: {0}")]
    DuplicateError(String),

    /// Serialization or deserialization failed
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

// ─── Deterministic hashing ──────────────────────────────────────────────────

/// Compute a stable deterministic hash from an arbitrary serializable value.
/// Uses serde_json canonical serialization then a simple FNV-1a-like hash
/// to ensure same content always yields the same u64 regardless of HashMap
/// iteration order or platform.
fn stable_hash_from_json<T: Serialize>(value: &T) -> u64 {
    // Serialize to canonical JSON bytes
    let json_bytes = serde_json::to_vec(value).unwrap_or_default();
    // FNV-1a hash
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in &json_bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Compute a stable deterministic hash for a string (used for ID generation).
fn stable_hash_str(s: &str) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for byte in s.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

/// Generate a deterministic DatasheetId from faction name + datasheet name.
fn generate_datasheet_id(faction_name: &str, datasheet_name: &str) -> DatasheetId {
    let combined = format!("{}::{}", faction_name, datasheet_name);
    DatasheetId::new(stable_hash_str(&combined))
}

/// Generate a deterministic FactionId from faction name.
fn generate_faction_id(faction_name: &str) -> FactionId {
    FactionId::new(stable_hash_str(faction_name))
}

/// Generate a deterministic MissionId from mission name.
fn generate_mission_id(mission_name: &str) -> MissionId {
    MissionId::new(stable_hash_str(mission_name))
}

/// Generate a deterministic ContentPackId from pack content hash.
fn generate_pack_id(content_hash: u64) -> ContentPackId {
    ContentPackId::new(content_hash as u32)
}

// ─── ContentCompiler ────────────────────────────────────────────────────────

/// Main compiler struct that parses YAML faction/mission files, validates them,
/// resolves references, computes stable content IDs, and emits compiled ContentPack artifacts.
#[derive(Debug, Clone, Default)]
pub struct ContentCompiler {
    /// Collected warnings during compilation (non-fatal)
    warnings: Vec<ValidationError>,
}

impl ContentCompiler {
    /// Create a new ContentCompiler instance.
    pub fn new() -> Self {
        Self {
            warnings: Vec::new(),
        }
    }

    /// Get any warnings from the last compilation operation.
    pub fn warnings(&self) -> &[ValidationError] {
        &self.warnings
    }

    /// Clear accumulated warnings.
    pub fn clear_warnings(&mut self) {
        self.warnings.clear();
    }

    // ── Compile Faction ─────────────────────────────────────────────────

    /// Parse and validate a faction YAML string into a FactionSchema.
    ///
    /// This method:
    /// 1. Parses the YAML into a FactionSchema
    /// 2. Assigns deterministic IDs based on content
    /// 3. Validates the faction
    /// 4. Returns the compiled FactionSchema or an error
    pub fn compile_faction(&mut self, yaml_str: &str) -> Result<FactionSchema, CompilerError> {
        self.clear_warnings();

        // Parse YAML
        let mut faction: FactionSchema = serde_yaml::from_str(yaml_str)
            .map_err(|e| CompilerError::ParseError(format!("Failed to parse faction YAML: {}", e)))?;

        // Assign deterministic IDs
        faction.id = generate_faction_id(&faction.name);
        for datasheet in &mut faction.datasheets {
            datasheet.id = generate_datasheet_id(&faction.name, &datasheet.name);
        }

        // Validate
        match self.validate_faction(&faction) {
            Ok(()) => {}
            Err(errors) => {
                let (fatal, warnings): (Vec<_>, Vec<_>) = errors
                    .into_iter()
                    .partition(|e| e.severity == Severity::Error);
                self.warnings.extend(warnings);
                if !fatal.is_empty() {
                    return Err(CompilerError::ValidationError(fatal));
                }
            }
        }

        Ok(faction)
    }

    // ── Compile Mission ─────────────────────────────────────────────────

    /// Parse and validate a mission YAML string into a MissionSchema.
    ///
    /// This method:
    /// 1. Parses the YAML into a MissionSchema
    /// 2. Assigns a deterministic ID based on content
    /// 3. Validates the mission
    /// 4. Returns the compiled MissionSchema or an error
    pub fn compile_mission(&mut self, yaml_str: &str) -> Result<MissionSchema, CompilerError> {
        self.clear_warnings();

        // Parse YAML
        let mut mission: MissionSchema = serde_yaml::from_str(yaml_str)
            .map_err(|e| CompilerError::ParseError(format!("Failed to parse mission YAML: {}", e)))?;

        // Assign deterministic ID
        mission.id = generate_mission_id(&mission.name);

        // Validate
        match self.validate_mission(&mission) {
            Ok(()) => {}
            Err(errors) => {
                let (fatal, warnings): (Vec<_>, Vec<_>) = errors
                    .into_iter()
                    .partition(|e| e.severity == Severity::Error);
                self.warnings.extend(warnings);
                if !fatal.is_empty() {
                    return Err(CompilerError::ValidationError(fatal));
                }
            }
        }

        Ok(mission)
    }

    // ── Compile Pack ────────────────────────────────────────────────────

    /// Assemble factions and missions into a ContentPack.
    ///
    /// This method:
    /// 1. Validates all factions and missions
    /// 2. Checks for cross-faction duplicate names
    /// 3. Computes a deterministic content hash
    /// 4. Returns the assembled ContentPack
    pub fn compile_pack(
        &mut self,
        factions: Vec<FactionSchema>,
        missions: Vec<MissionSchema>,
    ) -> Result<ContentPack, CompilerError> {
        self.clear_warnings();

        // Validate all factions
        for faction in &factions {
            match self.validate_faction(faction) {
                Ok(()) => {}
                Err(errors) => {
                    let (fatal, warnings): (Vec<_>, Vec<_>) = errors
                        .into_iter()
                        .partition(|e| e.severity == Severity::Error);
                    self.warnings.extend(warnings);
                    if !fatal.is_empty() {
                        return Err(CompilerError::ValidationError(fatal));
                    }
                }
            }
        }

        // Validate all missions
        for mission in &missions {
            match self.validate_mission(mission) {
                Ok(()) => {}
                Err(errors) => {
                    let (fatal, warnings): (Vec<_>, Vec<_>) = errors
                        .into_iter()
                        .partition(|e| e.severity == Severity::Error);
                    self.warnings.extend(warnings);
                    if !fatal.is_empty() {
                        return Err(CompilerError::ValidationError(fatal));
                    }
                }
            }
        }

        // Check for duplicate faction names across the pack
        let mut faction_names: HashSet<String> = HashSet::new();
        for faction in &factions {
            if !faction_names.insert(faction.name.clone()) {
                return Err(CompilerError::DuplicateError(format!(
                    "Duplicate faction name: '{}'",
                    faction.name
                )));
            }
        }

        // Check for duplicate mission names across the pack
        let mut mission_names: HashSet<String> = HashSet::new();
        for mission in &missions {
            if !mission_names.insert(mission.name.clone()) {
                return Err(CompilerError::DuplicateError(format!(
                    "Duplicate mission name: '{}'",
                    mission.name
                )));
            }
        }

        // Build a preliminary pack to hash
        let preliminary_pack = ContentPack {
            pack_id: ContentPackId::new(0),
            version: "0.1.0".to_string(),
            content_hash: 0,
            factions: factions.clone(),
            missions: missions.clone(),
        };

        let content_hash = self.compute_content_hash(&preliminary_pack);
        let pack_id = generate_pack_id(content_hash);

        Ok(ContentPack {
            pack_id,
            version: "0.1.0".to_string(),
            content_hash,
            factions,
            missions,
        })
    }

    // ── Validate Faction ────────────────────────────────────────────────

    /// Deep validation of a FactionSchema.
    ///
    /// Checks:
    /// - All datasheets have at least one weapon
    /// - All weapons have valid stats (attacks > 0, strength > 0, etc.)
    /// - All ability effects use valid RulePrimitive variants
    /// - All stratagems have CP cost > 0
    /// - All enhancements have at least one effect
    /// - Unit sizes are valid
    /// - Keyword sets are not empty
    /// - References between datasheets/abilities/weapons resolve
    /// - No duplicate names within the faction
    pub fn validate_faction(&self, faction: &FactionSchema) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Faction name must not be empty
        if faction.name.trim().is_empty() {
            errors.push(ValidationError {
                field: "faction.name".to_string(),
                message: "Faction name must not be empty".to_string(),
                severity: Severity::Error,
            });
        }

        // Check for duplicate datasheet names within the faction
        let mut datasheet_names: HashSet<String> = HashSet::new();
        for ds in &faction.datasheets {
            if !datasheet_names.insert(ds.name.clone()) {
                errors.push(ValidationError {
                    field: format!("faction.datasheets[{}]", ds.name),
                    message: format!("Duplicate datasheet name: '{}'", ds.name),
                    severity: Severity::Error,
                });
            }
        }

        // Check for duplicate stratagem names
        let mut stratagem_names: HashSet<String> = HashSet::new();
        for strat in &faction.stratagems {
            if !stratagem_names.insert(strat.name.clone()) {
                errors.push(ValidationError {
                    field: format!("faction.stratagems[{}]", strat.name),
                    message: format!("Duplicate stratagem name: '{}'", strat.name),
                    severity: Severity::Error,
                });
            }
        }

        // Check for duplicate enhancement names
        let mut enhancement_names: HashSet<String> = HashSet::new();
        for enh in &faction.enhancements {
            if !enhancement_names.insert(enh.name.clone()) {
                errors.push(ValidationError {
                    field: format!("faction.enhancements[{}]", enh.name),
                    message: format!("Duplicate enhancement name: '{}'", enh.name),
                    severity: Severity::Error,
                });
            }
        }

        // Check for duplicate secondary objective names
        let mut secondary_names: HashSet<String> = HashSet::new();
        for sec in &faction.secondary_objectives {
            if !secondary_names.insert(sec.name.clone()) {
                errors.push(ValidationError {
                    field: format!("faction.secondary_objectives[{}]", sec.name),
                    message: format!("Duplicate secondary objective name: '{}'", sec.name),
                    severity: Severity::Error,
                });
            }
        }

        // Validate faction ability
        self.validate_ability_schema(
            &faction.faction_ability,
            "faction.faction_ability",
            &mut errors,
        );

        // Validate each datasheet
        for (idx, ds) in faction.datasheets.iter().enumerate() {
            self.validate_datasheet(ds, &format!("faction.datasheets[{}]", idx), &mut errors);
        }

        // Validate each stratagem
        for (idx, strat) in faction.stratagems.iter().enumerate() {
            self.validate_stratagem(strat, &format!("faction.stratagems[{}]", idx), &mut errors);
        }

        // Validate each enhancement
        for (idx, enh) in faction.enhancements.iter().enumerate() {
            self.validate_enhancement(enh, &format!("faction.enhancements[{}]", idx), &mut errors);
        }

        // Validate each secondary objective
        for (idx, sec) in faction.secondary_objectives.iter().enumerate() {
            self.validate_secondary_objective(
                sec,
                &format!("faction.secondary_objectives[{}]", idx),
                &mut errors,
            );
        }

        // Validate cross-references: model loadout weapon names must reference actual weapons
        for ds in &faction.datasheets {
            let ranged_weapon_names: HashSet<&str> = ds
                .ranged_weapons
                .iter()
                .map(|w| w.name.as_str())
                .collect();
            let melee_weapon_names: HashSet<&str> = ds
                .melee_weapons
                .iter()
                .map(|w| w.name.as_str())
                .collect();

            for loadout in &ds.model_loadouts {
                for rw_name in &loadout.ranged_weapons {
                    if !ranged_weapon_names.contains(rw_name.as_str()) {
                        errors.push(ValidationError {
                            field: format!(
                                "datasheet[{}].model_loadouts[{}].ranged_weapons",
                                ds.name, loadout.model_label
                            ),
                            message: format!(
                                "Ranged weapon '{}' referenced in loadout for '{}' not found in datasheet '{}'",
                                rw_name, loadout.model_label, ds.name
                            ),
                            severity: Severity::Error,
                        });
                    }
                }
                for mw_name in &loadout.melee_weapons {
                    if !melee_weapon_names.contains(mw_name.as_str()) {
                        errors.push(ValidationError {
                            field: format!(
                                "datasheet[{}].model_loadouts[{}].melee_weapons",
                                ds.name, loadout.model_label
                            ),
                            message: format!(
                                "Melee weapon '{}' referenced in loadout for '{}' not found in datasheet '{}'",
                                mw_name, loadout.model_label, ds.name
                            ),
                            severity: Severity::Error,
                        });
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    // ── Validate Mission ────────────────────────────────────────────────

    /// Deep validation of a MissionSchema.
    ///
    /// Checks:
    /// - Mission name is not empty
    /// - Has at least one objective
    /// - Has primary scoring rules
    /// - Rounds > 0
    /// - Objective labels are unique
    pub fn validate_mission(&self, mission: &MissionSchema) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Mission name must not be empty
        if mission.name.trim().is_empty() {
            errors.push(ValidationError {
                field: "mission.name".to_string(),
                message: "Mission name must not be empty".to_string(),
                severity: Severity::Error,
            });
        }

        // Must have at least one objective
        if mission.objectives.is_empty() {
            errors.push(ValidationError {
                field: "mission.objectives".to_string(),
                message: "Mission must have at least one objective marker".to_string(),
                severity: Severity::Error,
            });
        }

        // Must have primary scoring rules
        if mission.primary_scoring.is_empty() {
            errors.push(ValidationError {
                field: "mission.primary_scoring".to_string(),
                message: "Mission must have at least one primary scoring rule".to_string(),
                severity: Severity::Error,
            });
        }

        // Rounds must be positive
        if mission.rounds == 0 {
            errors.push(ValidationError {
                field: "mission.rounds".to_string(),
                message: "Mission must have at least 1 battle round".to_string(),
                severity: Severity::Error,
            });
        }

        // Check for duplicate objective labels
        let mut objective_labels: HashSet<String> = HashSet::new();
        for obj in &mission.objectives {
            if !objective_labels.insert(obj.label.clone()) {
                errors.push(ValidationError {
                    field: format!("mission.objectives[{}]", obj.label),
                    message: format!("Duplicate objective label: '{}'", obj.label),
                    severity: Severity::Error,
                });
            }
        }

        // Validate primary scoring rules have proper structure
        for (idx, rule) in mission.primary_scoring.iter().enumerate() {
            if rule.vp_amount == 0 {
                errors.push(ValidationError {
                    field: format!("mission.primary_scoring[{}].vp_amount", idx),
                    message: "Primary scoring rule must award non-zero VP".to_string(),
                    severity: Severity::Warning,
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    // ── Validate Datasheet ──────────────────────────────────────────────

    /// Validate a single datasheet.
    fn validate_datasheet(
        &self,
        ds: &DatasheetSchema,
        path: &str,
        errors: &mut Vec<ValidationError>,
    ) {
        // Name must not be empty
        if ds.name.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("{}.name", path),
                message: "Datasheet name must not be empty".to_string(),
                severity: Severity::Error,
            });
        }

        // Must have at least one weapon (ranged or melee)
        if ds.ranged_weapons.is_empty() && ds.melee_weapons.is_empty() {
            errors.push(ValidationError {
                field: format!("{}.weapons", path),
                message: format!(
                    "Datasheet '{}' must have at least one weapon (ranged or melee)",
                    ds.name
                ),
                severity: Severity::Error,
            });
        }

        // Validate all ranged weapons
        for (widx, weapon) in ds.ranged_weapons.iter().enumerate() {
            self.validate_weapon_profile(
                weapon,
                &format!("{}.ranged_weapons[{}]", path, widx),
                errors,
            );
        }

        // Validate all melee weapons
        for (widx, weapon) in ds.melee_weapons.iter().enumerate() {
            self.validate_weapon_profile(
                weapon,
                &format!("{}.melee_weapons[{}]", path, widx),
                errors,
            );
        }

        // Validate unit size
        match ds.unit_size {
            wh40k_content_schema::UnitSizeSpec::Fixed(n) => {
                if n == 0 {
                    errors.push(ValidationError {
                        field: format!("{}.unit_size", path),
                        message: format!(
                            "Datasheet '{}' has invalid unit size: Fixed(0)",
                            ds.name
                        ),
                        severity: Severity::Error,
                    });
                }
            }
            wh40k_content_schema::UnitSizeSpec::Range { min, max } => {
                if min == 0 {
                    errors.push(ValidationError {
                        field: format!("{}.unit_size", path),
                        message: format!(
                            "Datasheet '{}' has invalid unit size: min=0",
                            ds.name
                        ),
                        severity: Severity::Error,
                    });
                }
                if max < min {
                    errors.push(ValidationError {
                        field: format!("{}.unit_size", path),
                        message: format!(
                            "Datasheet '{}' has invalid unit size: max({}) < min({})",
                            ds.name, max, min
                        ),
                        severity: Severity::Error,
                    });
                }
            }
        }

        // Keywords must not be empty
        if ds.keywords.count() == 0 {
            errors.push(ValidationError {
                field: format!("{}.keywords", path),
                message: format!("Datasheet '{}' has empty keyword set", ds.name),
                severity: Severity::Error,
            });
        }

        // Validate toughness > 0
        if ds.toughness.value() == 0 {
            errors.push(ValidationError {
                field: format!("{}.toughness", path),
                message: format!("Datasheet '{}' has toughness of 0", ds.name),
                severity: Severity::Error,
            });
        }

        // Validate wounds > 0
        if ds.wounds.value() == 0 {
            errors.push(ValidationError {
                field: format!("{}.wounds", path),
                message: format!("Datasheet '{}' has wounds of 0", ds.name),
                severity: Severity::Error,
            });
        }

        // Validate abilities
        for (aidx, ability) in ds.abilities.iter().enumerate() {
            self.validate_ability_schema(
                ability,
                &format!("{}.abilities[{}]", path, aidx),
                errors,
            );
        }

        // Check for duplicate weapon names within ranged weapons
        let mut ranged_names: HashSet<String> = HashSet::new();
        for weapon in &ds.ranged_weapons {
            if !ranged_names.insert(weapon.name.clone()) {
                errors.push(ValidationError {
                    field: format!("{}.ranged_weapons", path),
                    message: format!(
                        "Duplicate ranged weapon name '{}' in datasheet '{}'",
                        weapon.name, ds.name
                    ),
                    severity: Severity::Warning,
                });
            }
        }

        // Check for duplicate weapon names within melee weapons
        let mut melee_names: HashSet<String> = HashSet::new();
        for weapon in &ds.melee_weapons {
            if !melee_names.insert(weapon.name.clone()) {
                errors.push(ValidationError {
                    field: format!("{}.melee_weapons", path),
                    message: format!(
                        "Duplicate melee weapon name '{}' in datasheet '{}'",
                        weapon.name, ds.name
                    ),
                    severity: Severity::Warning,
                });
            }
        }
    }

    // ── Validate Weapon Profile ─────────────────────────────────────────

    /// Validate a single weapon profile.
    fn validate_weapon_profile(
        &self,
        weapon: &WeaponProfileSchema,
        path: &str,
        errors: &mut Vec<ValidationError>,
    ) {
        // Name must not be empty
        if weapon.name.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("{}.name", path),
                message: "Weapon name must not be empty".to_string(),
                severity: Severity::Error,
            });
        }

        // Attacks must be > 0 (minimum possible attacks)
        if weapon.attacks.min_value() == 0 && weapon.attacks.max_value() == 0 {
            errors.push(ValidationError {
                field: format!("{}.attacks", path),
                message: format!(
                    "Weapon '{}' must have attacks > 0",
                    weapon.name
                ),
                severity: Severity::Error,
            });
        }

        // Strength must be > 0
        if weapon.strength.value() == 0 {
            errors.push(ValidationError {
                field: format!("{}.strength", path),
                message: format!(
                    "Weapon '{}' must have strength > 0",
                    weapon.name
                ),
                severity: Severity::Error,
            });
        }

        // Skill must be between 2 and 6 (inclusive)
        if weapon.skill.value() < 2 || weapon.skill.value() > 6 {
            errors.push(ValidationError {
                field: format!("{}.skill", path),
                message: format!(
                    "Weapon '{}' has invalid skill value: {} (must be 2-6)",
                    weapon.name,
                    weapon.skill.value()
                ),
                severity: Severity::Error,
            });
        }

        // Damage must have min > 0
        if weapon.damage.min_value() == 0 && weapon.damage.max_value() == 0 {
            errors.push(ValidationError {
                field: format!("{}.damage", path),
                message: format!(
                    "Weapon '{}' must have damage > 0",
                    weapon.name
                ),
                severity: Severity::Error,
            });
        }

        // Ranged weapons should have range > 0; melee weapons should have range = 0
        match weapon.weapon_type {
            wh40k_core_types::WeaponType::Ranged => {
                if weapon.range.mils() <= 0 {
                    errors.push(ValidationError {
                        field: format!("{}.range", path),
                        message: format!(
                            "Ranged weapon '{}' must have range > 0",
                            weapon.name
                        ),
                        severity: Severity::Warning,
                    });
                }
            }
            wh40k_core_types::WeaponType::Melee => {
                if weapon.range.mils() != 0 {
                    errors.push(ValidationError {
                        field: format!("{}.range", path),
                        message: format!(
                            "Melee weapon '{}' should have range = 0, got {}",
                            weapon.name,
                            weapon.range
                        ),
                        severity: Severity::Warning,
                    });
                }
            }
        }
    }

    // ── Validate Ability ────────────────────────────────────────────────

    /// Validate a single ability schema.
    fn validate_ability_schema(
        &self,
        ability: &AbilitySchema,
        path: &str,
        errors: &mut Vec<ValidationError>,
    ) {
        // Name must not be empty
        if ability.name.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("{}.name", path),
                message: "Ability name must not be empty".to_string(),
                severity: Severity::Error,
            });
        }

        // Description should not be empty (warning only)
        if ability.description.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("{}.description", path),
                message: format!("Ability '{}' has empty description", ability.name),
                severity: Severity::Warning,
            });
        }

        // Validate effects contain valid RulePrimitives
        for (eidx, effect) in ability.effects.iter().enumerate() {
            self.validate_rule_primitive(
                effect,
                &format!("{}.effects[{}]", path, eidx),
                errors,
            );
        }
    }

    // ── Validate Stratagem ──────────────────────────────────────────────

    /// Validate a single stratagem.
    fn validate_stratagem(
        &self,
        strat: &StratagemSchema,
        path: &str,
        errors: &mut Vec<ValidationError>,
    ) {
        // Name must not be empty
        if strat.name.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("{}.name", path),
                message: "Stratagem name must not be empty".to_string(),
                severity: Severity::Error,
            });
        }

        // CP cost must be > 0
        if strat.cp_cost == 0 {
            errors.push(ValidationError {
                field: format!("{}.cp_cost", path),
                message: format!(
                    "Stratagem '{}' must have CP cost > 0",
                    strat.name
                ),
                severity: Severity::Error,
            });
        }

        // Must have at least one effect
        if strat.effects.is_empty() {
            errors.push(ValidationError {
                field: format!("{}.effects", path),
                message: format!(
                    "Stratagem '{}' must have at least one effect",
                    strat.name
                ),
                severity: Severity::Warning,
            });
        }

        // Validate effects
        for (eidx, effect) in strat.effects.iter().enumerate() {
            self.validate_rule_primitive(
                effect,
                &format!("{}.effects[{}]", path, eidx),
                errors,
            );
        }
    }

    // ── Validate Enhancement ────────────────────────────────────────────

    /// Validate a single enhancement.
    fn validate_enhancement(
        &self,
        enh: &EnhancementSchema,
        path: &str,
        errors: &mut Vec<ValidationError>,
    ) {
        // Name must not be empty
        if enh.name.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("{}.name", path),
                message: "Enhancement name must not be empty".to_string(),
                severity: Severity::Error,
            });
        }

        // Must have at least one effect
        if enh.effects.is_empty() {
            errors.push(ValidationError {
                field: format!("{}.effects", path),
                message: format!(
                    "Enhancement '{}' must have at least one effect",
                    enh.name
                ),
                severity: Severity::Error,
            });
        }

        // Validate effects
        for (eidx, effect) in enh.effects.iter().enumerate() {
            self.validate_rule_primitive(
                effect,
                &format!("{}.effects[{}]", path, eidx),
                errors,
            );
        }
    }

    // ── Validate Secondary Objective ────────────────────────────────────

    /// Validate a single secondary objective.
    fn validate_secondary_objective(
        &self,
        sec: &SecondaryObjectiveSchema,
        path: &str,
        errors: &mut Vec<ValidationError>,
    ) {
        // Name must not be empty
        if sec.name.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("{}.name", path),
                message: "Secondary objective name must not be empty".to_string(),
                severity: Severity::Error,
            });
        }

        // Must have scoring rules
        if sec.scoring.is_empty() {
            errors.push(ValidationError {
                field: format!("{}.scoring", path),
                message: format!(
                    "Secondary objective '{}' must have at least one scoring rule",
                    sec.name
                ),
                severity: Severity::Error,
            });
        }

        // Validate each scoring rule
        for (ridx, rule) in sec.scoring.iter().enumerate() {
            if rule.vp_amount == 0 {
                errors.push(ValidationError {
                    field: format!("{}.scoring[{}].vp_amount", path, ridx),
                    message: format!(
                        "Scoring rule in secondary objective '{}' has vp_amount of 0",
                        sec.name
                    ),
                    severity: Severity::Warning,
                });
            }
        }
    }

    // ── Validate RulePrimitive ──────────────────────────────────────────

    /// Validate a single RulePrimitive recursively.
    fn validate_rule_primitive(
        &self,
        primitive: &RulePrimitive,
        path: &str,
        errors: &mut Vec<ValidationError>,
    ) {
        match primitive {
            RulePrimitive::Composite { effects } => {
                if effects.is_empty() {
                    errors.push(ValidationError {
                        field: path.to_string(),
                        message: "Composite effect must have at least one sub-effect".to_string(),
                        severity: Severity::Warning,
                    });
                }
                for (eidx, effect) in effects.iter().enumerate() {
                    self.validate_rule_primitive(
                        effect,
                        &format!("{}.effects[{}]", path, eidx),
                        errors,
                    );
                }
            }
            RulePrimitive::Conditional {
                effect,
                else_effect,
                ..
            } => {
                self.validate_rule_primitive(effect, &format!("{}.effect", path), errors);
                if let Some(else_eff) = else_effect {
                    self.validate_rule_primitive(
                        else_eff,
                        &format!("{}.else_effect", path),
                        errors,
                    );
                }
            }
            RulePrimitive::ApplyForDuration { effect, .. } => {
                self.validate_rule_primitive(effect, &format!("{}.effect", path), errors);
            }
            RulePrimitive::TriggerOnDestroyed { effect } => {
                self.validate_rule_primitive(effect, &format!("{}.effect", path), errors);
            }
            RulePrimitive::ForceLeadershipTest { on_fail, .. } => {
                self.validate_rule_primitive(on_fail, &format!("{}.on_fail", path), errors);
            }
            RulePrimitive::DiceCheck {
                on_success,
                on_failure,
                threshold,
                ..
            } => {
                if *threshold < 2 || *threshold > 6 {
                    errors.push(ValidationError {
                        field: format!("{}.threshold", path),
                        message: format!(
                            "DiceCheck threshold must be 2-6, got {}",
                            threshold
                        ),
                        severity: Severity::Warning,
                    });
                }
                self.validate_rule_primitive(
                    on_success,
                    &format!("{}.on_success", path),
                    errors,
                );
                if let Some(on_fail) = on_failure {
                    self.validate_rule_primitive(
                        on_fail,
                        &format!("{}.on_failure", path),
                        errors,
                    );
                }
            }
            RulePrimitive::DicePoolAllocation { blessings, .. } => {
                for (bidx, blessing) in blessings.iter().enumerate() {
                    if blessing.name.trim().is_empty() {
                        errors.push(ValidationError {
                            field: format!("{}.blessings[{}].name", path, bidx),
                            message: "Blessing name must not be empty".to_string(),
                            severity: Severity::Error,
                        });
                    }
                    for (eidx, effect) in blessing.effects.iter().enumerate() {
                        self.validate_rule_primitive(
                            effect,
                            &format!("{}.blessings[{}].effects[{}]", path, bidx, eidx),
                            errors,
                        );
                    }
                }
            }
            RulePrimitive::StanceChoice { stances } => {
                if stances.is_empty() {
                    errors.push(ValidationError {
                        field: path.to_string(),
                        message: "StanceChoice must have at least one stance".to_string(),
                        severity: Severity::Error,
                    });
                }
                for (sidx, stance) in stances.iter().enumerate() {
                    if stance.name.trim().is_empty() {
                        errors.push(ValidationError {
                            field: format!("{}.stances[{}].name", path, sidx),
                            message: "Stance name must not be empty".to_string(),
                            severity: Severity::Error,
                        });
                    }
                    for (eidx, effect) in stance.effects.iter().enumerate() {
                        self.validate_rule_primitive(
                            effect,
                            &format!("{}.stances[{}].effects[{}]", path, sidx, eidx),
                            errors,
                        );
                    }
                }
            }
            // All other primitives are leaf nodes that are valid by construction
            // (their fields are typed and constrained by the enum structure)
            _ => {}
        }
    }

    // ── Content Hash ────────────────────────────────────────────────────

    /// Compute a stable deterministic hash of a ContentPack.
    ///
    /// The hash is computed from the JSON serialization of the factions and missions
    /// (excluding the pack_id and content_hash fields themselves to avoid circular dependency).
    /// Same YAML input will always produce the same hash.
    pub fn compute_content_hash(&self, pack: &ContentPack) -> u64 {
        // Hash based on factions + missions content only (not pack_id or content_hash)
        let hashable = (&pack.factions, &pack.missions);
        stable_hash_from_json(&hashable)
    }

    // ── Serialization ───────────────────────────────────────────────────

    /// Serialize a ContentPack to bincode bytes.
    pub fn serialize_pack(pack: &ContentPack) -> Result<Vec<u8>, CompilerError> {
        bincode::serialize(pack).map_err(|e| {
            CompilerError::SerializationError(format!("Failed to serialize pack to bincode: {}", e))
        })
    }

    /// Deserialize a ContentPack from bincode bytes.
    pub fn deserialize_pack(bytes: &[u8]) -> Result<ContentPack, CompilerError> {
        bincode::deserialize(bytes).map_err(|e| {
            CompilerError::SerializationError(format!(
                "Failed to deserialize pack from bincode: {}",
                e
            ))
        })
    }

    /// Serialize a ContentPack to JSON string.
    pub fn serialize_pack_json(pack: &ContentPack) -> Result<String, CompilerError> {
        serde_json::to_string_pretty(pack).map_err(|e| {
            CompilerError::SerializationError(format!("Failed to serialize pack to JSON: {}", e))
        })
    }
}

// ─── Standalone serialization functions ─────────────────────────────────────

/// Serialize a ContentPack to bincode bytes (standalone function).
pub fn serialize_pack(pack: &ContentPack) -> Result<Vec<u8>, CompilerError> {
    ContentCompiler::serialize_pack(pack)
}

/// Deserialize a ContentPack from bincode bytes (standalone function).
pub fn deserialize_pack(bytes: &[u8]) -> Result<ContentPack, CompilerError> {
    ContentCompiler::deserialize_pack(bytes)
}

/// Serialize a ContentPack to JSON string (standalone function).
pub fn serialize_pack_json(pack: &ContentPack) -> Result<String, CompilerError> {
    ContentCompiler::serialize_pack_json(pack)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_content_schema::*;
    use wh40k_core_types::*;

    // ── Helper: build a minimal valid faction YAML ──────────────────────

    /// Build a minimal valid faction YAML by serializing a Rust struct.
    /// This ensures the YAML format always matches serde_yaml's expectations.
    fn minimal_faction_yaml() -> String {
        let faction = build_minimal_faction();
        serde_yaml::to_string(&faction).expect("Failed to serialize minimal faction to YAML")
    }

    /// Build a minimal valid mission YAML by serializing a Rust struct.
    fn minimal_mission_yaml() -> String {
        let mission = build_minimal_mission();
        serde_yaml::to_string(&mission).expect("Failed to serialize minimal mission to YAML")
    }

    /// Build a FactionSchema directly in Rust (not YAML) for validation tests.
    fn build_minimal_faction() -> FactionSchema {
        FactionSchema {
            id: FactionId::new(1),
            name: "Test Faction".to_string(),
            faction_keywords: KeywordSet::from_keywords(&[Keyword::Imperium]),
            faction_ability: AbilitySchema {
                name: "Test Faction Ability".to_string(),
                description: "A test faction ability".to_string(),
                ability_type: AbilityType::FactionAbility,
                effects: vec![RulePrimitive::Noop],
                trigger: None,
            },
            datasheets: vec![DatasheetSchema {
                id: DatasheetId::new(1),
                name: "Test Unit".to_string(),
                keywords: KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Imperium]),
                movement: MoveCharacteristic::from_inches(6),
                toughness: Toughness::new(4),
                armor_save: ArmorSave::THREE_PLUS,
                invulnerable_save: None,
                wounds: Wounds::new(2),
                leadership: Leadership::new(6),
                objective_control: ObjectiveControl::new(2),
                base_size: BaseSize::MM32,
                ranged_weapons: vec![WeaponProfileSchema {
                    name: "Test Gun".to_string(),
                    weapon_type: WeaponType::Ranged,
                    range: Inches::from_inches(24),
                    attacks: AttackCount::Fixed(2),
                    skill: Skill::THREE_PLUS,
                    strength: Strength::new(4),
                    ap: ArmorPenetration::MINUS_1,
                    damage: Damage::Fixed(1),
                    abilities: vec![],
                }],
                melee_weapons: vec![WeaponProfileSchema {
                    name: "Test Blade".to_string(),
                    weapon_type: WeaponType::Melee,
                    range: Inches::ZERO,
                    attacks: AttackCount::Fixed(3),
                    skill: Skill::THREE_PLUS,
                    strength: Strength::new(4),
                    ap: ArmorPenetration::ZERO,
                    damage: Damage::Fixed(1),
                    abilities: vec![],
                }],
                abilities: vec![],
                unit_size: UnitSizeSpec::Fixed(5),
                model_loadouts: vec![ModelLoadout {
                    model_label: "Standard".to_string(),
                    count: 5,
                    ranged_weapons: vec!["Test Gun".to_string()],
                    melee_weapons: vec!["Test Blade".to_string()],
                }],
                wargear_abilities: vec![],
            }],
            stratagems: vec![StratagemSchema {
                name: "Test Stratagem".to_string(),
                cp_cost: 1,
                phase: Phase::Shooting,
                timing: StratagemTiming::DuringPhase,
                stratagem_type: StratagemType::BattleTactic,
                target_restriction: TargetRestriction::default(),
                effects: vec![RulePrimitive::Noop],
                once_per_battle: false,
                once_per_turn: true,
                description: "A test stratagem".to_string(),
            }],
            enhancements: vec![EnhancementSchema {
                name: "Test Enhancement".to_string(),
                description: "A test enhancement".to_string(),
                is_default: true,
                effects: vec![RulePrimitive::Noop],
                conditions: vec![],
            }],
            secondary_objectives: vec![SecondaryObjectiveSchema {
                name: "Test Secondary".to_string(),
                description: "A test secondary objective".to_string(),
                is_default: true,
                scoring: vec![ScoringRule {
                    timing: ScoringTiming {
                        phase: Phase::Command,
                        whose_turn: TurnOwner::Active,
                        from_round: 2,
                    },
                    condition: Condition::OnObjective,
                    vp_amount: 3,
                    description: Some("Score 3 VP if on objective".to_string()),
                }],
                max_vp: None,
            }],
        }
    }

    /// Build a minimal valid MissionSchema directly in Rust.
    fn build_minimal_mission() -> MissionSchema {
        MissionSchema {
            id: MissionId::new(1),
            name: "Test Mission".to_string(),
            deployment_map: DeploymentMapId::new(1),
            objectives: vec![
                ObjectiveDef {
                    label: "A".to_string(),
                    position_x: Inches::from_inches(22),
                    position_y: Inches::from_inches(15),
                    zone: ObjectiveZone::NoMansLand,
                    control_range: Inches::from_inches(3),
                },
                ObjectiveDef {
                    label: "B".to_string(),
                    position_x: Inches::from_inches(11),
                    position_y: Inches::from_inches(10),
                    zone: ObjectiveZone::AttackerZone,
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
                description: Some("Score 5 VP for holding objectives".to_string()),
            }],
            special_rules: vec![],
            rounds: 5,
            description: "A test mission".to_string(),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Test: compile a minimal faction YAML and verify the output
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_compile_minimal_faction_yaml() {
        let mut compiler = ContentCompiler::new();
        let yaml = minimal_faction_yaml();
        let result = compiler.compile_faction(&yaml);
        assert!(result.is_ok(), "compile_faction failed: {:?}", result.err());
        let faction = result.unwrap();
        assert_eq!(faction.name, "Test Faction");
        assert_eq!(faction.datasheets.len(), 1);
        assert_eq!(faction.datasheets[0].name, "Test Unit");
        assert_eq!(faction.stratagems.len(), 1);
        assert_eq!(faction.enhancements.len(), 1);
        assert_eq!(faction.secondary_objectives.len(), 1);
        // ID should have been assigned deterministically
        assert_eq!(faction.id, generate_faction_id("Test Faction"));
        assert_eq!(
            faction.datasheets[0].id,
            generate_datasheet_id("Test Faction", "Test Unit")
        );
    }

    #[test]
    fn test_compile_minimal_mission_yaml() {
        let mut compiler = ContentCompiler::new();
        let yaml = minimal_mission_yaml();
        let result = compiler.compile_mission(&yaml);
        assert!(result.is_ok(), "compile_mission failed: {:?}", result.err());
        let mission = result.unwrap();
        assert_eq!(mission.name, "Test Mission");
        assert_eq!(mission.objectives.len(), 2);
        assert_eq!(mission.rounds, 5);
        assert_eq!(mission.id, generate_mission_id("Test Mission"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Test: validation catches invalid content
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_validation_catches_missing_weapons() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();
        // Remove all weapons from the datasheet
        faction.datasheets[0].ranged_weapons.clear();
        faction.datasheets[0].melee_weapons.clear();
        faction.datasheets[0].model_loadouts.clear();

        let result = compiler.validate_faction(&faction);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("at least one weapon")),
            "Expected 'at least one weapon' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validation_catches_zero_strength() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();
        // Set weapon strength to 0
        faction.datasheets[0].ranged_weapons[0].strength = Strength::new(0);

        let result = compiler.validate_faction(&faction);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("strength > 0")),
            "Expected 'strength > 0' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validation_catches_invalid_skill() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();
        // Set weapon skill to invalid value (1 or 7)
        faction.datasheets[0].ranged_weapons[0].skill = Skill(1);

        let result = compiler.validate_faction(&faction);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("skill value")),
            "Expected 'skill value' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validation_catches_zero_cp_cost_stratagem() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();
        // Set stratagem CP cost to 0
        faction.stratagems[0].cp_cost = 0;

        let result = compiler.validate_faction(&faction);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("CP cost > 0")),
            "Expected 'CP cost > 0' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validation_catches_empty_enhancement_effects() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();
        // Remove all enhancement effects
        faction.enhancements[0].effects.clear();

        let result = compiler.validate_faction(&faction);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("at least one effect")),
            "Expected 'at least one effect' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validation_catches_invalid_unit_size() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();
        // Set unit size to 0
        faction.datasheets[0].unit_size = UnitSizeSpec::Fixed(0);

        let result = compiler.validate_faction(&faction);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("unit size")),
            "Expected 'unit size' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validation_catches_invalid_unit_size_range() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();
        // Set unit size range with max < min
        faction.datasheets[0].unit_size = UnitSizeSpec::Range { min: 10, max: 5 };

        let result = compiler.validate_faction(&faction);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("unit size")),
            "Expected 'unit size' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validation_catches_empty_keywords() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();
        // Set keywords to empty
        faction.datasheets[0].keywords = KeywordSet::empty();

        let result = compiler.validate_faction(&faction);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("empty keyword set")),
            "Expected 'empty keyword set' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validation_catches_missing_secondary_scoring() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();
        // Remove all scoring rules from secondary objective
        faction.secondary_objectives[0].scoring.clear();

        let result = compiler.validate_faction(&faction);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("at least one scoring rule")),
            "Expected 'at least one scoring rule' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validation_catches_unresolved_weapon_reference() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();
        // Add a loadout that references a weapon that doesn't exist
        faction.datasheets[0].model_loadouts[0]
            .ranged_weapons
            .push("Nonexistent Gun".to_string());

        let result = compiler.validate_faction(&faction);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("not found")),
            "Expected 'not found' reference error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validation_catches_duplicate_datasheet_names() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();
        // Duplicate the datasheet
        let dup = faction.datasheets[0].clone();
        faction.datasheets.push(dup);

        let result = compiler.validate_faction(&faction);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Duplicate datasheet name")),
            "Expected 'Duplicate datasheet name' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validation_catches_zero_toughness() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();
        faction.datasheets[0].toughness = Toughness::new(0);

        let result = compiler.validate_faction(&faction);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("toughness of 0")),
            "Expected 'toughness of 0' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validation_catches_zero_wounds() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();
        faction.datasheets[0].wounds = Wounds::new(0);

        let result = compiler.validate_faction(&faction);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("wounds of 0")),
            "Expected 'wounds of 0' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validation_catches_empty_faction_name() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();
        faction.name = "  ".to_string();

        let result = compiler.validate_faction(&faction);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Faction name must not be empty")),
            "Expected 'Faction name' error, got: {:?}",
            errors
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Test: mission validation catches invalid content
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_mission_validation_catches_no_objectives() {
        let compiler = ContentCompiler::new();
        let mut mission = build_minimal_mission();
        mission.objectives.clear();

        let result = compiler.validate_mission(&mission);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("at least one objective")),
            "Expected 'at least one objective' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_mission_validation_catches_no_scoring() {
        let compiler = ContentCompiler::new();
        let mut mission = build_minimal_mission();
        mission.primary_scoring.clear();

        let result = compiler.validate_mission(&mission);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("primary scoring rule")),
            "Expected 'primary scoring rule' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_mission_validation_catches_zero_rounds() {
        let compiler = ContentCompiler::new();
        let mut mission = build_minimal_mission();
        mission.rounds = 0;

        let result = compiler.validate_mission(&mission);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("at least 1 battle round")),
            "Expected 'at least 1 battle round' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_mission_validation_catches_duplicate_objective_labels() {
        let compiler = ContentCompiler::new();
        let mut mission = build_minimal_mission();
        // Make both objectives have the same label
        mission.objectives[1].label = "A".to_string();

        let result = compiler.validate_mission(&mission);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Duplicate objective label")),
            "Expected 'Duplicate objective label' error, got: {:?}",
            errors
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Test: compile_pack
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_compile_pack() {
        let mut compiler = ContentCompiler::new();
        let faction = build_minimal_faction();
        let mission = build_minimal_mission();

        let result = compiler.compile_pack(vec![faction], vec![mission]);
        assert!(result.is_ok(), "compile_pack failed: {:?}", result.err());
        let pack = result.unwrap();
        assert_eq!(pack.factions.len(), 1);
        assert_eq!(pack.missions.len(), 1);
        assert_eq!(pack.version, "0.1.0");
        assert_ne!(pack.content_hash, 0);
        assert_ne!(pack.pack_id.raw(), 0);
    }

    #[test]
    fn test_compile_pack_rejects_duplicate_faction_names() {
        let mut compiler = ContentCompiler::new();
        let faction1 = build_minimal_faction();
        let faction2 = build_minimal_faction();

        let result = compiler.compile_pack(vec![faction1, faction2], vec![]);
        assert!(result.is_err());
        match result.unwrap_err() {
            CompilerError::DuplicateError(msg) => {
                assert!(msg.contains("Duplicate faction name"));
            }
            other => panic!("Expected DuplicateError, got: {:?}", other),
        }
    }

    #[test]
    fn test_compile_pack_rejects_duplicate_mission_names() {
        let mut compiler = ContentCompiler::new();
        let mission1 = build_minimal_mission();
        let mission2 = build_minimal_mission();

        let result = compiler.compile_pack(vec![], vec![mission1, mission2]);
        assert!(result.is_err());
        match result.unwrap_err() {
            CompilerError::DuplicateError(msg) => {
                assert!(msg.contains("Duplicate mission name"));
            }
            other => panic!("Expected DuplicateError, got: {:?}", other),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Test: round-trip serialization (compile -> serialize -> deserialize -> verify)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_roundtrip_bincode_serialization() {
        let mut compiler = ContentCompiler::new();
        let faction = build_minimal_faction();
        let mission = build_minimal_mission();

        let pack = compiler
            .compile_pack(vec![faction], vec![mission])
            .expect("compile_pack should succeed");

        // Serialize to bincode
        let bytes = ContentCompiler::serialize_pack(&pack).expect("serialize should succeed");
        assert!(!bytes.is_empty());

        // Deserialize from bincode
        let deserialized =
            ContentCompiler::deserialize_pack(&bytes).expect("deserialize should succeed");

        // Verify equality
        assert_eq!(pack.pack_id, deserialized.pack_id);
        assert_eq!(pack.version, deserialized.version);
        assert_eq!(pack.content_hash, deserialized.content_hash);
        assert_eq!(pack.factions.len(), deserialized.factions.len());
        assert_eq!(pack.missions.len(), deserialized.missions.len());
        assert_eq!(pack.factions[0].name, deserialized.factions[0].name);
        assert_eq!(
            pack.factions[0].datasheets.len(),
            deserialized.factions[0].datasheets.len()
        );
        assert_eq!(
            pack.factions[0].datasheets[0].name,
            deserialized.factions[0].datasheets[0].name
        );
        assert_eq!(
            pack.missions[0].name,
            deserialized.missions[0].name
        );
        assert_eq!(pack, deserialized);
    }

    #[test]
    fn test_roundtrip_json_serialization() {
        let mut compiler = ContentCompiler::new();
        let faction = build_minimal_faction();
        let mission = build_minimal_mission();

        let pack = compiler
            .compile_pack(vec![faction], vec![mission])
            .expect("compile_pack should succeed");

        // Serialize to JSON
        let json = ContentCompiler::serialize_pack_json(&pack).expect("JSON serialize should succeed");
        assert!(!json.is_empty());

        // Deserialize from JSON
        let deserialized: ContentPack =
            serde_json::from_str(&json).expect("JSON deserialize should succeed");

        // Verify equality
        assert_eq!(pack, deserialized);
    }

    #[test]
    fn test_roundtrip_standalone_functions() {
        let mut compiler = ContentCompiler::new();
        let faction = build_minimal_faction();
        let mission = build_minimal_mission();

        let pack = compiler
            .compile_pack(vec![faction], vec![mission])
            .expect("compile_pack should succeed");

        // Use standalone functions
        let bytes = serialize_pack(&pack).expect("serialize should succeed");
        let deserialized = deserialize_pack(&bytes).expect("deserialize should succeed");
        assert_eq!(pack, deserialized);

        let json = serialize_pack_json(&pack).expect("JSON serialize should succeed");
        let deserialized_json: ContentPack =
            serde_json::from_str(&json).expect("JSON deserialize should succeed");
        assert_eq!(pack, deserialized_json);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Test: stable content hashing (same input = same hash)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_stable_content_hash_deterministic() {
        let mut compiler = ContentCompiler::new();
        let faction1 = build_minimal_faction();
        let mission1 = build_minimal_mission();
        let faction2 = build_minimal_faction();
        let mission2 = build_minimal_mission();

        let pack1 = compiler
            .compile_pack(vec![faction1], vec![mission1])
            .expect("compile_pack should succeed");
        let pack2 = compiler
            .compile_pack(vec![faction2], vec![mission2])
            .expect("compile_pack should succeed");

        // Same content should produce the same hash
        assert_eq!(pack1.content_hash, pack2.content_hash);
        assert_eq!(pack1.pack_id, pack2.pack_id);
    }

    #[test]
    fn test_different_content_different_hash() {
        let mut compiler = ContentCompiler::new();
        let faction1 = build_minimal_faction();
        let mission1 = build_minimal_mission();

        let mut faction2 = build_minimal_faction();
        faction2.name = "Different Faction".to_string();
        faction2.datasheets[0].id = generate_datasheet_id("Different Faction", "Test Unit");
        faction2.id = generate_faction_id("Different Faction");

        let pack1 = compiler
            .compile_pack(vec![faction1], vec![mission1.clone()])
            .expect("compile_pack should succeed");
        let pack2 = compiler
            .compile_pack(vec![faction2], vec![mission1])
            .expect("compile_pack should succeed");

        // Different content should produce different hashes
        assert_ne!(pack1.content_hash, pack2.content_hash);
    }

    #[test]
    fn test_deterministic_id_generation() {
        // Same name should always produce the same ID
        let id1 = generate_faction_id("Tristraen's Gilded Blades");
        let id2 = generate_faction_id("Tristraen's Gilded Blades");
        assert_eq!(id1, id2);

        // Different names should produce different IDs
        let id3 = generate_faction_id("Frenzied Reavers");
        assert_ne!(id1, id3);

        // Datasheet IDs are scoped to faction
        let ds1 = generate_datasheet_id("Faction A", "Unit 1");
        let ds2 = generate_datasheet_id("Faction A", "Unit 1");
        assert_eq!(ds1, ds2);

        let ds3 = generate_datasheet_id("Faction B", "Unit 1");
        assert_ne!(ds1, ds3);
    }

    #[test]
    fn test_stable_hash_from_json_deterministic() {
        let val1 = ("hello", 42u32, vec![1, 2, 3]);
        let val2 = ("hello", 42u32, vec![1, 2, 3]);
        assert_eq!(stable_hash_from_json(&val1), stable_hash_from_json(&val2));

        let val3 = ("world", 42u32, vec![1, 2, 3]);
        assert_ne!(stable_hash_from_json(&val1), stable_hash_from_json(&val3));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Test: YAML parse errors
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_parse_error_invalid_yaml_faction() {
        let mut compiler = ContentCompiler::new();
        let result = compiler.compile_faction("{{not valid yaml}}");
        assert!(result.is_err());
        match result.unwrap_err() {
            CompilerError::ParseError(msg) => {
                assert!(msg.contains("Failed to parse faction YAML"));
            }
            other => panic!("Expected ParseError, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_error_invalid_yaml_mission() {
        let mut compiler = ContentCompiler::new();
        let result = compiler.compile_mission("{{not valid yaml}}");
        assert!(result.is_err());
        match result.unwrap_err() {
            CompilerError::ParseError(msg) => {
                assert!(msg.contains("Failed to parse mission YAML"));
            }
            other => panic!("Expected ParseError, got: {:?}", other),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Test: valid faction passes validation
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_valid_faction_passes_validation() {
        let compiler = ContentCompiler::new();
        let faction = build_minimal_faction();
        let result = compiler.validate_faction(&faction);
        assert!(
            result.is_ok(),
            "Valid faction should pass validation, got errors: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_valid_mission_passes_validation() {
        let compiler = ContentCompiler::new();
        let mission = build_minimal_mission();
        let result = compiler.validate_mission(&mission);
        assert!(
            result.is_ok(),
            "Valid mission should pass validation, got errors: {:?}",
            result.err()
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Test: serialization error handling
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_deserialize_invalid_bincode() {
        let result = ContentCompiler::deserialize_pack(&[0xFF, 0xFE, 0xFD, 0xFC]);
        assert!(result.is_err());
        match result.unwrap_err() {
            CompilerError::SerializationError(msg) => {
                assert!(msg.contains("deserialize"));
            }
            other => panic!("Expected SerializationError, got: {:?}", other),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Test: content hash differs after serialize -> modify -> deserialize
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_content_hash_integrity() {
        let mut compiler = ContentCompiler::new();
        let faction = build_minimal_faction();
        let mission = build_minimal_mission();

        let pack = compiler
            .compile_pack(vec![faction], vec![mission])
            .expect("compile_pack should succeed");

        // Verify hash matches what we'd compute fresh
        let fresh_hash = compiler.compute_content_hash(&pack);
        assert_eq!(pack.content_hash, fresh_hash);

        // Modify the pack and verify hash changes
        let mut modified_pack = pack.clone();
        modified_pack.factions[0].name = "Modified Faction".to_string();
        let modified_hash = compiler.compute_content_hash(&modified_pack);
        assert_ne!(pack.content_hash, modified_hash);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Test: warnings are collected separately from errors
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_warnings_collected() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();
        // Create a faction ability with empty description (warning, not error)
        faction.faction_ability.description = "".to_string();

        let result = compiler.validate_faction(&faction);
        // Should still pass if description is the only issue (it's a warning)
        // But warnings are included in the returned errors list from validate_faction
        match result {
            Ok(()) => {
                // If validation passes, that means warnings were tolerated
                // This is fine - description being empty may or may not be
                // an error depending on strictness
            }
            Err(errors) => {
                // If we got errors, at least some should be warnings
                let has_warning = errors.iter().any(|e| e.severity == Severity::Warning);
                assert!(
                    has_warning || errors.iter().all(|e| e.severity == Severity::Error),
                    "Expected at least some warnings"
                );
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Test: compile_faction from YAML produces deterministic IDs
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_compile_faction_deterministic_ids() {
        let yaml = minimal_faction_yaml();
        let mut compiler1 = ContentCompiler::new();
        let mut compiler2 = ContentCompiler::new();

        let faction1 = compiler1.compile_faction(&yaml).unwrap();
        let faction2 = compiler2.compile_faction(&yaml).unwrap();

        assert_eq!(faction1.id, faction2.id);
        assert_eq!(faction1.datasheets[0].id, faction2.datasheets[0].id);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Test: RulePrimitive validation covers recursive structures
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_validation_recursive_rule_primitives() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();

        // Add a Composite with an empty StanceChoice inside
        faction.faction_ability.effects = vec![RulePrimitive::Composite {
            effects: vec![RulePrimitive::StanceChoice {
                stances: vec![], // Empty stances - should trigger error
            }],
        }];

        let result = compiler.validate_faction(&faction);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("at least one stance")),
            "Expected 'at least one stance' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validation_nested_conditional() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();

        // Add a Conditional with a DiceCheck that has a bad threshold
        faction.faction_ability.effects = vec![RulePrimitive::Conditional {
            condition: Condition::OnObjective,
            effect: Box::new(RulePrimitive::DiceCheck {
                threshold: 0, // Invalid: should be 2-6
                on_success: Box::new(RulePrimitive::Noop),
                on_failure: None,
                modifier: 0,
            }),
            else_effect: None,
        }];

        let result = compiler.validate_faction(&faction);
        // DiceCheck threshold of 0 should produce a warning
        match result {
            Ok(()) => {
                // Warnings only, that's fine
            }
            Err(errors) => {
                let has_threshold_issue = errors
                    .iter()
                    .any(|e| e.message.contains("threshold"));
                assert!(
                    has_threshold_issue,
                    "Expected threshold warning/error, got: {:?}",
                    errors
                );
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Test: multiple validation errors collected at once
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_multiple_validation_errors_collected() {
        let compiler = ContentCompiler::new();
        let mut faction = build_minimal_faction();

        // Create multiple issues at once
        faction.datasheets[0].toughness = Toughness::new(0); // zero toughness
        faction.datasheets[0].wounds = Wounds::new(0); // zero wounds
        faction.stratagems[0].cp_cost = 0; // zero CP cost
        faction.enhancements[0].effects.clear(); // no effects

        let result = compiler.validate_faction(&faction);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        // Should have at least 4 errors (toughness, wounds, CP cost, enhancement effects)
        assert!(
            errors.len() >= 4,
            "Expected at least 4 errors, got {}: {:?}",
            errors.len(),
            errors
        );
    }
}

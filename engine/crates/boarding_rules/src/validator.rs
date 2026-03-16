use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wh40k_boarding_content::faction_data::{BoardingFactionDef, BoardingDetachmentDef, BoardingUnitDatasheet};

use crate::roster::BoardingPatrol;

/// Universal Boarding Actions enhancements available to all detachments.
pub const UNIVERSAL_BA_ENHANCEMENTS: &[&str] = &[
    "Superior Boarding Tactics",
    "Close-Quarters Killer",
    "Peerless Leader",
    "Expert Breacher",
    "Personal Teleporter",
    "Trademark Weapon",
];

/// Maximum points allowed for a Boarding Patrol.
pub const MAX_BOARDING_PATROL_POINTS: u16 = 500;

/// Result of validating a Boarding Patrol roster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RosterValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

/// A validation error that makes the roster illegal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
}

/// A validation warning that does not make the roster illegal but indicates a potential issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub code: String,
    pub message: String,
}

impl RosterValidationResult {
    fn new() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn add_error(&mut self, code: &str, message: String) {
        self.valid = false;
        self.errors.push(ValidationError {
            code: code.to_string(),
            message,
        });
    }

    fn add_warning(&mut self, code: &str, message: String) {
        self.warnings.push(ValidationWarning {
            code: code.to_string(),
            message,
        });
    }
}

/// Validates a `BoardingPatrol` roster against a `BoardingFactionDef`.
pub struct BoardingRosterValidator<'a> {
    patrol: &'a BoardingPatrol,
    faction: &'a BoardingFactionDef,
}

impl<'a> BoardingRosterValidator<'a> {
    /// Create a new validator for the given patrol and faction definition.
    pub fn new(patrol: &'a BoardingPatrol, faction: &'a BoardingFactionDef) -> Self {
        Self { patrol, faction }
    }

    /// Run all validation checks and return the result.
    pub fn validate(&self) -> RosterValidationResult {
        let mut result = RosterValidationResult::new();

        // Rule 1: Points cap
        self.validate_points_cap(&mut result);

        // Rule 2: Faction match
        self.validate_faction_match(&mut result);

        // Rule 3: Detachment match
        let detachment = self.find_detachment();
        if detachment.is_none() {
            result.add_error(
                "INVALID_DETACHMENT",
                format!(
                    "Detachment '{}' not found in faction '{}'",
                    self.patrol.detachment_name, self.faction.faction_name
                ),
            );
            // Cannot continue validation without a valid detachment
            return result;
        }
        let detachment = detachment.unwrap();

        // Rule 4: Unit legality
        self.validate_unit_legality(&mut result, detachment);

        // Rule 5: Unit count limits
        self.validate_unit_count_limits(&mut result, detachment);

        // Rule 6: No duplicate EPIC HEROES
        self.validate_no_duplicate_epic_heroes(&mut result);

        // Rule 7: Model count validity
        self.validate_model_counts(&mut result, detachment);

        // Rule 8: Points cost accuracy
        self.validate_points_accuracy(&mut result, detachment);

        // Rule 9: Warlord requirements
        self.validate_warlord(&mut result);

        // Rule 10: Enhancement rules
        self.validate_enhancements(&mut result, detachment);

        // Rule 11: Deployment minimum (warning only)
        self.validate_deployment_minimum(&mut result);

        // Rule 12: Conditional limits (detachment-specific mustering rules)
        self.validate_conditional_limits(&mut result, detachment);

        result
    }

    /// Find the detachment definition matching the patrol's detachment_name.
    fn find_detachment(&self) -> Option<&'a BoardingDetachmentDef> {
        self.faction
            .detachments
            .iter()
            .find(|d| d.detachment_name == self.patrol.detachment_name)
    }

    /// Find a datasheet by name in the faction's datasheets.
    fn find_datasheet(&self, name: &str) -> Option<&'a BoardingUnitDatasheet> {
        self.faction.datasheets.iter().find(|d| d.name == name)
    }

    /// Rule 1: Total points must be <= 500.
    fn validate_points_cap(&self, result: &mut RosterValidationResult) {
        if self.patrol.total_points > MAX_BOARDING_PATROL_POINTS {
            result.add_error(
                "OVER_POINTS",
                format!(
                    "Roster total {} points exceeds the {} point limit",
                    self.patrol.total_points, MAX_BOARDING_PATROL_POINTS
                ),
            );
        }
    }

    /// Rule 2: Faction keyword must match.
    fn validate_faction_match(&self, result: &mut RosterValidationResult) {
        if self.patrol.faction_keyword != self.faction.faction_keyword {
            result.add_error(
                "FACTION_MISMATCH",
                format!(
                    "Patrol faction keyword '{}' does not match faction definition '{}'",
                    self.patrol.faction_keyword, self.faction.faction_keyword
                ),
            );
        }
    }

    /// Rule 4: Every unit's datasheet_name must be in the detachment's allowed_units list.
    fn validate_unit_legality(
        &self,
        result: &mut RosterValidationResult,
        detachment: &BoardingDetachmentDef,
    ) {
        let allowed_names: Vec<&str> = detachment
            .allowed_units
            .iter()
            .map(|u| u.datasheet_name.as_str())
            .collect();

        for (i, unit) in self.patrol.units.iter().enumerate() {
            if !allowed_names.contains(&unit.datasheet_name.as_str()) {
                result.add_error(
                    "INVALID_UNIT",
                    format!(
                        "Unit {} ('{}') is not in the allowed units list for detachment '{}'",
                        i, unit.datasheet_name, detachment.detachment_name
                    ),
                );
            }
        }
    }

    /// Rule 5: Each unit type must not exceed the max_count in allowed_units.
    fn validate_unit_count_limits(
        &self,
        result: &mut RosterValidationResult,
        detachment: &BoardingDetachmentDef,
    ) {
        // Count occurrences of each datasheet_name
        let mut unit_counts: HashMap<&str, u8> = HashMap::new();
        for unit in &self.patrol.units {
            *unit_counts.entry(unit.datasheet_name.as_str()).or_insert(0) += 1;
        }

        for (name, count) in &unit_counts {
            if let Some(allowed) = detachment
                .allowed_units
                .iter()
                .find(|u| u.datasheet_name == *name)
            {
                if let Some(max) = allowed.max_count {
                    if *count > max {
                        result.add_error(
                            "UNIT_COUNT_EXCEEDED",
                            format!(
                                "Unit '{}' appears {} times but max_count is {}",
                                name, count, max
                            ),
                        );
                    }
                }
            }
        }

        // Also check for shared character pool limits
        self.validate_character_pool(result, detachment);
    }

    /// Validate shared character pool limits from mustering rules.
    /// Looks for conditional_limit strings containing "Shared pool:" to enforce
    /// total character count limits.
    fn validate_character_pool(
        &self,
        result: &mut RosterValidationResult,
        detachment: &BoardingDetachmentDef,
    ) {
        // Collect all allowed_units that share a pool
        // The pattern is "Shared pool: up to N total characters"
        let mut pool_groups: HashMap<String, Vec<&str>> = HashMap::new();

        for unit_ref in &detachment.allowed_units {
            if let Some(ref cond) = unit_ref.conditional_limit {
                let cond_lower = cond.to_lowercase();
                if cond_lower.contains("shared pool:") {
                    pool_groups
                        .entry(cond.clone())
                        .or_default()
                        .push(&unit_ref.datasheet_name);
                }
            }
        }

        for (pool_desc, pool_members) in &pool_groups {
            // Parse "up to N total characters" from the description
            let pool_desc_lower = pool_desc.to_lowercase();
            if let Some(limit) = parse_pool_limit(&pool_desc_lower) {
                let pool_count: u8 = self
                    .patrol
                    .units
                    .iter()
                    .filter(|u| pool_members.contains(&u.datasheet_name.as_str()))
                    .count() as u8;

                if pool_count > limit {
                    result.add_error(
                        "CHARACTER_POOL_EXCEEDED",
                        format!(
                            "Character pool limit exceeded: {} characters selected from shared pool (max {}). Pool: {}",
                            pool_count, limit, pool_desc
                        ),
                    );
                }
            }
        }
    }

    /// Rule 6: Cannot include the same EPIC HERO more than once.
    fn validate_no_duplicate_epic_heroes(&self, result: &mut RosterValidationResult) {
        let mut epic_hero_names: Vec<&str> = Vec::new();

        for unit in &self.patrol.units {
            if let Some(datasheet) = self.find_datasheet(&unit.datasheet_name) {
                if datasheet.is_epic_hero {
                    if epic_hero_names.contains(&unit.datasheet_name.as_str()) {
                        result.add_error(
                            "DUPLICATE_EPIC_HERO",
                            format!(
                                "EPIC HERO '{}' is included more than once",
                                unit.datasheet_name
                            ),
                        );
                    } else {
                        epic_hero_names.push(&unit.datasheet_name);
                    }
                }
            }
        }
    }

    /// Rule 7: Each unit's model_count must be valid for the detachment's allowed_sizes.
    /// If allowed_sizes is empty, use the standard sizes from the datasheet's points entries.
    fn validate_model_counts(
        &self,
        result: &mut RosterValidationResult,
        detachment: &BoardingDetachmentDef,
    ) {
        for (i, unit) in self.patrol.units.iter().enumerate() {
            let allowed_ref = detachment
                .allowed_units
                .iter()
                .find(|u| u.datasheet_name == unit.datasheet_name);

            if let Some(unit_ref) = allowed_ref {
                let valid_sizes: Vec<u8> = if !unit_ref.allowed_sizes.is_empty() {
                    unit_ref.allowed_sizes.clone()
                } else if let Some(datasheet) = self.find_datasheet(&unit.datasheet_name) {
                    // Use standard sizes from the datasheet's points entries
                    datasheet.points.iter().map(|p| p.model_count).collect()
                } else {
                    // Can't determine valid sizes, skip
                    continue;
                };

                if !valid_sizes.contains(&unit.model_count) {
                    result.add_error(
                        "INVALID_MODEL_COUNT",
                        format!(
                            "Unit {} ('{}') has {} models but valid sizes are {:?}",
                            i, unit.datasheet_name, unit.model_count, valid_sizes
                        ),
                    );
                }
            }
            // If unit_ref is None, INVALID_UNIT error was already issued in validate_unit_legality
        }
    }

    /// Rule 8: Each unit's points cost must match the datasheet's points for the given model_count.
    /// If the detachment allows a non-standard size (not in the datasheet's points entries),
    /// use the half-points rule: cost = ceil(points_for_double_size / 2).
    fn validate_points_accuracy(
        &self,
        result: &mut RosterValidationResult,
        detachment: &BoardingDetachmentDef,
    ) {
        for (i, unit) in self.patrol.units.iter().enumerate() {
            if let Some(datasheet) = self.find_datasheet(&unit.datasheet_name) {
                let expected_points = self.calculate_expected_points(
                    datasheet,
                    unit.model_count,
                    detachment,
                    &unit.datasheet_name,
                );

                if let Some(expected) = expected_points {
                    if unit.points != expected {
                        result.add_error(
                            "INCORRECT_POINTS",
                            format!(
                                "Unit {} ('{}') with {} models costs {} points but should cost {} points",
                                i, unit.datasheet_name, unit.model_count, unit.points, expected
                            ),
                        );
                    }
                }
                // If expected_points is None, there's no matching entry in the datasheet
                // and the model count is invalid (already caught by validate_model_counts)
            }
        }
    }

    /// Calculate the expected points for a unit with the given model count.
    /// First checks the datasheet's standard points entries.
    /// If not found, checks if the detachment allows a non-standard size and
    /// applies the half-points rule.
    fn calculate_expected_points(
        &self,
        datasheet: &BoardingUnitDatasheet,
        model_count: u8,
        detachment: &BoardingDetachmentDef,
        datasheet_name: &str,
    ) -> Option<u16> {
        // First: check the datasheet's standard points entries for a direct match
        if let Some(entry) = datasheet.points.iter().find(|p| p.model_count == model_count) {
            return Some(entry.points);
        }

        // Second: check if this is a detachment-allowed non-standard size
        // Apply the half-points rule: look for an entry at double the model count
        let allowed_ref = detachment
            .allowed_units
            .iter()
            .find(|u| u.datasheet_name == datasheet_name);

        if let Some(unit_ref) = allowed_ref {
            if unit_ref.allowed_sizes.contains(&model_count) {
                // This size is explicitly allowed by the detachment but not in the datasheet
                let double_size = model_count * 2;
                if let Some(double_entry) =
                    datasheet.points.iter().find(|p| p.model_count == double_size)
                {
                    // Half-points rule: cost = ceil(points_for_double_size / 2)
                    return Some(double_entry.points.div_ceil(2));
                }
            }
        }

        None
    }

    /// Rule 9: Warlord requirements.
    fn validate_warlord(&self, result: &mut RosterValidationResult) {
        if self.patrol.units.is_empty() {
            result.add_error(
                "NO_UNITS",
                "Roster has no units".to_string(),
            );
            return;
        }

        match self.patrol.warlord_index {
            None => {
                result.add_error(
                    "NO_WARLORD",
                    "A Warlord must be designated (warlord_index is None)".to_string(),
                );
            }
            Some(wi) => {
                if wi >= self.patrol.units.len() {
                    result.add_error(
                        "INVALID_WARLORD_INDEX",
                        format!(
                            "warlord_index {} is out of bounds (roster has {} units)",
                            wi,
                            self.patrol.units.len()
                        ),
                    );
                    return;
                }

                // If any CHARACTER unit exists, warlord must be a CHARACTER
                let has_any_character = self.patrol.units.iter().any(|u| {
                    self.find_datasheet(&u.datasheet_name)
                        .map(|d| d.is_character)
                        .unwrap_or(false)
                });

                if has_any_character {
                    let warlord_unit = &self.patrol.units[wi];
                    let warlord_is_character = self
                        .find_datasheet(&warlord_unit.datasheet_name)
                        .map(|d| d.is_character)
                        .unwrap_or(false);

                    if !warlord_is_character {
                        result.add_error(
                            "WARLORD_NOT_CHARACTER",
                            format!(
                                "Warlord unit '{}' is not a CHARACTER, but the roster contains CHARACTER units. The Warlord must be a CHARACTER.",
                                warlord_unit.datasheet_name
                            ),
                        );
                    }
                }
            }
        }
    }

    /// Rule 10: Enhancement rules.
    fn validate_enhancements(
        &self,
        result: &mut RosterValidationResult,
        detachment: &BoardingDetachmentDef,
    ) {
        // At most 2 enhancements total
        if self.patrol.enhancements.len() > 2 {
            result.add_error(
                "TOO_MANY_ENHANCEMENTS",
                format!(
                    "{} enhancements assigned but maximum is 2",
                    self.patrol.enhancements.len()
                ),
            );
        }

        // No duplicate enhancement names
        let mut seen_names: Vec<&str> = Vec::new();
        for enh in &self.patrol.enhancements {
            if seen_names.contains(&enh.enhancement_name.as_str()) {
                result.add_error(
                    "DUPLICATE_ENHANCEMENT",
                    format!(
                        "Enhancement '{}' is assigned more than once",
                        enh.enhancement_name
                    ),
                );
            } else {
                seen_names.push(&enh.enhancement_name);
            }
        }

        // Validate each enhancement assignment
        for enh in &self.patrol.enhancements {
            // Unit index must be valid
            if enh.unit_index >= self.patrol.units.len() {
                result.add_error(
                    "INVALID_ENHANCEMENT_TARGET",
                    format!(
                        "Enhancement '{}' targets unit index {} which is out of bounds",
                        enh.enhancement_name, enh.unit_index
                    ),
                );
                continue;
            }

            let target_unit = &self.patrol.units[enh.unit_index];
            if let Some(datasheet) = self.find_datasheet(&target_unit.datasheet_name) {
                // Must be assigned to a CHARACTER unit
                if !datasheet.is_character {
                    result.add_error(
                        "ENHANCEMENT_NOT_ON_CHARACTER",
                        format!(
                            "Enhancement '{}' is assigned to '{}' which is not a CHARACTER",
                            enh.enhancement_name, target_unit.datasheet_name
                        ),
                    );
                }

                // EPIC HEROES cannot receive enhancements
                if datasheet.is_epic_hero {
                    result.add_error(
                        "ENHANCEMENT_ON_EPIC_HERO",
                        format!(
                            "Enhancement '{}' is assigned to EPIC HERO '{}' which cannot receive enhancements",
                            enh.enhancement_name, target_unit.datasheet_name
                        ),
                    );
                }
            }

            // Enhancement name must exist in detachment enhancements or universal BA enhancements
            let in_detachment = detachment
                .enhancements
                .iter()
                .any(|e| e.name == enh.enhancement_name);
            let in_universal = UNIVERSAL_BA_ENHANCEMENTS.contains(&enh.enhancement_name.as_str());

            if !in_detachment && !in_universal {
                result.add_error(
                    "INVALID_ENHANCEMENT",
                    format!(
                        "Enhancement '{}' is not available in detachment '{}' or in the universal BA enhancements list",
                        enh.enhancement_name, detachment.detachment_name
                    ),
                );
            }
        }

        // Check that no single unit has more than one enhancement
        let mut units_with_enhancements: HashMap<usize, u8> = HashMap::new();
        for enh in &self.patrol.enhancements {
            *units_with_enhancements.entry(enh.unit_index).or_insert(0) += 1;
        }
        for (unit_idx, count) in &units_with_enhancements {
            if *count > 1 && *unit_idx < self.patrol.units.len() {
                result.add_error(
                    "MULTIPLE_ENHANCEMENTS_ON_UNIT",
                    format!(
                        "Unit {} ('{}') has {} enhancements but each CHARACTER may only take one",
                        unit_idx, self.patrol.units[*unit_idx].datasheet_name, count
                    ),
                );
            }
        }
    }

    /// Rule 11: At least half the units (rounded up) must be deployed on the battlefield.
    /// This is a warning, not a hard error, since reserve decisions happen later.
    fn validate_deployment_minimum(&self, result: &mut RosterValidationResult) {
        if self.patrol.units.is_empty() {
            return;
        }

        let total_units = self.patrol.units.len();
        let min_deployed = total_units.div_ceil(2);

        result.add_warning(
            "DEPLOYMENT_REMINDER",
            format!(
                "Roster has {} units. At least {} must be deployed on the battlefield (not held in reserves).",
                total_units, min_deployed
            ),
        );
    }

    /// Rule 12: Conditional limits from detachment-specific mustering rules.
    fn validate_conditional_limits(
        &self,
        result: &mut RosterValidationResult,
        detachment: &BoardingDetachmentDef,
    ) {
        // Check conditional_limit strings on allowed_units
        for unit_ref in &detachment.allowed_units {
            if let Some(ref cond) = unit_ref.conditional_limit {
                let cond_lower = cond.to_lowercase();

                // Skip shared pool constraints (already handled by validate_character_pool)
                if cond_lower.contains("shared pool:") {
                    continue;
                }

                // Handle "Combined total of X units cannot exceed the number of Y units"
                // e.g., "Combined total of Jakhals units cannot exceed the number of Khorne Berzerkers units"
                if cond_lower.contains("cannot exceed the number of") {
                    self.validate_cannot_exceed_rule(result, &unit_ref.datasheet_name, cond);
                }
            }
        }
    }

    /// Validate a "cannot exceed" conditional limit.
    /// Parses the pattern: "Combined total of X units cannot exceed the number of Y units"
    fn validate_cannot_exceed_rule(
        &self,
        result: &mut RosterValidationResult,
        limited_unit_name: &str,
        condition: &str,
    ) {
        // Count the limited unit (e.g., "Jakhals")
        let limited_count = self
            .patrol
            .units
            .iter()
            .filter(|u| u.datasheet_name == limited_unit_name)
            .count();

        // Parse the target unit name from the condition
        // Pattern: "... cannot exceed the number of <TARGET_NAME> units"
        let cond_lower = condition.to_lowercase();
        if let Some(idx) = cond_lower.find("cannot exceed the number of ") {
            let after = &condition[idx + "cannot exceed the number of ".len()..];
            // Remove trailing " units" or similar
            let target_name = after
                .trim()
                .trim_end_matches(" units")
                .trim_end_matches(" unit")
                .trim();

            // Count the target unit in the roster (case-insensitive match)
            let target_count = self
                .patrol
                .units
                .iter()
                .filter(|u| u.datasheet_name.to_lowercase() == target_name.to_lowercase())
                .count();

            if limited_count > target_count {
                result.add_error(
                    "CONDITIONAL_LIMIT_VIOLATED",
                    format!(
                        "{} units ({}) cannot exceed the number of {} units ({}). Condition: {}",
                        limited_unit_name, limited_count, target_name, target_count, condition
                    ),
                );
            }
        }
    }
}

/// Parse a pool limit number from a "shared pool: up to N total characters" string.
fn parse_pool_limit(desc: &str) -> Option<u8> {
    // Look for "up to N total"
    if let Some(idx) = desc.find("up to ") {
        let after = &desc[idx + "up to ".len()..];
        // Parse the number
        let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        num_str.parse().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roster::{BoardingPatrol, EnhancementAssignment, SelectedUnit};
    use wh40k_boarding_content::faction_data::BoardingFactionDef;

    fn load_world_eaters() -> BoardingFactionDef {
        let json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/boarding_actions/factions/world_eaters_boarding_butchers.json"
        ))
        .unwrap();
        serde_json::from_str(&json).unwrap()
    }

    fn load_space_marines() -> BoardingFactionDef {
        let json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/boarding_actions/factions/space_marines_terminator_assault.json"
        ))
        .unwrap();
        serde_json::from_str(&json).unwrap()
    }

    fn load_astra_militarum() -> BoardingFactionDef {
        let json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/boarding_actions/factions/astra_militarum_tempestus.json"
        ))
        .unwrap();
        serde_json::from_str(&json).unwrap()
    }

    /// Test 1: Build a legal Boarding Butchers roster and validate it passes.
    /// Kharn 100pts + Master of Executions 60pts + Khorne Berzerkers x5 90pts
    /// + Eightbound x3 135pts + Jakhals x10 65pts = 450pts
    #[test]
    fn test_valid_boarding_butchers_roster() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "Kharn The Betrayer".to_string(),
            model_count: 1,
            points: 100,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Master of Executions".to_string(),
            model_count: 1,
            points: 60,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Eightbound".to_string(),
            model_count: 3,
            points: 135,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Jakhals".to_string(),
            model_count: 10,
            points: 65,
            wargear_selections: vec![],
        });

        // Designate Kharn as Warlord (index 0)
        patrol.warlord_index = Some(0);

        // Give an enhancement to Master of Executions (index 1, non-epic CHARACTER)
        patrol.enhancements.push(EnhancementAssignment {
            enhancement_name: "Chosen of Khorne".to_string(),
            unit_index: 1,
        });

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(
            result.valid,
            "Valid roster should pass validation, but got errors: {:?}",
            result.errors
        );
        assert_eq!(patrol.total_points, 450);
    }

    /// Test 2: Build a roster that exceeds 500 points. Validate it fails with OVER_POINTS.
    #[test]
    fn test_over_points_rejected() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "Kharn The Betrayer".to_string(),
            model_count: 1,
            points: 100,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 10,
            points: 180,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Chaos Terminators".to_string(),
            model_count: 5,
            points: 175,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Eightbound".to_string(),
            model_count: 3,
            points: 135,
            wargear_selections: vec![],
        });

        // 100 + 180 + 175 + 135 = 590 > 500
        patrol.warlord_index = Some(0);

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.code == "OVER_POINTS"),
            "Expected OVER_POINTS error, got: {:?}",
            result.errors
        );
    }

    /// Test 3: Try to include Kharn twice. Validate it fails with DUPLICATE_EPIC_HERO.
    #[test]
    fn test_duplicate_epic_hero_rejected() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "Kharn The Betrayer".to_string(),
            model_count: 1,
            points: 100,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Kharn The Betrayer".to_string(),
            model_count: 1,
            points: 100,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });

        patrol.warlord_index = Some(0);

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.code == "DUPLICATE_EPIC_HERO"),
            "Expected DUPLICATE_EPIC_HERO error, got: {:?}",
            result.errors
        );
    }

    /// Test 4: Include a unit not in the detachment's allowed list. Validate it fails.
    #[test]
    fn test_invalid_unit_rejected() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "Space Marine Terminators".to_string(),
            model_count: 5,
            points: 200,
            wargear_selections: vec![],
        });

        patrol.warlord_index = Some(0);

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.code == "INVALID_UNIT"),
            "Expected INVALID_UNIT error, got: {:?}",
            result.errors
        );
    }

    /// Test 5: Include more copies of a unit than max_count allows. Validate it fails.
    #[test]
    fn test_unit_count_exceeded() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        // Eightbound max_count is 1, add 2
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Eightbound".to_string(),
            model_count: 3,
            points: 135,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Eightbound".to_string(),
            model_count: 3,
            points: 135,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });

        patrol.warlord_index = Some(2);

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.code == "UNIT_COUNT_EXCEEDED"),
            "Expected UNIT_COUNT_EXCEEDED error, got: {:?}",
            result.errors
        );
    }

    /// Test 6: Set warlord to a non-CHARACTER unit when CHARACTERs exist. Validate it fails.
    #[test]
    fn test_warlord_must_be_character() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "Kharn The Betrayer".to_string(),
            model_count: 1,
            points: 100,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });

        // Set warlord to the Berzerkers (non-CHARACTER) instead of Kharn
        patrol.warlord_index = Some(1);

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.code == "WARLORD_NOT_CHARACTER"),
            "Expected WARLORD_NOT_CHARACTER error, got: {:?}",
            result.errors
        );
    }

    /// Test 7: Assign 3 enhancements. Validate it fails.
    #[test]
    fn test_too_many_enhancements() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "Master of Executions".to_string(),
            model_count: 1,
            points: 60,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });

        patrol.warlord_index = Some(0);

        // Assign 3 enhancements (only 1 character, but testing the count)
        patrol.enhancements.push(EnhancementAssignment {
            enhancement_name: "Chosen of Khorne".to_string(),
            unit_index: 0,
        });
        patrol.enhancements.push(EnhancementAssignment {
            enhancement_name: "Battle Lust".to_string(),
            unit_index: 0,
        });
        patrol.enhancements.push(EnhancementAssignment {
            enhancement_name: "Close-Quarters Killer".to_string(),
            unit_index: 0,
        });

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.code == "TOO_MANY_ENHANCEMENTS"),
            "Expected TOO_MANY_ENHANCEMENTS error, got: {:?}",
            result.errors
        );
    }

    /// Test 8: Assign the same enhancement twice. Validate it fails.
    #[test]
    fn test_duplicate_enhancement_rejected() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "Master of Executions".to_string(),
            model_count: 1,
            points: 60,
            wargear_selections: vec![],
        });
        // Add a second non-epic character to assign second enhancement to.
        // World Eaters only have Master of Executions as the non-epic character,
        // so we'll just put 2 and accept the unit_count error for this test.
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Master of Executions".to_string(),
            model_count: 1,
            points: 60,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });

        patrol.warlord_index = Some(0);

        patrol.enhancements.push(EnhancementAssignment {
            enhancement_name: "Chosen of Khorne".to_string(),
            unit_index: 0,
        });
        patrol.enhancements.push(EnhancementAssignment {
            enhancement_name: "Chosen of Khorne".to_string(),
            unit_index: 1,
        });

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.code == "DUPLICATE_ENHANCEMENT"),
            "Expected DUPLICATE_ENHANCEMENT error, got: {:?}",
            result.errors
        );
    }

    /// Test 9: Assign enhancement to an EPIC HERO. Validate it fails.
    #[test]
    fn test_enhancement_on_epic_hero_rejected() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "Kharn The Betrayer".to_string(),
            model_count: 1,
            points: 100,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });

        patrol.warlord_index = Some(0);

        // Assign enhancement to Kharn (EPIC HERO)
        patrol.enhancements.push(EnhancementAssignment {
            enhancement_name: "Chosen of Khorne".to_string(),
            unit_index: 0,
        });

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.code == "ENHANCEMENT_ON_EPIC_HERO"),
            "Expected ENHANCEMENT_ON_EPIC_HERO error, got: {:?}",
            result.errors
        );
    }

    /// Test 10: Use a universal BA enhancement (e.g., "Close-Quarters Killer"). Validate it passes.
    #[test]
    fn test_universal_enhancement_accepted() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "Master of Executions".to_string(),
            model_count: 1,
            points: 60,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });

        patrol.warlord_index = Some(0);

        // Assign a universal BA enhancement
        patrol.enhancements.push(EnhancementAssignment {
            enhancement_name: "Close-Quarters Killer".to_string(),
            unit_index: 0,
        });

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(
            result.valid,
            "Universal enhancement should be accepted, but got errors: {:?}",
            result.errors
        );
    }

    /// Test 11: For Boarding Butchers, add more Jakhals units than Berzerkers units.
    /// Validate it fails with CONDITIONAL_LIMIT_VIOLATED.
    #[test]
    fn test_conditional_limit_jakhals() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        // 1 Berzerkers unit but 2 Jakhals units: violates the rule
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Jakhals".to_string(),
            model_count: 10,
            points: 65,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Jakhals".to_string(),
            model_count: 10,
            points: 65,
            wargear_selections: vec![],
        });

        patrol.warlord_index = Some(0);

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.code == "CONDITIONAL_LIMIT_VIOLATED"),
            "Expected CONDITIONAL_LIMIT_VIOLATED error, got: {:?}",
            result.errors
        );
    }

    /// Test 12: Build a legal Space Marines Terminator Assault roster. Validate it passes.
    /// Captain in Terminator Armour 95pts + Terminator Squad x5 195pts + Terminator Assault Squad x5 200pts = 490pts
    #[test]
    fn test_valid_terminator_assault_roster() {
        let faction = load_space_marines();
        let mut patrol = BoardingPatrol::new(
            "Adeptus Astartes".to_string(),
            "ADEPTUS ASTARTES".to_string(),
            "Terminator Assault".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "Captain in Terminator Armour".to_string(),
            model_count: 1,
            points: 95,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Terminator Squad".to_string(),
            model_count: 5,
            points: 195,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Terminator Assault Squad".to_string(),
            model_count: 5,
            points: 200,
            wargear_selections: vec![],
        });

        patrol.warlord_index = Some(0);

        // Assign a detachment-specific enhancement
        patrol.enhancements.push(EnhancementAssignment {
            enhancement_name: "Seal of Indomitability".to_string(),
            unit_index: 0,
        });

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(
            result.valid,
            "Valid Terminator Assault roster should pass, but got errors: {:?}",
            result.errors
        );
        assert_eq!(patrol.total_points, 490);
    }

    /// Test 13: Build a legal Astra Militarum Tempestus roster. Validate it passes.
    /// Commissar 30pts + Tempestus Scions x10 140pts + Tempestus Scions x5 70pts
    /// + Tempestus Scions x5 70pts + Bullgryn Squad x3 100pts = 410pts
    #[test]
    fn test_valid_tempestus_roster() {
        let faction = load_astra_militarum();
        let mut patrol = BoardingPatrol::new(
            "Astra Militarum".to_string(),
            "ASTRA MILITARUM".to_string(),
            "Tempestus Boarding Regiment".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "Commissar".to_string(),
            model_count: 1,
            points: 30,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Tempestus Scions".to_string(),
            model_count: 10,
            points: 140,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Tempestus Scions".to_string(),
            model_count: 5,
            points: 70,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Tempestus Scions".to_string(),
            model_count: 5,
            points: 70,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Bullgryn Squad".to_string(),
            model_count: 3,
            points: 100,
            wargear_selections: vec![],
        });

        // Commissar is a CHARACTER, must be warlord
        patrol.warlord_index = Some(0);

        // Assign a detachment enhancement
        patrol.enhancements.push(EnhancementAssignment {
            enhancement_name: "Covert Breach".to_string(),
            unit_index: 0,
        });

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(
            result.valid,
            "Valid Tempestus roster should pass, but got errors: {:?}",
            result.errors
        );
        assert_eq!(patrol.total_points, 410);
    }

    /// Test: Invalid model count is rejected.
    #[test]
    fn test_invalid_model_count() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        // Khorne Berzerkers allowed sizes are [5, 10], try 7
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 7,
            points: 90,
            wargear_selections: vec![],
        });

        patrol.warlord_index = Some(0);

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.code == "INVALID_MODEL_COUNT"),
            "Expected INVALID_MODEL_COUNT error, got: {:?}",
            result.errors
        );
    }

    /// Test: Incorrect points cost is rejected.
    #[test]
    fn test_incorrect_points_rejected() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        // Khorne Berzerkers x5 should be 90, not 100
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 100,
            wargear_selections: vec![],
        });

        patrol.warlord_index = Some(0);

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.code == "INCORRECT_POINTS"),
            "Expected INCORRECT_POINTS error, got: {:?}",
            result.errors
        );
    }

    /// Test: No warlord designated is rejected.
    #[test]
    fn test_no_warlord_rejected() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });

        // No warlord_index set (None)

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.code == "NO_WARLORD"),
            "Expected NO_WARLORD error, got: {:?}",
            result.errors
        );
    }

    /// Test: Invalid detachment name is rejected.
    #[test]
    fn test_invalid_detachment_rejected() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Nonexistent Detachment".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });
        patrol.warlord_index = Some(0);

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.code == "INVALID_DETACHMENT"),
            "Expected INVALID_DETACHMENT error, got: {:?}",
            result.errors
        );
    }

    /// Test: Faction mismatch is rejected.
    #[test]
    fn test_faction_mismatch_rejected() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "Space Marines".to_string(),
            "ADEPTUS ASTARTES".to_string(),
            "Boarding Butchers".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });
        patrol.warlord_index = Some(0);

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.code == "FACTION_MISMATCH"),
            "Expected FACTION_MISMATCH error, got: {:?}",
            result.errors
        );
    }

    /// Test: Enhancement assigned to non-CHARACTER is rejected.
    #[test]
    fn test_enhancement_on_non_character_rejected() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "Master of Executions".to_string(),
            model_count: 1,
            points: 60,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });

        patrol.warlord_index = Some(0);

        // Assign enhancement to Berzerkers (non-CHARACTER)
        patrol.enhancements.push(EnhancementAssignment {
            enhancement_name: "Chosen of Khorne".to_string(),
            unit_index: 1,
        });

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.code == "ENHANCEMENT_NOT_ON_CHARACTER"),
            "Expected ENHANCEMENT_NOT_ON_CHARACTER error, got: {:?}",
            result.errors
        );
    }

    /// Test: Invalid enhancement name is rejected.
    #[test]
    fn test_invalid_enhancement_name_rejected() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "Master of Executions".to_string(),
            model_count: 1,
            points: 60,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });

        patrol.warlord_index = Some(0);

        // Assign a nonexistent enhancement
        patrol.enhancements.push(EnhancementAssignment {
            enhancement_name: "Nonexistent Enhancement".to_string(),
            unit_index: 0,
        });

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.code == "INVALID_ENHANCEMENT"),
            "Expected INVALID_ENHANCEMENT error, got: {:?}",
            result.errors
        );
    }

    /// Test: Character pool limit is enforced.
    /// World Eaters has "Shared pool: up to 2 total characters" for Kharn and Master of Executions.
    /// Since max_count for each is 1 and there are only 2 characters in the pool,
    /// this is implicitly enforced. But we can test with a scenario that exceeds individual max_count.
    #[test]
    fn test_character_pool_limit() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        // Both Kharn and Master of Executions share a pool of up to 2 characters.
        // Adding both is fine (2 total).
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Kharn The Betrayer".to_string(),
            model_count: 1,
            points: 100,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Master of Executions".to_string(),
            model_count: 1,
            points: 60,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });

        patrol.warlord_index = Some(0);

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        // Should be valid since 2 <= 2
        assert!(
            result.valid,
            "2 characters from shared pool of 2 should be valid, but got errors: {:?}",
            result.errors
        );
    }

    /// Test: Conditional limit - Jakhals count equal to Berzerkers count is valid.
    #[test]
    fn test_conditional_limit_jakhals_equal() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        // 2 Berzerkers units and 2 Jakhals units: valid (2 <= 2)
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Jakhals".to_string(),
            model_count: 10,
            points: 65,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "Jakhals".to_string(),
            model_count: 10,
            points: 65,
            wargear_selections: vec![],
        });

        patrol.warlord_index = Some(0);

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(
            result.valid,
            "Jakhals count equal to Berzerkers count should be valid, but got errors: {:?}",
            result.errors
        );
    }

    /// Test: Warlord index out of bounds is rejected.
    #[test]
    fn test_warlord_index_out_of_bounds() {
        let faction = load_world_eaters();
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "Khorne Berzerkers".to_string(),
            model_count: 5,
            points: 90,
            wargear_selections: vec![],
        });

        patrol.warlord_index = Some(5); // out of bounds

        let validator = BoardingRosterValidator::new(&patrol, &faction);
        let result = validator.validate();

        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.code == "INVALID_WARLORD_INDEX"),
            "Expected INVALID_WARLORD_INDEX error, got: {:?}",
            result.errors
        );
    }

    /// Test: All universal enhancements are recognized.
    #[test]
    fn test_all_universal_enhancements() {
        let faction = load_world_eaters();

        for enh_name in UNIVERSAL_BA_ENHANCEMENTS {
            let mut patrol = BoardingPatrol::new(
                "World Eaters".to_string(),
                "WORLD EATERS".to_string(),
                "Boarding Butchers".to_string(),
            );

            patrol.add_unit(SelectedUnit {
                datasheet_name: "Master of Executions".to_string(),
                model_count: 1,
                points: 60,
                wargear_selections: vec![],
            });
            patrol.add_unit(SelectedUnit {
                datasheet_name: "Khorne Berzerkers".to_string(),
                model_count: 5,
                points: 90,
                wargear_selections: vec![],
            });

            patrol.warlord_index = Some(0);
            patrol.enhancements.push(EnhancementAssignment {
                enhancement_name: enh_name.to_string(),
                unit_index: 0,
            });

            let validator = BoardingRosterValidator::new(&patrol, &faction);
            let result = validator.validate();

            assert!(
                result.valid,
                "Universal enhancement '{}' should be accepted, but got errors: {:?}",
                enh_name, result.errors
            );
        }
    }

    /// Test: Roster remove_unit correctly adjusts indices.
    #[test]
    fn test_roster_remove_unit() {
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        patrol.add_unit(SelectedUnit {
            datasheet_name: "A".to_string(),
            model_count: 1,
            points: 100,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "B".to_string(),
            model_count: 1,
            points: 50,
            wargear_selections: vec![],
        });
        patrol.add_unit(SelectedUnit {
            datasheet_name: "C".to_string(),
            model_count: 1,
            points: 75,
            wargear_selections: vec![],
        });

        patrol.warlord_index = Some(2); // C
        patrol.enhancements.push(EnhancementAssignment {
            enhancement_name: "Test".to_string(),
            unit_index: 2,
        });

        // Remove unit B (index 1)
        let removed = patrol.remove_unit(1);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().datasheet_name, "B");
        assert_eq!(patrol.units.len(), 2);
        assert_eq!(patrol.total_points, 175);
        // Warlord was at index 2, now should be 1
        assert_eq!(patrol.warlord_index, Some(1));
        // Enhancement was at index 2, now should be 1
        assert_eq!(patrol.enhancements[0].unit_index, 1);

        // Remove unit A (index 0, the warlord-adjacent unit)
        patrol.remove_unit(0);
        assert_eq!(patrol.units.len(), 1);
        assert_eq!(patrol.warlord_index, Some(0));
        assert_eq!(patrol.enhancements[0].unit_index, 0);
    }

    /// Test: Roster recalculate_points works correctly.
    #[test]
    fn test_roster_recalculate_points() {
        let mut patrol = BoardingPatrol::new(
            "World Eaters".to_string(),
            "WORLD EATERS".to_string(),
            "Boarding Butchers".to_string(),
        );

        patrol.units.push(SelectedUnit {
            datasheet_name: "A".to_string(),
            model_count: 1,
            points: 100,
            wargear_selections: vec![],
        });
        patrol.units.push(SelectedUnit {
            datasheet_name: "B".to_string(),
            model_count: 1,
            points: 50,
            wargear_selections: vec![],
        });

        // total_points is 0 because we pushed directly
        assert_eq!(patrol.total_points, 0);

        patrol.recalculate_points();
        assert_eq!(patrol.total_points, 150);
    }
}

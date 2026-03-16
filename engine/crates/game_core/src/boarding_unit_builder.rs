//! Boarding Actions unit builder — converts BoardingUnitDatasheet (string-based JSON)
//! into UnitState/ModelState (strongly-typed runtime structs).
//!
//! This module bridges the gap between the data-driven boarding actions content
//! (which uses human-readable strings for stats) and the game engine's runtime
//! types (which use strongly-typed newtypes).
//!
//! Source: boarding_actions_complete_v3.md
//! Source: boarding_actions_integration_rust.md

use wh40k_boarding_content::faction_data::{
    BoardingFactionDef, BoardingUnitDatasheet,
    WeaponProfile as ContentWeaponProfile,
};
use wh40k_core_types::{
    ArmorPenetration, ArmorSave, AttackCount, BaseSize, Damage, DatasheetId, FeelNoPain,
    Inches, InvulnerableSave, Keyword, KeywordSet, Leadership, ModelId, MoveCharacteristic,
    ObjectiveControl, PlayerId, Position, Skill, Strength, Toughness, UnitId,
    WeaponAbility, WeaponAbilitySet, WeaponId, WeaponProfile, WeaponType, Wounds,
};

use crate::unit::{ModelState, UnitState};

// ---------------------------------------------------------------------------
// ID counters
// ---------------------------------------------------------------------------

/// Tracks sequential ID assignment for units, models, weapons, and datasheets.
pub struct IdAllocator {
    next_unit: u32,
    next_model: u32,
    next_weapon: u32,
    next_datasheet: u32,
}

impl IdAllocator {
    /// Create a new allocator with base offsets for a player.
    /// Player A uses IDs starting at 0, Player B at 100/1000/10000.
    pub fn for_player(player_id: PlayerId) -> Self {
        if player_id.raw() == 0 {
            Self {
                next_unit: 0,
                next_model: 0,
                next_weapon: 0,
                next_datasheet: 500,
            }
        } else {
            Self {
                next_unit: 100,
                next_model: 1000,
                next_weapon: 10000,
                next_datasheet: 600,
            }
        }
    }

    fn alloc_unit(&mut self) -> UnitId {
        let id = UnitId::new(self.next_unit);
        self.next_unit += 1;
        id
    }

    fn alloc_model(&mut self) -> ModelId {
        let id = ModelId::new(self.next_model);
        self.next_model += 1;
        id
    }

    fn alloc_weapon(&mut self) -> WeaponId {
        let id = WeaponId::new(self.next_weapon);
        self.next_weapon += 1;
        id
    }

    fn alloc_datasheet(&mut self) -> DatasheetId {
        let id = DatasheetId::new(self.next_datasheet);
        self.next_datasheet += 1;
        id
    }
}

// ---------------------------------------------------------------------------
// String parsers for stat characteristics
// ---------------------------------------------------------------------------

/// Parse movement string like "6\"" or "5\"" into MoveCharacteristic.
fn parse_movement(s: &str) -> MoveCharacteristic {
    let clean = s.replace('"', "").replace('\"', "").trim().to_string();
    let value: i32 = clean.parse().unwrap_or(6);
    MoveCharacteristic::from_inches(value)
}

/// Parse save string like "2+" or "3+" into ArmorSave.
fn parse_armor_save(s: &str) -> ArmorSave {
    let clean = s.replace('+', "").trim().to_string();
    let value: u8 = clean.parse().unwrap_or(7);
    ArmorSave(value)
}

/// Parse leadership string like "6+" or "7+" into Leadership.
fn parse_leadership(s: &str) -> Leadership {
    let clean = s.replace('+', "").trim().to_string();
    let value: u8 = clean.parse().unwrap_or(7);
    Leadership::new(value)
}

/// Parse attacks string like "4", "D6", "D3+1", "D6+2", "2D6" into AttackCount.
fn parse_attacks(s: &str) -> AttackCount {
    let s = s.trim();
    if s.eq_ignore_ascii_case("D6") {
        AttackCount::D6
    } else if s.eq_ignore_ascii_case("D3") {
        AttackCount::D3
    } else if let Some(rest) = s.strip_prefix("D6+").or_else(|| s.strip_prefix("d6+")) {
        let bonus: u8 = rest.parse().unwrap_or(0);
        AttackCount::D6Plus(bonus)
    } else if let Some(rest) = s.strip_prefix("D3+").or_else(|| s.strip_prefix("d3+")) {
        let bonus: u8 = rest.parse().unwrap_or(0);
        AttackCount::D3Plus(bonus)
    } else if s.eq_ignore_ascii_case("2D6") {
        // 2D6 attacks — approximate as D6Plus(3) for average equivalence
        // (true average: 7, D6Plus(3) average: 6.5 — close enough for game engine)
        AttackCount::D6Plus(4)
    } else {
        let value: u8 = s.parse().unwrap_or(1);
        AttackCount::Fixed(value)
    }
}

/// Parse damage string like "2", "D3", "D6", "D3+1", "D6+2" into Damage.
fn parse_damage(s: &str) -> Damage {
    let s = s.trim();
    if s.eq_ignore_ascii_case("D6") {
        Damage::D6
    } else if s.eq_ignore_ascii_case("D3") {
        Damage::D3
    } else if let Some(rest) = s.strip_prefix("D6+").or_else(|| s.strip_prefix("d6+")) {
        let bonus: u8 = rest.parse().unwrap_or(0);
        Damage::D6Plus(bonus)
    } else if let Some(rest) = s.strip_prefix("D3+").or_else(|| s.strip_prefix("d3+")) {
        let bonus: u8 = rest.parse().unwrap_or(0);
        Damage::D3Plus(bonus)
    } else {
        let value: u8 = s.parse().unwrap_or(1);
        Damage::Fixed(value)
    }
}

/// Parse AP string like "-1", "-2", "0" into ArmorPenetration.
fn parse_ap(s: &str) -> ArmorPenetration {
    let s = s.trim();
    let value: i8 = s.parse().unwrap_or(0);
    ArmorPenetration::new(value)
}

/// Parse skill string like "2+", "3+", "N/A" into Skill.
/// Returns Skill(7) for "N/A" (auto-hit weapons like Torrent).
fn parse_skill(s: &str) -> Skill {
    let s = s.trim();
    if s == "N/A" || s == "n/a" || s == "-" {
        Skill(7) // Will be used with Torrent auto-hit
    } else {
        let clean = s.replace('+', "");
        let value: u8 = clean.parse().unwrap_or(4);
        Skill(value)
    }
}

/// Parse range string like "24\"", "12\"", "Melee" into (WeaponType, Inches).
fn parse_range(s: &str) -> (WeaponType, Inches) {
    let s = s.trim();
    if s.eq_ignore_ascii_case("Melee") || s == "-" || s.is_empty() {
        (WeaponType::Melee, Inches::ZERO)
    } else {
        let clean = s.replace('"', "").replace('\"', "");
        let value: i32 = clean.parse().unwrap_or(0);
        if value == 0 {
            (WeaponType::Melee, Inches::ZERO)
        } else {
            (WeaponType::Ranged, Inches::from_inches(value))
        }
    }
}

/// Parse a single weapon ability string into a WeaponAbility.
/// Returns None for unrecognized abilities.
fn parse_weapon_ability(s: &str) -> Option<WeaponAbility> {
    let s = s.trim();

    // Exact matches
    match s {
        "Assault" => return Some(WeaponAbility::Assault),
        "Heavy" => return Some(WeaponAbility::Heavy),
        "Pistol" => return Some(WeaponAbility::Pistol),
        "Blast" => return Some(WeaponAbility::Blast),
        "Torrent" => return Some(WeaponAbility::Torrent),
        "Twin-linked" | "Twin-Linked" => return Some(WeaponAbility::TwinLinked),
        "Lethal Hits" => return Some(WeaponAbility::LethalHits),
        "Devastating Wounds" => return Some(WeaponAbility::DevastatingWounds),
        "Hazardous" => return Some(WeaponAbility::Hazardous),
        "Precision" => return Some(WeaponAbility::Precision),
        "Indirect Fire" => return Some(WeaponAbility::IndirectFire),
        "Ignores Cover" => return Some(WeaponAbility::IgnoresCover),
        "Lance" => return Some(WeaponAbility::Lance),
        "Extra Attacks" => return Some(WeaponAbility::ExtraAttacks),
        _ => {}
    }

    // Parametric matches: "Sustained Hits 1", "Rapid Fire 2", "Anti-Infantry 4+", "Melta 2"
    if let Some(rest) = s.strip_prefix("Sustained Hits ") {
        let n: u8 = rest.parse().unwrap_or(1);
        return Some(WeaponAbility::SustainedHits(n));
    }
    if let Some(rest) = s.strip_prefix("Rapid Fire ") {
        let n: u8 = rest.parse().unwrap_or(1);
        return Some(WeaponAbility::RapidFire(n));
    }
    if let Some(rest) = s.strip_prefix("Melta ") {
        let n: u8 = rest.parse().unwrap_or(1);
        return Some(WeaponAbility::Melta(n));
    }

    // Anti-X N+ patterns: "Anti-Infantry 4+", "Anti-Monster 3+", "Anti-Vehicle 3+"
    if let Some(rest) = s.strip_prefix("Anti-") {
        // Split "Infantry 4+" into keyword and threshold
        let parts: Vec<&str> = rest.rsplitn(2, ' ').collect();
        if parts.len() == 2 {
            let threshold_str = parts[0].replace('+', "");
            let keyword_str = parts[1];
            let threshold: u8 = threshold_str.parse().unwrap_or(4);
            let keyword = match keyword_str.to_uppercase().as_str() {
                "INFANTRY" => Keyword::Infantry,
                "MONSTER" => Keyword::Monster,
                "VEHICLE" => Keyword::Vehicle,
                "MOUNTED" => Keyword::Mounted,
                "BEAST" => Keyword::Beast,
                "SWARM" => Keyword::Swarm,
                "TERMINATOR" => Keyword::Terminator,
                "CHARACTER" => Keyword::Character,
                "FLY" => Keyword::Fly,
                "PSYKER" => Keyword::Psyker,
                _ => Keyword::Infantry, // Fallback
            };
            return Some(WeaponAbility::Anti(keyword, threshold));
        }
    }

    // "Psychic" and "Witchfire" are not standard weapon abilities — skip them
    None
}

/// Parse all weapon ability strings into a WeaponAbilitySet.
fn parse_weapon_abilities(abilities: &[String]) -> WeaponAbilitySet {
    let parsed: Vec<WeaponAbility> = abilities
        .iter()
        .filter_map(|s| parse_weapon_ability(s))
        .collect();
    WeaponAbilitySet::from_abilities(parsed)
}

// ---------------------------------------------------------------------------
// Invulnerable save extraction
// ---------------------------------------------------------------------------

/// Extract invulnerable save from a datasheet's abilities list.
/// Looks for ability_type == "invulnerable_save" and parses the description.
fn extract_invulnerable_save(
    abilities: &[wh40k_boarding_content::faction_data::AbilityDef],
) -> Option<InvulnerableSave> {
    for ability in abilities {
        if ability.ability_type == "invulnerable_save" {
            // Parse description like "This model has a 4+ invulnerable save."
            // Look for pattern "N+" where N is a digit
            for word in ability.description.split_whitespace() {
                let clean = word.replace('+', "").replace('.', "").replace(',', "");
                if let Ok(val) = clean.parse::<u8>() {
                    if (2..=6).contains(&val) {
                        return Some(InvulnerableSave(val));
                    }
                }
            }
            // If we found the ability but couldn't parse, assume 4+
            return Some(InvulnerableSave::FOUR_PLUS);
        }
    }
    None
}

/// Extract Feel No Pain from abilities (looks for patterns like "5+" in FNP descriptions).
fn extract_feel_no_pain(
    abilities: &[wh40k_boarding_content::faction_data::AbilityDef],
) -> Option<FeelNoPain> {
    for ability in abilities {
        let desc_lower = ability.description.to_lowercase();
        if desc_lower.contains("feel no pain") || desc_lower.contains("fnp") {
            for word in ability.description.split_whitespace() {
                let clean = word.replace('+', "").replace('.', "").replace(',', "");
                if let Ok(val) = clean.parse::<u8>() {
                    if (4..=6).contains(&val) {
                        return Some(FeelNoPain(val));
                    }
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Base size determination
// ---------------------------------------------------------------------------

/// Determine the base size for a model based on its keywords and unit type.
/// This is an approximation since base size isn't in the JSON data.
fn determine_base_size(keywords: &KeywordSet, wounds: u8, is_character: bool) -> BaseSize {
    if keywords.has(Keyword::Monster) {
        BaseSize::MM60
    } else if keywords.has(Keyword::Terminator) || keywords.has(Keyword::TerminatorArmour) {
        BaseSize::MM40
    } else if keywords.has(Keyword::Ogryn) {
        BaseSize::MM40
    } else if keywords.has(Keyword::Spawn) {
        BaseSize::MM50
    } else if is_character && wounds >= 5 {
        BaseSize::MM40
    } else if is_character {
        BaseSize::MM32
    } else {
        BaseSize::MM28 // Default infantry base
    }
}

// ---------------------------------------------------------------------------
// Weapon conversion
// ---------------------------------------------------------------------------

/// Convert a content weapon profile (string-based) to a runtime weapon profile (typed).
fn convert_weapon(
    content_wp: &ContentWeaponProfile,
    alloc: &mut IdAllocator,
    is_melee: bool,
) -> WeaponProfile {
    let (weapon_type, range) = if is_melee {
        (WeaponType::Melee, Inches::ZERO)
    } else {
        parse_range(&content_wp.range)
    };

    WeaponProfile {
        id: alloc.alloc_weapon(),
        name: content_wp.name.clone(),
        weapon_type,
        range,
        attacks: parse_attacks(&content_wp.attacks),
        skill: parse_skill(&content_wp.skill),
        strength: Strength::new(content_wp.strength),
        ap: parse_ap(&content_wp.ap),
        damage: parse_damage(&content_wp.damage),
        abilities: parse_weapon_abilities(&content_wp.abilities),
    }
}

// ---------------------------------------------------------------------------
// Unit building
// ---------------------------------------------------------------------------

/// Build a complete UnitState from a BoardingUnitDatasheet and a model count.
///
/// # Arguments
/// - `datasheet` - The unit's full datasheet from the boarding faction JSON
/// - `model_count` - Number of models to create for this unit
/// - `owner` - Which player owns this unit
/// - `alloc` - ID allocator for unique IDs
///
/// # Returns
/// A fully initialized UnitState with all models, weapons, and characteristics set.
pub fn build_unit_from_datasheet(
    datasheet: &BoardingUnitDatasheet,
    model_count: u8,
    owner: PlayerId,
    alloc: &mut IdAllocator,
) -> UnitState {
    let unit_id = alloc.alloc_unit();
    let datasheet_id = alloc.alloc_datasheet();

    // Build keyword set from string keywords
    let mut all_keyword_strings = datasheet.keywords.clone();
    all_keyword_strings.extend(datasheet.faction_keywords.iter().cloned());
    let keywords = KeywordSet::from_keyword_strings(&all_keyword_strings);

    // Parse profile characteristics
    let movement = parse_movement(&datasheet.profile.movement);
    let toughness = Toughness::new(datasheet.profile.toughness);
    let armor_save = parse_armor_save(&datasheet.profile.save);
    let leadership = parse_leadership(&datasheet.profile.leadership);
    let oc = ObjectiveControl::new(datasheet.profile.oc);
    let wounds = Wounds::new(datasheet.profile.wounds);

    // Extract invulnerable save and FNP from abilities
    let invuln = extract_invulnerable_save(&datasheet.abilities);
    let fnp = extract_feel_no_pain(&datasheet.abilities);

    // Determine base size
    let base_size = determine_base_size(&keywords, datasheet.profile.wounds, datasheet.is_character);

    // Determine if the first model is a leader
    let is_leader_unit = datasheet.is_character || datasheet.is_epic_hero;

    // Build models
    let models: Vec<ModelState> = (0..model_count)
        .map(|i| {
            // Convert weapons for each model (each model gets its own weapon instances)
            let ranged: Vec<WeaponProfile> = datasheet
                .ranged_weapons
                .iter()
                .map(|w| convert_weapon(w, alloc, false))
                .collect();

            let melee: Vec<WeaponProfile> = datasheet
                .melee_weapons
                .iter()
                .map(|w| convert_weapon(w, alloc, true))
                .collect();

            let model_id = alloc.alloc_model();
            let is_leader = is_leader_unit && i == 0;

            ModelState::new(
                model_id,
                unit_id,
                wounds,
                Position::from_inches(0, 0), // Undeployed position
                base_size,
                ranged,
                melee,
                is_leader,
                fnp,
            )
        })
        .collect();

    UnitState::new(
        unit_id,
        owner,
        datasheet.name.clone(),
        datasheet_id,
        keywords,
        models,
        movement,
        toughness,
        armor_save,
        invuln,
        leadership,
        oc,
    )
}

// ---------------------------------------------------------------------------
// Roster building
// ---------------------------------------------------------------------------

/// A roster selection for game creation, matching the frontend's SelectedUnit type.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RosterSelection {
    pub datasheet_name: String,
    pub model_count: u8,
    pub points: u16,
    #[serde(default)]
    pub wargear_selections: Vec<usize>,
}

/// A complete roster for one player, ready for unit instantiation.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PlayerRoster {
    pub faction_name: String,
    pub faction_keyword: String,
    pub detachment_name: String,
    pub units: Vec<RosterSelection>,
    pub warlord_index: Option<usize>,
    #[serde(default)]
    pub enhancements: Vec<EnhancementSelection>,
    pub total_points: u16,
}

/// Enhancement assigned to a unit.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EnhancementSelection {
    pub enhancement_name: String,
    pub unit_index: usize,
}

/// Build all units for a player roster from faction data.
///
/// Looks up each selected unit in the faction's datasheets and creates
/// the corresponding UnitState with the requested model count.
///
/// # Arguments
/// - `roster` - The player's roster selections
/// - `faction` - The full faction definition (with datasheets)
/// - `owner` - Which player owns these units
/// - `alloc` - ID allocator for unique IDs
///
/// # Returns
/// Vec of UnitState for all selected units, ready for deployment.
pub fn build_roster_units(
    roster: &PlayerRoster,
    faction: &BoardingFactionDef,
    owner: PlayerId,
    alloc: &mut IdAllocator,
) -> Vec<UnitState> {
    let mut units = Vec::new();

    for (idx, selection) in roster.units.iter().enumerate() {
        // Find the datasheet by name
        let datasheet = faction
            .datasheets
            .iter()
            .find(|ds| ds.name == selection.datasheet_name);

        if let Some(ds) = datasheet {
            let mut unit = build_unit_from_datasheet(ds, selection.model_count, owner, alloc);

            // Mark warlord
            if roster.warlord_index == Some(idx) {
                // The warlord gets Leader keyword if not already present
                if !unit.keywords.has(Keyword::Leader) && unit.keywords.has(Keyword::Character) {
                    unit.keywords |= KeywordSet::from_keyword(Keyword::Leader);
                }
            }

            units.push(unit);
        }
    }

    units
}

// ---------------------------------------------------------------------------
// Auto-roster generation (for AI opponent)
// ---------------------------------------------------------------------------

/// Auto-generate a balanced roster for the AI opponent from faction data.
///
/// Strategy: Pick one character first (preferably an epic hero or the cheapest character),
/// then fill with battleline units, then other allowed units up to 500 points.
pub fn auto_generate_roster(
    faction: &BoardingFactionDef,
    detachment_index: usize,
) -> PlayerRoster {
    let detachment = &faction.detachments[detachment_index.min(faction.detachments.len() - 1)];
    let max_points: u16 = 500;

    let mut selected_units: Vec<RosterSelection> = Vec::new();
    let mut total_points: u16 = 0;
    let mut warlord_index: Option<usize> = None;

    // Helper: find cheapest available size for a datasheet
    let find_datasheet = |name: &str| -> Option<&BoardingUnitDatasheet> {
        faction.datasheets.iter().find(|ds| ds.name == name)
    };

    // Phase 1: Add one character (warlord)
    for unit_ref in &detachment.allowed_units {
        if let Some(ds) = find_datasheet(&unit_ref.datasheet_name) {
            if ds.is_character || ds.is_epic_hero {
                if let Some(cheapest) = ds.points.iter().min_by_key(|p| p.points) {
                    if total_points + cheapest.points <= max_points {
                        warlord_index = Some(selected_units.len());
                        selected_units.push(RosterSelection {
                            datasheet_name: ds.name.clone(),
                            model_count: cheapest.model_count,
                            points: cheapest.points,
                            wargear_selections: Vec::new(),
                        });
                        total_points += cheapest.points;
                        break;
                    }
                }
            }
        }
    }

    // Phase 2: Add battleline units
    for unit_ref in &detachment.allowed_units {
        if let Some(ds) = find_datasheet(&unit_ref.datasheet_name) {
            if ds.is_battleline {
                // Check if already selected
                let already_selected = selected_units.iter().filter(|s| s.datasheet_name == ds.name).count();
                let max_count = unit_ref.max_count.unwrap_or(3) as usize;
                if already_selected >= max_count {
                    continue;
                }

                // Pick the smallest (cheapest) size
                if let Some(cheapest) = ds.points.iter().min_by_key(|p| p.points) {
                    if total_points + cheapest.points <= max_points {
                        selected_units.push(RosterSelection {
                            datasheet_name: ds.name.clone(),
                            model_count: cheapest.model_count,
                            points: cheapest.points,
                            wargear_selections: Vec::new(),
                        });
                        total_points += cheapest.points;
                    }
                }
            }
        }
    }

    // Phase 3: Fill remaining points with other units
    for unit_ref in &detachment.allowed_units {
        if total_points >= max_points {
            break;
        }
        if let Some(ds) = find_datasheet(&unit_ref.datasheet_name) {
            // Skip if already selected at max count
            let already_selected = selected_units.iter().filter(|s| s.datasheet_name == ds.name).count();
            let max_count = unit_ref.max_count.unwrap_or(3) as usize;
            if already_selected >= max_count {
                continue;
            }

            // Find the largest size that fits
            if let Some(best_fit) = ds.points
                .iter()
                .filter(|p| total_points + p.points <= max_points)
                .max_by_key(|p| p.points)
            {
                selected_units.push(RosterSelection {
                    datasheet_name: ds.name.clone(),
                    model_count: best_fit.model_count,
                    points: best_fit.points,
                    wargear_selections: Vec::new(),
                });
                total_points += best_fit.points;
            }
        }
    }

    // If no warlord was set and we have units, make the first character the warlord
    if warlord_index.is_none() && !selected_units.is_empty() {
        warlord_index = Some(0);
    }

    PlayerRoster {
        faction_name: faction.faction_name.clone(),
        faction_keyword: faction.faction_keyword.clone(),
        detachment_name: detachment.detachment_name.clone(),
        units: selected_units,
        warlord_index,
        enhancements: Vec::new(),
        total_points,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_core_types::UnitStatus;

    #[test]
    fn test_parse_movement() {
        assert_eq!(parse_movement("6\"").distance(), Inches::from_inches(6));
        assert_eq!(parse_movement("5\"").distance(), Inches::from_inches(5));
        assert_eq!(parse_movement("12\"").distance(), Inches::from_inches(12));
    }

    #[test]
    fn test_parse_armor_save() {
        assert_eq!(parse_armor_save("2+"), ArmorSave::TWO_PLUS);
        assert_eq!(parse_armor_save("3+"), ArmorSave::THREE_PLUS);
        assert_eq!(parse_armor_save("4+"), ArmorSave::FOUR_PLUS);
        assert_eq!(parse_armor_save("6+"), ArmorSave::SIX_PLUS);
    }

    #[test]
    fn test_parse_attacks() {
        assert_eq!(parse_attacks("4"), AttackCount::Fixed(4));
        assert_eq!(parse_attacks("D6"), AttackCount::D6);
        assert_eq!(parse_attacks("D3"), AttackCount::D3);
        assert_eq!(parse_attacks("D6+1"), AttackCount::D6Plus(1));
        assert_eq!(parse_attacks("D3+2"), AttackCount::D3Plus(2));
    }

    #[test]
    fn test_parse_damage() {
        assert_eq!(parse_damage("2"), Damage::Fixed(2));
        assert_eq!(parse_damage("D3"), Damage::D3);
        assert_eq!(parse_damage("D6"), Damage::D6);
        assert_eq!(parse_damage("D6+1"), Damage::D6Plus(1));
    }

    #[test]
    fn test_parse_ap() {
        assert_eq!(parse_ap("-1"), ArmorPenetration::MINUS_1);
        assert_eq!(parse_ap("-2"), ArmorPenetration::MINUS_2);
        assert_eq!(parse_ap("0"), ArmorPenetration::ZERO);
        assert_eq!(parse_ap("-3"), ArmorPenetration::MINUS_3);
    }

    #[test]
    fn test_parse_skill() {
        assert_eq!(parse_skill("2+"), Skill::TWO_PLUS);
        assert_eq!(parse_skill("3+"), Skill::THREE_PLUS);
        assert_eq!(parse_skill("N/A"), Skill(7));
    }

    #[test]
    fn test_parse_range() {
        let (wt, r) = parse_range("24\"");
        assert_eq!(wt, WeaponType::Ranged);
        assert_eq!(r, Inches::from_inches(24));

        let (wt, r) = parse_range("Melee");
        assert_eq!(wt, WeaponType::Melee);
        assert_eq!(r, Inches::ZERO);
    }

    #[test]
    fn test_parse_weapon_abilities() {
        assert_eq!(
            parse_weapon_ability("Lethal Hits"),
            Some(WeaponAbility::LethalHits)
        );
        assert_eq!(
            parse_weapon_ability("Sustained Hits 1"),
            Some(WeaponAbility::SustainedHits(1))
        );
        assert_eq!(
            parse_weapon_ability("Rapid Fire 2"),
            Some(WeaponAbility::RapidFire(2))
        );
        assert_eq!(
            parse_weapon_ability("Anti-Infantry 4+"),
            Some(WeaponAbility::Anti(Keyword::Infantry, 4))
        );
        assert_eq!(
            parse_weapon_ability("Anti-Monster 3+"),
            Some(WeaponAbility::Anti(Keyword::Monster, 3))
        );
        assert_eq!(
            parse_weapon_ability("Melta 2"),
            Some(WeaponAbility::Melta(2))
        );
        assert_eq!(
            parse_weapon_ability("Devastating Wounds"),
            Some(WeaponAbility::DevastatingWounds)
        );
    }

    #[test]
    fn test_keyword_set_from_strings() {
        let keywords = KeywordSet::from_keyword_strings(&[
            "INFANTRY".to_string(),
            "CHARACTER".to_string(),
            "IMPERIUM".to_string(),
            "ADEPTUS ASTARTES".to_string(),
            "ARJAC ROCKFIST".to_string(), // Should be silently ignored
        ]);
        assert!(keywords.has(Keyword::Infantry));
        assert!(keywords.has(Keyword::Character));
        assert!(keywords.has(Keyword::Imperium));
        assert!(keywords.has(Keyword::Astartes));
        assert_eq!(keywords.count(), 4); // ARJAC ROCKFIST ignored
    }

    #[test]
    fn test_build_unit_from_datasheet() {
        // Create a minimal test datasheet
        let ds = BoardingUnitDatasheet {
            name: "Test Squad".to_string(),
            points: vec![wh40k_boarding_content::faction_data::PointsCost {
                model_count: 5,
                points: 100,
            }],
            is_epic_hero: false,
            is_character: false,
            is_battleline: true,
            profile: wh40k_boarding_content::faction_data::UnitProfile {
                movement: "6\"".to_string(),
                toughness: 4,
                save: "3+".to_string(),
                wounds: 2,
                leadership: "7+".to_string(),
                oc: 2,
            },
            ranged_weapons: vec![ContentWeaponProfile {
                name: "Boltgun".to_string(),
                range: "24\"".to_string(),
                attacks: "2".to_string(),
                skill: "3+".to_string(),
                strength: 4,
                ap: "0".to_string(),
                damage: "1".to_string(),
                abilities: vec!["Rapid Fire 1".to_string()],
            }],
            melee_weapons: vec![ContentWeaponProfile {
                name: "Close combat weapon".to_string(),
                range: "Melee".to_string(),
                attacks: "2".to_string(),
                skill: "3+".to_string(),
                strength: 4,
                ap: "0".to_string(),
                damage: "1".to_string(),
                abilities: vec![],
            }],
            abilities: vec![],
            leader_info: None,
            keywords: vec!["INFANTRY".to_string(), "BATTLELINE".to_string()],
            faction_keywords: vec!["IMPERIUM".to_string(), "ADEPTUS ASTARTES".to_string()],
            wargear_options: vec![],
            unit_composition: vec![],
        };

        let mut alloc = IdAllocator::for_player(PlayerId::new(0));
        let unit = build_unit_from_datasheet(&ds, 5, PlayerId::new(0), &mut alloc);

        assert_eq!(unit.name, "Test Squad");
        assert_eq!(unit.models.len(), 5);
        assert_eq!(unit.base_toughness, Toughness::new(4));
        assert_eq!(unit.base_armor_save, ArmorSave::THREE_PLUS);
        assert_eq!(unit.base_oc, ObjectiveControl::new(2));
        assert!(unit.keywords.has(Keyword::Infantry));
        assert!(unit.keywords.has(Keyword::Battleline));
        assert!(unit.keywords.has(Keyword::Imperium));
        assert!(unit.keywords.has(Keyword::Astartes));
        assert_eq!(unit.status, UnitStatus::Undeployed);

        // Check each model has weapons
        for model in &unit.models {
            assert_eq!(model.ranged_weapons.len(), 1);
            assert_eq!(model.melee_weapons.len(), 1);
            assert_eq!(model.wounds_max, Wounds::new(2));
            assert!(model.alive);
        }
    }
}

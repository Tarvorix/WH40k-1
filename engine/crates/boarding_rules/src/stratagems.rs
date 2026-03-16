//! Boarding Actions Universal Stratagems.
//!
//! Defines the 5 universal Boarding Actions stratagems as static data,
//! and provides lookup/availability helpers.
//!
//! Source: boarding_actions_complete_v3.md Section 4

use serde::{Deserialize, Serialize};
use wh40k_core_types::GameMode;

// ---------------------------------------------------------------------------
// Stratagem data structure
// ---------------------------------------------------------------------------

/// A Boarding Actions stratagem definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardingStratagem {
    /// Unique identifier for this stratagem.
    pub id: &'static str,
    /// Display name.
    pub name: &'static str,
    /// CP cost to use this stratagem.
    pub cp_cost: u8,
    /// When this stratagem can be used.
    pub timing: &'static str,
    /// Full description of the stratagem's effect.
    pub description: &'static str,
}

// ---------------------------------------------------------------------------
// Universal Boarding Actions Stratagems
// ---------------------------------------------------------------------------

/// The 5 universal Boarding Actions stratagems.
///
/// Source: boarding_actions_complete_v3.md Section 4
pub const UNIVERSAL_BA_STRATAGEMS: &[BoardingStratagem] = &[
    BoardingStratagem {
        id: "ba_command_reroll",
        name: "Command Re-roll",
        cp_cost: 1,
        timing: "Any phase, after a qualifying roll",
        description: "Re-roll one hit roll, wound roll, damage roll, saving throw, advance roll, charge roll, desperate escape test, hazardous test, or roll for number of attacks.",
    },
    BoardingStratagem {
        id: "ba_battlefield_command",
        name: "Battlefield Command",
        cp_cost: 1,
        timing: "Start or end of any phase",
        description: "Select one Leader unit and one friendly Bodyguard unit within 6\" that the Leader could normally join. Choose one of the Leader's Leader abilities; until the next Command phase, the Bodyguard unit is treated as being led by that Leader for that ability only.",
    },
    BoardingStratagem {
        id: "ba_counter_offensive",
        name: "Counter-Offensive",
        cp_cost: 2,
        timing: "Fight phase, after an enemy unit has fought",
        description: "Select one of your units within Engagement Range of an enemy unit that has not fought this phase. That unit fights next.",
    },
    BoardingStratagem {
        id: "ba_insane_bravery",
        name: "Insane Bravery",
        cp_cost: 1,
        timing: "Battle-shock step of your Command phase, immediately after failing a test",
        description: "The unit is treated as having passed the Battle-shock test; it is not Battle-shocked.",
    },
    BoardingStratagem {
        id: "ba_explosive_clearance",
        name: "Explosive Clearance",
        cp_cost: 1,
        timing: "Your Shooting phase",
        description: "Select one unit not selected to shoot. Pick one model with a Blast weapon; until end of phase: count models NOT visible when determining Blast target size, and attacks from that weapon can be allocated to models NOT visible to the attacker.",
    },
];

// ---------------------------------------------------------------------------
// Universal Boarding Actions Enhancement names
// ---------------------------------------------------------------------------

/// The 6 universal Boarding Actions enhancement names.
///
/// This mirrors the UNIVERSAL_BA_ENHANCEMENTS constant in validator.rs.
///
/// Source: boarding_actions_complete_v3.md Section 5
pub const UNIVERSAL_BA_ENHANCEMENT_NAMES: &[&str] = &[
    "Superior Boarding Tactics",
    "Close-Quarters Killer",
    "Peerless Leader",
    "Expert Breacher",
    "Personal Teleporter",
    "Trademark Weapon",
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Check whether a specific Boarding Actions stratagem is available in the
/// given game mode.
///
/// In Boarding Actions mode, only BA stratagems are available. Normal
/// core/Codex stratagems are not used.
///
/// Source: boarding_actions_complete_v3.md Section 4
/// "You cannot use normal core/Codex stratagem sets here."
pub fn is_ba_stratagem_available(stratagem_id: &str, game_mode: GameMode) -> bool {
    if game_mode != GameMode::BoardingActions {
        return false;
    }
    UNIVERSAL_BA_STRATAGEMS
        .iter()
        .any(|s| s.id == stratagem_id)
}

/// Get the full list of universal Boarding Actions stratagems.
pub fn get_universal_ba_stratagems() -> &'static [BoardingStratagem] {
    UNIVERSAL_BA_STRATAGEMS
}

/// Look up a specific stratagem by ID.
pub fn get_stratagem_by_id(stratagem_id: &str) -> Option<&'static BoardingStratagem> {
    UNIVERSAL_BA_STRATAGEMS
        .iter()
        .find(|s| s.id == stratagem_id)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stratagem_count() {
        assert_eq!(UNIVERSAL_BA_STRATAGEMS.len(), 5);
    }

    #[test]
    fn test_stratagem_ids_unique() {
        let ids: Vec<&str> = UNIVERSAL_BA_STRATAGEMS.iter().map(|s| s.id).collect();
        for (i, id) in ids.iter().enumerate() {
            for (j, other_id) in ids.iter().enumerate() {
                if i != j {
                    assert_ne!(id, other_id, "Duplicate stratagem ID: {}", id);
                }
            }
        }
    }

    #[test]
    fn test_stratagem_names() {
        let names: Vec<&str> = UNIVERSAL_BA_STRATAGEMS.iter().map(|s| s.name).collect();
        assert!(names.contains(&"Command Re-roll"));
        assert!(names.contains(&"Battlefield Command"));
        assert!(names.contains(&"Counter-Offensive"));
        assert!(names.contains(&"Insane Bravery"));
        assert!(names.contains(&"Explosive Clearance"));
    }

    #[test]
    fn test_stratagem_cp_costs() {
        let command_reroll = get_stratagem_by_id("ba_command_reroll").unwrap();
        assert_eq!(command_reroll.cp_cost, 1);

        let battlefield_command = get_stratagem_by_id("ba_battlefield_command").unwrap();
        assert_eq!(battlefield_command.cp_cost, 1);

        let counter_offensive = get_stratagem_by_id("ba_counter_offensive").unwrap();
        assert_eq!(counter_offensive.cp_cost, 2);

        let insane_bravery = get_stratagem_by_id("ba_insane_bravery").unwrap();
        assert_eq!(insane_bravery.cp_cost, 1);

        let explosive_clearance = get_stratagem_by_id("ba_explosive_clearance").unwrap();
        assert_eq!(explosive_clearance.cp_cost, 1);
    }

    #[test]
    fn test_is_ba_stratagem_available_in_ba_mode() {
        assert!(is_ba_stratagem_available("ba_command_reroll", GameMode::BoardingActions));
        assert!(is_ba_stratagem_available("ba_battlefield_command", GameMode::BoardingActions));
        assert!(is_ba_stratagem_available("ba_counter_offensive", GameMode::BoardingActions));
        assert!(is_ba_stratagem_available("ba_insane_bravery", GameMode::BoardingActions));
        assert!(is_ba_stratagem_available("ba_explosive_clearance", GameMode::BoardingActions));
    }

    #[test]
    fn test_is_ba_stratagem_not_available_in_combat_patrol() {
        assert!(!is_ba_stratagem_available("ba_command_reroll", GameMode::CombatPatrol));
        assert!(!is_ba_stratagem_available("ba_explosive_clearance", GameMode::CombatPatrol));
    }

    #[test]
    fn test_unknown_stratagem_not_available() {
        assert!(!is_ba_stratagem_available("nonexistent_stratagem", GameMode::BoardingActions));
        assert!(!is_ba_stratagem_available("core_command_reroll", GameMode::BoardingActions));
    }

    #[test]
    fn test_get_universal_ba_stratagems() {
        let stratagems = get_universal_ba_stratagems();
        assert_eq!(stratagems.len(), 5);
        assert_eq!(stratagems[0].id, "ba_command_reroll");
    }

    #[test]
    fn test_get_stratagem_by_id_found() {
        let strat = get_stratagem_by_id("ba_counter_offensive");
        assert!(strat.is_some());
        assert_eq!(strat.unwrap().name, "Counter-Offensive");
    }

    #[test]
    fn test_get_stratagem_by_id_not_found() {
        let strat = get_stratagem_by_id("nonexistent");
        assert!(strat.is_none());
    }

    #[test]
    fn test_enhancement_names_count() {
        assert_eq!(UNIVERSAL_BA_ENHANCEMENT_NAMES.len(), 6);
    }

    #[test]
    fn test_enhancement_names_content() {
        assert!(UNIVERSAL_BA_ENHANCEMENT_NAMES.contains(&"Superior Boarding Tactics"));
        assert!(UNIVERSAL_BA_ENHANCEMENT_NAMES.contains(&"Close-Quarters Killer"));
        assert!(UNIVERSAL_BA_ENHANCEMENT_NAMES.contains(&"Peerless Leader"));
        assert!(UNIVERSAL_BA_ENHANCEMENT_NAMES.contains(&"Expert Breacher"));
        assert!(UNIVERSAL_BA_ENHANCEMENT_NAMES.contains(&"Personal Teleporter"));
        assert!(UNIVERSAL_BA_ENHANCEMENT_NAMES.contains(&"Trademark Weapon"));
    }

    #[test]
    fn test_stratagem_serialization() {
        let strat = get_stratagem_by_id("ba_command_reroll").unwrap();
        let json = serde_json::to_string(strat).unwrap();
        assert!(json.contains("ba_command_reroll"));
        assert!(json.contains("Command Re-roll"));
        // Verify the JSON is well-formed by parsing it as a generic Value
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["id"], "ba_command_reroll");
        assert_eq!(value["cp_cost"], 1);
        assert_eq!(value["name"], "Command Re-roll");
    }
}

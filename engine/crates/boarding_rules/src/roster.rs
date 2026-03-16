use serde::{Deserialize, Serialize};

/// A complete Boarding Patrol army roster ready for validation and game creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardingPatrol {
    /// Name of the faction (e.g., "World Eaters")
    pub faction_name: String,
    /// Faction keyword (e.g., "WORLD EATERS")
    pub faction_keyword: String,
    /// Name of the selected detachment (e.g., "Boarding Butchers")
    pub detachment_name: String,
    /// Units selected for the patrol
    pub units: Vec<SelectedUnit>,
    /// Index of the unit designated as Warlord (into the units vec)
    pub warlord_index: Option<usize>,
    /// Selected enhancements (up to 2, each assigned to a CHARACTER unit)
    pub enhancements: Vec<EnhancementAssignment>,
    /// Total points cost of the roster
    pub total_points: u16,
}

/// A unit selected for the Boarding Patrol roster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedUnit {
    /// Name of the datasheet (must match a datasheet in the faction data)
    pub datasheet_name: String,
    /// Number of models in this unit
    pub model_count: u8,
    /// Points cost for this unit at this model count
    pub points: u16,
    /// Selected wargear options (indices into the datasheet's wargear_options)
    pub wargear_selections: Vec<usize>,
}

/// An enhancement assigned to a specific CHARACTER unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancementAssignment {
    /// Name of the enhancement
    pub enhancement_name: String,
    /// Index of the unit receiving this enhancement (into the units vec)
    pub unit_index: usize,
}

impl BoardingPatrol {
    /// Create a new empty patrol for a given faction and detachment.
    pub fn new(faction_name: String, faction_keyword: String, detachment_name: String) -> Self {
        Self {
            faction_name,
            faction_keyword,
            detachment_name,
            units: Vec::new(),
            warlord_index: None,
            enhancements: Vec::new(),
            total_points: 0,
        }
    }

    /// Add a unit to the roster and recalculate total points.
    pub fn add_unit(&mut self, unit: SelectedUnit) {
        self.total_points += unit.points;
        self.units.push(unit);
    }

    /// Remove a unit by index and recalculate total points.
    pub fn remove_unit(&mut self, index: usize) -> Option<SelectedUnit> {
        if index < self.units.len() {
            let unit = self.units.remove(index);
            self.total_points -= unit.points;
            // Fix warlord index if needed
            if let Some(wi) = self.warlord_index {
                if wi == index {
                    self.warlord_index = None;
                } else if wi > index {
                    self.warlord_index = Some(wi - 1);
                }
            }
            // Fix enhancement indices
            self.enhancements.retain(|e| e.unit_index != index);
            for e in &mut self.enhancements {
                if e.unit_index > index {
                    e.unit_index -= 1;
                }
            }
            Some(unit)
        } else {
            None
        }
    }

    /// Recalculate total points from all units.
    pub fn recalculate_points(&mut self) {
        self.total_points = self.units.iter().map(|u| u.points).sum();
    }
}

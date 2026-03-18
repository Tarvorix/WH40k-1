//! Unit and model state types.
//!
//! Source: implementation_v3.md Sections 8.3-8.4
//! Source: 40k_revised.md - Unit characteristics, Battle-shock, OC
//! Source: CP_Rules.md - Combat Patrol unit rules

use serde::{Deserialize, Serialize};

use wh40k_core_types::{
    AllocationStatus, ArmorSave, BaseSize, DatasheetId, EngagementStatus, FeelNoPain, Inches,
    InvulnerableSave, Keyword, KeywordSet, Leadership, ModelId, MoveCharacteristic,
    ObjectiveControl, PlayerId, Position, Toughness, UnitId, UnitStatus, WeaponProfile, Wounds,
};

// ---------------------------------------------------------------------------
// UnitState
// ---------------------------------------------------------------------------

/// The full runtime state of a unit on the battlefield.
///
/// Source: implementation_v3.md Section 8.3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitState {
    /// Unique identifier for this unit.
    pub id: UnitId,

    /// Which player owns this unit.
    pub owner: PlayerId,

    /// Display name (e.g., "Custodian Guard", "Khorne Berzerkers").
    pub name: String,

    /// Link to the datasheet template this unit was created from.
    pub datasheet_id: DatasheetId,

    /// All keywords for this unit (Infantry, Character, Battleline, faction, etc.).
    pub keywords: KeywordSet,

    /// The individual models composing this unit.
    pub models: Vec<ModelState>,

    /// Base movement characteristic (before modifiers).
    pub base_movement: MoveCharacteristic,

    /// Base toughness characteristic.
    pub base_toughness: Toughness,

    /// Base armor save characteristic.
    pub base_armor_save: ArmorSave,

    /// Invulnerable save, if any.
    pub invulnerable_save: Option<InvulnerableSave>,

    /// Base leadership characteristic (tested on 2D6, pass if <= Ld).
    pub base_leadership: Leadership,

    /// Base objective control characteristic.
    pub base_oc: ObjectiveControl,

    /// Current status (OnBattlefield, InStrategicReserves, Destroyed, Undeployed).
    pub status: UnitStatus,

    /// Whether this unit is currently battle-shocked.
    pub battle_shocked: bool,

    /// Whether this unit is below half starting strength.
    pub below_half_strength: bool,

    /// Current engagement status (Engaged or NotEngaged).
    pub engagement_status: EngagementStatus,

    /// If this unit has an attached leader, the leader's UnitId.
    /// Attached leaders are treated as part of the bodyguard unit for targeting.
    pub attached_leader: Option<UnitId>,

    /// If this unit acts as a bodyguard for a leader unit.
    pub bodyguard_for: Option<UnitId>,

    /// If this unit is in reserves, what type.
    pub reserve_type: Option<ReserveType>,

    /// Active wargear abilities (e.g., Praesidium Shield granting +1 wound).
    pub wargear_abilities: Vec<WargearAbilityState>,

    /// Enhancement OC override. If set, replaces base_oc when calculating effective OC.
    /// Used by enhancements like Fearsome Presence (OC 5) and Watchman of Terra (OC 4).
    /// Source: Frenzied_Reavers.md - Fearsome Presence, Custodes.md - Watchman of Terra
    pub enhancement_oc_override: Option<ObjectiveControl>,

    /// Whether the enhancement OC override requires engagement range.
    /// If true, the override only applies when the unit is engaged.
    /// Source: Custodes.md - Watchman of Terra (OC 4 in Engagement Range)
    pub enhancement_oc_requires_engaged: bool,

    /// Deadly Demise value (0 = none). When the last model is destroyed, roll D6.
    /// On 6, deal this many mortal wounds to all units within 6".
    /// Source: 40k_revised.md - Deadly Demise X
    pub deadly_demise: u8,

    /// Scouts pre-game move distance, if this unit has the Scouts ability.
    /// None means the unit does not have Scouts. Some(distance) is the max Scouts move.
    /// Source: 40k_revised.md §12.5 - "Scouts X\""
    pub scouts_distance: Option<Inches>,

    /// Whether this unit has the Infiltrators ability.
    /// If true, the unit can deploy anywhere on the battlefield >9" from enemy DZ
    /// and >9" from all enemy models.
    /// Source: 40k_revised.md §12.4 - Infiltrators
    pub has_infiltrators: bool,

    /// The starting model count for this unit (for half-strength calculations).
    starting_model_count: usize,
}

impl UnitState {
    /// Create a new UnitState with the given parameters.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: UnitId,
        owner: PlayerId,
        name: String,
        datasheet_id: DatasheetId,
        keywords: KeywordSet,
        models: Vec<ModelState>,
        base_movement: MoveCharacteristic,
        base_toughness: Toughness,
        base_armor_save: ArmorSave,
        invulnerable_save: Option<InvulnerableSave>,
        base_leadership: Leadership,
        base_oc: ObjectiveControl,
    ) -> Self {
        let starting_count = models.len();
        Self {
            id,
            owner,
            name,
            datasheet_id,
            keywords,
            models,
            base_movement,
            base_toughness,
            base_armor_save,
            invulnerable_save,
            base_leadership,
            base_oc,
            status: UnitStatus::Undeployed,
            battle_shocked: false,
            below_half_strength: false,
            engagement_status: EngagementStatus::NotEngaged,
            attached_leader: None,
            bodyguard_for: None,
            reserve_type: None,
            wargear_abilities: Vec::new(),
            enhancement_oc_override: None,
            enhancement_oc_requires_engaged: false,
            deadly_demise: 0,
            scouts_distance: None,
            has_infiltrators: false,
            starting_model_count: starting_count,
        }
    }

    /// Count of alive models in this unit.
    pub fn models_alive(&self) -> usize {
        self.models.iter().filter(|m| m.alive).count()
    }

    /// Whether this unit has been completely destroyed (all models dead).
    pub fn is_destroyed(&self) -> bool {
        self.status == UnitStatus::Destroyed || self.models_alive() == 0
    }

    /// Whether this unit is below half starting strength.
    /// A unit is below half if alive models < ceil(starting / 2).
    /// A single-model unit is below half when at half wounds or less.
    pub fn is_below_half_strength(&self) -> bool {
        let alive = self.models_alive();
        if self.starting_model_count <= 1 {
            // Single-model unit: below half strength when at half wounds or less
            if let Some(model) = self.models.first() {
                if model.alive {
                    let half_wounds = model.wounds_max.value().div_ceil(2);
                    return model.wounds_remaining.value() < half_wounds;
                }
            }
            return true; // model is dead
        }
        let half = self.starting_model_count.div_ceil(2);
        alive < half
    }

    /// The number of models this unit started with.
    pub fn starting_model_count(&self) -> usize {
        self.starting_model_count
    }

    /// Get the combined starting strength of an attached unit (leader + bodyguard).
    /// Source: 40k_revised.md §12.8
    pub fn attached_starting_strength(leader: &UnitState, bodyguard: &UnitState) -> usize {
        leader.starting_model_count as usize + bodyguard.starting_model_count as usize
    }

    /// Total wounds remaining across all alive models.
    pub fn total_wounds_remaining(&self) -> u16 {
        self.models
            .iter()
            .filter(|m| m.alive)
            .map(|m| m.wounds_remaining.value() as u16)
            .sum()
    }

    /// Effective Objective Control value.
    /// Battle-shocked units have OC 0.
    ///
    /// Source: 40k_revised.md - "Battle-shocked units... have an OC of 0"
    pub fn effective_oc(&self) -> ObjectiveControl {
        if self.battle_shocked {
            ObjectiveControl::ZERO
        } else if let Some(override_oc) = self.enhancement_oc_override {
            // Check if the override requires engagement
            if self.enhancement_oc_requires_engaged {
                if self.engagement_status == EngagementStatus::Engaged {
                    override_oc
                } else {
                    self.base_oc
                }
            } else {
                override_oc
            }
        } else {
            self.base_oc
        }
    }

    /// Check if this unit has a specific keyword.
    pub fn has_keyword(&self, keyword: Keyword) -> bool {
        self.keywords.has(keyword)
    }

    /// Check if this unit is a CHARACTER.
    pub fn is_character(&self) -> bool {
        self.keywords.has(Keyword::Character)
    }

    /// Check if this unit is INFANTRY.
    pub fn is_infantry(&self) -> bool {
        self.keywords.has(Keyword::Infantry)
    }

    /// Check if this unit is a MONSTER.
    pub fn is_monster(&self) -> bool {
        self.keywords.has(Keyword::Monster)
    }

    /// Check if this unit is BATTLELINE.
    pub fn is_battleline(&self) -> bool {
        self.keywords.has(Keyword::Battleline)
    }

    /// Check if this unit is currently on the battlefield.
    pub fn is_on_battlefield(&self) -> bool {
        self.status == UnitStatus::OnBattlefield
    }

    /// Check if this unit is in reserves.
    pub fn is_in_reserves(&self) -> bool {
        self.status == UnitStatus::InStrategicReserves
    }

    /// Update the below_half_strength flag based on current state.
    pub fn update_below_half_strength(&mut self) {
        self.below_half_strength = self.is_below_half_strength();
    }

    /// Get the first alive model's position (used as unit reference position).
    pub fn reference_position(&self) -> Option<Position> {
        self.models
            .iter()
            .find(|m| m.alive)
            .map(|m| m.position)
    }

    /// Get all alive models.
    pub fn alive_models(&self) -> Vec<&ModelState> {
        self.models.iter().filter(|m| m.alive).collect()
    }

    /// Get all alive models mutably.
    pub fn alive_models_mut(&mut self) -> Vec<&mut ModelState> {
        self.models.iter_mut().filter(|m| m.alive).collect()
    }

    /// Get a specific model by ID.
    pub fn model(&self, id: ModelId) -> Option<&ModelState> {
        self.models.iter().find(|m| m.id == id)
    }

    /// Get a mutable reference to a specific model by ID.
    pub fn model_mut(&mut self, id: ModelId) -> Option<&mut ModelState> {
        self.models.iter_mut().find(|m| m.id == id)
    }
}

// ---------------------------------------------------------------------------
// ModelState
// ---------------------------------------------------------------------------

/// The full runtime state of an individual model within a unit.
///
/// Source: implementation_v3.md Section 8.4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelState {
    /// Unique identifier for this model.
    pub id: ModelId,

    /// Which unit this model belongs to.
    pub unit_id: UnitId,

    /// Whether this model is alive.
    pub alive: bool,

    /// Maximum wounds this model can have.
    pub wounds_max: Wounds,

    /// Current wounds remaining.
    pub wounds_remaining: Wounds,

    /// Current position on the board.
    pub position: Position,

    /// Base size in millimeters (for movement and coherency).
    pub base_size: BaseSize,

    /// Ranged weapon profiles available to this model.
    pub ranged_weapons: Vec<WeaponProfile>,

    /// Melee weapon profiles available to this model.
    pub melee_weapons: Vec<WeaponProfile>,

    /// Current wound allocation status (for attack resolution ordering).
    pub allocation_status: AllocationStatus,

    /// Whether this model is the leader/CHARACTER of the unit (for targeting).
    pub is_leader: bool,

    /// Feel No Pain save value, if any.
    pub feel_no_pain: Option<FeelNoPain>,
}

impl ModelState {
    /// Create a new alive model at a given position.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ModelId,
        unit_id: UnitId,
        wounds: Wounds,
        position: Position,
        base_size: BaseSize,
        ranged_weapons: Vec<WeaponProfile>,
        melee_weapons: Vec<WeaponProfile>,
        is_leader: bool,
        feel_no_pain: Option<FeelNoPain>,
    ) -> Self {
        Self {
            id,
            unit_id,
            alive: true,
            wounds_max: wounds,
            wounds_remaining: wounds,
            position,
            base_size,
            ranged_weapons,
            melee_weapons,
            allocation_status: AllocationStatus::Unwounded,
            is_leader,
            feel_no_pain,
        }
    }

    /// Apply damage to this model. Returns true if the model was destroyed.
    pub fn apply_damage(&mut self, damage: u8) -> bool {
        if !self.alive {
            return false;
        }

        if damage >= self.wounds_remaining.value() {
            self.wounds_remaining = Wounds::ZERO;
            self.alive = false;
            true
        } else {
            self.wounds_remaining = Wounds::new(self.wounds_remaining.value() - damage);
            self.allocation_status = AllocationStatus::WoundedAllocated;
            false
        }
    }

    /// Heal this model by the given amount, up to max wounds.
    pub fn heal(&mut self, amount: u8) {
        if !self.alive {
            return;
        }
        let new_wounds = (self.wounds_remaining.value() + amount).min(self.wounds_max.value());
        self.wounds_remaining = Wounds::new(new_wounds);
        if self.wounds_remaining == self.wounds_max {
            self.allocation_status = AllocationStatus::Unwounded;
        }
    }

    /// Whether this model has taken any wounds (but is still alive).
    pub fn is_wounded(&self) -> bool {
        self.alive && self.wounds_remaining < self.wounds_max
    }

    /// Whether this model is at full health.
    pub fn is_at_full_health(&self) -> bool {
        self.alive && self.wounds_remaining == self.wounds_max
    }
}

// ---------------------------------------------------------------------------
// ReserveType
// ---------------------------------------------------------------------------

/// How a unit arrives from reserves.
///
/// Source: 40k_revised.md - Reserves, Strategic Reserves, Deep Strike
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReserveType {
    /// Strategic Reserves: arrives from a board edge.
    StrategicReserves,

    /// Deep Strike: can be placed anywhere >9" from enemies.
    DeepStrike,
}

// ---------------------------------------------------------------------------
// WargearAbilityState
// ---------------------------------------------------------------------------

/// Active wargear ability state on a unit.
///
/// Represents active wargear effects like Praesidium Shield (+1 wound per model).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WargearAbilityState {
    /// Display name of the wargear ability.
    pub name: String,

    /// Description of the effect.
    pub description: String,

    /// Whether this ability is currently active.
    pub active: bool,
}

impl WargearAbilityState {
    /// Create a new active wargear ability.
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            active: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Coherency Functions
// ---------------------------------------------------------------------------

/// Check if a unit is in coherency.
///
/// Rules (Source: 40k_revised.md §1.7):
/// - Single model units are always in coherency.
/// - 2-6 models: each model within 2" horizontally and 5" vertically of at least 1 other model.
/// - 7+ models: each model within 2" horizontally and 5" vertically of at least 2 other models.
///
/// Only alive models are considered. Dead models are ignored entirely.
pub fn check_unit_coherency(models: &[ModelState]) -> bool {
    let alive_positions: Vec<wh40k_geometry::ModelPosition> = models
        .iter()
        .filter(|m| m.alive)
        .map(|m| wh40k_geometry::ModelPosition::new(m.position, m.base_size))
        .collect();

    wh40k_geometry::check_coherency(&alive_positions).is_coherent()
}

/// Find models that need to be removed to restore coherency.
///
/// Returns indices into the original `models` slice (not filtered alive indices)
/// for models that should be removed. Removes the fewest models possible.
///
/// The algorithm works by iteratively finding the alive model that violates
/// coherency and has the fewest coherent neighbors, removing it, and repeating
/// until coherency is restored or only one model remains.
///
/// Models removed this way count as destroyed but do not trigger "when destroyed" rules.
///
/// Source: 40k_revised.md §1.7 — "the controlling player removes models (one at a time)
/// until coherency is restored"
pub fn models_to_remove_for_coherency(models: &[ModelState]) -> Vec<usize> {
    // Build a working set of (original_index, position, base_size, still_present)
    let mut working: Vec<(usize, wh40k_core_types::Position, wh40k_core_types::BaseSize, bool)> = models
        .iter()
        .enumerate()
        .map(|(i, m)| (i, m.position, m.base_size, m.alive))
        .collect();

    let mut removed_indices: Vec<usize> = Vec::new();

    loop {
        // Collect alive/present models into ModelPosition for coherency check
        let alive_entries: Vec<(usize, wh40k_geometry::ModelPosition)> = working
            .iter()
            .filter(|(_, _, _, present)| *present)
            .map(|(orig_idx, pos, base, _)| {
                (*orig_idx, wh40k_geometry::ModelPosition::new(*pos, *base))
            })
            .collect();

        let alive_count = alive_entries.len();

        // 0 or 1 models are always coherent
        if alive_count <= 1 {
            break;
        }

        let positions: Vec<wh40k_geometry::ModelPosition> =
            alive_entries.iter().map(|(_, mp)| *mp).collect();

        let result = wh40k_geometry::check_coherency(&positions);

        if result.is_coherent() {
            break;
        }

        // Find which models are violating coherency
        let violating = match &result {
            wh40k_geometry::CoherencyResult::OutOfCoherency { violating_models } => {
                violating_models.clone()
            }
            _ => break, // Coherent or SingleModel — should not reach here
        };

        // Among violating models, find the one with the fewest coherent neighbors.
        // This is the "worst offender" and removing it is most likely to help.
        let mut worst_violator_alive_idx: Option<usize> = None;
        let mut worst_neighbor_count = usize::MAX;

        for &alive_idx in &violating {
            let mut neighbor_count = 0;
            for (j, mp) in positions.iter().enumerate() {
                if j == alive_idx {
                    continue;
                }
                if wh40k_geometry::within_coherency_range_2d(
                    positions[alive_idx].position,
                    positions[alive_idx].base_size,
                    mp.position,
                    mp.base_size,
                ) {
                    neighbor_count += 1;
                }
            }
            if neighbor_count < worst_neighbor_count {
                worst_neighbor_count = neighbor_count;
                worst_violator_alive_idx = Some(alive_idx);
            }
        }

        // Remove the worst violator
        if let Some(alive_idx) = worst_violator_alive_idx {
            let orig_idx = alive_entries[alive_idx].0;
            working[orig_idx].3 = false; // Mark as no longer present
            removed_indices.push(orig_idx);
        } else {
            // Safety: should not happen, but break to avoid infinite loop
            break;
        }

        // Guard: if we've removed everything down to 1 model, stop
        let remaining = working.iter().filter(|(_, _, _, p)| *p).count();
        if remaining <= 1 {
            break;
        }

        // Additional guard: don't remove more models than exist minus one
        // (must leave at least one model alive)
        if removed_indices.len() >= models.iter().filter(|m| m.alive).count() - 1 {
            break;
        }
    }

    removed_indices
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_core_types::{
        AttackCount, ArmorPenetration, Damage, Inches, Skill, Strength, WeaponAbilitySet,
        WeaponId, WeaponType,
    };

    fn make_test_weapon(name: &str, weapon_type: WeaponType) -> WeaponProfile {
        WeaponProfile {
            id: WeaponId::new(0),
            name: name.to_string(),
            weapon_type,
            range: if weapon_type == WeaponType::Ranged {
                Inches::from_inches(24)
            } else {
                Inches::ZERO
            },
            attacks: AttackCount::Fixed(2),
            skill: Skill::THREE_PLUS,
            strength: Strength::new(4),
            ap: ArmorPenetration::MINUS_1,
            damage: Damage::Fixed(1),
            abilities: WeaponAbilitySet::new(),
        }
    }

    fn make_test_model(id: u32, unit_id: UnitId) -> ModelState {
        ModelState::new(
            ModelId::new(id),
            unit_id,
            Wounds::new(3),
            Position::from_inches(10, 10),
            BaseSize::MM32,
            vec![make_test_weapon("Boltgun", WeaponType::Ranged)],
            vec![make_test_weapon("Combat blade", WeaponType::Melee)],
            false,
            None,
        )
    }

    fn make_test_unit(id: u32, model_count: usize) -> UnitState {
        let unit_id = UnitId::new(id);
        let models: Vec<ModelState> = (0..model_count)
            .map(|i| make_test_model(id * 100 + i as u32, unit_id))
            .collect();

        UnitState::new(
            unit_id,
            PlayerId::new(0),
            "Test Unit".to_string(),
            DatasheetId::new(1),
            KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Battleline]),
            models,
            MoveCharacteristic::from_inches(6),
            Toughness::new(4),
            ArmorSave::THREE_PLUS,
            None,
            Leadership::new(7),
            ObjectiveControl::new(2),
        )
    }

    #[test]
    fn test_unit_models_alive() {
        let unit = make_test_unit(1, 5);
        assert_eq!(unit.models_alive(), 5);
        assert_eq!(unit.starting_model_count(), 5);
    }

    #[test]
    fn test_unit_is_destroyed() {
        let mut unit = make_test_unit(1, 3);
        assert!(!unit.is_destroyed());

        // Kill all models
        for model in &mut unit.models {
            model.alive = false;
        }
        assert!(unit.is_destroyed());
    }

    #[test]
    fn test_unit_is_destroyed_by_status() {
        let mut unit = make_test_unit(1, 3);
        unit.status = UnitStatus::Destroyed;
        assert!(unit.is_destroyed());
    }

    #[test]
    fn test_unit_below_half_strength_multi_model() {
        let mut unit = make_test_unit(1, 6);
        // 6 models, half = 3, below half means < 3 alive

        // 6 alive - not below half
        assert!(!unit.is_below_half_strength());

        // Kill 3: 3 alive - not below half (half of 6 = 3)
        unit.models[0].alive = false;
        unit.models[1].alive = false;
        unit.models[2].alive = false;
        assert!(!unit.is_below_half_strength());

        // Kill another: 2 alive - below half
        unit.models[3].alive = false;
        assert!(unit.is_below_half_strength());
    }

    #[test]
    fn test_unit_below_half_strength_single_model() {
        let mut unit = make_test_unit(1, 1);
        // Single model with 3 wounds: below half when < 2 wounds
        assert!(!unit.is_below_half_strength());

        unit.models[0].wounds_remaining = Wounds::new(2);
        assert!(!unit.is_below_half_strength());

        unit.models[0].wounds_remaining = Wounds::new(1);
        assert!(unit.is_below_half_strength());
    }

    #[test]
    fn test_unit_total_wounds_remaining() {
        let mut unit = make_test_unit(1, 3);
        // 3 models with 3 wounds each = 9 total
        assert_eq!(unit.total_wounds_remaining(), 9);

        unit.models[0].wounds_remaining = Wounds::new(1);
        assert_eq!(unit.total_wounds_remaining(), 7);

        unit.models[1].alive = false;
        assert_eq!(unit.total_wounds_remaining(), 4); // 1 + 3 = 4, dead model not counted
    }

    #[test]
    fn test_unit_effective_oc() {
        let mut unit = make_test_unit(1, 3);
        assert_eq!(unit.effective_oc().value(), 2);

        unit.battle_shocked = true;
        assert_eq!(unit.effective_oc().value(), 0);

        unit.battle_shocked = false;
        assert_eq!(unit.effective_oc().value(), 2);
    }

    #[test]
    fn test_unit_keyword_checks() {
        let unit = make_test_unit(1, 1);
        assert!(unit.has_keyword(Keyword::Infantry));
        assert!(unit.has_keyword(Keyword::Battleline));
        assert!(!unit.has_keyword(Keyword::Character));
        assert!(!unit.has_keyword(Keyword::Monster));

        assert!(unit.is_infantry());
        assert!(unit.is_battleline());
        assert!(!unit.is_character());
        assert!(!unit.is_monster());
    }

    #[test]
    fn test_unit_character() {
        let unit_id = UnitId::new(10);
        let model = ModelState::new(
            ModelId::new(100),
            unit_id,
            Wounds::new(5),
            Position::from_inches(5, 5),
            BaseSize::MM40,
            Vec::new(),
            Vec::new(),
            true,
            None,
        );

        let unit = UnitState::new(
            unit_id,
            PlayerId::new(0),
            "Test Character".to_string(),
            DatasheetId::new(2),
            KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Character]),
            vec![model],
            MoveCharacteristic::from_inches(6),
            Toughness::new(5),
            ArmorSave::TWO_PLUS,
            Some(InvulnerableSave::FOUR_PLUS),
            Leadership::new(6),
            ObjectiveControl::new(1),
        );

        assert!(unit.is_character());
        assert!(unit.is_infantry());
        assert!(!unit.is_monster());
    }

    #[test]
    fn test_model_apply_damage() {
        let mut model = make_test_model(1, UnitId::new(0));
        assert_eq!(model.wounds_remaining.value(), 3);
        assert!(model.alive);

        // Apply 1 damage - wounded but alive
        let destroyed = model.apply_damage(1);
        assert!(!destroyed);
        assert!(model.alive);
        assert_eq!(model.wounds_remaining.value(), 2);
        assert_eq!(model.allocation_status, AllocationStatus::WoundedAllocated);

        // Apply 2 more damage - destroyed
        let destroyed = model.apply_damage(2);
        assert!(destroyed);
        assert!(!model.alive);
        assert_eq!(model.wounds_remaining.value(), 0);
    }

    #[test]
    fn test_model_apply_overkill_damage() {
        let mut model = make_test_model(1, UnitId::new(0));
        // Apply 10 damage to 3-wound model
        let destroyed = model.apply_damage(10);
        assert!(destroyed);
        assert!(!model.alive);
        assert_eq!(model.wounds_remaining.value(), 0);
    }

    #[test]
    fn test_model_apply_damage_to_dead() {
        let mut model = make_test_model(1, UnitId::new(0));
        model.alive = false;
        let destroyed = model.apply_damage(1);
        assert!(!destroyed); // already dead, no change
    }

    #[test]
    fn test_model_heal() {
        let mut model = make_test_model(1, UnitId::new(0));
        model.wounds_remaining = Wounds::new(1);
        model.allocation_status = AllocationStatus::WoundedAllocated;

        model.heal(1);
        assert_eq!(model.wounds_remaining.value(), 2);
        assert_eq!(model.allocation_status, AllocationStatus::WoundedAllocated);

        model.heal(10); // Overheal capped at max
        assert_eq!(model.wounds_remaining.value(), 3);
        assert_eq!(model.allocation_status, AllocationStatus::Unwounded);
    }

    #[test]
    fn test_model_heal_dead() {
        let mut model = make_test_model(1, UnitId::new(0));
        model.alive = false;
        model.wounds_remaining = Wounds::ZERO;
        model.heal(3);
        // Dead models can't be healed
        assert!(!model.alive);
        assert_eq!(model.wounds_remaining.value(), 0);
    }

    #[test]
    fn test_model_wounded_and_full_health() {
        let mut model = make_test_model(1, UnitId::new(0));
        assert!(model.is_at_full_health());
        assert!(!model.is_wounded());

        model.apply_damage(1);
        assert!(!model.is_at_full_health());
        assert!(model.is_wounded());
    }

    #[test]
    fn test_unit_reference_position() {
        let mut unit = make_test_unit(1, 3);
        assert!(unit.reference_position().is_some());

        // Kill all models
        for model in &mut unit.models {
            model.alive = false;
        }
        assert!(unit.reference_position().is_none());
    }

    #[test]
    fn test_unit_alive_models() {
        let mut unit = make_test_unit(1, 5);
        assert_eq!(unit.alive_models().len(), 5);

        unit.models[0].alive = false;
        unit.models[2].alive = false;
        assert_eq!(unit.alive_models().len(), 3);
    }

    #[test]
    fn test_unit_model_lookup() {
        let unit = make_test_unit(1, 3);
        let model_id = unit.models[1].id;
        assert!(unit.model(model_id).is_some());
        assert_eq!(unit.model(model_id).unwrap().id, model_id);
        assert!(unit.model(ModelId::new(9999)).is_none());
    }

    #[test]
    fn test_unit_on_battlefield() {
        let mut unit = make_test_unit(1, 3);
        assert!(!unit.is_on_battlefield()); // starts Undeployed
        assert!(!unit.is_in_reserves());

        unit.status = UnitStatus::OnBattlefield;
        assert!(unit.is_on_battlefield());
        assert!(!unit.is_in_reserves());

        unit.status = UnitStatus::InStrategicReserves;
        assert!(!unit.is_on_battlefield());
        assert!(unit.is_in_reserves());
    }

    #[test]
    fn test_unit_update_below_half() {
        let mut unit = make_test_unit(1, 4);
        unit.update_below_half_strength();
        assert!(!unit.below_half_strength);

        unit.models[0].alive = false;
        unit.models[1].alive = false;
        unit.models[2].alive = false;
        unit.update_below_half_strength();
        assert!(unit.below_half_strength);
    }

    #[test]
    fn test_wargear_ability_state() {
        let ability = WargearAbilityState::new(
            "Praesidium Shield".to_string(),
            "+1 wound per model".to_string(),
        );
        assert!(ability.active);
        assert_eq!(ability.name, "Praesidium Shield");
    }

    #[test]
    fn test_unit_serialization() {
        let unit = make_test_unit(1, 2);
        let json = serde_json::to_string(&unit).unwrap();
        let back: UnitState = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, unit.id);
        assert_eq!(back.name, unit.name);
        assert_eq!(back.models.len(), 2);
        assert_eq!(back.starting_model_count(), 2);
    }

    #[test]
    fn test_below_half_strength_odd_count() {
        // 5 models: half = 3 (ceiling of 5/2), below half means < 3
        let mut unit = make_test_unit(1, 5);
        assert!(!unit.is_below_half_strength()); // 5 alive

        unit.models[0].alive = false;
        unit.models[1].alive = false;
        assert!(!unit.is_below_half_strength()); // 3 alive, not below (ceil(5/2)=3)

        unit.models[2].alive = false;
        assert!(unit.is_below_half_strength()); // 2 alive, below half
    }

    // ===== Coherency Tests =====

    fn make_model_at(id: u32, unit_id: UnitId, x: i32, y: i32) -> ModelState {
        ModelState::new(
            ModelId::new(id),
            unit_id,
            Wounds::new(3),
            Position::from_inches(x, y),
            BaseSize::MM32,
            Vec::new(),
            Vec::new(),
            false,
            None,
        )
    }

    #[test]
    fn test_coherency_single_model_always_coherent() {
        let unit_id = UnitId::new(1);
        let models = vec![make_model_at(1, unit_id, 10, 10)];
        assert!(check_unit_coherency(&models));
    }

    #[test]
    fn test_coherency_empty_is_coherent() {
        let models: Vec<ModelState> = vec![];
        assert!(check_unit_coherency(&models));
    }

    #[test]
    fn test_coherency_two_models_within_2_inches() {
        let unit_id = UnitId::new(1);
        let models = vec![
            make_model_at(1, unit_id, 10, 10),
            make_model_at(2, unit_id, 11, 10), // 1" away
        ];
        assert!(check_unit_coherency(&models));
    }

    #[test]
    fn test_coherency_two_models_beyond_2_inches() {
        let unit_id = UnitId::new(1);
        let models = vec![
            make_model_at(1, unit_id, 10, 10),
            make_model_at(2, unit_id, 20, 10), // 10" away
        ];
        assert!(!check_unit_coherency(&models));
    }

    #[test]
    fn test_coherency_three_models_chain() {
        let unit_id = UnitId::new(1);
        // A---B---C each 1" apart, chain is fine for 2-6 models (need 1 neighbor)
        let models = vec![
            make_model_at(1, unit_id, 10, 10),
            make_model_at(2, unit_id, 11, 10),
            make_model_at(3, unit_id, 12, 10),
        ];
        assert!(check_unit_coherency(&models));
    }

    #[test]
    fn test_coherency_ignores_dead_models() {
        let unit_id = UnitId::new(1);
        let mut models = vec![
            make_model_at(1, unit_id, 10, 10),
            make_model_at(2, unit_id, 20, 10), // far away, but dead
            make_model_at(3, unit_id, 11, 10), // close to model 1
        ];
        models[1].alive = false;
        assert!(check_unit_coherency(&models));
    }

    #[test]
    fn test_coherency_all_dead_is_coherent() {
        let unit_id = UnitId::new(1);
        let mut models = vec![
            make_model_at(1, unit_id, 10, 10),
            make_model_at(2, unit_id, 20, 20),
        ];
        models[0].alive = false;
        models[1].alive = false;
        // No alive models = trivially coherent (empty set)
        assert!(check_unit_coherency(&models));
    }

    #[test]
    fn test_coherency_seven_plus_needs_two_neighbors() {
        let unit_id = UnitId::new(1);
        // 7 models in a line, each 2" apart (so end models only have 1 neighbor)
        // Model 0 at (10,10) has neighbor at (12,10)=2" away only -> needs 2 -> incoherent
        let models: Vec<ModelState> = (0..7)
            .map(|i| make_model_at(i as u32, unit_id, 10 + (i as i32) * 2, 10))
            .collect();
        // End models (0 and 6) only have 1 neighbor each within 2", so this should fail
        assert!(!check_unit_coherency(&models));
    }

    #[test]
    fn test_coherency_seven_models_clustered() {
        let unit_id = UnitId::new(1);
        // 7 models all within 1" of each other (clustered)
        // Each has 6 neighbors -> satisfies the 2-neighbor requirement
        let models = vec![
            make_model_at(1, unit_id, 10, 10),
            make_model_at(2, unit_id, 10, 11),
            make_model_at(3, unit_id, 11, 10),
            make_model_at(4, unit_id, 11, 11),
            make_model_at(5, unit_id, 10, 9),
            make_model_at(6, unit_id, 9, 10),
            make_model_at(7, unit_id, 9, 11),
        ];
        assert!(check_unit_coherency(&models));
    }

    #[test]
    fn test_coherency_six_models_chain_is_ok() {
        let unit_id = UnitId::new(1);
        // 6 models in a chain (1" apart), only need 1 neighbor each
        let models: Vec<ModelState> = (0..6)
            .map(|i| make_model_at(i as u32, unit_id, 10 + i as i32, 10))
            .collect();
        assert!(check_unit_coherency(&models));
    }

    #[test]
    fn test_models_to_remove_already_coherent() {
        let unit_id = UnitId::new(1);
        let models = vec![
            make_model_at(1, unit_id, 10, 10),
            make_model_at(2, unit_id, 11, 10),
        ];
        let to_remove = models_to_remove_for_coherency(&models);
        assert!(to_remove.is_empty(), "Already coherent, nothing to remove");
    }

    #[test]
    fn test_models_to_remove_one_straggler() {
        let unit_id = UnitId::new(1);
        // 3 models: two close, one far away
        let models = vec![
            make_model_at(1, unit_id, 10, 10),
            make_model_at(2, unit_id, 11, 10), // close to model 0
            make_model_at(3, unit_id, 30, 30), // far from everyone
        ];
        let to_remove = models_to_remove_for_coherency(&models);
        assert_eq!(to_remove.len(), 1);
        assert_eq!(to_remove[0], 2, "The far model at index 2 should be removed");
    }

    #[test]
    fn test_models_to_remove_restores_coherency() {
        let unit_id = UnitId::new(1);
        // 4 models: two pairs far from each other
        let mut models = vec![
            make_model_at(1, unit_id, 10, 10),
            make_model_at(2, unit_id, 11, 10),
            make_model_at(3, unit_id, 30, 30),
            make_model_at(4, unit_id, 31, 30),
        ];
        assert!(!check_unit_coherency(&models));

        let to_remove = models_to_remove_for_coherency(&models);
        // Remove the indicated models
        for &idx in &to_remove {
            models[idx].alive = false;
        }
        // After removal, should be coherent
        assert!(check_unit_coherency(&models));
    }

    #[test]
    fn test_models_to_remove_single_model_nothing() {
        let unit_id = UnitId::new(1);
        let models = vec![make_model_at(1, unit_id, 10, 10)];
        let to_remove = models_to_remove_for_coherency(&models);
        assert!(to_remove.is_empty());
    }

    #[test]
    fn test_models_to_remove_seven_plus_line() {
        let unit_id = UnitId::new(1);
        // 7 models in a line, each 2" apart — end models only have 1 neighbor (needs 2)
        // After removal, should be coherent
        let mut models: Vec<ModelState> = (0..7)
            .map(|i| make_model_at(i as u32, unit_id, 10 + (i as i32) * 2, 10))
            .collect();
        assert!(!check_unit_coherency(&models));

        let to_remove = models_to_remove_for_coherency(&models);
        assert!(!to_remove.is_empty(), "Should need to remove models from a 7-model line");

        for &idx in &to_remove {
            models[idx].alive = false;
        }
        assert!(check_unit_coherency(&models), "Should be coherent after removal");
    }
}

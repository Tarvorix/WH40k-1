//! WH40K Engine - EvalHeuristic crate
//!
//! Heuristic position evaluator for the AI search engine.
//! Computes a scalar evaluation score from game state features using
//! weighted scoring terms derived from WH40K tactical principles.
//!
//! # Design
//!
//! Before NNUE becomes strong, this heuristic evaluator provides:
//! - Early engine strength for playable AI
//! - Search debugging and baseline comparisons
//! - Training bootstrap targets for NNUE
//! - Move ordering priors for alpha-beta efficiency
//!
//! # Heuristic Terms (from implementation_v3.md Section 13.2)
//!
//! 1. VP differential
//! 2. Projected scoring next turn
//! 3. Objective holding strength
//! 4. Kill potential
//! 5. Survival odds
//! 6. Leader exposure
//! 7. Reserve leverage
//! 8. Battle-shock pressure
//! 9. Charge threat
//! 10. Retaliation risk
//! 11. Mission-specific leverage
//! 12. CP utility remaining
//!
//! Source: implementation_v3.md Sections 13.1-13.3

use wh40k_core_types::{GameOutcome, PlayerId};
use wh40k_game_core::state::GameState;
use wh40k_eval_features::{
    extract_features, DistanceBucket, GlobalFeatures, ObjectiveFeatures,
    Score, UnitFeatures, SCORE_DRAW, SCORE_LOSS, SCORE_WIN,
};

// ============================================================================
// Evaluator Trait
// ============================================================================

/// Trait for position evaluators.
/// All evaluators (heuristic, NNUE, hybrid) implement this interface.
pub trait Evaluator {
    /// Evaluate the current game state from the perspective of the given player.
    ///
    /// Returns a Score where:
    /// - Positive values favor the perspective player
    /// - Negative values favor the opponent
    /// - SCORE_WIN / SCORE_LOSS for terminal states
    /// - Scale: ~100 per projected VP differential point
    fn evaluate(&self, state: &GameState, perspective: PlayerId) -> Score;

    /// Returns a descriptive name for this evaluator (for diagnostics).
    fn name(&self) -> &str;
}

// ============================================================================
// Heuristic Weights
// ============================================================================

/// Tunable weight vector for the heuristic evaluator.
/// Each weight controls the relative importance of a scoring term.
/// Weights are in centipawns-like units where ~100 = 1 projected VP.
///
/// Positive weights reward the perspective player for having the feature.
/// The default weights are tuned for Combat Patrol play.
#[derive(Debug, Clone)]
pub struct HeuristicWeights {
    // === Core VP/CP weights ===
    /// Weight per VP differential point. Direct measure of winning.
    pub vp_differential: Score,
    /// Weight for projected scoring opportunities next turn.
    pub projected_scoring: Score,
    /// Weight per CP differential. CP enables stratagems.
    pub cp_utility: Score,

    // === Objective control weights ===
    /// Weight per controlled objective (beyond enemy count).
    pub objective_control: Score,
    /// Bonus weight per OC advantage point at an objective.
    pub objective_holding_strength: Score,
    /// Weight for contesting an enemy-held objective.
    pub objective_contest: Score,

    // === Unit combat weights ===
    /// Weight per unit threat score point (offensive potential).
    pub kill_potential: Score,
    /// Weight per unit durability score point (defensive strength).
    pub survival_odds: Score,
    /// Weight per unit value score point (overall unit quality).
    pub unit_value: Score,

    // === Positional weights ===
    /// Penalty for exposed leaders/warlords (negative = penalty).
    pub leader_exposure: Score,
    /// Weight for units in reserves (potential future impact).
    pub reserve_leverage: Score,
    /// Weight per battle-shocked enemy unit (opponent debuff).
    pub battle_shock_pressure: Score,
    /// Weight for own battle-shocked units (self debuff, negative).
    pub own_battle_shock_penalty: Score,

    // === Threat weights ===
    /// Weight for charge threat potential (units able to charge high-value targets).
    pub charge_threat: Score,
    /// Penalty for being exposed to enemy charges (negative).
    pub retaliation_risk: Score,

    // === Mission weights ===
    /// Weight for mission-specific scoring leverage.
    pub mission_leverage: Score,

    // === Attrition weights ===
    /// Weight for model count advantage ratio.
    pub model_advantage: Score,
    /// Weight for wound count advantage ratio.
    pub wound_advantage: Score,

    // === Scaling factors ===
    /// How much to scale the early-game weights (rounds 1-2).
    pub early_game_scale: i32,
    /// How much to scale the mid-game weights (rounds 3-4).
    pub mid_game_scale: i32,
    /// How much to scale the late-game weights (round 5).
    pub late_game_scale: i32,
}

impl Default for HeuristicWeights {
    /// Default weights tuned for Combat Patrol games.
    /// These values are based on WH40K 10th Edition tactical principles:
    /// - VP is king: direct scoring is the most important factor
    /// - Objective control is the primary way to score VP
    /// - Combat effectiveness enables objective control
    /// - Positioning sets up future turns
    fn default() -> Self {
        Self {
            // Direct VP/CP
            vp_differential: 150,       // 1.5x per VP point (VP wins games)
            projected_scoring: 80,      // Projected future VP less certain
            cp_utility: 30,             // CP enables stratagems but not as direct

            // Objective control (key to scoring VP)
            objective_control: 120,     // Controlling objectives is critical
            objective_holding_strength: 15, // OC advantage matters for security
            objective_contest: 60,      // Contesting denies enemy VP

            // Combat effectiveness
            kill_potential: 8,          // Per-point of threat score
            survival_odds: 6,          // Per-point of durability
            unit_value: 4,             // Per-point of combined value

            // Positional
            leader_exposure: -50,       // Penalty for exposed leader
            reserve_leverage: 25,       // Reserves have future potential
            battle_shock_pressure: 40,  // Battle-shocked enemies are weakened
            own_battle_shock_penalty: -35, // Own battle-shock is bad

            // Threat
            charge_threat: 45,          // Charge threats are powerful
            retaliation_risk: -30,      // Exposure to charges is risky

            // Mission
            mission_leverage: 60,       // Mission-specific scoring matters

            // Attrition
            model_advantage: 50,        // Model count advantage matters
            wound_advantage: 30,        // Raw wound advantage

            // Game phase scaling (in thousandths, 1000 = 1.0x)
            early_game_scale: 1000,
            mid_game_scale: 1000,
            late_game_scale: 1000,
        }
    }
}

impl HeuristicWeights {
    /// Create a weight set optimized for aggressive play.
    /// Emphasizes kill potential, charge threats, and offensive stratagems.
    pub fn aggressive() -> Self {
        let mut w = Self::default();
        w.kill_potential = 12;
        w.charge_threat = 65;
        w.survival_odds = 4;
        w.retaliation_risk = -15;
        w.objective_control = 100;
        w
    }

    /// Create a weight set optimized for defensive/objective play.
    /// Emphasizes objective holding, survival, and VP scoring.
    pub fn defensive() -> Self {
        let mut w = Self::default();
        w.objective_control = 150;
        w.objective_holding_strength = 25;
        w.survival_odds = 10;
        w.kill_potential = 5;
        w.charge_threat = 25;
        w.retaliation_risk = -50;
        w
    }

    /// Get the game-phase scaling factor for the current round.
    /// Returns a multiplier in thousandths (1000 = 1.0x).
    pub fn phase_scale(&self, battle_round: u8) -> i32 {
        match battle_round {
            0 | 1 | 2 => self.early_game_scale,
            3 | 4 => self.mid_game_scale,
            _ => self.late_game_scale,
        }
    }
}

// ============================================================================
// Heuristic Evaluator
// ============================================================================

/// A rules-based heuristic evaluator using weighted scoring terms.
///
/// Extracts features from the game state and computes a weighted sum
/// of tactical indicators to produce a position evaluation score.
///
/// This evaluator is designed to be:
/// - Fast enough for deep search (no heap allocations in hot path)
/// - Accurate enough for strong baseline play
/// - Useful as move ordering prior for alpha-beta
/// - A training bootstrap target for NNUE
pub struct HeuristicEvaluator {
    /// The weight configuration.
    pub weights: HeuristicWeights,
}

impl HeuristicEvaluator {
    /// Create a new heuristic evaluator with the given weights.
    pub fn new(weights: HeuristicWeights) -> Self {
        Self { weights }
    }

    /// Create a heuristic evaluator with default weights.
    pub fn with_defaults() -> Self {
        Self::new(HeuristicWeights::default())
    }

    /// Compute a quick evaluation score for move ordering.
    /// This is a lighter-weight evaluation suitable for sorting
    /// candidate moves before full search.
    ///
    /// Returns a score where higher = more promising to search first.
    pub fn move_ordering_score(&self, state: &GameState, perspective: PlayerId) -> Score {
        // Use a subset of the full evaluation for speed
        let features = extract_features(state, perspective);
        let mut score: Score = 0;

        // VP differential is the strongest signal
        score += features.global.vp_differential as Score * 100;

        // Objective control advantage
        let obj_advantage = features.global.own_objectives_controlled as i32
            - features.global.enemy_objectives_controlled as i32;
        score += obj_advantage * 80;

        // Model advantage ratio
        if features.global.enemy_models_remaining > 0 {
            let own_ratio = features.global.own_model_ratio as i32;
            let enemy_ratio = features.global.enemy_model_ratio as i32;
            score += (own_ratio - enemy_ratio) / 5;
        }

        // Battle-shock advantage
        let shock_diff = features.global.enemy_battle_shocked_count as i32
            - features.global.own_battle_shocked_count as i32;
        score += shock_diff * 20;

        score
    }

    // ========================================================================
    // Individual heuristic term evaluators
    // ========================================================================

    /// Term 1: VP Differential
    /// Direct measure of who is winning. Each VP point is weighted heavily
    /// because VP is the win condition.
    fn eval_vp_differential(&self, global: &GlobalFeatures) -> Score {
        global.vp_differential as Score * self.weights.vp_differential
    }

    /// Term 2: Projected Scoring Next Turn
    /// Estimate how many VP the perspective player could score next turn
    /// based on current objective control and game state.
    fn eval_projected_scoring(
        &self,
        global: &GlobalFeatures,
        objectives: &[ObjectiveFeatures],
    ) -> Score {
        let mut projected_own_vp: i32 = 0;
        let mut projected_enemy_vp: i32 = 0;

        // Primary objective scoring: 5 VP per controlled objective (simplified)
        // In actual games, scoring is more nuanced, but this approximation works
        for obj in objectives {
            if obj.controller > 0 {
                // Perspective player controls this objective
                projected_own_vp += obj.vp_value as i32;
            } else if obj.controller < 0 {
                // Enemy controls this objective
                projected_enemy_vp += obj.vp_value as i32;
            }
            // Contested objectives contribute partially
            if obj.controller == 0 && obj.own_oc_in_range > 0 && obj.enemy_oc_in_range > 0 {
                // Split credit based on OC ratio
                let total_oc = obj.own_oc_in_range + obj.enemy_oc_in_range;
                if total_oc > 0 {
                    projected_own_vp += (obj.vp_value as i32 * obj.own_oc_in_range as i32)
                        / (total_oc as i32 * 2);
                    projected_enemy_vp += (obj.vp_value as i32 * obj.enemy_oc_in_range as i32)
                        / (total_oc as i32 * 2);
                }
            }
        }

        // Game progress bonus: scoring becomes more urgent in later rounds
        let urgency_bonus = match global.battle_round {
            1 | 2 => 0,
            3 => 10,
            4 => 20,
            _ => 30,
        };

        let projected_diff = projected_own_vp - projected_enemy_vp + urgency_bonus;
        projected_diff * self.weights.projected_scoring / 10
    }

    /// Term 3: Objective Holding Strength
    /// Rewards having more OC on objectives than the enemy.
    /// Secure objectives that can't easily be contested are more valuable.
    fn eval_objective_holding(
        &self,
        objectives: &[ObjectiveFeatures],
    ) -> Score {
        let mut score: Score = 0;

        for obj in objectives {
            // Controlled objectives contribute their OC advantage
            if obj.controller > 0 {
                // We control it
                score += self.weights.objective_control;
                score += obj.oc_advantage.max(0) as Score * self.weights.objective_holding_strength;

                // Bonus for no nearby enemies (secure hold)
                if obj.enemy_units_within_6 == 0 {
                    score += self.weights.objective_holding_strength * 2;
                }

                // Zone bonus: controlling enemy DZ objectives is harder and more valuable
                if obj.is_enemy_dz {
                    score += self.weights.objective_control / 2;
                }
            } else if obj.controller < 0 {
                // Enemy controls it - penalty
                score -= self.weights.objective_control;

                // But if we're contesting it, partial credit
                if obj.own_oc_in_range > 0 {
                    score += self.weights.objective_contest;
                }
            } else if obj.own_oc_in_range > 0 {
                // Contested or approaching - partial credit for presence
                score += self.weights.objective_contest / 2;
            }

            // Bonus for having units nearby even if not in range
            // (they can move onto it next turn)
            match obj.nearest_own_unit_distance {
                DistanceBucket::Engaged | DistanceBucket::MeleeReach => {
                    score += 15;
                }
                DistanceBucket::ShortCharge | DistanceBucket::MediumCharge => {
                    score += 8;
                }
                DistanceBucket::LongCharge => {
                    score += 4;
                }
                _ => {}
            }
        }

        score
    }

    /// Term 4: Kill Potential
    /// Rewards having units with high threat scores (offensive output).
    fn eval_kill_potential(&self, units: &[UnitFeatures]) -> Score {
        let mut own_threat: i32 = 0;
        let mut enemy_threat: i32 = 0;

        for unit in units {
            if !unit.is_on_battlefield {
                continue;
            }
            let threat = unit.threat_score as i32;
            if unit.is_own {
                own_threat += threat;
            } else {
                enemy_threat += threat;
            }
        }

        // Scale by health ratio - damaged units have less actual threat
        let own_effective = own_threat;
        let enemy_effective = enemy_threat;

        (own_effective - enemy_effective) * self.weights.kill_potential / 100
    }

    /// Term 5: Survival Odds
    /// Rewards having units with high durability scores.
    fn eval_survival_odds(&self, units: &[UnitFeatures]) -> Score {
        let mut own_durability: i32 = 0;
        let mut enemy_durability: i32 = 0;

        for unit in units {
            if !unit.is_on_battlefield {
                continue;
            }
            // Scale durability by remaining wounds ratio
            let effective_durability =
                (unit.durability_score as i32 * unit.wounds_remaining_ratio as i32) / 1000;
            if unit.is_own {
                own_durability += effective_durability;
            } else {
                enemy_durability += effective_durability;
            }
        }

        (own_durability - enemy_durability) * self.weights.survival_odds / 100
    }

    /// Term 6: Leader Exposure
    /// Penalizes having character/warlord units in vulnerable positions.
    /// Leaders are high-value targets that give away VP when killed.
    fn eval_leader_exposure(&self, units: &[UnitFeatures]) -> Score {
        let mut score: Score = 0;

        for unit in units {
            if !unit.is_on_battlefield || !unit.is_character {
                continue;
            }

            let exposure = self.compute_unit_exposure(unit);

            if unit.is_own {
                // Own leader exposed = bad
                if exposure > 0 {
                    score += self.weights.leader_exposure * exposure / 100;
                    // Extra penalty for warlord
                    if unit.is_warlord {
                        score += self.weights.leader_exposure * exposure / 200;
                    }
                }
            } else {
                // Enemy leader exposed = good for us
                if exposure > 0 {
                    score -= self.weights.leader_exposure * exposure / 100;
                    if unit.is_warlord {
                        score -= self.weights.leader_exposure * exposure / 200;
                    }
                }
            }
        }

        score
    }

    /// Compute an exposure score for a unit (0-100).
    /// Higher = more exposed to enemy threats.
    fn compute_unit_exposure(&self, unit: &UnitFeatures) -> i32 {
        let mut exposure: i32 = 0;

        // Close to enemies = exposed
        match unit.nearest_enemy_distance {
            DistanceBucket::Engaged => exposure += 80,
            DistanceBucket::MeleeReach => exposure += 60,
            DistanceBucket::ShortCharge => exposure += 50,
            DistanceBucket::MediumCharge => exposure += 35,
            DistanceBucket::LongCharge => exposure += 20,
            DistanceBucket::ShortRange => exposure += 10,
            _ => {}
        }

        // Low health = more vulnerable to being finished off
        if unit.wounds_remaining_ratio < 500 {
            exposure += 30;
        } else if unit.wounds_remaining_ratio < 750 {
            exposure += 15;
        }

        // Enemies in charge range = direct threat
        exposure += unit.enemies_in_charge_range as i32 * 10;

        // No nearby allies = isolated and vulnerable
        match unit.nearest_ally_distance {
            DistanceBucket::OutOfRange | DistanceBucket::LongRange | DistanceBucket::MediumRange => {
                exposure += 15;
            }
            _ => {}
        }

        exposure.min(100)
    }

    /// Term 7: Reserve Leverage
    /// Rewards having units in reserves that can arrive to make an impact.
    /// Reserves are more valuable in earlier rounds when they can still arrive.
    fn eval_reserve_leverage(
        &self,
        global: &GlobalFeatures,
        units: &[UnitFeatures],
    ) -> Score {
        let mut own_reserve_value: i32 = 0;
        let mut enemy_reserve_value: i32 = 0;

        for unit in units {
            if !unit.is_in_reserves {
                continue;
            }

            // Reserve value is based on unit quality
            let base_value = unit.value_score as i32;

            // Reserves are more valuable earlier in the game
            // (more turns to use them)
            let round_multiplier = match global.battle_round {
                1 => 120, // Can't arrive round 1, but setting up
                2 => 100, // First available arrival
                3 => 80,  // Still useful
                4 => 50,  // Running out of time
                _ => 20,  // Last round, limited impact
            };

            let value = base_value * round_multiplier / 100;

            if unit.is_own {
                own_reserve_value += value;
            } else {
                enemy_reserve_value += value;
            }
        }

        (own_reserve_value - enemy_reserve_value) * self.weights.reserve_leverage / 1000
    }

    /// Term 8: Battle-Shock Pressure
    /// Rewards having battle-shocked enemies (OC 0, can't use stratagems).
    /// Penalizes having own battle-shocked units.
    fn eval_battle_shock_pressure(
        &self,
        global: &GlobalFeatures,
        units: &[UnitFeatures],
    ) -> Score {
        let mut score: Score = 0;

        // Global count-based scoring
        score +=
            global.enemy_battle_shocked_count as Score * self.weights.battle_shock_pressure;
        score +=
            global.own_battle_shocked_count as Score * self.weights.own_battle_shock_penalty;

        // Per-unit scoring for battle-shocked units near objectives
        for unit in units {
            if !unit.is_battle_shocked || !unit.is_on_battlefield {
                continue;
            }

            // Battle-shocked units on objectives are especially impactful
            // (they have OC 0, so they can't hold)
            let on_objective = matches!(
                unit.nearest_objective_distance,
                DistanceBucket::Engaged | DistanceBucket::MeleeReach
            );

            if on_objective {
                if unit.is_own {
                    // Our battle-shocked unit on objective = very bad
                    score += self.weights.own_battle_shock_penalty;
                } else {
                    // Enemy battle-shocked unit on objective = very good
                    score += self.weights.battle_shock_pressure;
                }
            }
        }

        // Units near half strength could become battle-shocked
        // Apply mild pressure based on units close to the threshold
        for unit in units {
            if unit.is_battle_shocked || !unit.is_on_battlefield {
                continue;
            }
            // Near half-strength: 400-600 ratio
            if unit.wounds_remaining_ratio >= 400 && unit.wounds_remaining_ratio <= 600 {
                if unit.is_own {
                    // Our unit is vulnerable to battle-shock
                    score -= 10;
                } else {
                    // Enemy unit might become battle-shocked
                    score += 10;
                }
            }
        }

        score
    }

    /// Term 9: Charge Threat
    /// Rewards having units positioned to make impactful charges.
    /// Charges can swing objective control and deal massive damage.
    fn eval_charge_threat(&self, units: &[UnitFeatures]) -> Score {
        let mut own_charge_potential: i32 = 0;
        let mut enemy_charge_potential: i32 = 0;

        for unit in units {
            if !unit.is_on_battlefield || unit.is_battle_shocked {
                continue;
            }

            if !unit.can_charge || unit.enemies_in_charge_range == 0 {
                continue;
            }

            // Charge potential based on melee capability and number of targets
            let melee_strength = unit.total_melee_attacks as i32
                * unit.max_melee_strength.max(1) as i32;

            // Weight by number of chargeable targets
            let target_factor = unit.enemies_in_charge_range.min(3) as i32;

            // Closer targets are more likely to be reached
            let distance_factor = match unit.nearest_enemy_distance {
                DistanceBucket::MeleeReach | DistanceBucket::ShortCharge => 100,
                DistanceBucket::MediumCharge => 75,
                DistanceBucket::LongCharge => 40,
                _ => 10,
            };

            let potential = melee_strength * target_factor * distance_factor / 100;

            if unit.is_own {
                own_charge_potential += potential;
            } else {
                enemy_charge_potential += potential;
            }
        }

        let own_score = own_charge_potential * self.weights.charge_threat / 1000;
        let enemy_score = enemy_charge_potential * self.weights.retaliation_risk.abs() / 1000;

        own_score - enemy_score
    }

    /// Term 10: Retaliation Risk
    /// Penalizes leaving high-value units exposed to enemy counter-attacks.
    /// Complements the charge threat evaluation from the opponent's perspective.
    fn eval_retaliation_risk(&self, units: &[UnitFeatures]) -> Score {
        let mut score: Score = 0;

        for unit in units {
            if !unit.is_own || !unit.is_on_battlefield {
                continue;
            }

            // Check if this unit is exposed to enemy shooting or charges
            let shooting_exposure = unit.enemies_in_shooting_range.min(4) as i32;
            let charge_exposure = unit.enemies_in_charge_range.min(3) as i32;

            // Higher-value units being exposed is worse
            let value_weight = unit.value_score as i32;

            // Low health makes retaliation more dangerous
            let health_vulnerability = if unit.wounds_remaining_ratio < 300 {
                150 // Very vulnerable
            } else if unit.wounds_remaining_ratio < 500 {
                120
            } else if unit.wounds_remaining_ratio < 750 {
                100
            } else {
                80
            };

            let exposure_score = (shooting_exposure * 5 + charge_exposure * 15)
                * value_weight
                * health_vulnerability
                / 100000;

            score += exposure_score * self.weights.retaliation_risk / 100;
        }

        score
    }

    /// Term 11: Mission-Specific Leverage
    /// Evaluates position quality in terms of mission scoring potential.
    /// Different missions weight objectives and positions differently.
    fn eval_mission_leverage(
        &self,
        global: &GlobalFeatures,
        objectives: &[ObjectiveFeatures],
    ) -> Score {
        let mut score: Score = 0;

        // No Man's Land objectives are typically the most contested
        // and valuable for primary mission scoring
        let nml_controlled: i32 = objectives
            .iter()
            .filter(|o| o.is_no_mans_land && o.controller > 0)
            .count() as i32;
        let nml_enemy: i32 = objectives
            .iter()
            .filter(|o| o.is_no_mans_land && o.controller < 0)
            .count() as i32;

        score += (nml_controlled - nml_enemy) * self.weights.mission_leverage / 3;

        // Controlling more objectives than the enemy is important for
        // most mission primary objectives
        let obj_advantage = global.own_objectives_controlled as i32
            - global.enemy_objectives_controlled as i32;
        score += obj_advantage * self.weights.mission_leverage / 4;

        // In later rounds, having more units on the board matters more
        // for secondary objectives (e.g., Behind Enemy Lines, No Prisoners)
        if global.battle_round >= 3 {
            let unit_advantage = global.own_units_on_board as i32
                - global.enemy_units_on_board as i32;
            score += unit_advantage * 10;
        }

        score
    }

    /// Term 12: CP Utility
    /// Rewards having more command points than the opponent.
    /// CP enables stratagem usage which can be game-changing.
    fn eval_cp_utility(&self, global: &GlobalFeatures) -> Score {
        let cp_diff = global.cp_differential as Score;

        // Base CP value
        let mut score = cp_diff * self.weights.cp_utility;

        // CP is more valuable when the game is close
        // and when there are more rounds remaining to use them
        let rounds_remaining = 5_i32.saturating_sub(global.battle_round as i32).max(0);
        score += cp_diff * rounds_remaining * 5;

        score
    }

    /// Term 13: Model/Wound Advantage (Attrition)
    /// Evaluates the attrition race - who is losing fewer models/wounds
    /// relative to their starting strength.
    fn eval_attrition(&self, global: &GlobalFeatures) -> Score {
        let mut score: Score = 0;

        // Model ratio advantage
        let model_diff = global.own_model_ratio as i32 - global.enemy_model_ratio as i32;
        score += model_diff * self.weights.model_advantage / 1000;

        // Wound ratio advantage
        if global.own_wounds_remaining > 0 || global.enemy_wounds_remaining > 0 {
            let own_wounds = global.own_wounds_remaining as i32;
            let enemy_wounds = global.enemy_wounds_remaining as i32;
            let total = own_wounds + enemy_wounds;
            if total > 0 {
                // Advantage in raw wound count
                let wound_diff = own_wounds - enemy_wounds;
                score += wound_diff * self.weights.wound_advantage / total.max(1);
            }
        }

        // Tabling detection: if enemy is nearly tabled, large bonus
        if global.enemy_units_on_board <= 1 && global.own_units_on_board >= 3 {
            score += 300; // Near-tabling is extremely advantageous
        }
        if global.own_units_on_board <= 1 && global.enemy_units_on_board >= 3 {
            score -= 300;
        }

        score
    }

    /// Evaluate a terminal game state.
    /// Returns SCORE_WIN, SCORE_LOSS, or SCORE_DRAW for completed games.
    fn eval_terminal(&self, state: &GameState, perspective: PlayerId) -> Option<Score> {
        match state.game_outcome {
            GameOutcome::InProgress => None,
            GameOutcome::Victory(winner) => {
                if winner == perspective {
                    // Prefer faster wins (subtract round count)
                    Some(SCORE_WIN - state.battle_round.number() as Score)
                } else {
                    // Prefer slower losses (add round count)
                    Some(SCORE_LOSS + state.battle_round.number() as Score)
                }
            }
            GameOutcome::Draw => Some(SCORE_DRAW),
        }
    }
}

impl Evaluator for HeuristicEvaluator {
    fn evaluate(&self, state: &GameState, perspective: PlayerId) -> Score {
        // Check for terminal states first
        if let Some(terminal_score) = self.eval_terminal(state, perspective) {
            return terminal_score;
        }

        // Check for tabling
        if state.is_player_tabled(perspective) {
            return SCORE_LOSS + state.battle_round.number() as Score;
        }
        let opponent = state.opponent_id(perspective);
        if state.is_player_tabled(opponent) {
            return SCORE_WIN - state.battle_round.number() as Score;
        }

        // Extract features from game state
        let features = extract_features(state, perspective);

        // Compute all heuristic terms
        let term_vp = self.eval_vp_differential(&features.global);
        let term_projected = self.eval_projected_scoring(&features.global, &features.objectives);
        let term_objectives = self.eval_objective_holding(&features.objectives);
        let term_kill = self.eval_kill_potential(&features.units);
        let term_survival = self.eval_survival_odds(&features.units);
        let term_leader = self.eval_leader_exposure(&features.units);
        let term_reserves = self.eval_reserve_leverage(&features.global, &features.units);
        let term_shock = self.eval_battle_shock_pressure(&features.global, &features.units);
        let term_charge = self.eval_charge_threat(&features.units);
        let term_retaliation = self.eval_retaliation_risk(&features.units);
        let term_mission = self.eval_mission_leverage(&features.global, &features.objectives);
        let term_cp = self.eval_cp_utility(&features.global);
        let term_attrition = self.eval_attrition(&features.global);

        // Sum all terms
        let raw_score = term_vp
            + term_projected
            + term_objectives
            + term_kill
            + term_survival
            + term_leader
            + term_reserves
            + term_shock
            + term_charge
            + term_retaliation
            + term_mission
            + term_cp
            + term_attrition;

        // Apply game-phase scaling
        let phase_scale = self.weights.phase_scale(features.global.battle_round);
        let scaled_score = raw_score * phase_scale / 1000;

        // Clamp to valid range (between LOSS and WIN)
        scaled_score.clamp(SCORE_LOSS + 100, SCORE_WIN - 100)
    }

    fn name(&self) -> &str {
        "HeuristicEvaluator"
    }
}

// ============================================================================
// Evaluation Breakdown (for diagnostics)
// ============================================================================

/// Detailed breakdown of evaluation terms for diagnostics and debugging.
/// Useful for understanding why the evaluator prefers one position over another.
#[derive(Debug, Clone)]
pub struct EvalBreakdown {
    /// VP differential term.
    pub vp_differential: Score,
    /// Projected scoring term.
    pub projected_scoring: Score,
    /// Objective holding term.
    pub objective_holding: Score,
    /// Kill potential term.
    pub kill_potential: Score,
    /// Survival odds term.
    pub survival_odds: Score,
    /// Leader exposure term.
    pub leader_exposure: Score,
    /// Reserve leverage term.
    pub reserve_leverage: Score,
    /// Battle-shock pressure term.
    pub battle_shock_pressure: Score,
    /// Charge threat term.
    pub charge_threat: Score,
    /// Retaliation risk term.
    pub retaliation_risk: Score,
    /// Mission-specific leverage term.
    pub mission_leverage: Score,
    /// CP utility term.
    pub cp_utility: Score,
    /// Attrition term.
    pub attrition: Score,
    /// Final score after scaling and clamping.
    pub total: Score,
}

impl EvalBreakdown {
    /// Compute a full evaluation breakdown for diagnostics.
    pub fn compute(
        evaluator: &HeuristicEvaluator,
        state: &GameState,
        perspective: PlayerId,
    ) -> Self {
        let features = extract_features(state, perspective);

        let vp_differential = evaluator.eval_vp_differential(&features.global);
        let projected_scoring =
            evaluator.eval_projected_scoring(&features.global, &features.objectives);
        let objective_holding = evaluator.eval_objective_holding(&features.objectives);
        let kill_potential = evaluator.eval_kill_potential(&features.units);
        let survival_odds = evaluator.eval_survival_odds(&features.units);
        let leader_exposure = evaluator.eval_leader_exposure(&features.units);
        let reserve_leverage =
            evaluator.eval_reserve_leverage(&features.global, &features.units);
        let battle_shock_pressure =
            evaluator.eval_battle_shock_pressure(&features.global, &features.units);
        let charge_threat = evaluator.eval_charge_threat(&features.units);
        let retaliation_risk = evaluator.eval_retaliation_risk(&features.units);
        let mission_leverage =
            evaluator.eval_mission_leverage(&features.global, &features.objectives);
        let cp_utility = evaluator.eval_cp_utility(&features.global);
        let attrition = evaluator.eval_attrition(&features.global);

        let raw_total = vp_differential
            + projected_scoring
            + objective_holding
            + kill_potential
            + survival_odds
            + leader_exposure
            + reserve_leverage
            + battle_shock_pressure
            + charge_threat
            + retaliation_risk
            + mission_leverage
            + cp_utility
            + attrition;

        let phase_scale = evaluator.weights.phase_scale(features.global.battle_round);
        let total = (raw_total * phase_scale / 1000).clamp(SCORE_LOSS + 100, SCORE_WIN - 100);

        Self {
            vp_differential,
            projected_scoring,
            objective_holding,
            kill_potential,
            survival_odds,
            leader_exposure,
            reserve_leverage,
            battle_shock_pressure,
            charge_threat,
            retaliation_risk,
            mission_leverage,
            cp_utility,
            attrition,
            total,
        }
    }
}

impl std::fmt::Display for EvalBreakdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Evaluation Breakdown ===")?;
        writeln!(f, "  VP Differential:     {:+6}", self.vp_differential)?;
        writeln!(f, "  Projected Scoring:   {:+6}", self.projected_scoring)?;
        writeln!(f, "  Objective Holding:   {:+6}", self.objective_holding)?;
        writeln!(f, "  Kill Potential:      {:+6}", self.kill_potential)?;
        writeln!(f, "  Survival Odds:       {:+6}", self.survival_odds)?;
        writeln!(f, "  Leader Exposure:     {:+6}", self.leader_exposure)?;
        writeln!(f, "  Reserve Leverage:    {:+6}", self.reserve_leverage)?;
        writeln!(f, "  Battle-Shock:        {:+6}", self.battle_shock_pressure)?;
        writeln!(f, "  Charge Threat:       {:+6}", self.charge_threat)?;
        writeln!(f, "  Retaliation Risk:    {:+6}", self.retaliation_risk)?;
        writeln!(f, "  Mission Leverage:    {:+6}", self.mission_leverage)?;
        writeln!(f, "  CP Utility:          {:+6}", self.cp_utility)?;
        writeln!(f, "  Attrition:           {:+6}", self.attrition)?;
        writeln!(f, "  ───────────────────────────")?;
        writeln!(f, "  TOTAL:               {:+6}", self.total)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_weights() {
        let weights = HeuristicWeights::default();
        assert!(weights.vp_differential > 0);
        assert!(weights.objective_control > 0);
        assert!(weights.kill_potential > 0);
        assert!(weights.leader_exposure < 0);
        assert!(weights.retaliation_risk < 0);
    }

    #[test]
    fn test_aggressive_weights() {
        let aggressive = HeuristicWeights::aggressive();
        let default = HeuristicWeights::default();
        assert!(aggressive.kill_potential > default.kill_potential);
        assert!(aggressive.charge_threat > default.charge_threat);
    }

    #[test]
    fn test_defensive_weights() {
        let defensive = HeuristicWeights::defensive();
        let default = HeuristicWeights::default();
        assert!(defensive.objective_control > default.objective_control);
        assert!(defensive.survival_odds > default.survival_odds);
    }

    #[test]
    fn test_phase_scale() {
        let weights = HeuristicWeights::default();
        assert_eq!(weights.phase_scale(1), 1000);
        assert_eq!(weights.phase_scale(2), 1000);
        assert_eq!(weights.phase_scale(3), 1000);
        assert_eq!(weights.phase_scale(5), 1000);
    }

    #[test]
    fn test_evaluator_creation() {
        let eval = HeuristicEvaluator::with_defaults();
        assert_eq!(eval.name(), "HeuristicEvaluator");
    }

    #[test]
    fn test_evaluator_custom_weights() {
        let weights = HeuristicWeights::aggressive();
        let eval = HeuristicEvaluator::new(weights);
        assert_eq!(eval.weights.kill_potential, 12);
    }

    #[test]
    fn test_unit_exposure_calculation() {
        let eval = HeuristicEvaluator::with_defaults();

        // Create a mock unit feature that's exposed
        let exposed_unit = UnitFeatures {
            unit_id: wh40k_core_types::UnitId::new(1),
            is_own: true,
            models_remaining_ratio: 400,
            wounds_remaining_ratio: 400,
            effective_oc: 2,
            is_battle_shocked: false,
            is_engaged: false,
            is_in_reserves: false,
            is_on_battlefield: true,
            is_character: true,
            is_battleline: false,
            is_warlord: true,
            has_invulnerable_save: true,
            toughness: 5,
            best_save: 3,
            total_melee_attacks: 6,
            total_ranged_attacks: 0,
            max_weapon_range: 0,
            max_melee_strength: 6,
            max_ranged_strength: 0,
            best_ap: -2,
            max_damage: 3,
            threat_score: 400,
            durability_score: 300,
            value_score: 500,
            nearest_objective_distance: DistanceBucket::MeleeReach,
            nearest_enemy_distance: DistanceBucket::ShortCharge,
            nearest_ally_distance: DistanceBucket::LongRange,
            can_charge: true,
            enemies_in_shooting_range: 2,
            enemies_in_charge_range: 2,
            has_moved: false,
            has_shot: false,
            has_fought: false,
            charged_this_turn: false,
            advanced_this_turn: false,
            position: None,
        };

        let exposure = eval.compute_unit_exposure(&exposed_unit);
        // Should be high: close to enemies, low health, isolated
        assert!(exposure > 50, "Exposure should be > 50, got {}", exposure);
    }

    #[test]
    fn test_eval_breakdown_display() {
        let breakdown = EvalBreakdown {
            vp_differential: 300,
            projected_scoring: 80,
            objective_holding: 120,
            kill_potential: 50,
            survival_odds: 30,
            leader_exposure: -25,
            reserve_leverage: 10,
            battle_shock_pressure: 40,
            charge_threat: 45,
            retaliation_risk: -20,
            mission_leverage: 60,
            cp_utility: 30,
            attrition: 25,
            total: 745,
        };

        let display = format!("{}", breakdown);
        assert!(display.contains("VP Differential"));
        assert!(display.contains("TOTAL"));
        assert!(display.contains("+745"));
    }

    #[test]
    fn test_score_clamping() {
        // Verify the score constants are properly ordered
        assert!(SCORE_LOSS < SCORE_DRAW);
        assert!(SCORE_DRAW < SCORE_WIN);
        assert!(SCORE_LOSS + 100 < SCORE_WIN - 100);
    }

    #[test]
    fn test_eval_cp_utility_basic() {
        let eval = HeuristicEvaluator::with_defaults();
        let global_advantage = GlobalFeatures {
            battle_round: 2,
            phase: 0,
            vp_differential: 0,
            cp_differential: 2,
            own_models_remaining: 10,
            enemy_models_remaining: 10,
            own_model_ratio: 1000,
            enemy_model_ratio: 1000,
            own_wounds_remaining: 30,
            enemy_wounds_remaining: 30,
            own_reserves_count: 0,
            enemy_reserves_count: 0,
            own_battle_shocked_count: 0,
            enemy_battle_shocked_count: 0,
            own_objectives_controlled: 2,
            enemy_objectives_controlled: 2,
            contested_objectives: 1,
            own_total_oc: 8,
            enemy_total_oc: 8,
            is_first_player: true,
            game_progress: 400,
            own_units_on_board: 4,
            enemy_units_on_board: 4,
        };

        let score = eval.eval_cp_utility(&global_advantage);
        // With +2 CP differential, the score should be positive
        assert!(score > 0, "CP advantage should give positive score, got {}", score);
    }

    #[test]
    fn test_eval_attrition_advantage() {
        let eval = HeuristicEvaluator::with_defaults();

        // Perspective player has big model advantage
        let global = GlobalFeatures {
            battle_round: 3,
            phase: 0,
            vp_differential: 0,
            cp_differential: 0,
            own_models_remaining: 10,
            enemy_models_remaining: 3,
            own_model_ratio: 800,
            enemy_model_ratio: 300,
            own_wounds_remaining: 30,
            enemy_wounds_remaining: 10,
            own_reserves_count: 0,
            enemy_reserves_count: 0,
            own_battle_shocked_count: 0,
            enemy_battle_shocked_count: 0,
            own_objectives_controlled: 3,
            enemy_objectives_controlled: 1,
            contested_objectives: 0,
            own_total_oc: 8,
            enemy_total_oc: 2,
            is_first_player: true,
            game_progress: 600,
            own_units_on_board: 4,
            enemy_units_on_board: 1,
        };

        let score = eval.eval_attrition(&global);
        // Near-tabling bonus should kick in (enemy has 1 unit, we have 4)
        assert!(score > 200, "Near-tabling should give large bonus, got {}", score);
    }
}

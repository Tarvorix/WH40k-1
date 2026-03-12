//! WH40K Engine - Dice crate
//!
//! Deterministic dice subsystem that owns ALL random behavior in the engine.
//! Every random event is seeded, reproducible, logged, and auditable.
//!
//! # Architecture
//!
//! - [`SeedBundle`] holds the root seed and metadata for game-level determinism.
//! - [`DiceContext`] wraps a deterministic `SmallRng` with stream/context info
//!   for child seed derivation.
//! - [`DiceRoller`] provides all dice-rolling methods (d6, 2d6, d3, nd6, 5d6)
//!   and maintains a [`DiceLog`] for full audit trail.
//! - [`DiceRollRecord`] captures every roll with roll_id, context, values, and
//!   purpose for replay fidelity.
//!
//! # Determinism guarantee
//!
//! Same seed + same sequence of operations = same results, always.
//! ALL randomness MUST go through this system for replay fidelity.
//!
//! Source: implementation_v3.md Section 6.3 (Layer C - Deterministic dice and randomness)

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use wh40k_core_types::DiceRollId;

/// Create a SmallRng from a 32-byte seed in a platform-aware manner.
/// On wasm32, SmallRng uses Xoshiro128++ which needs a 16-byte seed,
/// so we truncate. On other platforms, SmallRng uses Xoshiro256++ (32 bytes).
fn small_rng_from_seed_32(seed: [u8; 32]) -> SmallRng {
    #[cfg(target_pointer_width = "32")]
    {
        let mut small_seed = [0u8; 16];
        small_seed.copy_from_slice(&seed[..16]);
        SmallRng::from_seed(small_seed)
    }
    #[cfg(not(target_pointer_width = "32"))]
    {
        SmallRng::from_seed(seed)
    }
}

// ---------------------------------------------------------------------------
// SeedBundle
// ---------------------------------------------------------------------------

/// Holds the root seed and metadata for game-level determinism.
/// Created once per game and stored in the game state / replay header.
///
/// The root seed is used to derive all child seeds throughout the game,
/// ensuring the entire game is reproducible from this single value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SeedBundle {
    /// 32-byte root seed for the game. All randomness derives from this.
    pub root_seed: [u8; 32],
    /// Human-readable label for the seed (e.g., "game-2026-03-11-001")
    pub label: String,
    /// Unix timestamp (seconds) when the seed was created.
    pub created_at: u64,
    /// Optional description of why this seed was chosen (e.g., "tournament round 3")
    pub description: Option<String>,
}

impl SeedBundle {
    /// Create a new SeedBundle from a root seed value.
    pub fn new(root_seed: [u8; 32], label: String, created_at: u64) -> Self {
        Self {
            root_seed,
            label,
            created_at,
            description: None,
        }
    }

    /// Create a SeedBundle from a simple u64 seed (for testing/convenience).
    /// The u64 is expanded to fill the 32-byte seed deterministically.
    pub fn from_u64(seed: u64, label: String) -> Self {
        let mut root_seed = [0u8; 32];
        let bytes = seed.to_le_bytes();
        // Fill all 32 bytes deterministically from the u64 seed.
        // Use a simple expansion: copy the 8 bytes, then XOR-shift to fill the rest.
        root_seed[0..8].copy_from_slice(&bytes);
        for i in 8..32 {
            // Mix bytes deterministically
            root_seed[i] = root_seed[i - 8]
                .wrapping_mul(37)
                .wrapping_add(root_seed[i - 1])
                .wrapping_add(i as u8);
        }
        Self {
            root_seed,
            label,
            created_at: 0,
            description: None,
        }
    }

    /// Create a SeedBundle with a description.
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Derive a child seed from this bundle for a specific stream.
    /// Uses the root seed mixed with the stream identifier to produce
    /// a deterministic but unique child seed.
    pub fn derive_child_seed(&self, stream_kind: &StreamKind, action_id: u32) -> [u8; 32] {
        derive_child_seed_from_bytes(&self.root_seed, stream_kind, action_id)
    }
}

/// Derive a 32-byte child seed by mixing a parent seed with stream and action info.
fn derive_child_seed_from_bytes(
    parent_seed: &[u8; 32],
    stream_kind: &StreamKind,
    action_id: u32,
) -> [u8; 32] {
    let mut child = [0u8; 32];

    // Encode the stream kind as a discriminant byte
    let stream_byte = match stream_kind {
        StreamKind::HitRoll => 1u8,
        StreamKind::WoundRoll => 2,
        StreamKind::SaveRoll => 3,
        StreamKind::DamageRoll => 4,
        StreamKind::AdvanceRoll => 5,
        StreamKind::ChargeRoll => 6,
        StreamKind::BattleShockTest => 7,
        StreamKind::BlessingsOfKhorne => 8,
        StreamKind::HazardousTest => 9,
        StreamKind::FeelNoPainRoll => 10,
        StreamKind::RandomAttacks => 11,
        StreamKind::RandomDamage => 12,
        StreamKind::MiscRoll => 13,
        StreamKind::SearchBranch => 14,
        StreamKind::DesperateEscape => 15,
    };

    let action_bytes = action_id.to_le_bytes();

    // Mix parent seed with stream and action info using a simple but effective scheme.
    // For each byte: parent[i] XOR (stream_byte rotated) XOR (action contribution)
    for i in 0..32 {
        let stream_contribution = stream_byte.wrapping_mul((i as u8).wrapping_add(1));
        let action_contribution = action_bytes[i % 4].wrapping_add(i as u8);
        child[i] = parent_seed[i]
            ^ stream_contribution
            ^ action_contribution;
    }

    // Additional mixing pass to improve avalanche properties
    for i in 0..32 {
        child[i] = child[i]
            .wrapping_mul(0x6D)
            .wrapping_add(child[(i + 17) % 32]);
    }

    child
}

// ---------------------------------------------------------------------------
// StreamKind - what kind of random event this is
// ---------------------------------------------------------------------------

/// Identifies the type of random event stream for child seed derivation.
/// Each stream kind produces independent random sequences even from the same root seed.
///
/// Source: implementation_v3.md Section 6.3 - stream_kind field
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StreamKind {
    /// Hit rolls in shooting or melee
    HitRoll,
    /// Wound rolls
    WoundRoll,
    /// Saving throws (armour and invulnerable)
    SaveRoll,
    /// Damage rolls (D3, D6, etc.)
    DamageRoll,
    /// Advance rolls (D6 added to movement)
    AdvanceRoll,
    /// Charge rolls (2D6)
    ChargeRoll,
    /// Battle-shock tests (2D6 vs Leadership)
    BattleShockTest,
    /// Blessings of Khorne (5D6 allocation)
    BlessingsOfKhorne,
    /// Hazardous weapon tests
    HazardousTest,
    /// Feel No Pain rolls
    FeelNoPainRoll,
    /// Random number of attacks (D6, D3, etc.)
    RandomAttacks,
    /// Random damage values
    RandomDamage,
    /// Miscellaneous rolls (first turn, etc.)
    MiscRoll,
    /// Search branch sampling (for AI search tree exploration)
    SearchBranch,
    /// Desperate escape tests
    DesperateEscape,
}

// ---------------------------------------------------------------------------
// RollPurpose - why a roll was made
// ---------------------------------------------------------------------------

/// Describes the game-rules purpose of a dice roll, for audit trail clarity.
/// This captures the "why" of each roll so replays and logs are human-readable.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RollPurpose {
    /// Hit roll for a specific weapon attack
    HitRoll {
        attacker_unit: u32,
        target_unit: u32,
        weapon_id: u32,
    },
    /// Wound roll after a successful hit
    WoundRoll {
        attacker_unit: u32,
        target_unit: u32,
        weapon_id: u32,
    },
    /// Saving throw by the defender
    SaveRoll {
        defender_unit: u32,
        model_id: u32,
        save_type: SaveRollType,
    },
    /// Damage roll for variable damage weapons
    DamageRoll {
        weapon_id: u32,
        damage_type: String,
    },
    /// Advance roll for movement
    AdvanceRoll {
        unit_id: u32,
    },
    /// Charge distance roll
    ChargeRoll {
        charging_unit: u32,
    },
    /// Battle-shock test
    BattleShockTest {
        unit_id: u32,
    },
    /// Blessings of Khorne allocation roll
    BlessingsOfKhorne,
    /// Hazardous weapon test
    HazardousTest {
        model_id: u32,
        weapon_id: u32,
    },
    /// Feel No Pain roll
    FeelNoPain {
        model_id: u32,
        wounds_to_ignore: u8,
    },
    /// Random number of attacks for a weapon
    RandomAttacks {
        weapon_id: u32,
    },
    /// Random damage for a weapon
    RandomDamage {
        weapon_id: u32,
    },
    /// Determine first turn
    DetermineFirstTurn,
    /// Desperate escape test
    DesperateEscape {
        unit_id: u32,
        model_id: u32,
    },
    /// Search branch sampling (AI internal)
    SearchSampling {
        branch_id: u32,
    },
    /// Fight on Death roll (Total Carnage blessing)
    /// Source: Frenzied_Reavers.md - "Total Carnage" blessing
    FightOnDeath {
        model_id: u32,
    },
    /// Ability test roll (e.g., Warrior Exemplar CP gain check)
    AbilityTest {
        ability_name: String,
    },
    /// Stratagem effect roll (e.g., Grenade mortal wounds, Tank Shock, Bloodlust move)
    StratagemEffect {
        stratagem_name: String,
        roll_index: u32,
    },
    /// Generic/miscellaneous roll
    Misc {
        description: String,
    },
}

/// Type of saving throw being rolled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SaveRollType {
    Armour,
    Invulnerable,
}

// ---------------------------------------------------------------------------
// DiceContext
// ---------------------------------------------------------------------------

/// Deterministic random context that encapsulates a `SmallRng` with metadata
/// for reproducibility and child seed derivation.
///
/// Every `DiceContext` is created from a seed and can derive child contexts
/// for sub-operations (e.g., a charge phase context spawning individual
/// charge roll contexts).
///
/// Source: implementation_v3.md Section 6.3
///
/// ```text
/// DiceContext
///   root_seed
///   stream_kind
///   state_fingerprint
///   action_id
///   resolution_index
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiceContext {
    /// The seed bytes that initialized this context's RNG.
    pub seed: [u8; 32],
    /// What kind of random stream this context represents.
    pub stream_kind: StreamKind,
    /// A fingerprint of the game state when this context was created,
    /// used for replay verification.
    pub state_fingerprint: u64,
    /// The action/command ID that triggered creation of this context.
    pub action_id: u32,
    /// Counter for sequential resolutions within this context.
    pub resolution_index: u32,
    /// The actual RNG. Not serialized; reconstructed from seed on deserialization.
    #[serde(skip)]
    rng: Option<SmallRng>,
}

impl DiceContext {
    /// Create a new DiceContext from explicit seed bytes.
    pub fn new(
        seed: [u8; 32],
        stream_kind: StreamKind,
        state_fingerprint: u64,
        action_id: u32,
    ) -> Self {
        let rng = small_rng_from_seed_32(seed);
        Self {
            seed,
            stream_kind,
            state_fingerprint,
            action_id,
            resolution_index: 0,
            rng: Some(rng),
        }
    }

    /// Create a DiceContext from a SeedBundle for a specific stream and action.
    pub fn from_bundle(
        bundle: &SeedBundle,
        stream_kind: StreamKind,
        state_fingerprint: u64,
        action_id: u32,
    ) -> Self {
        let seed = bundle.derive_child_seed(&stream_kind, action_id);
        Self::new(seed, stream_kind, state_fingerprint, action_id)
    }

    /// Derive a child context for a sub-operation.
    /// The child gets a new seed derived from this context's seed plus the
    /// child's stream kind and an action_id offset.
    pub fn derive_child(
        &self,
        stream_kind: StreamKind,
        child_action_id: u32,
        state_fingerprint: u64,
    ) -> DiceContext {
        let child_seed = derive_child_seed_from_bytes(&self.seed, &stream_kind, child_action_id);
        DiceContext::new(child_seed, stream_kind, state_fingerprint, child_action_id)
    }

    /// Ensure the RNG is initialized (needed after deserialization).
    fn ensure_rng(&mut self) {
        if self.rng.is_none() {
            self.rng = Some(small_rng_from_seed_32(self.seed));
            // Fast-forward through the number of resolutions already consumed
            // to restore the correct RNG state.
            let rng = self.rng.as_mut().unwrap();
            for _ in 0..self.resolution_index {
                let _: u32 = rng.gen();
            }
        }
    }

    /// Get the next random u32 from the RNG, advancing the resolution index.
    fn next_u32(&mut self) -> u32 {
        self.ensure_rng();
        self.resolution_index += 1;
        self.rng.as_mut().unwrap().gen()
    }

    /// Roll a single value in range [1, sides] (inclusive).
    fn roll_die(&mut self, sides: u8) -> u8 {
        let raw = self.next_u32();
        (raw % sides as u32) as u8 + 1
    }
}

impl PartialEq for DiceContext {
    fn eq(&self, other: &Self) -> bool {
        self.seed == other.seed
            && self.stream_kind == other.stream_kind
            && self.state_fingerprint == other.state_fingerprint
            && self.action_id == other.action_id
            && self.resolution_index == other.resolution_index
    }
}

impl Eq for DiceContext {}

// ---------------------------------------------------------------------------
// DiceRollRecord + DiceLog
// ---------------------------------------------------------------------------

/// A single recorded dice roll with full context for audit/replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiceRollRecord {
    /// Unique identifier for this roll in the dice log.
    pub roll_id: DiceRollId,
    /// What type of roll this is.
    pub stream_kind: StreamKind,
    /// The game-rules purpose of this roll.
    pub purpose: RollPurpose,
    /// The individual die values rolled (e.g., [3, 5] for 2D6).
    pub values: Vec<u8>,
    /// The total/result of the roll (sum for multi-die, or single value).
    pub total: u32,
    /// State fingerprint at the time of the roll.
    pub state_fingerprint: u64,
    /// Action ID that triggered this roll.
    pub action_id: u32,
    /// Resolution index within the context.
    pub resolution_index: u32,
}

/// Serializable audit trail of all dice rolls in a game.
/// Supports append, query, and serialization for replay verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiceLog {
    /// All recorded rolls in chronological order.
    records: Vec<DiceRollRecord>,
    /// Counter for generating sequential DiceRollIds.
    next_roll_id: u32,
}

impl DiceLog {
    /// Create a new empty dice log.
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            next_roll_id: 0,
        }
    }

    /// Record a dice roll and return the assigned DiceRollId.
    #[allow(clippy::too_many_arguments)]
    pub fn record(
        &mut self,
        stream_kind: StreamKind,
        purpose: RollPurpose,
        values: Vec<u8>,
        total: u32,
        state_fingerprint: u64,
        action_id: u32,
        resolution_index: u32,
    ) -> DiceRollId {
        let roll_id = DiceRollId::new(self.next_roll_id);
        self.next_roll_id += 1;

        self.records.push(DiceRollRecord {
            roll_id,
            stream_kind,
            purpose,
            values,
            total,
            state_fingerprint,
            action_id,
            resolution_index,
        });

        roll_id
    }

    /// Get all recorded rolls.
    pub fn records(&self) -> &[DiceRollRecord] {
        &self.records
    }

    /// Get a specific roll by its DiceRollId.
    pub fn get(&self, roll_id: DiceRollId) -> Option<&DiceRollRecord> {
        self.records.iter().find(|r| r.roll_id == roll_id)
    }

    /// Get the total number of rolls recorded.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Get all rolls matching a specific stream kind.
    pub fn rolls_by_stream(&self, stream_kind: StreamKind) -> Vec<&DiceRollRecord> {
        self.records
            .iter()
            .filter(|r| r.stream_kind == stream_kind)
            .collect()
    }

    /// Get all rolls for a specific action ID.
    pub fn rolls_by_action(&self, action_id: u32) -> Vec<&DiceRollRecord> {
        self.records
            .iter()
            .filter(|r| r.action_id == action_id)
            .collect()
    }

    /// Clear the log (for testing or when starting a new game segment).
    pub fn clear(&mut self) {
        self.records.clear();
        self.next_roll_id = 0;
    }

    /// Serialize the dice log to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize a dice log from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for DiceLog {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// DiceRoller
// ---------------------------------------------------------------------------

/// The primary interface for all dice rolling in the engine.
///
/// `DiceRoller` wraps a `DiceContext` and maintains a `DiceLog` so that every
/// roll is deterministic, auditable, and replayable.
///
/// All randomness in the game MUST go through a `DiceRoller` instance.
///
/// # Usage
///
/// ```
/// use wh40k_dice::*;
///
/// let bundle = SeedBundle::from_u64(42, "test".into());
/// let ctx = DiceContext::from_bundle(&bundle, StreamKind::MiscRoll, 0, 0);
/// let mut roller = DiceRoller::new(ctx);
///
/// let (value, _roll_id) = roller.roll_d6(RollPurpose::DetermineFirstTurn);
/// assert!(value >= 1 && value <= 6);
/// assert_eq!(roller.log().len(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiceRoller {
    /// The deterministic random context.
    context: DiceContext,
    /// Audit trail of all rolls made through this roller.
    log: DiceLog,
}

impl DiceRoller {
    /// Create a new DiceRoller with the given context and an empty log.
    pub fn new(context: DiceContext) -> Self {
        Self {
            context,
            log: DiceLog::new(),
        }
    }

    /// Create a new DiceRoller with the given context and an existing log.
    /// Useful for resuming from a saved state.
    pub fn with_log(context: DiceContext, log: DiceLog) -> Self {
        Self { context, log }
    }

    /// Create a DiceRoller directly from a SeedBundle for convenience.
    pub fn from_bundle(
        bundle: &SeedBundle,
        stream_kind: StreamKind,
        state_fingerprint: u64,
        action_id: u32,
    ) -> Self {
        let context = DiceContext::from_bundle(bundle, stream_kind, state_fingerprint, action_id);
        Self::new(context)
    }

    /// Get a reference to the dice log.
    pub fn log(&self) -> &DiceLog {
        &self.log
    }

    /// Get a mutable reference to the dice log.
    pub fn log_mut(&mut self) -> &mut DiceLog {
        &mut self.log
    }

    /// Take ownership of the dice log (consumes the roller).
    pub fn into_log(self) -> DiceLog {
        self.log
    }

    /// Get a reference to the current DiceContext.
    pub fn context(&self) -> &DiceContext {
        &self.context
    }

    /// Get the current state fingerprint.
    pub fn state_fingerprint(&self) -> u64 {
        self.context.state_fingerprint
    }

    /// Update the state fingerprint (call this when game state changes).
    pub fn set_state_fingerprint(&mut self, fingerprint: u64) {
        self.context.state_fingerprint = fingerprint;
    }

    /// Get the current action ID.
    pub fn action_id(&self) -> u32 {
        self.context.action_id
    }

    /// Update the action ID (call this when processing a new command/action).
    pub fn set_action_id(&mut self, action_id: u32) {
        self.context.action_id = action_id;
    }

    /// Derive a child DiceRoller for a sub-operation (e.g., search branch).
    /// The child gets its own independent context and empty log.
    pub fn derive_child(
        &self,
        stream_kind: StreamKind,
        child_action_id: u32,
        state_fingerprint: u64,
    ) -> DiceRoller {
        let child_ctx = self
            .context
            .derive_child(stream_kind, child_action_id, state_fingerprint);
        DiceRoller::new(child_ctx)
    }

    // -----------------------------------------------------------------------
    // Core roll methods
    // -----------------------------------------------------------------------

    /// Roll a single D6 (result: 1-6).
    /// Returns (value, roll_id).
    pub fn roll_d6(&mut self, purpose: RollPurpose) -> (u8, DiceRollId) {
        let resolution_index = self.context.resolution_index;
        let value = self.context.roll_die(6);
        let roll_id = self.log.record(
            self.context.stream_kind,
            purpose,
            vec![value],
            value as u32,
            self.context.state_fingerprint,
            self.context.action_id,
            resolution_index,
        );
        (value, roll_id)
    }

    /// Roll 2D6 and return the sum (result: 2-12).
    /// Returns (sum, individual_values, roll_id).
    pub fn roll_2d6(&mut self, purpose: RollPurpose) -> (u8, [u8; 2], DiceRollId) {
        let resolution_index = self.context.resolution_index;
        let d1 = self.context.roll_die(6);
        let d2 = self.context.roll_die(6);
        let total = d1 + d2;
        let roll_id = self.log.record(
            self.context.stream_kind,
            purpose,
            vec![d1, d2],
            total as u32,
            self.context.state_fingerprint,
            self.context.action_id,
            resolution_index,
        );
        (total, [d1, d2], roll_id)
    }

    /// Roll a D3 (result: 1-3).
    /// Implemented as (D6 + 1) / 2 per 40K convention: {1,2}->1, {3,4}->2, {5,6}->3.
    /// Returns (value, roll_id).
    pub fn roll_d3(&mut self, purpose: RollPurpose) -> (u8, DiceRollId) {
        let resolution_index = self.context.resolution_index;
        let raw_d6 = self.context.roll_die(6);
        let value = raw_d6.div_ceil(2); // {1,2}->1, {3,4}->2, {5,6}->3
        let roll_id = self.log.record(
            self.context.stream_kind,
            purpose,
            vec![raw_d6], // Log the raw D6 for full transparency
            value as u32,
            self.context.state_fingerprint,
            self.context.action_id,
            resolution_index,
        );
        (value, roll_id)
    }

    /// Roll N D6 dice and return all individual values.
    /// Returns (values_vec, sum, roll_id).
    pub fn roll_nd6(&mut self, n: u8, purpose: RollPurpose) -> (Vec<u8>, u32, DiceRollId) {
        let resolution_index = self.context.resolution_index;
        let mut values = Vec::with_capacity(n as usize);
        let mut total: u32 = 0;
        for _ in 0..n {
            let v = self.context.roll_die(6);
            total += v as u32;
            values.push(v);
        }
        let roll_id = self.log.record(
            self.context.stream_kind,
            purpose,
            values.clone(),
            total,
            self.context.state_fingerprint,
            self.context.action_id,
            resolution_index,
        );
        (values, total, roll_id)
    }

    /// Roll 5D6 specifically for Blessings of Khorne.
    /// Returns the 5 individual values (not summed, as they are allocated individually).
    /// Returns (values_array, roll_id).
    ///
    /// Source: Frenzied_Reavers.md - Blessings of Khorne
    /// "roll five D6 ... allocate those dice to one or two of the blessings"
    pub fn roll_5d6(&mut self, purpose: RollPurpose) -> ([u8; 5], DiceRollId) {
        let resolution_index = self.context.resolution_index;
        let mut values = [0u8; 5];
        let mut total: u32 = 0;
        for v in values.iter_mut() {
            *v = self.context.roll_die(6);
            total += *v as u32;
        }
        let roll_id = self.log.record(
            self.context.stream_kind,
            purpose,
            values.to_vec(),
            total,
            self.context.state_fingerprint,
            self.context.action_id,
            resolution_index,
        );
        (values, roll_id)
    }

    /// Roll multiple D6s for batch hit rolls, wound rolls, or save rolls.
    /// Each die is rolled independently and values are returned.
    /// This is equivalent to roll_nd6 but named for semantic clarity in combat resolution.
    /// Returns (values_vec, roll_id).
    pub fn roll_batch_d6(&mut self, count: u8, purpose: RollPurpose) -> (Vec<u8>, DiceRollId) {
        let (values, total, roll_id) = self.roll_nd6(count, purpose);
        let _ = total; // Total not used for batch rolls, individual values matter
        (values, roll_id)
    }

    /// Roll a D6 and add a modifier (e.g., D6+1 for damage).
    /// Returns (final_value, raw_d6, roll_id).
    pub fn roll_d6_plus(&mut self, modifier: u8, purpose: RollPurpose) -> (u8, u8, DiceRollId) {
        let resolution_index = self.context.resolution_index;
        let raw = self.context.roll_die(6);
        let result = raw + modifier;
        let roll_id = self.log.record(
            self.context.stream_kind,
            purpose,
            vec![raw],
            result as u32,
            self.context.state_fingerprint,
            self.context.action_id,
            resolution_index,
        );
        (result, raw, roll_id)
    }

    /// Roll a D3 and add a modifier (e.g., D3+1 for damage).
    /// Returns (final_value, raw_d6, roll_id).
    pub fn roll_d3_plus(&mut self, modifier: u8, purpose: RollPurpose) -> (u8, u8, DiceRollId) {
        let resolution_index = self.context.resolution_index;
        let raw_d6 = self.context.roll_die(6);
        let d3_value = raw_d6.div_ceil(2);
        let result = d3_value + modifier;
        let roll_id = self.log.record(
            self.context.stream_kind,
            purpose,
            vec![raw_d6],
            result as u32,
            self.context.state_fingerprint,
            self.context.action_id,
            resolution_index,
        );
        (result, raw_d6, roll_id)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a test roller with a known seed
    fn test_roller(seed: u64) -> DiceRoller {
        let bundle = SeedBundle::from_u64(seed, "test".into());
        let ctx = DiceContext::from_bundle(&bundle, StreamKind::MiscRoll, 0, 0);
        DiceRoller::new(ctx)
    }

    fn test_purpose() -> RollPurpose {
        RollPurpose::Misc {
            description: "test".into(),
        }
    }

    // =======================================================================
    // SeedBundle tests
    // =======================================================================

    #[test]
    fn test_seed_bundle_creation() {
        let bundle = SeedBundle::from_u64(42, "game-001".into());
        assert_eq!(bundle.label, "game-001");
        assert_eq!(bundle.created_at, 0);
        assert!(bundle.description.is_none());
    }

    #[test]
    fn test_seed_bundle_with_description() {
        let bundle =
            SeedBundle::from_u64(42, "game-001".into()).with_description("tournament".into());
        assert_eq!(bundle.description, Some("tournament".into()));
    }

    #[test]
    fn test_seed_bundle_from_full_seed() {
        let seed = [7u8; 32];
        let bundle = SeedBundle::new(seed, "full-seed".into(), 1234567890);
        assert_eq!(bundle.root_seed, [7u8; 32]);
        assert_eq!(bundle.created_at, 1234567890);
    }

    #[test]
    fn test_seed_bundle_deterministic_expansion() {
        let b1 = SeedBundle::from_u64(42, "a".into());
        let b2 = SeedBundle::from_u64(42, "b".into());
        // Same seed value should produce same root_seed bytes
        assert_eq!(b1.root_seed, b2.root_seed);
    }

    #[test]
    fn test_seed_bundle_different_seeds() {
        let b1 = SeedBundle::from_u64(42, "a".into());
        let b2 = SeedBundle::from_u64(43, "b".into());
        assert_ne!(b1.root_seed, b2.root_seed);
    }

    #[test]
    fn test_seed_bundle_serialization() {
        let bundle =
            SeedBundle::from_u64(42, "game-001".into()).with_description("test game".into());
        let json = serde_json::to_string(&bundle).unwrap();
        let back: SeedBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(bundle, back);
    }

    // =======================================================================
    // Child seed derivation tests
    // =======================================================================

    #[test]
    fn test_child_seed_derivation_deterministic() {
        let bundle = SeedBundle::from_u64(42, "test".into());
        let child1 = bundle.derive_child_seed(&StreamKind::HitRoll, 1);
        let child2 = bundle.derive_child_seed(&StreamKind::HitRoll, 1);
        assert_eq!(child1, child2);
    }

    #[test]
    fn test_child_seed_different_streams() {
        let bundle = SeedBundle::from_u64(42, "test".into());
        let child_hit = bundle.derive_child_seed(&StreamKind::HitRoll, 1);
        let child_wound = bundle.derive_child_seed(&StreamKind::WoundRoll, 1);
        assert_ne!(child_hit, child_wound);
    }

    #[test]
    fn test_child_seed_different_actions() {
        let bundle = SeedBundle::from_u64(42, "test".into());
        let child1 = bundle.derive_child_seed(&StreamKind::HitRoll, 1);
        let child2 = bundle.derive_child_seed(&StreamKind::HitRoll, 2);
        assert_ne!(child1, child2);
    }

    #[test]
    fn test_child_seed_different_parent_seeds() {
        let b1 = SeedBundle::from_u64(42, "a".into());
        let b2 = SeedBundle::from_u64(99, "b".into());
        let c1 = b1.derive_child_seed(&StreamKind::HitRoll, 1);
        let c2 = b2.derive_child_seed(&StreamKind::HitRoll, 1);
        assert_ne!(c1, c2);
    }

    // =======================================================================
    // DiceContext tests
    // =======================================================================

    #[test]
    fn test_dice_context_creation() {
        let bundle = SeedBundle::from_u64(42, "test".into());
        let ctx = DiceContext::from_bundle(&bundle, StreamKind::HitRoll, 12345, 7);
        assert_eq!(ctx.stream_kind, StreamKind::HitRoll);
        assert_eq!(ctx.state_fingerprint, 12345);
        assert_eq!(ctx.action_id, 7);
        assert_eq!(ctx.resolution_index, 0);
    }

    #[test]
    fn test_dice_context_derive_child() {
        let bundle = SeedBundle::from_u64(42, "test".into());
        let parent = DiceContext::from_bundle(&bundle, StreamKind::MiscRoll, 0, 0);
        let child = parent.derive_child(StreamKind::HitRoll, 5, 100);
        assert_eq!(child.stream_kind, StreamKind::HitRoll);
        assert_eq!(child.action_id, 5);
        assert_eq!(child.state_fingerprint, 100);
        assert_ne!(child.seed, parent.seed);
    }

    #[test]
    fn test_dice_context_produces_values_in_range() {
        let bundle = SeedBundle::from_u64(42, "test".into());
        let mut ctx = DiceContext::from_bundle(&bundle, StreamKind::MiscRoll, 0, 0);
        for _ in 0..1000 {
            let v = ctx.roll_die(6);
            assert!(v >= 1 && v <= 6, "D6 out of range: {}", v);
        }
    }

    #[test]
    fn test_dice_context_serialization_roundtrip() {
        let bundle = SeedBundle::from_u64(42, "test".into());
        let mut ctx = DiceContext::from_bundle(&bundle, StreamKind::HitRoll, 999, 5);
        // Make some rolls to advance state
        let v1 = ctx.roll_die(6);
        let v2 = ctx.roll_die(6);

        // Serialize
        let json = serde_json::to_string(&ctx).unwrap();
        // Deserialize
        let mut ctx2: DiceContext = serde_json::from_str(&json).unwrap();

        // After deserialization, the RNG should be reconstructed and produce the same next value
        let v3_original = ctx.roll_die(6);
        let v3_restored = ctx2.roll_die(6);
        assert_eq!(v3_original, v3_restored);

        // Verify we used the first two values correctly
        let _ = (v1, v2);
    }

    // =======================================================================
    // DiceRoller - roll_d6 tests
    // =======================================================================

    #[test]
    fn test_roll_d6_in_range() {
        let mut roller = test_roller(42);
        for _ in 0..1000 {
            let (value, _) = roller.roll_d6(test_purpose());
            assert!(value >= 1 && value <= 6, "D6 out of range: {}", value);
        }
    }

    #[test]
    fn test_roll_d6_logged() {
        let mut roller = test_roller(42);
        let (value, roll_id) = roller.roll_d6(test_purpose());
        assert_eq!(roller.log().len(), 1);
        let record = roller.log().get(roll_id).unwrap();
        assert_eq!(record.values, vec![value]);
        assert_eq!(record.total, value as u32);
        assert_eq!(record.stream_kind, StreamKind::MiscRoll);
    }

    // =======================================================================
    // DiceRoller - roll_2d6 tests
    // =======================================================================

    #[test]
    fn test_roll_2d6_in_range() {
        let mut roller = test_roller(42);
        for _ in 0..1000 {
            let (sum, [d1, d2], _) = roller.roll_2d6(test_purpose());
            assert!(d1 >= 1 && d1 <= 6, "D6 out of range: {}", d1);
            assert!(d2 >= 1 && d2 <= 6, "D6 out of range: {}", d2);
            assert_eq!(sum, d1 + d2);
            assert!(sum >= 2 && sum <= 12, "2D6 sum out of range: {}", sum);
        }
    }

    #[test]
    fn test_roll_2d6_logged() {
        let mut roller = test_roller(42);
        let (sum, [d1, d2], roll_id) = roller.roll_2d6(test_purpose());
        let record = roller.log().get(roll_id).unwrap();
        assert_eq!(record.values, vec![d1, d2]);
        assert_eq!(record.total, sum as u32);
    }

    // =======================================================================
    // DiceRoller - roll_d3 tests
    // =======================================================================

    #[test]
    fn test_roll_d3_in_range() {
        let mut roller = test_roller(42);
        for _ in 0..1000 {
            let (value, _) = roller.roll_d3(test_purpose());
            assert!(value >= 1 && value <= 3, "D3 out of range: {}", value);
        }
    }

    #[test]
    fn test_roll_d3_logged_with_raw_d6() {
        let mut roller = test_roller(42);
        let (d3_value, roll_id) = roller.roll_d3(test_purpose());
        let record = roller.log().get(roll_id).unwrap();
        // The raw D6 value should be stored
        let raw_d6 = record.values[0];
        assert!(raw_d6 >= 1 && raw_d6 <= 6);
        // Verify D3 conversion
        assert_eq!(d3_value, (raw_d6 + 1) / 2);
        // The total should be the D3 value
        assert_eq!(record.total, d3_value as u32);
    }

    #[test]
    fn test_roll_d3_conversion_correctness() {
        // Verify the D3 conversion formula: (d6 + 1) / 2
        assert_eq!((1u8 + 1) / 2, 1); // D6=1 -> D3=1
        assert_eq!((2u8 + 1) / 2, 1); // D6=2 -> D3=1
        assert_eq!((3u8 + 1) / 2, 2); // D6=3 -> D3=2
        assert_eq!((4u8 + 1) / 2, 2); // D6=4 -> D3=2
        assert_eq!((5u8 + 1) / 2, 3); // D6=5 -> D3=3
        assert_eq!((6u8 + 1) / 2, 3); // D6=6 -> D3=3
    }

    // =======================================================================
    // DiceRoller - roll_nd6 tests
    // =======================================================================

    #[test]
    fn test_roll_nd6_correct_count() {
        let mut roller = test_roller(42);
        for n in 1..=10u8 {
            let (values, _, _) = roller.roll_nd6(n, test_purpose());
            assert_eq!(values.len(), n as usize);
        }
    }

    #[test]
    fn test_roll_nd6_values_in_range() {
        let mut roller = test_roller(42);
        let (values, sum, _) = roller.roll_nd6(10, test_purpose());
        let mut expected_sum: u32 = 0;
        for &v in &values {
            assert!(v >= 1 && v <= 6, "D6 out of range: {}", v);
            expected_sum += v as u32;
        }
        assert_eq!(sum, expected_sum);
    }

    #[test]
    fn test_roll_nd6_zero_dice() {
        let mut roller = test_roller(42);
        let (values, sum, _) = roller.roll_nd6(0, test_purpose());
        assert!(values.is_empty());
        assert_eq!(sum, 0);
    }

    // =======================================================================
    // DiceRoller - roll_5d6 (Blessings of Khorne) tests
    // =======================================================================

    #[test]
    fn test_roll_5d6_returns_five_values() {
        let mut roller = test_roller(42);
        let (values, _) = roller.roll_5d6(RollPurpose::BlessingsOfKhorne);
        assert_eq!(values.len(), 5);
    }

    #[test]
    fn test_roll_5d6_values_in_range() {
        let mut roller = test_roller(42);
        for _ in 0..100 {
            let (values, _) = roller.roll_5d6(RollPurpose::BlessingsOfKhorne);
            for &v in &values {
                assert!(v >= 1 && v <= 6, "D6 out of range: {}", v);
            }
        }
    }

    #[test]
    fn test_roll_5d6_logged() {
        let mut roller = test_roller(42);
        let (values, roll_id) = roller.roll_5d6(RollPurpose::BlessingsOfKhorne);
        let record = roller.log().get(roll_id).unwrap();
        assert_eq!(record.values.len(), 5);
        assert_eq!(record.values, values.to_vec());
        let expected_total: u32 = values.iter().map(|&v| v as u32).sum();
        assert_eq!(record.total, expected_total);
        assert_eq!(record.stream_kind, StreamKind::MiscRoll);
    }

    // =======================================================================
    // DiceRoller - roll_d6_plus and roll_d3_plus tests
    // =======================================================================

    #[test]
    fn test_roll_d6_plus() {
        let mut roller = test_roller(42);
        for _ in 0..1000 {
            let (result, raw, _) = roller.roll_d6_plus(
                2,
                RollPurpose::DamageRoll {
                    weapon_id: 1,
                    damage_type: "D6+2".into(),
                },
            );
            assert!(raw >= 1 && raw <= 6);
            assert_eq!(result, raw + 2);
            assert!(result >= 3 && result <= 8);
        }
    }

    #[test]
    fn test_roll_d3_plus() {
        let mut roller = test_roller(42);
        for _ in 0..1000 {
            let (result, raw_d6, _) = roller.roll_d3_plus(
                1,
                RollPurpose::DamageRoll {
                    weapon_id: 1,
                    damage_type: "D3+1".into(),
                },
            );
            assert!(raw_d6 >= 1 && raw_d6 <= 6);
            let d3 = (raw_d6 + 1) / 2;
            assert_eq!(result, d3 + 1);
            assert!(result >= 2 && result <= 4);
        }
    }

    // =======================================================================
    // DiceRoller - batch roll tests
    // =======================================================================

    #[test]
    fn test_roll_batch_d6() {
        let mut roller = test_roller(42);
        let (values, _) = roller.roll_batch_d6(
            5,
            RollPurpose::HitRoll {
                attacker_unit: 1,
                target_unit: 2,
                weapon_id: 3,
            },
        );
        assert_eq!(values.len(), 5);
        for &v in &values {
            assert!(v >= 1 && v <= 6);
        }
    }

    // =======================================================================
    // Reproducibility / property tests
    // =======================================================================

    #[test]
    fn test_reproducibility_same_seed_same_results() {
        // This is the CRITICAL property: same seed + same sequence = same results
        let mut roller1 = test_roller(42);
        let mut roller2 = test_roller(42);

        for _ in 0..100 {
            let (v1, _) = roller1.roll_d6(test_purpose());
            let (v2, _) = roller2.roll_d6(test_purpose());
            assert_eq!(v1, v2, "Reproducibility violated!");
        }
    }

    #[test]
    fn test_reproducibility_2d6() {
        let mut r1 = test_roller(999);
        let mut r2 = test_roller(999);

        for _ in 0..100 {
            let (sum1, vals1, _) = r1.roll_2d6(test_purpose());
            let (sum2, vals2, _) = r2.roll_2d6(test_purpose());
            assert_eq!(sum1, sum2);
            assert_eq!(vals1, vals2);
        }
    }

    #[test]
    fn test_reproducibility_d3() {
        let mut r1 = test_roller(777);
        let mut r2 = test_roller(777);

        for _ in 0..100 {
            let (v1, _) = r1.roll_d3(test_purpose());
            let (v2, _) = r2.roll_d3(test_purpose());
            assert_eq!(v1, v2);
        }
    }

    #[test]
    fn test_reproducibility_nd6() {
        let mut r1 = test_roller(555);
        let mut r2 = test_roller(555);

        for n in 1..=10 {
            let (vals1, sum1, _) = r1.roll_nd6(n, test_purpose());
            let (vals2, sum2, _) = r2.roll_nd6(n, test_purpose());
            assert_eq!(vals1, vals2);
            assert_eq!(sum1, sum2);
        }
    }

    #[test]
    fn test_reproducibility_5d6() {
        let mut r1 = test_roller(333);
        let mut r2 = test_roller(333);

        for _ in 0..100 {
            let (vals1, _) = r1.roll_5d6(RollPurpose::BlessingsOfKhorne);
            let (vals2, _) = r2.roll_5d6(RollPurpose::BlessingsOfKhorne);
            assert_eq!(vals1, vals2);
        }
    }

    #[test]
    fn test_reproducibility_mixed_operations() {
        // Verify that a mixed sequence of different roll types is reproducible
        let mut r1 = test_roller(12345);
        let mut r2 = test_roller(12345);

        let (d6_1, _) = r1.roll_d6(test_purpose());
        let (d6_2, _) = r2.roll_d6(test_purpose());
        assert_eq!(d6_1, d6_2);

        let (sum1, _, _) = r1.roll_2d6(test_purpose());
        let (sum2, _, _) = r2.roll_2d6(test_purpose());
        assert_eq!(sum1, sum2);

        let (d3_1, _) = r1.roll_d3(test_purpose());
        let (d3_2, _) = r2.roll_d3(test_purpose());
        assert_eq!(d3_1, d3_2);

        let (vals1, _, _) = r1.roll_nd6(4, test_purpose());
        let (vals2, _, _) = r2.roll_nd6(4, test_purpose());
        assert_eq!(vals1, vals2);

        let (five1, _) = r1.roll_5d6(RollPurpose::BlessingsOfKhorne);
        let (five2, _) = r2.roll_5d6(RollPurpose::BlessingsOfKhorne);
        assert_eq!(five1, five2);
    }

    #[test]
    fn test_different_seeds_produce_different_results() {
        let mut r1 = test_roller(1);
        let mut r2 = test_roller(2);

        let mut same_count = 0;
        for _ in 0..100 {
            let (v1, _) = r1.roll_d6(test_purpose());
            let (v2, _) = r2.roll_d6(test_purpose());
            if v1 == v2 {
                same_count += 1;
            }
        }
        // With different seeds, we expect different sequences.
        // Probability of all 100 matching is essentially 0.
        assert!(
            same_count < 50,
            "Different seeds produced suspiciously similar results: {}/100 same",
            same_count
        );
    }

    // =======================================================================
    // DiceLog tests
    // =======================================================================

    #[test]
    fn test_dice_log_creation() {
        let log = DiceLog::new();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn test_dice_log_record_and_retrieve() {
        let mut log = DiceLog::new();
        let id = log.record(
            StreamKind::HitRoll,
            RollPurpose::HitRoll {
                attacker_unit: 1,
                target_unit: 2,
                weapon_id: 3,
            },
            vec![4],
            4,
            0,
            1,
            0,
        );
        assert_eq!(log.len(), 1);
        let record = log.get(id).unwrap();
        assert_eq!(record.values, vec![4]);
        assert_eq!(record.total, 4);
    }

    #[test]
    fn test_dice_log_sequential_ids() {
        let mut log = DiceLog::new();
        let id1 = log.record(StreamKind::HitRoll, test_purpose(), vec![3], 3, 0, 0, 0);
        let id2 = log.record(StreamKind::WoundRoll, test_purpose(), vec![5], 5, 0, 0, 1);
        assert_eq!(id1.raw(), 0);
        assert_eq!(id2.raw(), 1);
    }

    #[test]
    fn test_dice_log_filter_by_stream() {
        let mut log = DiceLog::new();
        log.record(StreamKind::HitRoll, test_purpose(), vec![3], 3, 0, 0, 0);
        log.record(StreamKind::WoundRoll, test_purpose(), vec![5], 5, 0, 0, 1);
        log.record(StreamKind::HitRoll, test_purpose(), vec![6], 6, 0, 0, 2);

        let hit_rolls = log.rolls_by_stream(StreamKind::HitRoll);
        assert_eq!(hit_rolls.len(), 2);

        let wound_rolls = log.rolls_by_stream(StreamKind::WoundRoll);
        assert_eq!(wound_rolls.len(), 1);

        let charge_rolls = log.rolls_by_stream(StreamKind::ChargeRoll);
        assert_eq!(charge_rolls.len(), 0);
    }

    #[test]
    fn test_dice_log_filter_by_action() {
        let mut log = DiceLog::new();
        log.record(StreamKind::HitRoll, test_purpose(), vec![3], 3, 0, 100, 0);
        log.record(StreamKind::WoundRoll, test_purpose(), vec![5], 5, 0, 100, 1);
        log.record(StreamKind::HitRoll, test_purpose(), vec![6], 6, 0, 200, 0);

        let action_100 = log.rolls_by_action(100);
        assert_eq!(action_100.len(), 2);

        let action_200 = log.rolls_by_action(200);
        assert_eq!(action_200.len(), 1);
    }

    #[test]
    fn test_dice_log_clear() {
        let mut log = DiceLog::new();
        log.record(StreamKind::HitRoll, test_purpose(), vec![3], 3, 0, 0, 0);
        log.record(StreamKind::WoundRoll, test_purpose(), vec![5], 5, 0, 0, 1);
        assert_eq!(log.len(), 2);

        log.clear();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn test_dice_log_serialization() {
        let mut roller = test_roller(42);
        roller.roll_d6(RollPurpose::DetermineFirstTurn);
        roller.roll_2d6(RollPurpose::BattleShockTest { unit_id: 1 });
        roller.roll_5d6(RollPurpose::BlessingsOfKhorne);

        let log = roller.log();
        let json = log.to_json().unwrap();
        let restored = DiceLog::from_json(&json).unwrap();

        assert_eq!(log.len(), restored.len());
        for (a, b) in log.records().iter().zip(restored.records().iter()) {
            assert_eq!(a.roll_id, b.roll_id);
            assert_eq!(a.values, b.values);
            assert_eq!(a.total, b.total);
            assert_eq!(a.stream_kind, b.stream_kind);
        }
    }

    // =======================================================================
    // DiceRoller state management tests
    // =======================================================================

    #[test]
    fn test_roller_state_fingerprint() {
        let mut roller = test_roller(42);
        assert_eq!(roller.state_fingerprint(), 0);

        roller.set_state_fingerprint(999);
        assert_eq!(roller.state_fingerprint(), 999);

        // Roll should record the current fingerprint
        let (_, roll_id) = roller.roll_d6(test_purpose());
        let record = roller.log().get(roll_id).unwrap();
        assert_eq!(record.state_fingerprint, 999);
    }

    #[test]
    fn test_roller_action_id() {
        let mut roller = test_roller(42);
        assert_eq!(roller.action_id(), 0);

        roller.set_action_id(42);
        assert_eq!(roller.action_id(), 42);

        let (_, roll_id) = roller.roll_d6(test_purpose());
        let record = roller.log().get(roll_id).unwrap();
        assert_eq!(record.action_id, 42);
    }

    // =======================================================================
    // Child roller tests (for search branches)
    // =======================================================================

    #[test]
    fn test_child_roller_independent() {
        let parent = test_roller(42);
        let mut child1 = parent.derive_child(StreamKind::SearchBranch, 1, 0);
        let mut child2 = parent.derive_child(StreamKind::SearchBranch, 2, 0);

        // Children with different action IDs should produce different sequences
        let mut same_count = 0;
        for _ in 0..50 {
            let (v1, _) = child1.roll_d6(test_purpose());
            let (v2, _) = child2.roll_d6(test_purpose());
            if v1 == v2 {
                same_count += 1;
            }
        }
        assert!(
            same_count < 40,
            "Child rollers with different IDs produced suspiciously similar results"
        );
    }

    #[test]
    fn test_child_roller_deterministic() {
        let parent = test_roller(42);
        let mut child_a = parent.derive_child(StreamKind::SearchBranch, 1, 0);
        let mut child_b = parent.derive_child(StreamKind::SearchBranch, 1, 0);

        // Same parent + same parameters = same child results
        for _ in 0..100 {
            let (v1, _) = child_a.roll_d6(test_purpose());
            let (v2, _) = child_b.roll_d6(test_purpose());
            assert_eq!(v1, v2, "Child roller determinism violated!");
        }
    }

    #[test]
    fn test_child_roller_does_not_affect_parent() {
        let mut parent1 = test_roller(42);
        let mut parent2 = test_roller(42);

        // Roll some dice from parent1, derive a child, roll from child
        let (p1_v1, _) = parent1.roll_d6(test_purpose());

        // Derive child from parent1 and roll from it
        let mut child = parent1.derive_child(StreamKind::SearchBranch, 1, 0);
        let _ = child.roll_d6(test_purpose());
        let _ = child.roll_d6(test_purpose());

        // Parent1's next roll should be the same as parent2's second roll
        let (p1_v2, _) = parent1.roll_d6(test_purpose());

        let (p2_v1, _) = parent2.roll_d6(test_purpose());
        let (p2_v2, _) = parent2.roll_d6(test_purpose());

        assert_eq!(p1_v1, p2_v1);
        assert_eq!(p1_v2, p2_v2);
    }

    // =======================================================================
    // Distribution tests (statistical sanity checks)
    // =======================================================================

    #[test]
    fn test_d6_distribution_roughly_uniform() {
        let mut roller = test_roller(42);
        let mut counts = [0u32; 6];
        let n = 6000;

        for _ in 0..n {
            let (v, _) = roller.roll_d6(test_purpose());
            counts[(v - 1) as usize] += 1;
        }

        // Each face should appear roughly n/6 = 1000 times.
        // With 6000 rolls, we expect ~1000 per face.
        // Allow a generous margin (+-300 from expected) for statistical variation.
        let expected = n / 6;
        for (i, &count) in counts.iter().enumerate() {
            let diff = (count as i32 - expected as i32).unsigned_abs();
            assert!(
                diff < 300,
                "D6 face {} appeared {} times (expected ~{}, diff {}). Distribution may be biased.",
                i + 1,
                count,
                expected,
                diff
            );
        }
    }

    #[test]
    fn test_d3_distribution_roughly_uniform() {
        let mut roller = test_roller(42);
        let mut counts = [0u32; 3];
        let n = 3000;

        for _ in 0..n {
            let (v, _) = roller.roll_d3(test_purpose());
            counts[(v - 1) as usize] += 1;
        }

        let expected = n / 3;
        for (i, &count) in counts.iter().enumerate() {
            let diff = (count as i32 - expected as i32).unsigned_abs();
            assert!(
                diff < 200,
                "D3 face {} appeared {} times (expected ~{}). Distribution may be biased.",
                i + 1,
                count,
                expected
            );
        }
    }

    // =======================================================================
    // RollPurpose tests
    // =======================================================================

    #[test]
    fn test_roll_purpose_serialization() {
        let purposes = vec![
            RollPurpose::HitRoll {
                attacker_unit: 1,
                target_unit: 2,
                weapon_id: 3,
            },
            RollPurpose::WoundRoll {
                attacker_unit: 4,
                target_unit: 5,
                weapon_id: 6,
            },
            RollPurpose::SaveRoll {
                defender_unit: 7,
                model_id: 8,
                save_type: SaveRollType::Invulnerable,
            },
            RollPurpose::AdvanceRoll { unit_id: 9 },
            RollPurpose::ChargeRoll { charging_unit: 10 },
            RollPurpose::BattleShockTest { unit_id: 11 },
            RollPurpose::BlessingsOfKhorne,
            RollPurpose::DetermineFirstTurn,
            RollPurpose::Misc {
                description: "test".into(),
            },
        ];

        for purpose in &purposes {
            let json = serde_json::to_string(purpose).unwrap();
            let back: RollPurpose = serde_json::from_str(&json).unwrap();
            assert_eq!(purpose, &back);
        }
    }

    // =======================================================================
    // StreamKind tests
    // =======================================================================

    #[test]
    fn test_stream_kind_serialization() {
        let kinds = vec![
            StreamKind::HitRoll,
            StreamKind::WoundRoll,
            StreamKind::SaveRoll,
            StreamKind::DamageRoll,
            StreamKind::AdvanceRoll,
            StreamKind::ChargeRoll,
            StreamKind::BattleShockTest,
            StreamKind::BlessingsOfKhorne,
            StreamKind::HazardousTest,
            StreamKind::FeelNoPainRoll,
            StreamKind::RandomAttacks,
            StreamKind::RandomDamage,
            StreamKind::MiscRoll,
            StreamKind::SearchBranch,
            StreamKind::DesperateEscape,
        ];

        for kind in &kinds {
            let json = serde_json::to_string(kind).unwrap();
            let back: StreamKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, &back);
        }
    }

    // =======================================================================
    // DiceRoller serialization tests
    // =======================================================================

    #[test]
    fn test_roller_serialization_preserves_log() {
        let mut roller = test_roller(42);
        roller.roll_d6(RollPurpose::DetermineFirstTurn);
        roller.roll_2d6(RollPurpose::ChargeRoll { charging_unit: 1 });
        roller.roll_d3(RollPurpose::DamageRoll {
            weapon_id: 1,
            damage_type: "D3".into(),
        });

        let json = serde_json::to_string(&roller).unwrap();
        let restored: DiceRoller = serde_json::from_str(&json).unwrap();

        assert_eq!(roller.log().len(), restored.log().len());
        for (a, b) in roller.log().records().iter().zip(restored.log().records().iter()) {
            assert_eq!(a.roll_id, b.roll_id);
            assert_eq!(a.values, b.values);
            assert_eq!(a.total, b.total);
        }
    }

    #[test]
    fn test_roller_serialization_continues_deterministically() {
        let mut roller = test_roller(42);
        // Roll some dice
        roller.roll_d6(test_purpose());
        roller.roll_d6(test_purpose());
        roller.roll_d6(test_purpose());

        // Serialize and deserialize
        let json = serde_json::to_string(&roller).unwrap();
        let mut restored: DiceRoller = serde_json::from_str(&json).unwrap();

        // Both should produce the same next values
        let (v1, _) = roller.roll_d6(test_purpose());
        let (v2, _) = restored.roll_d6(test_purpose());
        assert_eq!(v1, v2, "Roller deserialization broke determinism!");

        let (v1, _) = roller.roll_d6(test_purpose());
        let (v2, _) = restored.roll_d6(test_purpose());
        assert_eq!(v1, v2, "Roller deserialization broke determinism on second roll!");
    }

    // =======================================================================
    // DiceRollRecord tests
    // =======================================================================

    #[test]
    fn test_dice_roll_record_serialization() {
        let record = DiceRollRecord {
            roll_id: DiceRollId::new(42),
            stream_kind: StreamKind::HitRoll,
            purpose: RollPurpose::HitRoll {
                attacker_unit: 1,
                target_unit: 2,
                weapon_id: 3,
            },
            values: vec![4, 5, 6],
            total: 15,
            state_fingerprint: 999,
            action_id: 7,
            resolution_index: 3,
        };

        let json = serde_json::to_string(&record).unwrap();
        let back: DiceRollRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record, back);
    }

    // =======================================================================
    // into_log test
    // =======================================================================

    #[test]
    fn test_into_log() {
        let mut roller = test_roller(42);
        roller.roll_d6(test_purpose());
        roller.roll_d6(test_purpose());

        let log = roller.into_log();
        assert_eq!(log.len(), 2);
    }

    // =======================================================================
    // with_log test
    // =======================================================================

    #[test]
    fn test_with_log() {
        let mut roller = test_roller(42);
        roller.roll_d6(test_purpose());
        roller.roll_d6(test_purpose());
        let log = roller.into_log();

        // Create a new roller reusing the existing log
        let bundle = SeedBundle::from_u64(99, "new".into());
        let ctx = DiceContext::from_bundle(&bundle, StreamKind::MiscRoll, 0, 0);
        let mut roller2 = DiceRoller::with_log(ctx, log);
        assert_eq!(roller2.log().len(), 2);

        roller2.roll_d6(test_purpose());
        assert_eq!(roller2.log().len(), 3);
    }

    // =======================================================================
    // Comprehensive game-like scenario test
    // =======================================================================

    #[test]
    fn test_full_combat_scenario_reproducible() {
        // Simulate a simplified shooting phase:
        // 1. Roll for random attacks (D6)
        // 2. Roll hit rolls
        // 3. Roll wound rolls
        // 4. Roll saves
        // 5. Roll damage

        fn run_combat(seed: u64) -> Vec<u8> {
            let bundle = SeedBundle::from_u64(seed, "combat".into());
            let ctx = DiceContext::from_bundle(&bundle, StreamKind::HitRoll, 0, 0);
            let mut roller = DiceRoller::new(ctx);
            let mut results = Vec::new();

            // Step 1: Random attacks
            let (attacks, _) = roller.roll_d6(RollPurpose::RandomAttacks { weapon_id: 1 });
            results.push(attacks);

            // Step 2: Hit rolls
            for _ in 0..attacks {
                let (hit, _) = roller.roll_d6(RollPurpose::HitRoll {
                    attacker_unit: 1,
                    target_unit: 2,
                    weapon_id: 1,
                });
                results.push(hit);
            }

            // Step 3: Wound rolls (for hits that succeeded, say >= 3)
            let hits: Vec<u8> = results[1..]
                .iter()
                .filter(|&&v| v >= 3)
                .copied()
                .collect();
            for _ in 0..hits.len() {
                let (wound, _) = roller.roll_d6(RollPurpose::WoundRoll {
                    attacker_unit: 1,
                    target_unit: 2,
                    weapon_id: 1,
                });
                results.push(wound);
            }

            results
        }

        let r1 = run_combat(42);
        let r2 = run_combat(42);
        assert_eq!(r1, r2, "Full combat scenario not reproducible!");

        let r3 = run_combat(43);
        // Different seed should (very likely) produce different results
        // We just check they're not all the same
        assert_ne!(r1, r3, "Different seeds produced identical combat results");
    }

    // =======================================================================
    // Edge case tests
    // =======================================================================

    #[test]
    fn test_many_rolls_stay_deterministic() {
        let mut r1 = test_roller(42);
        let mut r2 = test_roller(42);

        // Roll 10,000 dice and verify every single one matches
        for i in 0..10_000 {
            let (v1, _) = r1.roll_d6(test_purpose());
            let (v2, _) = r2.roll_d6(test_purpose());
            assert_eq!(v1, v2, "Determinism broke at roll #{}", i);
        }
    }

    #[test]
    fn test_all_stream_kinds_produce_different_sequences() {
        let bundle = SeedBundle::from_u64(42, "test".into());
        let streams = vec![
            StreamKind::HitRoll,
            StreamKind::WoundRoll,
            StreamKind::SaveRoll,
            StreamKind::DamageRoll,
            StreamKind::AdvanceRoll,
            StreamKind::ChargeRoll,
            StreamKind::BattleShockTest,
            StreamKind::BlessingsOfKhorne,
        ];

        let mut sequences: Vec<Vec<u8>> = Vec::new();
        for stream in &streams {
            let ctx = DiceContext::from_bundle(&bundle, *stream, 0, 0);
            let mut roller = DiceRoller::new(ctx);
            let mut seq = Vec::new();
            for _ in 0..20 {
                let (v, _) = roller.roll_d6(test_purpose());
                seq.push(v);
            }
            sequences.push(seq);
        }

        // Verify that at least most pairs are different
        let mut different_pairs = 0;
        let total_pairs = sequences.len() * (sequences.len() - 1) / 2;
        for i in 0..sequences.len() {
            for j in (i + 1)..sequences.len() {
                if sequences[i] != sequences[j] {
                    different_pairs += 1;
                }
            }
        }
        assert_eq!(
            different_pairs, total_pairs,
            "Some stream kinds produced identical sequences"
        );
    }
}

//! WH40K Engine - Transposition crate
//!
//! Hash-indexed transposition table for caching search results.
//! Used by the search engine to avoid re-evaluating positions already explored.
//!
//! # Design
//!
//! - Fixed-size hash table with power-of-two sizing
//! - Uses game state hash (from hasher.rs) as key
//! - Stores evaluation score, depth, bound type, and best move index
//! - Replacement policy: always-replace (simplest, effective for game tree search)
//! - Thread-safe: uses atomic operations for lockless concurrent access (future)
//!
//! Source: implementation_v3.md Section 11.6 (Search algorithms)

use serde::{Deserialize, Serialize};

// ============================================================================
// TT Bound Type
// ============================================================================

/// The type of bound stored in a transposition table entry.
/// In alpha-beta search, not all nodes produce exact scores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TTBound {
    /// Exact score: the node was fully searched.
    Exact,
    /// Lower bound (beta cutoff): the true score is >= stored score.
    /// This means we found a move so good that the opponent wouldn't allow this position.
    LowerBound,
    /// Upper bound (fail-low): the true score is <= stored score.
    /// No move improved alpha, so the score is at most this value.
    UpperBound,
}

// ============================================================================
// TT Entry
// ============================================================================

/// A single entry in the transposition table.
/// Stores cached evaluation information for a game position.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TTEntry {
    /// Full hash key of the game state for verification.
    /// Used to detect hash collisions.
    pub hash_key: u64,
    /// The evaluation score at this position.
    pub score: i32,
    /// The search depth at which this entry was computed.
    /// Higher depth = more reliable.
    pub depth: u8,
    /// The type of bound (exact, lower, upper).
    pub bound: TTBound,
    /// Index of the best move found at this position.
    /// References into the candidate list at this node.
    /// u16::MAX means no best move recorded.
    pub best_move_index: u16,
    /// The search generation when this entry was written.
    /// Used for aging: entries from old searches are replaceable.
    pub generation: u16,
}

impl TTEntry {
    /// Create a new TTEntry.
    pub fn new(
        hash_key: u64,
        score: i32,
        depth: u8,
        bound: TTBound,
        best_move_index: u16,
        generation: u16,
    ) -> Self {
        Self {
            hash_key,
            score,
            depth,
            bound,
            best_move_index,
            generation,
        }
    }

    /// Check if this entry has a valid best move index.
    pub fn has_best_move(&self) -> bool {
        self.best_move_index != u16::MAX
    }
}

impl Default for TTEntry {
    fn default() -> Self {
        Self {
            hash_key: 0,
            score: 0,
            depth: 0,
            bound: TTBound::UpperBound,
            best_move_index: u16::MAX,
            generation: 0,
        }
    }
}

// ============================================================================
// TT Probe Result
// ============================================================================

/// Result of probing the transposition table.
#[derive(Debug, Clone, Copy)]
pub enum TTProbeResult {
    /// No entry found for this hash (cache miss).
    Miss,
    /// Entry found and can provide a cutoff (score is usable).
    Hit(TTEntry),
    /// Entry found but from insufficient depth (still useful for move ordering).
    ShallowHit(TTEntry),
}

impl TTProbeResult {
    /// Returns true if the probe found an entry (hit or shallow hit).
    pub fn found(&self) -> bool {
        !matches!(self, TTProbeResult::Miss)
    }

    /// Returns the entry if found, regardless of depth.
    pub fn entry(&self) -> Option<&TTEntry> {
        match self {
            TTProbeResult::Hit(e) | TTProbeResult::ShallowHit(e) => Some(e),
            TTProbeResult::Miss => None,
        }
    }

    /// Returns the best move index if an entry was found.
    pub fn best_move_index(&self) -> Option<u16> {
        self.entry()
            .filter(|e| e.has_best_move())
            .map(|e| e.best_move_index)
    }
}

// ============================================================================
// Transposition Table
// ============================================================================

/// A fixed-size hash table for caching search results.
///
/// The table uses a power-of-two sizing for efficient modular indexing.
/// Replacement policy is always-replace with generation-based aging.
///
/// # Usage
///
/// ```ignore
/// let mut tt = TranspositionTable::new(1 << 16); // 65536 entries
/// tt.store(hash, score, depth, bound, best_move);
/// if let TTProbeResult::Hit(entry) = tt.probe(hash, depth) {
///     // Use cached score
/// }
/// ```
pub struct TranspositionTable {
    /// The backing storage for TT entries.
    entries: Vec<TTEntry>,
    /// Mask for fast modular indexing (size - 1, requires power-of-two size).
    mask: usize,
    /// Current search generation for aging.
    generation: u16,
    /// Number of entries stored (for statistics).
    stored_count: u64,
    /// Number of probe hits (for statistics).
    hit_count: u64,
    /// Number of probe misses (for statistics).
    miss_count: u64,
    /// Number of overwrites (for statistics).
    overwrite_count: u64,
}

impl TranspositionTable {
    /// Create a new transposition table with the given number of entries.
    /// Size will be rounded up to the nearest power of two.
    ///
    /// # Arguments
    /// * `size` - Desired number of entries (will be rounded up to power of 2).
    ///   Minimum size is 1024.
    pub fn new(size: usize) -> Self {
        let actual_size = size.max(1024).next_power_of_two();
        Self {
            entries: vec![TTEntry::default(); actual_size],
            mask: actual_size - 1,
            generation: 0,
            stored_count: 0,
            hit_count: 0,
            miss_count: 0,
            overwrite_count: 0,
        }
    }

    /// Create a transposition table sized in megabytes.
    /// Each entry is approximately 24 bytes.
    pub fn with_mb(mb: usize) -> Self {
        let entry_size = std::mem::size_of::<TTEntry>();
        let entries = (mb * 1024 * 1024) / entry_size;
        Self::new(entries)
    }

    /// Probe the transposition table for a position.
    ///
    /// # Arguments
    /// * `hash` - The state hash to look up.
    /// * `required_depth` - The minimum depth needed for a score cutoff.
    ///
    /// # Returns
    /// - `TTProbeResult::Hit` if an entry with sufficient depth is found.
    /// - `TTProbeResult::ShallowHit` if an entry is found but at insufficient depth.
    /// - `TTProbeResult::Miss` if no entry matches.
    pub fn probe(&mut self, hash: u64, required_depth: u8) -> TTProbeResult {
        let index = (hash as usize) & self.mask;
        let entry = &self.entries[index];

        if entry.hash_key != hash {
            self.miss_count += 1;
            return TTProbeResult::Miss;
        }

        self.hit_count += 1;

        if entry.depth >= required_depth {
            TTProbeResult::Hit(*entry)
        } else {
            TTProbeResult::ShallowHit(*entry)
        }
    }

    /// Store an entry in the transposition table.
    ///
    /// Uses a replacement policy that prefers:
    /// 1. Empty slots
    /// 2. Entries from older generations
    /// 3. Entries with lower depth
    ///
    /// # Arguments
    /// * `hash` - The state hash key.
    /// * `score` - The evaluation score.
    /// * `depth` - The search depth at which this was computed.
    /// * `bound` - The type of score bound.
    /// * `best_move_index` - Index of the best move (u16::MAX for none).
    pub fn store(
        &mut self,
        hash: u64,
        score: i32,
        depth: u8,
        bound: TTBound,
        best_move_index: u16,
    ) {
        let index = (hash as usize) & self.mask;
        let existing = &self.entries[index];

        // Replacement policy:
        // Always replace if:
        // - Slot is empty (hash_key == 0)
        // - Same position (hash_key matches) with deeper or equal depth
        // - Old generation
        // - New entry has greater depth
        let should_replace = existing.hash_key == 0
            || existing.hash_key == hash
            || existing.generation != self.generation
            || depth >= existing.depth;

        if should_replace {
            if existing.hash_key != 0 && existing.hash_key != hash {
                self.overwrite_count += 1;
            }
            self.entries[index] = TTEntry::new(
                hash,
                score,
                depth,
                bound,
                best_move_index,
                self.generation,
            );
            self.stored_count += 1;
        }
    }

    /// Increment the generation counter.
    /// Call this at the start of each new search from the root.
    pub fn new_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// Clear all entries in the table.
    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = TTEntry::default();
        }
        self.generation = 0;
        self.stored_count = 0;
        self.hit_count = 0;
        self.miss_count = 0;
        self.overwrite_count = 0;
    }

    /// Returns the current generation.
    pub fn generation(&self) -> u16 {
        self.generation
    }

    /// Returns the number of entries in the table.
    pub fn capacity(&self) -> usize {
        self.entries.len()
    }

    /// Returns the occupancy ratio [0.0, 1.0] (approximate).
    pub fn occupancy(&self) -> f64 {
        // Sample first 1000 entries to estimate occupancy
        let sample_size = self.entries.len().min(1000);
        let used = self.entries[..sample_size]
            .iter()
            .filter(|e| e.hash_key != 0)
            .count();
        used as f64 / sample_size as f64
    }

    /// Returns TT statistics.
    pub fn stats(&self) -> TTStats {
        TTStats {
            capacity: self.capacity(),
            generation: self.generation,
            stored_count: self.stored_count,
            hit_count: self.hit_count,
            miss_count: self.miss_count,
            overwrite_count: self.overwrite_count,
            occupancy: self.occupancy(),
            hit_rate: if self.hit_count + self.miss_count > 0 {
                self.hit_count as f64 / (self.hit_count + self.miss_count) as f64
            } else {
                0.0
            },
        }
    }

    /// Prefetch a cache line for the given hash (no-op on most platforms,
    /// but useful as a hint for future optimization with SIMD/threading).
    #[inline]
    pub fn prefetch(&self, hash: u64) {
        let index = (hash as usize) & self.mask;
        // Compiler hint: we'll access this entry soon
        let _ = &self.entries[index];
    }
}

// ============================================================================
// TT Statistics
// ============================================================================

/// Statistics about transposition table usage.
#[derive(Debug, Clone)]
pub struct TTStats {
    /// Total capacity (number of entries).
    pub capacity: usize,
    /// Current generation.
    pub generation: u16,
    /// Total entries stored.
    pub stored_count: u64,
    /// Total probe hits.
    pub hit_count: u64,
    /// Total probe misses.
    pub miss_count: u64,
    /// Total overwrites.
    pub overwrite_count: u64,
    /// Occupancy ratio [0.0, 1.0].
    pub occupancy: f64,
    /// Hit rate [0.0, 1.0].
    pub hit_rate: f64,
}

impl std::fmt::Display for TTStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TT[cap={}, gen={}, stored={}, hits={}, misses={}, overwrites={}, \
             occ={:.1}%, hit_rate={:.1}%]",
            self.capacity,
            self.generation,
            self.stored_count,
            self.hit_count,
            self.miss_count,
            self.overwrite_count,
            self.occupancy * 100.0,
            self.hit_rate * 100.0,
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tt_creation() {
        let tt = TranspositionTable::new(1024);
        assert_eq!(tt.capacity(), 1024);
        assert_eq!(tt.generation(), 0);
    }

    #[test]
    fn test_tt_power_of_two_rounding() {
        let tt = TranspositionTable::new(1000);
        assert_eq!(tt.capacity(), 1024); // Rounds up to next power of 2
    }

    #[test]
    fn test_tt_minimum_size() {
        let tt = TranspositionTable::new(1);
        assert_eq!(tt.capacity(), 1024); // Minimum is 1024
    }

    #[test]
    fn test_tt_store_and_probe() {
        let mut tt = TranspositionTable::new(1024);
        let hash = 0xDEADBEEF_12345678u64;
        let score = 42;
        let depth = 3;

        tt.store(hash, score, depth, TTBound::Exact, 5);

        match tt.probe(hash, depth) {
            TTProbeResult::Hit(entry) => {
                assert_eq!(entry.hash_key, hash);
                assert_eq!(entry.score, score);
                assert_eq!(entry.depth, depth);
                assert_eq!(entry.bound, TTBound::Exact);
                assert_eq!(entry.best_move_index, 5);
            }
            _ => panic!("Expected hit"),
        }
    }

    #[test]
    fn test_tt_probe_miss() {
        let mut tt = TranspositionTable::new(1024);
        let hash = 0xDEADBEEF_12345678u64;
        let other_hash = 0xCAFEBABE_87654321u64;

        tt.store(hash, 100, 5, TTBound::Exact, 0);

        match tt.probe(other_hash, 1) {
            TTProbeResult::Miss => {} // Expected
            _ => panic!("Expected miss"),
        }
    }

    #[test]
    fn test_tt_shallow_hit() {
        let mut tt = TranspositionTable::new(1024);
        let hash = 0xDEADBEEFu64;

        tt.store(hash, 50, 2, TTBound::LowerBound, 3);

        // Probe with higher depth requirement
        match tt.probe(hash, 5) {
            TTProbeResult::ShallowHit(entry) => {
                assert_eq!(entry.score, 50);
                assert_eq!(entry.depth, 2);
                assert_eq!(entry.best_move_index, 3);
            }
            _ => panic!("Expected shallow hit"),
        }
    }

    #[test]
    fn test_tt_generation() {
        let mut tt = TranspositionTable::new(1024);
        assert_eq!(tt.generation(), 0);

        tt.new_generation();
        assert_eq!(tt.generation(), 1);

        tt.new_generation();
        assert_eq!(tt.generation(), 2);
    }

    #[test]
    fn test_tt_replacement_depth() {
        let mut tt = TranspositionTable::new(1024);
        let hash = 0xDEADBEEFu64;

        // Store at depth 3
        tt.store(hash, 100, 3, TTBound::Exact, 0);

        // Overwrite with deeper search (depth 5)
        tt.store(hash, 200, 5, TTBound::Exact, 1);

        match tt.probe(hash, 1) {
            TTProbeResult::Hit(entry) => {
                assert_eq!(entry.score, 200);
                assert_eq!(entry.depth, 5);
                assert_eq!(entry.best_move_index, 1);
            }
            _ => panic!("Expected hit with updated entry"),
        }
    }

    #[test]
    fn test_tt_generation_replacement() {
        let mut tt = TranspositionTable::new(1024);
        let hash = 0xDEADBEEFu64;

        // Store at generation 0
        tt.store(hash, 100, 10, TTBound::Exact, 0);

        // New generation
        tt.new_generation();

        // Collision on same index with new generation
        // The old entry should be replaced even if it had higher depth
        let colliding_hash = hash; // Same hash, different generation
        tt.store(colliding_hash, 50, 1, TTBound::LowerBound, 2);

        match tt.probe(colliding_hash, 1) {
            TTProbeResult::Hit(entry) => {
                assert_eq!(entry.score, 50);
                assert_eq!(entry.generation, 1);
            }
            _ => panic!("Expected hit with new generation entry"),
        }
    }

    #[test]
    fn test_tt_clear() {
        let mut tt = TranspositionTable::new(1024);
        let hash = 0xDEADBEEFu64;

        tt.store(hash, 100, 5, TTBound::Exact, 0);
        tt.clear();

        match tt.probe(hash, 1) {
            TTProbeResult::Miss => {} // Expected after clear
            _ => panic!("Expected miss after clear"),
        }
    }

    #[test]
    fn test_tt_stats() {
        let mut tt = TranspositionTable::new(1024);
        let hash = 0xDEADBEEFu64;

        tt.store(hash, 100, 5, TTBound::Exact, 0);
        let _ = tt.probe(hash, 5);
        let _ = tt.probe(0x12345678u64, 1);

        let stats = tt.stats();
        assert_eq!(stats.capacity, 1024);
        assert_eq!(stats.stored_count, 1);
        assert_eq!(stats.hit_count, 1);
        assert_eq!(stats.miss_count, 1);
    }

    #[test]
    fn test_tt_probe_result_methods() {
        let entry = TTEntry::new(0xDEAD, 42, 3, TTBound::Exact, 5, 0);
        let hit = TTProbeResult::Hit(entry);
        assert!(hit.found());
        assert_eq!(hit.best_move_index(), Some(5));

        let miss = TTProbeResult::Miss;
        assert!(!miss.found());
        assert_eq!(miss.best_move_index(), None);

        let no_move = TTEntry::new(0xBEEF, 10, 2, TTBound::UpperBound, u16::MAX, 0);
        let shallow = TTProbeResult::ShallowHit(no_move);
        assert!(shallow.found());
        assert_eq!(shallow.best_move_index(), None);
    }

    #[test]
    fn test_tt_with_mb() {
        let tt = TranspositionTable::with_mb(1);
        // 1 MB / ~24 bytes per entry ≈ ~43690, rounded to 32768 (next power of 2)
        assert!(tt.capacity() >= 32768);
        assert!(tt.capacity().is_power_of_two());
    }

    #[test]
    fn test_tt_bound_types() {
        let mut tt = TranspositionTable::new(1024);

        tt.store(1, 100, 3, TTBound::Exact, 0);
        tt.store(2, -50, 3, TTBound::LowerBound, 1);
        tt.store(3, 200, 3, TTBound::UpperBound, 2);

        if let TTProbeResult::Hit(e) = tt.probe(1, 3) {
            assert_eq!(e.bound, TTBound::Exact);
        }
        if let TTProbeResult::Hit(e) = tt.probe(2, 3) {
            assert_eq!(e.bound, TTBound::LowerBound);
        }
        if let TTProbeResult::Hit(e) = tt.probe(3, 3) {
            assert_eq!(e.bound, TTBound::UpperBound);
        }
    }

    #[test]
    fn test_tt_entry_default() {
        let entry = TTEntry::default();
        assert_eq!(entry.hash_key, 0);
        assert_eq!(entry.score, 0);
        assert_eq!(entry.depth, 0);
        assert!(!entry.has_best_move());
    }

    #[test]
    fn test_tt_stats_display() {
        let tt = TranspositionTable::new(1024);
        let stats = tt.stats();
        let display = format!("{}", stats);
        assert!(display.contains("TT["));
        assert!(display.contains("cap=1024"));
    }
}

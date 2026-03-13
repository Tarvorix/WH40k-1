//! Keyword system for unit and model classification.
//!
//! Keywords are enum-based with bitflag `KeywordSet` for fast, type-safe,
//! compile-time checked keyword matching. New factions add enum variants
//! and recompile - no logic changes needed.
//!
//! Source: implementation_v3.md - Keyword system design
//! Source: 40k_revised.md - Keywords
//! Source: Custodes.md, Frenzied_Reavers.md - Faction-specific keywords

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

/// Individual keyword for a unit or model.
/// Extensible: add new variants for new factions.
///
/// Keywords are divided into categories:
/// - Type keywords (Infantry, Monster, Vehicle, etc.)
/// - Faction keywords (Imperium, Chaos, Khorne, etc.)
/// - Role keywords (Battleline, Character, etc.)
/// - Unit-specific keywords (named units)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum Keyword {
    // === Type Keywords ===
    Infantry = 0,
    Monster = 1,
    Vehicle = 2,
    Mounted = 3,
    Beast = 4,
    Swarm = 5,
    Terminator = 6,

    // === Role Keywords ===
    Character = 7,
    Battleline = 8,
    DedicatedTransport = 9,
    EpicHero = 10,
    Leader = 11,

    // === Faction Keywords - Imperium ===
    Imperium = 12,
    AdeptusCustodes = 13,
    SpaceMarines = 14,
    Astartes = 15,

    // === Faction Keywords - Chaos ===
    Chaos = 16,
    Khorne = 17,
    Daemon = 18,
    WorldEaters = 19,

    // === Unit-Specific Keywords - Custodes ===
    BladeChampion = 20,
    Tristraen = 21,
    CustodianGuard = 22,
    CustodianWardens = 23,
    AllarusCustodians = 24,

    // === Unit-Specific Keywords - World Eaters ===
    DaemonPrince = 25,
    Vorrakh = 26,
    MasterOfExecutions = 27,
    Berzerkers = 28,
    Jakhals = 29,

    // === Reserved for future factions ===
    // Add new keywords here for additional factions.
    // Then recompile. No engine logic changes needed.
    Tyranids = 30,
    Orks = 31,
    Aeldari = 32,
    Drukhari = 33,
    Necrons = 34,
    TauEmpire = 35,
    GscCults = 36,
    DeathGuard = 37,
    ThousandSons = 38,
    GreyKnights = 39,
    SistersOfBattle = 40,
    Psyker = 41,
    Fly = 42,
    Walker = 43,
    Grenades = 44,
    Smoke = 45,
    Aircraft = 46,
    Titanic = 47,
}

impl Keyword {
    /// Convert to a KeywordSet containing just this keyword.
    pub fn as_set(self) -> KeywordSet {
        KeywordSet::from_keyword(self)
    }

    /// Get the bit index for this keyword (for KeywordSet).
    pub fn bit_index(self) -> u8 {
        self as u8
    }
}

bitflags! {
    /// A set of keywords represented as a bitflag for O(1) membership testing.
    /// Supports up to 64 keywords. If more are needed, expand to u128.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct KeywordSet: u64 {
        const EMPTY = 0;
        const INFANTRY = 1 << 0;
        const MONSTER = 1 << 1;
        const VEHICLE = 1 << 2;
        const MOUNTED = 1 << 3;
        const BEAST = 1 << 4;
        const SWARM = 1 << 5;
        const TERMINATOR = 1 << 6;
        const CHARACTER = 1 << 7;
        const BATTLELINE = 1 << 8;
        const DEDICATED_TRANSPORT = 1 << 9;
        const EPIC_HERO = 1 << 10;
        const LEADER = 1 << 11;
        const IMPERIUM = 1 << 12;
        const ADEPTUS_CUSTODES = 1 << 13;
        const SPACE_MARINES = 1 << 14;
        const ASTARTES = 1 << 15;
        const CHAOS = 1 << 16;
        const KHORNE = 1 << 17;
        const DAEMON = 1 << 18;
        const WORLD_EATERS = 1 << 19;
        const BLADE_CHAMPION = 1 << 20;
        const TRISTRAEN = 1 << 21;
        const CUSTODIAN_GUARD = 1 << 22;
        const CUSTODIAN_WARDENS = 1 << 23;
        const ALLARUS_CUSTODIANS = 1 << 24;
        const DAEMON_PRINCE = 1 << 25;
        const VORRAKH = 1 << 26;
        const MASTER_OF_EXECUTIONS = 1 << 27;
        const BERZERKERS = 1 << 28;
        const JAKHALS = 1 << 29;
        const TYRANIDS = 1 << 30;
        const ORKS = 1 << 31;
        const AELDARI = 1 << 32;
        const DRUKHARI = 1 << 33;
        const NECRONS = 1 << 34;
        const TAU_EMPIRE = 1 << 35;
        const GSC_CULTS = 1 << 36;
        const DEATH_GUARD = 1 << 37;
        const THOUSAND_SONS = 1 << 38;
        const GREY_KNIGHTS = 1 << 39;
        const SISTERS_OF_BATTLE = 1 << 40;
        const PSYKER = 1 << 41;
        const FLY = 1 << 42;
        const WALKER = 1 << 43;
        const GRENADES = 1 << 44;
        const SMOKE = 1 << 45;
        const AIRCRAFT = 1 << 46;
        const TITANIC = 1 << 47;
    }
}

impl KeywordSet {
    /// Create a KeywordSet from a single keyword.
    pub fn from_keyword(kw: Keyword) -> Self {
        KeywordSet::from_bits_truncate(1u64 << (kw as u8))
    }

    /// Create a KeywordSet from a slice of keywords.
    pub fn from_keywords(keywords: &[Keyword]) -> Self {
        let mut set = KeywordSet::empty();
        for kw in keywords {
            set |= KeywordSet::from_keyword(*kw);
        }
        set
    }

    /// Check if this set contains a specific keyword.
    pub fn has(self, kw: Keyword) -> bool {
        self.contains(KeywordSet::from_keyword(kw))
    }

    /// Check if this set has ANY keyword from another set (intersection is non-empty).
    pub fn has_any(self, other: KeywordSet) -> bool {
        self.intersects(other)
    }

    /// Check if this set has ALL keywords from another set.
    pub fn has_all(self, other: KeywordSet) -> bool {
        self.contains(other)
    }

    /// Count the number of keywords in this set.
    pub fn count(self) -> u32 {
        self.bits().count_ones()
    }

    /// Iterate over the bit indices of keywords in this set.
    pub fn bit_indices(self) -> KeywordBitIter {
        KeywordBitIter {
            remaining: self.bits(),
        }
    }
}

impl Default for KeywordSet {
    fn default() -> Self {
        KeywordSet::empty()
    }
}

// Manual serde implementation for KeywordSet using raw u64 bits
impl Serialize for KeywordSet {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.bits().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for KeywordSet {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bits = u64::deserialize(deserializer)?;
        Ok(KeywordSet::from_bits_truncate(bits))
    }
}

/// Iterator over bit indices of keywords in a KeywordSet.
pub struct KeywordBitIter {
    remaining: u64,
}

impl Iterator for KeywordBitIter {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            None
        } else {
            let bit = self.remaining.trailing_zeros() as u8;
            self.remaining &= self.remaining - 1; // Clear lowest set bit
            Some(bit)
        }
    }
}

// Pre-built keyword sets for common faction compositions

impl KeywordSet {
    // Tristraen keywords: INFANTRY, CHARACTER, IMPERIUM, BLADE CHAMPION, TRISTRAEN
    pub const TRISTRAEN_KEYWORDS: KeywordSet = KeywordSet::from_bits_truncate(
        KeywordSet::INFANTRY.bits()
            | KeywordSet::CHARACTER.bits()
            | KeywordSet::IMPERIUM.bits()
            | KeywordSet::ADEPTUS_CUSTODES.bits()
            | KeywordSet::BLADE_CHAMPION.bits()
            | KeywordSet::TRISTRAEN.bits(),
    );

    // Custodian Guard keywords: INFANTRY, BATTLELINE, IMPERIUM, CUSTODIAN GUARD
    pub const CUSTODIAN_GUARD_KEYWORDS: KeywordSet = KeywordSet::from_bits_truncate(
        KeywordSet::INFANTRY.bits()
            | KeywordSet::BATTLELINE.bits()
            | KeywordSet::IMPERIUM.bits()
            | KeywordSet::ADEPTUS_CUSTODES.bits()
            | KeywordSet::CUSTODIAN_GUARD.bits(),
    );

    // Custodian Wardens keywords: INFANTRY, IMPERIUM, CUSTODIAN WARDENS
    pub const CUSTODIAN_WARDENS_KEYWORDS: KeywordSet = KeywordSet::from_bits_truncate(
        KeywordSet::INFANTRY.bits()
            | KeywordSet::IMPERIUM.bits()
            | KeywordSet::ADEPTUS_CUSTODES.bits()
            | KeywordSet::CUSTODIAN_WARDENS.bits(),
    );

    // Allarus Custodians keywords: INFANTRY, TERMINATOR, IMPERIUM, ALLARUS CUSTODIANS
    pub const ALLARUS_CUSTODIANS_KEYWORDS: KeywordSet = KeywordSet::from_bits_truncate(
        KeywordSet::INFANTRY.bits()
            | KeywordSet::TERMINATOR.bits()
            | KeywordSet::IMPERIUM.bits()
            | KeywordSet::ADEPTUS_CUSTODES.bits()
            | KeywordSet::ALLARUS_CUSTODIANS.bits(),
    );

    // Vorrakh keywords: MONSTER, CHARACTER, CHAOS, KHORNE, DAEMON, DAEMON PRINCE, VORRAKH
    pub const VORRAKH_KEYWORDS: KeywordSet = KeywordSet::from_bits_truncate(
        KeywordSet::MONSTER.bits()
            | KeywordSet::CHARACTER.bits()
            | KeywordSet::CHAOS.bits()
            | KeywordSet::KHORNE.bits()
            | KeywordSet::DAEMON.bits()
            | KeywordSet::WORLD_EATERS.bits()
            | KeywordSet::DAEMON_PRINCE.bits()
            | KeywordSet::VORRAKH.bits(),
    );

    // Master of Executions keywords: INFANTRY, CHARACTER, CHAOS, KHORNE, MASTER OF EXECUTIONS
    pub const MASTER_OF_EXECUTIONS_KEYWORDS: KeywordSet = KeywordSet::from_bits_truncate(
        KeywordSet::INFANTRY.bits()
            | KeywordSet::CHARACTER.bits()
            | KeywordSet::CHAOS.bits()
            | KeywordSet::KHORNE.bits()
            | KeywordSet::WORLD_EATERS.bits()
            | KeywordSet::MASTER_OF_EXECUTIONS.bits(),
    );

    // Khorne Berzerkers keywords: INFANTRY, BATTLELINE, CHAOS, KHORNE, BERZERKERS
    pub const BERZERKERS_KEYWORDS: KeywordSet = KeywordSet::from_bits_truncate(
        KeywordSet::INFANTRY.bits()
            | KeywordSet::BATTLELINE.bits()
            | KeywordSet::CHAOS.bits()
            | KeywordSet::KHORNE.bits()
            | KeywordSet::WORLD_EATERS.bits()
            | KeywordSet::BERZERKERS.bits(),
    );

    // Jakhals keywords: INFANTRY, CHAOS, KHORNE, JAKHALS
    pub const JAKHALS_KEYWORDS: KeywordSet = KeywordSet::from_bits_truncate(
        KeywordSet::INFANTRY.bits()
            | KeywordSet::CHAOS.bits()
            | KeywordSet::KHORNE.bits()
            | KeywordSet::WORLD_EATERS.bits()
            | KeywordSet::JAKHALS.bits(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_to_set() {
        let set = Keyword::Infantry.as_set();
        assert!(set.has(Keyword::Infantry));
        assert!(!set.has(Keyword::Monster));
    }

    #[test]
    fn test_keyword_set_from_keywords() {
        let set = KeywordSet::from_keywords(&[
            Keyword::Infantry,
            Keyword::Character,
            Keyword::Imperium,
        ]);
        assert!(set.has(Keyword::Infantry));
        assert!(set.has(Keyword::Character));
        assert!(set.has(Keyword::Imperium));
        assert!(!set.has(Keyword::Monster));
        assert_eq!(set.count(), 3);
    }

    #[test]
    fn test_keyword_set_has_any() {
        let unit_keywords = KeywordSet::from_keywords(&[
            Keyword::Infantry,
            Keyword::Character,
            Keyword::Imperium,
        ]);
        let required = KeywordSet::from_keywords(&[Keyword::Character, Keyword::Monster]);
        assert!(unit_keywords.has_any(required)); // has Character

        let other = KeywordSet::from_keywords(&[Keyword::Monster, Keyword::Vehicle]);
        assert!(!unit_keywords.has_any(other)); // has neither
    }

    #[test]
    fn test_keyword_set_has_all() {
        let unit_keywords = KeywordSet::from_keywords(&[
            Keyword::Infantry,
            Keyword::Character,
            Keyword::Imperium,
        ]);
        let required =
            KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Character]);
        assert!(unit_keywords.has_all(required));

        let too_many = KeywordSet::from_keywords(&[
            Keyword::Infantry,
            Keyword::Character,
            Keyword::Monster,
        ]);
        assert!(!unit_keywords.has_all(too_many));
    }

    #[test]
    fn test_keyword_set_operations() {
        let a = KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Character]);
        let b = KeywordSet::from_keywords(&[Keyword::Character, Keyword::Imperium]);

        // Union
        let union = a | b;
        assert_eq!(union.count(), 3);

        // Intersection
        let intersection = a & b;
        assert_eq!(intersection.count(), 1);
        assert!(intersection.has(Keyword::Character));
    }

    #[test]
    fn test_tristraen_keywords() {
        let kw = KeywordSet::TRISTRAEN_KEYWORDS;
        assert!(kw.has(Keyword::Infantry));
        assert!(kw.has(Keyword::Character));
        assert!(kw.has(Keyword::Imperium));
        assert!(kw.has(Keyword::AdeptusCustodes));
        assert!(kw.has(Keyword::BladeChampion));
        assert!(kw.has(Keyword::Tristraen));
        assert!(!kw.has(Keyword::Monster));
        assert!(!kw.has(Keyword::Chaos));
    }

    #[test]
    fn test_vorrakh_keywords() {
        let kw = KeywordSet::VORRAKH_KEYWORDS;
        assert!(kw.has(Keyword::Monster));
        assert!(kw.has(Keyword::Character));
        assert!(kw.has(Keyword::Chaos));
        assert!(kw.has(Keyword::Khorne));
        assert!(kw.has(Keyword::Daemon));
        assert!(kw.has(Keyword::DaemonPrince));
        assert!(kw.has(Keyword::Vorrakh));
        assert!(!kw.has(Keyword::Infantry));
        assert!(!kw.has(Keyword::Imperium));
    }

    #[test]
    fn test_keyword_set_serialization() {
        let set = KeywordSet::TRISTRAEN_KEYWORDS;
        let json = serde_json::to_string(&set).unwrap();
        let back: KeywordSet = serde_json::from_str(&json).unwrap();
        assert_eq!(set, back);
    }

    #[test]
    fn test_keyword_set_empty() {
        let empty = KeywordSet::empty();
        assert_eq!(empty.count(), 0);
        assert!(!empty.has(Keyword::Infantry));
    }

    #[test]
    fn test_keyword_set_bit_indices() {
        let set = KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Character]);
        let bits: Vec<u8> = set.bit_indices().collect();
        assert_eq!(bits.len(), 2);
        assert!(bits.contains(&(Keyword::Infantry as u8)));
        assert!(bits.contains(&(Keyword::Character as u8)));
    }
}

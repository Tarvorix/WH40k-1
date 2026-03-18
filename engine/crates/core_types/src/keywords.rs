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

    // === Faction Keywords - Chaos Space Marines (Boarding Actions) ===
    HereticAstartes = 30,
    ChaosUndivided = 31,

    // === Faction Keywords - Astra Militarum (Boarding Actions) ===
    AstraMilitarum = 32,
    MilitarumTempestus = 33,

    // === Type/Role Keywords - Boarding Actions ===
    Ogryn = 34,
    Possessed = 35,
    Eightbound = 36,
    Officer = 37,
    Spawn = 38,
    Chosen = 39,
    TerminatorArmour = 40,

    // === Special Keywords ===
    Psyker = 41,
    Fly = 42,
    Walker = 43,
    Grenades = 44,
    Smoke = 45,
    Aircraft = 46,
    Titanic = 47,

    // === Reserved for future factions ===
    Tyranids = 48,
    Orks = 49,
    Aeldari = 50,
    Drukhari = 51,
    Necrons = 52,
    TauEmpire = 53,
    GscCults = 54,
    DeathGuard = 55,
    ThousandSons = 56,
    GreyKnights = 57,
    SistersOfBattle = 58,
    Damned = 59,
    Tacticus = 60,
    /// The TRANSPORT keyword — units that can carry other units.
    /// Source: 40k_revised.md §6.1
    Transport = 61,
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

    /// Parse a keyword string (from Boarding Actions JSON datasheets) into a Keyword enum.
    /// Returns None for unit-specific keywords that aren't mechanically relevant
    /// (e.g., "KHARN THE BETRAYER", "ARJAC ROCKFIST").
    pub fn from_keyword_str(s: &str) -> Option<Self> {
        match s.trim().to_uppercase().as_str() {
            // Type keywords
            "INFANTRY" => Some(Keyword::Infantry),
            "MONSTER" => Some(Keyword::Monster),
            "VEHICLE" => Some(Keyword::Vehicle),
            "MOUNTED" => Some(Keyword::Mounted),
            "BEAST" => Some(Keyword::Beast),
            "SWARM" => Some(Keyword::Swarm),
            "TERMINATOR" => Some(Keyword::Terminator),
            "OGRYN" => Some(Keyword::Ogryn),
            "SPAWN" => Some(Keyword::Spawn),

            // Role keywords
            "CHARACTER" => Some(Keyword::Character),
            "BATTLELINE" => Some(Keyword::Battleline),
            "DEDICATED TRANSPORT" => Some(Keyword::DedicatedTransport),
            "EPIC HERO" => Some(Keyword::EpicHero),
            "LEADER" => Some(Keyword::Leader),
            "OFFICER" => Some(Keyword::Officer),

            // Faction keywords - Imperium
            "IMPERIUM" => Some(Keyword::Imperium),
            "ADEPTUS CUSTODES" => Some(Keyword::AdeptusCustodes),
            "SPACE MARINES" => Some(Keyword::SpaceMarines),
            "ADEPTUS ASTARTES" => Some(Keyword::Astartes),

            // Faction keywords - Chaos
            "CHAOS" => Some(Keyword::Chaos),
            "KHORNE" => Some(Keyword::Khorne),
            "DAEMON" => Some(Keyword::Daemon),
            "WORLD EATERS" => Some(Keyword::WorldEaters),
            "HERETIC ASTARTES" => Some(Keyword::HereticAstartes),
            "CHAOS UNDIVIDED" => Some(Keyword::ChaosUndivided),

            // Faction keywords - Astra Militarum
            "ASTRA MILITARUM" => Some(Keyword::AstraMilitarum),
            "MILITARUM TEMPESTUS" => Some(Keyword::MilitarumTempestus),

            // Unit type keywords
            "POSSESSED" => Some(Keyword::Possessed),
            "EIGHTBOUND" | "EXALTED EIGHTBOUND" => Some(Keyword::Eightbound),
            "CHOSEN" => Some(Keyword::Chosen),
            "TERMINATOR ARMOUR" => Some(Keyword::TerminatorArmour),
            "DAMNED" => Some(Keyword::Damned),
            "TACTICUS" => Some(Keyword::Tacticus),

            // Special keywords
            "PSYKER" => Some(Keyword::Psyker),
            "FLY" => Some(Keyword::Fly),
            "WALKER" => Some(Keyword::Walker),
            "GRENADES" => Some(Keyword::Grenades),
            "SMOKE" => Some(Keyword::Smoke),
            "AIRCRAFT" => Some(Keyword::Aircraft),
            "TITANIC" => Some(Keyword::Titanic),
            "TRANSPORT" => Some(Keyword::Transport),

            // Unit-specific keywords (not mechanically relevant, silently ignored)
            _ => None,
        }
    }
}

impl KeywordSet {
    /// Build a KeywordSet from a slice of keyword strings (from BA JSON datasheets).
    /// Unknown keywords are silently ignored.
    pub fn from_keyword_strings(strings: &[String]) -> Self {
        let mut set = KeywordSet::empty();
        for s in strings {
            if let Some(kw) = Keyword::from_keyword_str(s) {
                set |= KeywordSet::from_keyword(kw);
            }
        }
        set
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
        // BA faction keywords
        const HERETIC_ASTARTES = 1 << 30;
        const CHAOS_UNDIVIDED = 1 << 31;
        const ASTRA_MILITARUM = 1 << 32;
        const MILITARUM_TEMPESTUS = 1 << 33;
        // BA type/role keywords
        const OGRYN = 1 << 34;
        const POSSESSED = 1 << 35;
        const EIGHTBOUND = 1 << 36;
        const OFFICER = 1 << 37;
        const SPAWN = 1 << 38;
        const CHOSEN = 1 << 39;
        const TERMINATOR_ARMOUR = 1 << 40;
        // Special keywords
        const PSYKER = 1 << 41;
        const FLY = 1 << 42;
        const WALKER = 1 << 43;
        const GRENADES = 1 << 44;
        const SMOKE = 1 << 45;
        const AIRCRAFT = 1 << 46;
        const TITANIC = 1 << 47;
        // Reserved
        const TYRANIDS = 1 << 48;
        const ORKS = 1 << 49;
        const AELDARI = 1 << 50;
        const DRUKHARI = 1 << 51;
        const NECRONS = 1 << 52;
        const TAU_EMPIRE = 1 << 53;
        const GSC_CULTS = 1 << 54;
        const DEATH_GUARD = 1 << 55;
        const THOUSAND_SONS = 1 << 56;
        const GREY_KNIGHTS = 1 << 57;
        const SISTERS_OF_BATTLE = 1 << 58;
        const DAMNED = 1 << 59;
        const TACTICUS = 1 << 60;
        const TRANSPORT = 1 << 61;
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

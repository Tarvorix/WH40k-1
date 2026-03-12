//! Stable ID types for all game entities.
//!
//! All IDs are Copy/Clone/Hash/Eq/Serialize/Deserialize newtype wrappers
//! around u32 for compact representation and fast comparison.
//!
//! Source: implementation_v3.md Section 6.1

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[repr(transparent)]
        pub struct $name(pub u32);

        impl $name {
            pub const fn new(id: u32) -> Self {
                Self(id)
            }

            pub const fn raw(self) -> u32 {
                self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<u32> for $name {
            fn from(id: u32) -> Self {
                Self(id)
            }
        }

        impl From<$name> for u32 {
            fn from(id: $name) -> u32 {
                id.0
            }
        }
    };
}

define_id!(
    /// Identifies a player in the game (typically 0 or 1 for Combat Patrol).
    PlayerId
);

define_id!(
    /// Identifies a unit on the battlefield.
    UnitId
);

define_id!(
    /// Identifies an individual model within a unit.
    ModelId
);

define_id!(
    /// Identifies a weapon profile.
    WeaponId
);

define_id!(
    /// Identifies an ability (faction, unit, or wargear ability).
    AbilityId
);

define_id!(
    /// Identifies a stratagem.
    StratagemId
);

define_id!(
    /// Identifies an enhancement.
    EnhancementId
);

define_id!(
    /// Identifies an objective marker on the battlefield.
    ObjectiveId
);

define_id!(
    /// Identifies a mission.
    MissionId
);

define_id!(
    /// Identifies a faction.
    FactionId
);

define_id!(
    /// Identifies a datasheet (unit type template).
    DatasheetId
);

define_id!(
    /// Identifies a secondary objective.
    SecondaryObjectiveId
);

define_id!(
    /// Identifies a content pack (compiled faction/mission data).
    ContentPackId
);

define_id!(
    /// Identifies a specific game event in the event log.
    EventId
);

define_id!(
    /// Identifies a command in the command history.
    CommandId
);

define_id!(
    /// Identifies a dice roll in the dice log.
    DiceRollId
);

define_id!(
    /// Identifies a custom rule handler (escape hatch for irregular rules).
    CustomRuleHandlerId
);

/// Counter for generating sequential IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdGenerator {
    next: u32,
}

impl IdGenerator {
    pub const fn new() -> Self {
        Self { next: 0 }
    }

    pub const fn starting_at(start: u32) -> Self {
        Self { next: start }
    }

    pub fn next_id<T: From<u32>>(&mut self) -> T {
        let id = self.next;
        self.next += 1;
        T::from(id)
    }
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_creation() {
        let id = PlayerId::new(42);
        assert_eq!(id.raw(), 42);
        assert_eq!(id, PlayerId(42));
    }

    #[test]
    fn test_id_from_u32() {
        let id: UnitId = 7u32.into();
        assert_eq!(id.raw(), 7);
    }

    #[test]
    fn test_id_into_u32() {
        let id = ModelId::new(99);
        let raw: u32 = id.into();
        assert_eq!(raw, 99);
    }

    #[test]
    fn test_id_equality() {
        assert_eq!(WeaponId::new(1), WeaponId::new(1));
        assert_ne!(WeaponId::new(1), WeaponId::new(2));
    }

    #[test]
    fn test_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(AbilityId::new(1));
        set.insert(AbilityId::new(2));
        set.insert(AbilityId::new(1)); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_id_serialization() {
        let id = StratagemId::new(42);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "42");
        let deserialized: StratagemId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, id);
    }

    #[test]
    fn test_id_generator() {
        let mut gen = IdGenerator::new();
        let a: UnitId = gen.next_id();
        let b: UnitId = gen.next_id();
        let c: UnitId = gen.next_id();
        assert_eq!(a.raw(), 0);
        assert_eq!(b.raw(), 1);
        assert_eq!(c.raw(), 2);
    }

    #[test]
    fn test_id_generator_starting_at() {
        let mut gen = IdGenerator::starting_at(100);
        let a: ModelId = gen.next_id();
        assert_eq!(a.raw(), 100);
    }

    #[test]
    fn test_id_debug_format() {
        let id = PlayerId::new(5);
        assert_eq!(format!("{:?}", id), "PlayerId(5)");
    }

    #[test]
    fn test_id_display_format() {
        let id = PlayerId::new(5);
        assert_eq!(format!("{}", id), "5");
    }
}

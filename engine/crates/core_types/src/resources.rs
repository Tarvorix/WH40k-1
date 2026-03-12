//! Resource types for game state tracking.
//!
//! Wraps numeric values with type safety for command points, victory points,
//! wounds, characteristics, and other game resources.
//!
//! Source: implementation_v3.md Section 6.1
//! Source: 40k_revised.md - Unit characteristics

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Sub, SubAssign};

macro_rules! define_resource {
    ($(#[$meta:meta])* $name:ident, $inner:ty, $display_suffix:expr) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default)]
        pub struct $name(pub $inner);

        impl $name {
            pub const ZERO: $name = $name(0);

            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            pub const fn value(self) -> $inner {
                self.0
            }

            pub fn saturating_sub(self, rhs: Self) -> Self {
                Self(self.0.saturating_sub(rhs.0))
            }

            pub fn saturating_add(self, rhs: Self) -> Self {
                Self(self.0.saturating_add(rhs.0))
            }
        }

        impl Add for $name {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                Self(self.0 + rhs.0)
            }
        }

        impl AddAssign for $name {
            fn add_assign(&mut self, rhs: Self) {
                self.0 += rhs.0;
            }
        }

        impl Sub for $name {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                Self(self.0 - rhs.0)
            }
        }

        impl SubAssign for $name {
            fn sub_assign(&mut self, rhs: Self) {
                self.0 -= rhs.0;
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}{}", self.0, $display_suffix)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}{}", self.0, $display_suffix)
            }
        }
    };
}

define_resource!(
    /// Command Points available for stratagem use.
    /// Source: CP_Rules.md - "you start the battle with 0 Command points"
    CommandPoints, i8, "CP"
);

define_resource!(
    /// Victory Points accumulated during the game.
    VictoryPoints, i16, "VP"
);

define_resource!(
    /// Number of wounds a model can sustain / has remaining.
    Wounds, u8, "W"
);

define_resource!(
    /// Number of attacks a weapon makes.
    Attacks, u8, "A"
);

define_resource!(
    /// Toughness characteristic of a model.
    Toughness, u8, "T"
);

define_resource!(
    /// Strength characteristic of a weapon.
    Strength, u8, "S"
);

define_resource!(
    /// Leadership characteristic (tested on 2D6, pass if <= Ld).
    Leadership, u8, "Ld"
);

define_resource!(
    /// Objective Control characteristic.
    ObjectiveControl, u8, "OC"
);

/// Armour save characteristic (2+ to 7+, where 7+ means no save).
/// Lower is better. Unmodified 1 always fails.
/// Source: 40k_revised.md - "SAVING THROWS"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ArmorSave(pub u8);

impl ArmorSave {
    /// Best possible save (2+)
    pub const TWO_PLUS: ArmorSave = ArmorSave(2);
    pub const THREE_PLUS: ArmorSave = ArmorSave(3);
    pub const FOUR_PLUS: ArmorSave = ArmorSave(4);
    pub const FIVE_PLUS: ArmorSave = ArmorSave(5);
    pub const SIX_PLUS: ArmorSave = ArmorSave(6);
    /// No save
    pub const NONE: ArmorSave = ArmorSave(7);

    pub const fn value(self) -> u8 {
        self.0
    }

    /// Apply AP modifier. Save can't be worse than 7+ (no save).
    /// Source: 40k_revised.md - "Modify the saving throw by the AP"
    pub fn modified_by_ap(self, ap: ArmorPenetration) -> ArmorSave {
        let modified = self.0 as i8 + ap.0.abs();
        ArmorSave(modified.min(7) as u8)
    }

    /// Whether a roll of `value` passes this save.
    /// Unmodified 1 always fails.
    pub fn passes(self, roll: u8) -> bool {
        roll >= self.0
    }
}

/// Invulnerable save (not modified by AP).
/// Source: 40k_revised.md - "INVULNERABLE SAVES"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct InvulnerableSave(pub u8);

impl InvulnerableSave {
    pub const FOUR_PLUS: InvulnerableSave = InvulnerableSave(4);
    pub const FIVE_PLUS: InvulnerableSave = InvulnerableSave(5);
    pub const SIX_PLUS: InvulnerableSave = InvulnerableSave(6);
    pub const NONE: InvulnerableSave = InvulnerableSave(7);

    pub const fn value(self) -> u8 {
        self.0
    }

    pub fn has_invulnerable(self) -> bool {
        self.0 <= 6
    }

    pub fn passes(self, roll: u8) -> bool {
        roll >= self.0
    }
}

/// Armor Penetration value of a weapon (stored as negative, e.g., AP-2 = ArmorPenetration(-2)).
/// Source: 40k_revised.md - AP characteristic
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ArmorPenetration(pub i8);

impl ArmorPenetration {
    pub const ZERO: ArmorPenetration = ArmorPenetration(0);
    pub const MINUS_1: ArmorPenetration = ArmorPenetration(-1);
    pub const MINUS_2: ArmorPenetration = ArmorPenetration(-2);
    pub const MINUS_3: ArmorPenetration = ArmorPenetration(-3);
    pub const MINUS_4: ArmorPenetration = ArmorPenetration(-4);

    pub const fn new(ap: i8) -> Self {
        Self(ap)
    }

    pub const fn value(self) -> i8 {
        self.0
    }
}

/// Damage characteristic of a weapon.
/// Can be fixed or variable (D3, D6, D6+1, etc.)
/// Source: 40k_revised.md - Damage characteristic
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Damage {
    /// Fixed damage value
    Fixed(u8),
    /// Roll D3 for damage
    D3,
    /// Roll D6 for damage
    D6,
    /// Roll D3 + fixed bonus
    D3Plus(u8),
    /// Roll D6 + fixed bonus
    D6Plus(u8),
}

impl Damage {
    /// Minimum possible damage
    pub fn min_value(self) -> u8 {
        match self {
            Damage::Fixed(n) => n,
            Damage::D3 => 1,
            Damage::D6 => 1,
            Damage::D3Plus(n) => 1 + n,
            Damage::D6Plus(n) => 1 + n,
        }
    }

    /// Maximum possible damage
    pub fn max_value(self) -> u8 {
        match self {
            Damage::Fixed(n) => n,
            Damage::D3 => 3,
            Damage::D6 => 6,
            Damage::D3Plus(n) => 3 + n,
            Damage::D6Plus(n) => 6 + n,
        }
    }

    /// Average damage (for AI evaluation)
    pub fn expected_value(self) -> f64 {
        match self {
            Damage::Fixed(n) => n as f64,
            Damage::D3 => 2.0,
            Damage::D6 => 3.5,
            Damage::D3Plus(n) => 2.0 + n as f64,
            Damage::D6Plus(n) => 3.5 + n as f64,
        }
    }
}

/// Move characteristic of a model.
/// Source: 40k_revised.md - Movement characteristic
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MoveCharacteristic(pub super::Inches);

impl MoveCharacteristic {
    pub const fn from_inches(inches: i32) -> Self {
        Self(super::Inches::from_inches(inches))
    }

    pub const fn distance(self) -> super::Inches {
        self.0
    }
}

/// Ballistic Skill or Weapon Skill (e.g., 2+ means hits on 2+).
/// Source: 40k_revised.md - Hit Roll
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Skill(pub u8);

impl Skill {
    pub const TWO_PLUS: Skill = Skill(2);
    pub const THREE_PLUS: Skill = Skill(3);
    pub const FOUR_PLUS: Skill = Skill(4);
    pub const FIVE_PLUS: Skill = Skill(5);
    pub const SIX_PLUS: Skill = Skill(6);

    pub const fn value(self) -> u8 {
        self.0
    }

    /// Whether a roll of `value` hits (unmodified).
    /// Unmodified 1 always fails.
    pub fn hits(self, roll: u8) -> bool {
        roll >= self.0
    }
}

/// Feel No Pain save value (e.g., 5+ means ignore wound on 5+).
/// Source: 40k_revised.md - "FEEL NO PAIN"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FeelNoPain(pub u8);

impl FeelNoPain {
    pub const FOUR_PLUS: FeelNoPain = FeelNoPain(4);
    pub const FIVE_PLUS: FeelNoPain = FeelNoPain(5);
    pub const SIX_PLUS: FeelNoPain = FeelNoPain(6);
    pub const NONE: FeelNoPain = FeelNoPain(7);

    pub const fn value(self) -> u8 {
        self.0
    }

    pub fn has_fnp(self) -> bool {
        self.0 <= 6
    }

    pub fn passes(self, roll: u8) -> bool {
        roll >= self.0
    }
}

/// Number of attacks for a weapon, which can be variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackCount {
    /// Fixed number of attacks
    Fixed(u8),
    /// D6 attacks (e.g., Balistus Grenade Launcher)
    D6,
    /// D3 attacks
    D3,
    /// D6 + fixed bonus
    D6Plus(u8),
    /// D3 + fixed bonus
    D3Plus(u8),
}

impl AttackCount {
    pub fn min_value(self) -> u8 {
        match self {
            AttackCount::Fixed(n) => n,
            AttackCount::D6 => 1,
            AttackCount::D3 => 1,
            AttackCount::D6Plus(n) => 1 + n,
            AttackCount::D3Plus(n) => 1 + n,
        }
    }

    pub fn max_value(self) -> u8 {
        match self {
            AttackCount::Fixed(n) => n,
            AttackCount::D6 => 6,
            AttackCount::D3 => 3,
            AttackCount::D6Plus(n) => 6 + n,
            AttackCount::D3Plus(n) => 3 + n,
        }
    }

    pub fn expected_value(self) -> f64 {
        match self {
            AttackCount::Fixed(n) => n as f64,
            AttackCount::D6 => 3.5,
            AttackCount::D3 => 2.0,
            AttackCount::D6Plus(n) => 3.5 + n as f64,
            AttackCount::D3Plus(n) => 2.0 + n as f64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_points_arithmetic() {
        let mut cp = CommandPoints::new(3);
        cp += CommandPoints::new(1);
        assert_eq!(cp.value(), 4);
        cp -= CommandPoints::new(2);
        assert_eq!(cp.value(), 2);
    }

    #[test]
    fn test_victory_points() {
        let vp = VictoryPoints::new(5) + VictoryPoints::new(3);
        assert_eq!(vp.value(), 8);
    }

    #[test]
    fn test_armor_save_ap_modification() {
        let save = ArmorSave::TWO_PLUS;
        // AP-2 modifies 2+ to 4+
        let modified = save.modified_by_ap(ArmorPenetration::MINUS_2);
        assert_eq!(modified.value(), 4);

        // AP-0 doesn't change save
        let unmod = save.modified_by_ap(ArmorPenetration::ZERO);
        assert_eq!(unmod.value(), 2);

        // Heavy AP can't make save worse than 7+
        let heavy = ArmorSave::SIX_PLUS.modified_by_ap(ArmorPenetration::MINUS_3);
        assert_eq!(heavy.value(), 7);
    }

    #[test]
    fn test_armor_save_passes() {
        assert!(ArmorSave::TWO_PLUS.passes(2));
        assert!(ArmorSave::TWO_PLUS.passes(6));
        assert!(!ArmorSave::TWO_PLUS.passes(1));
        assert!(!ArmorSave::FOUR_PLUS.passes(3));
        assert!(ArmorSave::FOUR_PLUS.passes(4));
    }

    #[test]
    fn test_invulnerable_save() {
        assert!(InvulnerableSave::FOUR_PLUS.has_invulnerable());
        assert!(!InvulnerableSave::NONE.has_invulnerable());
        assert!(InvulnerableSave::FOUR_PLUS.passes(4));
        assert!(!InvulnerableSave::FOUR_PLUS.passes(3));
    }

    #[test]
    fn test_damage_values() {
        assert_eq!(Damage::Fixed(2).min_value(), 2);
        assert_eq!(Damage::Fixed(2).max_value(), 2);
        assert_eq!(Damage::D3.min_value(), 1);
        assert_eq!(Damage::D3.max_value(), 3);
        assert_eq!(Damage::D6Plus(1).min_value(), 2);
        assert_eq!(Damage::D6Plus(1).max_value(), 7);
    }

    #[test]
    fn test_skill() {
        assert!(Skill::TWO_PLUS.hits(2));
        assert!(Skill::TWO_PLUS.hits(6));
        assert!(!Skill::TWO_PLUS.hits(1));
        assert!(!Skill::FOUR_PLUS.hits(3));
        assert!(Skill::FOUR_PLUS.hits(4));
    }

    #[test]
    fn test_feel_no_pain() {
        assert!(FeelNoPain::FIVE_PLUS.has_fnp());
        assert!(!FeelNoPain::NONE.has_fnp());
        assert!(FeelNoPain::FIVE_PLUS.passes(5));
        assert!(!FeelNoPain::FIVE_PLUS.passes(4));
    }

    #[test]
    fn test_attack_count() {
        assert_eq!(AttackCount::Fixed(5).min_value(), 5);
        assert_eq!(AttackCount::Fixed(5).max_value(), 5);
        assert_eq!(AttackCount::D6.min_value(), 1);
        assert_eq!(AttackCount::D6.max_value(), 6);
        assert_eq!(AttackCount::D6Plus(2).min_value(), 3);
    }

    #[test]
    fn test_saturating_arithmetic() {
        let w = Wounds::new(3);
        assert_eq!(w.saturating_sub(Wounds::new(5)), Wounds::ZERO);
        assert_eq!(w.saturating_add(Wounds::new(250)), Wounds::new(253));
    }

    #[test]
    fn test_resource_serialization() {
        let cp = CommandPoints::new(3);
        let json = serde_json::to_string(&cp).unwrap();
        let back: CommandPoints = serde_json::from_str(&json).unwrap();
        assert_eq!(cp, back);
    }
}

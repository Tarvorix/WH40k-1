//! Measurement types for deterministic game geometry.
//!
//! Uses fixed-point integer arithmetic (thousandths of an inch) to ensure
//! deterministic behavior across platforms. No floating-point in game logic.
//!
//! Source: implementation_v3.md Section 6.2

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub, Mul, Div, Neg};

/// Fixed-point distance in thousandths of an inch.
/// 1 inch = 1000 mils. This gives sub-millimeter precision while remaining
/// fully deterministic with integer arithmetic.
///
/// Example: 6 inches = Inches(6000), 0.5 inches = Inches(500)
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct Inches(pub i32);

impl Inches {
    /// Number of mils per inch
    pub const MILS_PER_INCH: i32 = 1000;

    /// Zero distance
    pub const ZERO: Inches = Inches(0);

    /// Create from whole inches
    pub const fn from_inches(inches: i32) -> Self {
        Self(inches * Self::MILS_PER_INCH)
    }

    /// Create from mils (thousandths of an inch)
    pub const fn from_mils(mils: i32) -> Self {
        Self(mils)
    }

    /// Create from inches with fractional part (e.g., 1.5 inches = from_inches_frac(1, 500))
    pub const fn from_inches_frac(whole: i32, frac_mils: i32) -> Self {
        Self(whole * Self::MILS_PER_INCH + frac_mils)
    }

    /// Get the raw value in mils
    pub const fn mils(self) -> i32 {
        self.0
    }

    /// Get the whole inches part
    pub const fn whole_inches(self) -> i32 {
        self.0 / Self::MILS_PER_INCH
    }

    /// Get the fractional mils part
    pub const fn frac_mils(self) -> i32 {
        self.0 % Self::MILS_PER_INCH
    }

    /// Convert to f64 for display purposes only (never use in game logic)
    pub fn as_f64(self) -> f64 {
        self.0 as f64 / Self::MILS_PER_INCH as f64
    }

    /// Absolute value
    pub const fn abs(self) -> Self {
        if self.0 < 0 {
            Self(-self.0)
        } else {
            self
        }
    }

    /// Maximum of two distances
    pub const fn max(self, other: Self) -> Self {
        if self.0 > other.0 {
            self
        } else {
            other
        }
    }

    /// Minimum of two distances
    pub const fn min(self, other: Self) -> Self {
        if self.0 < other.0 {
            self
        } else {
            other
        }
    }

    // Common game distances
    /// Engagement range horizontal (1 inch)
    pub const ENGAGEMENT_RANGE: Inches = Inches::from_inches(1);
    /// Engagement range vertical (5 inches)
    pub const ENGAGEMENT_RANGE_VERTICAL: Inches = Inches::from_inches(5);
    /// Objective control range horizontal (3 inches)
    pub const OBJECTIVE_RANGE: Inches = Inches::from_inches(3);
    /// Objective control range vertical (5 inches)
    pub const OBJECTIVE_RANGE_VERTICAL: Inches = Inches::from_inches(5);
    /// Coherency range horizontal (2 inches)
    pub const COHERENCY_RANGE: Inches = Inches::from_inches(2);
    /// Coherency range vertical (5 inches)
    pub const COHERENCY_RANGE_VERTICAL: Inches = Inches::from_inches(5);
    /// Standard pile-in distance (3 inches)
    pub const PILE_IN: Inches = Inches::from_inches(3);
    /// Extended pile-in distance (6 inches, Gilded Spear / Rage-fuelled)
    pub const PILE_IN_EXTENDED: Inches = Inches::from_inches(6);
    /// Standard consolidation distance (3 inches)
    pub const CONSOLIDATE: Inches = Inches::from_inches(3);
    /// Extended consolidation distance (6 inches)
    pub const CONSOLIDATE_EXTENDED: Inches = Inches::from_inches(6);
    /// Deep Strike minimum distance from enemies (9 inches)
    pub const DEEP_STRIKE_MIN_DISTANCE: Inches = Inches::from_inches(9);
    /// Heroic Intervention range (6 inches)
    pub const HEROIC_INTERVENTION: Inches = Inches::from_inches(6);
}

impl Add for Inches {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Inches {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Mul<i32> for Inches {
    type Output = Self;
    fn mul(self, rhs: i32) -> Self {
        Self(self.0 * rhs)
    }
}

impl Div<i32> for Inches {
    type Output = Self;
    fn div(self, rhs: i32) -> Self {
        Self(self.0 / rhs)
    }
}

impl Neg for Inches {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl fmt::Debug for Inches {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whole = self.whole_inches();
        let frac = self.frac_mils().abs();
        if frac == 0 {
            write!(f, "{}\"", whole)
        } else {
            write!(f, "{}.{:03}\"", whole, frac)
        }
    }
}

impl fmt::Display for Inches {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// 2D position on the battlefield in fixed-point mils.
/// Origin is bottom-left corner of the board.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Position {
    pub x: Inches,
    pub y: Inches,
}

impl Position {
    pub const ORIGIN: Position = Position {
        x: Inches::ZERO,
        y: Inches::ZERO,
    };

    pub const fn new(x: Inches, y: Inches) -> Self {
        Self { x, y }
    }

    /// Create from whole inch coordinates
    pub const fn from_inches(x: i32, y: i32) -> Self {
        Self {
            x: Inches::from_inches(x),
            y: Inches::from_inches(y),
        }
    }

    /// Squared distance between two positions (avoids sqrt for comparison).
    /// Returns value in mils^2.
    pub fn distance_squared(self, other: Position) -> i64 {
        let dx = (self.x.0 - other.x.0) as i64;
        let dy = (self.y.0 - other.y.0) as i64;
        dx * dx + dy * dy
    }

    /// Euclidean distance between two positions using integer sqrt approximation.
    /// Uses isqrt for deterministic integer square root.
    pub fn distance(self, other: Position) -> Inches {
        let dist_sq = self.distance_squared(other);
        Inches(isqrt(dist_sq) as i32)
    }

    /// Check if this position is within a given range of another position.
    /// Uses squared comparison to avoid sqrt when possible.
    pub fn within_range(self, other: Position, range: Inches) -> bool {
        let range_sq = (range.0 as i64) * (range.0 as i64);
        self.distance_squared(other) <= range_sq
    }

    /// Translate position by an offset
    pub fn translate(self, dx: Inches, dy: Inches) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }
}

impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// Integer square root using Newton's method.
/// Deterministic across all platforms.
fn isqrt(n: i64) -> i64 {
    if n < 0 {
        return 0;
    }
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Model base size in millimeters (diameter).
/// Source: Standard base sizes for 40K models
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BaseSize(pub u16);

impl BaseSize {
    /// 25mm base (standard infantry)
    pub const MM25: BaseSize = BaseSize(25);
    /// 28mm base
    pub const MM28: BaseSize = BaseSize(28);
    /// 32mm base (standard Marines, Custodes infantry)
    pub const MM32: BaseSize = BaseSize(32);
    /// 40mm base (Terminators, some characters)
    pub const MM40: BaseSize = BaseSize(40);
    /// 50mm base (large infantry, small monsters)
    pub const MM50: BaseSize = BaseSize(50);
    /// 60mm base (Daemon Prince, larger models)
    pub const MM60: BaseSize = BaseSize(60);
    /// 80mm base (large monsters)
    pub const MM80: BaseSize = BaseSize(80);
    /// Objective marker size
    pub const OBJECTIVE_40MM: BaseSize = BaseSize(40);

    pub const fn diameter_mm(self) -> u16 {
        self.0
    }

    /// Convert to inches (approximate, for game distance purposes).
    /// 25.4mm = 1 inch
    pub fn radius_inches(self) -> Inches {
        // radius in mils = (diameter_mm / 2) * 1000 / 25.4
        // Simplified: radius_mils = diameter_mm * 500 / 254 ≈ diameter_mm * 1.969
        // Use integer math: (diameter * 500 * 100) / 2540
        let radius_mils = (self.0 as i32 * 50000) / 2540;
        Inches(radius_mils)
    }
}

/// Board dimensions for Combat Patrol.
/// Source: CP_Rules.md - "44\" x 30\" battlefield"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BoardDimensions {
    pub width: Inches,
    pub height: Inches,
}

impl BoardDimensions {
    /// Standard Combat Patrol board: 44" x 30"
    pub const COMBAT_PATROL: BoardDimensions = BoardDimensions {
        width: Inches::from_inches(44),
        height: Inches::from_inches(30),
    };

    pub fn contains(&self, pos: Position) -> bool {
        pos.x >= Inches::ZERO
            && pos.x <= self.width
            && pos.y >= Inches::ZERO
            && pos.y <= self.height
    }
}

/// Axis-aligned bounding rectangle for terrain/zones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rect {
    pub min_x: Inches,
    pub min_y: Inches,
    pub max_x: Inches,
    pub max_y: Inches,
}

impl Rect {
    pub const fn new(min_x: Inches, min_y: Inches, max_x: Inches, max_y: Inches) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    pub fn from_inches(min_x: i32, min_y: i32, max_x: i32, max_y: i32) -> Self {
        Self {
            min_x: Inches::from_inches(min_x),
            min_y: Inches::from_inches(min_y),
            max_x: Inches::from_inches(max_x),
            max_y: Inches::from_inches(max_y),
        }
    }

    pub fn contains(&self, pos: Position) -> bool {
        pos.x >= self.min_x
            && pos.x <= self.max_x
            && pos.y >= self.min_y
            && pos.y <= self.max_y
    }

    pub fn width(&self) -> Inches {
        self.max_x - self.min_x
    }

    pub fn height(&self) -> Inches {
        self.max_y - self.min_y
    }

    pub fn center(&self) -> Position {
        Position {
            x: Inches((self.min_x.0 + self.max_x.0) / 2),
            y: Inches((self.min_y.0 + self.max_y.0) / 2),
        }
    }
}

/// Simple polygon represented as a list of vertices for deployment zones.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Polygon {
    pub vertices: Vec<Position>,
}

impl Polygon {
    pub fn new(vertices: Vec<Position>) -> Self {
        Self { vertices }
    }

    /// Point-in-polygon test using ray casting algorithm.
    /// Deterministic using integer arithmetic.
    pub fn contains(&self, point: Position) -> bool {
        let n = self.vertices.len();
        if n < 3 {
            return false;
        }

        let mut inside = false;
        let mut j = n - 1;
        for i in 0..n {
            let yi = self.vertices[i].y.0 as i64;
            let yj = self.vertices[j].y.0 as i64;
            let xi = self.vertices[i].x.0 as i64;
            let xj = self.vertices[j].x.0 as i64;
            let px = point.x.0 as i64;
            let py = point.y.0 as i64;

            // Check if the edge crosses the horizontal ray from point going right
            if (yi > py) != (yj > py) {
                // Compute x-intersection of edge with horizontal line y=py
                // x_intersect = xi + (xj - xi) * (py - yi) / (yj - yi)
                // Compare: px < x_intersect
                // Equivalent to: (px - xi) * (yj - yi) < (xj - xi) * (py - yi)
                // But we must account for the sign of (yj - yi)
                let lhs = (px - xi) * (yj - yi);
                let rhs = (xj - xi) * (py - yi);
                if (yj > yi && lhs < rhs) || (yj < yi && lhs > rhs) {
                    inside = !inside;
                }
            }
            j = i;
        }
        inside
    }

    /// Check if a point is wholly within the polygon (all points of the model's
    /// base must be inside). For simplicity, we check if the center point is inside
    /// with a margin equal to the base radius.
    pub fn wholly_contains_circle(&self, center: Position, radius: Inches) -> bool {
        // A conservative check: the center must be inside, and
        // the center must be at least 'radius' away from all edges.
        if !self.contains(center) {
            return false;
        }

        let n = self.vertices.len();
        let radius_sq = (radius.0 as i64) * (radius.0 as i64);
        for i in 0..n {
            let j = (i + 1) % n;
            let dist_sq = point_to_segment_distance_squared(
                center,
                self.vertices[i],
                self.vertices[j],
            );
            if dist_sq < radius_sq {
                return false;
            }
        }
        true
    }
}

/// Squared distance from a point to a line segment.
/// Uses integer arithmetic for determinism.
fn point_to_segment_distance_squared(p: Position, a: Position, b: Position) -> i64 {
    let ab_x = (b.x.0 - a.x.0) as i64;
    let ab_y = (b.y.0 - a.y.0) as i64;
    let ap_x = (p.x.0 - a.x.0) as i64;
    let ap_y = (p.y.0 - a.y.0) as i64;

    let ab_sq = ab_x * ab_x + ab_y * ab_y;
    if ab_sq == 0 {
        // a and b are the same point
        return ap_x * ap_x + ap_y * ap_y;
    }

    // Project p onto line ab, clamped to [0, 1]
    let t_num = ap_x * ab_x + ap_y * ab_y;

    if t_num <= 0 {
        // Closest to a
        ap_x * ap_x + ap_y * ap_y
    } else if t_num >= ab_sq {
        // Closest to b
        let bp_x = (p.x.0 - b.x.0) as i64;
        let bp_y = (p.y.0 - b.y.0) as i64;
        bp_x * bp_x + bp_y * bp_y
    } else {
        // Closest point is on the segment
        // dist^2 = |ap|^2 - (ap . ab)^2 / |ab|^2
        let ap_sq = ap_x * ap_x + ap_y * ap_y;
        ap_sq - (t_num * t_num) / ab_sq
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inches_creation() {
        assert_eq!(Inches::from_inches(6).mils(), 6000);
        assert_eq!(Inches::from_mils(1500).mils(), 1500);
        assert_eq!(Inches::from_inches_frac(1, 500).mils(), 1500);
    }

    #[test]
    fn test_inches_parts() {
        let d = Inches::from_mils(6500);
        assert_eq!(d.whole_inches(), 6);
        assert_eq!(d.frac_mils(), 500);
    }

    #[test]
    fn test_inches_arithmetic() {
        let a = Inches::from_inches(3);
        let b = Inches::from_inches(2);
        assert_eq!((a + b).mils(), 5000);
        assert_eq!((a - b).mils(), 1000);
        assert_eq!((a * 2).mils(), 6000);
        assert_eq!((a / 3).mils(), 1000);
        assert_eq!((-a).mils(), -3000);
    }

    #[test]
    fn test_inches_comparison() {
        assert!(Inches::from_inches(3) < Inches::from_inches(6));
        assert!(Inches::from_inches(6) > Inches::from_inches(3));
        assert_eq!(Inches::from_inches(5), Inches::from_inches(5));
    }

    #[test]
    fn test_inches_display() {
        assert_eq!(format!("{}", Inches::from_inches(6)), "6\"");
        assert_eq!(format!("{}", Inches::from_mils(6500)), "6.500\"");
    }

    #[test]
    fn test_position_distance() {
        let a = Position::from_inches(0, 0);
        let b = Position::from_inches(3, 4);
        let dist = a.distance(b);
        assert_eq!(dist, Inches::from_inches(5)); // 3-4-5 triangle
    }

    #[test]
    fn test_position_distance_zero() {
        let a = Position::from_inches(5, 5);
        assert_eq!(a.distance(a), Inches::ZERO);
    }

    #[test]
    fn test_position_within_range() {
        let a = Position::from_inches(0, 0);
        let b = Position::from_inches(3, 4);
        assert!(a.within_range(b, Inches::from_inches(5)));
        assert!(a.within_range(b, Inches::from_inches(6)));
        assert!(!a.within_range(b, Inches::from_inches(4)));
    }

    #[test]
    fn test_board_dimensions() {
        let board = BoardDimensions::COMBAT_PATROL;
        assert_eq!(board.width, Inches::from_inches(44));
        assert_eq!(board.height, Inches::from_inches(30));
        assert!(board.contains(Position::from_inches(22, 15)));
        assert!(board.contains(Position::from_inches(0, 0)));
        assert!(board.contains(Position::from_inches(44, 30)));
        assert!(!board.contains(Position::from_inches(45, 15)));
        assert!(!board.contains(Position::from_inches(-1, 15)));
    }

    #[test]
    fn test_rect_contains() {
        let r = Rect::from_inches(10, 10, 20, 20);
        assert!(r.contains(Position::from_inches(15, 15)));
        assert!(r.contains(Position::from_inches(10, 10)));
        assert!(!r.contains(Position::from_inches(9, 15)));
        assert!(!r.contains(Position::from_inches(21, 15)));
    }

    #[test]
    fn test_rect_dimensions() {
        let r = Rect::from_inches(10, 5, 20, 15);
        assert_eq!(r.width(), Inches::from_inches(10));
        assert_eq!(r.height(), Inches::from_inches(10));
        assert_eq!(r.center(), Position::from_inches(15, 10));
    }

    #[test]
    fn test_polygon_contains_square() {
        let poly = Polygon::new(vec![
            Position::from_inches(0, 0),
            Position::from_inches(10, 0),
            Position::from_inches(10, 10),
            Position::from_inches(0, 10),
        ]);
        assert!(poly.contains(Position::from_inches(5, 5)));
        assert!(!poly.contains(Position::from_inches(11, 5)));
        assert!(!poly.contains(Position::from_inches(-1, 5)));
    }

    #[test]
    fn test_polygon_contains_triangle() {
        let poly = Polygon::new(vec![
            Position::from_inches(0, 0),
            Position::from_inches(10, 0),
            Position::from_inches(5, 10),
        ]);
        assert!(poly.contains(Position::from_inches(5, 3)));
        assert!(!poly.contains(Position::from_inches(0, 10)));
    }

    #[test]
    fn test_base_size() {
        let base = BaseSize::MM32;
        assert_eq!(base.diameter_mm(), 32);
        // 32mm / 2 = 16mm radius
        // 16mm / 25.4mm = ~0.630 inches = ~630 mils
        let radius = base.radius_inches();
        assert!(radius.mils() > 600 && radius.mils() < 660);
    }

    #[test]
    fn test_inches_serialization() {
        let d = Inches::from_inches(6);
        let json = serde_json::to_string(&d).unwrap();
        let back: Inches = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }

    #[test]
    fn test_position_serialization() {
        let p = Position::from_inches(10, 20);
        let json = serde_json::to_string(&p).unwrap();
        let back: Position = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn test_isqrt_values() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(1), 1);
        assert_eq!(isqrt(4), 2);
        assert_eq!(isqrt(9), 3);
        assert_eq!(isqrt(25), 5);
        assert_eq!(isqrt(100), 10);
        // Non-perfect square: isqrt(8) should be 2 (floor)
        assert_eq!(isqrt(8), 2);
    }

    #[test]
    fn test_engagement_range_constants() {
        assert_eq!(Inches::ENGAGEMENT_RANGE, Inches::from_inches(1));
        assert_eq!(Inches::OBJECTIVE_RANGE, Inches::from_inches(3));
        assert_eq!(Inches::COHERENCY_RANGE, Inches::from_inches(2));
        assert_eq!(Inches::DEEP_STRIKE_MIN_DISTANCE, Inches::from_inches(9));
    }
}

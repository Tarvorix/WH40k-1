//! WH40K Engine - Core Types crate
//!
//! Defines stable foundational types: IDs, enums, measurements, resources,
//! keywords, and weapon abilities. This crate has almost no rules logic.
//!
//! Source: implementation_v3.md Section 6.1 (Layer A - Core types)

pub mod ids;
pub mod enums;
pub mod measurements;
pub mod resources;
pub mod keywords;
pub mod weapons;

// Re-export everything at crate root for convenience
pub use ids::*;
pub use enums::*;
pub use measurements::*;
pub use resources::*;
pub use keywords::*;
pub use weapons::*;

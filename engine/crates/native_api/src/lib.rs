//! WH40K Engine - Native API crate
//!
//! Provides headless CLI commands for the WH40K Combat Patrol engine:
//! - `play` - Run AI vs AI games
//! - `benchmark` - Run AI performance benchmarks
//! - `verify` - Verify replay determinism
//! - `selfplay` - Generate self-play training data
//!
//! Source: implementation_v3.md Phase 10

pub mod play;
pub mod benchmark;
pub mod verify;
pub mod selfplay_cmd;

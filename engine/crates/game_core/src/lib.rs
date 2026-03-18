//! WH40K Engine - GameCore crate
//!
//! The CENTRAL crate of the engine containing:
//! - Game state model (state, unit, effect)
//! - Command validation and execution
//! - Phase state machine and progression
//! - Scenario loading and game initialization
//!
//! Source: implementation_v3.md Sections 8.1-8.6 (State model),
//!         6.4 (Command system), 6.6 (Rules runtime)

pub mod state;
pub mod unit;
pub mod effect;
pub mod phase;
pub mod validator;
pub mod executor;
pub mod scenario;
pub mod boarding_unit_builder;
pub mod boarding_map_layouts;
pub mod combat;
pub mod stratagem;
pub mod scoring;
pub mod hasher;

// Re-export primary types at crate root for convenience
pub use state::{
    GameState, PlayerState, TurnFlags, FactionRoundFlags,
    StratagemUsageTracker, MissionProgress,
};
pub use unit::{UnitState, ModelState, ReserveType, WargearAbilityState, check_unit_coherency, models_to_remove_for_coherency};
pub use effect::{ActiveEffect, EffectSource, EffectType};
pub use phase::PhaseStateMachine;
pub use validator::CommandValidator;
pub use executor::{CommandExecutor, ExecutionError};
pub use scenario::ScenarioLoader;

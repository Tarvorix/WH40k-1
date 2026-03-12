//! WH40K Engine - MCP Server crate
//!
//! Implements the Model Context Protocol (MCP) server for the WH40K Combat Patrol
//! engine, allowing external AI agents (ChatGPT, Claude, Gemini, etc.) to play
//! complete games through structured tool calls.
//!
//! # Architecture
//!
//! The MCP server provides 10 tools over JSON-RPC 2.0:
//!
//! - `game.create_session` - Create a new game session
//! - `game.load_scenario` - Load a scenario from saved state
//! - `game.get_observation` - Get game state observation (public/private/debug)
//! - `game.list_legal_actions` - List all legal actions
//! - `game.apply_action` - Apply an action by index
//! - `game.step_until_decision` - Auto-advance using AI
//! - `game.get_replay_log` - Get replay data
//! - `game.get_score` - Get current scores
//! - `game.reset` - Reset session to initial state
//! - `game.end_session` - Clean up session
//!
//! # Observation Model
//!
//! Three view modes control information visibility:
//! - **Public**: information any legal player can see
//! - **Player(N)**: player-specific view (includes private data)
//! - **Debug**: complete internal state for testing
//!
//! # Transport
//!
//! Uses Content-Length framed JSON-RPC 2.0 over stdio (same as LSP).
//!
//! Source: implementation_v3.md Phase 10, mcp_api.md

pub mod error;
pub mod observation;
pub mod protocol;
pub mod server;
pub mod session;
pub mod tools;

// Re-export key types for convenience
pub use error::McpError;
pub use observation::{GameObservation, ViewMode, build_observation};
pub use protocol::{
    JsonRpcRequest, JsonRpcResponse, JsonRpcError,
    ToolCallParams, ToolCallResult, ToolDefinition,
    InitializeResult, ServerCapabilities, ServerInfo,
    build_tool_definitions,
};
pub use server::McpServer;
pub use session::{Session, SessionConfig, SessionManager};
pub use tools::dispatch_tool;

//! MCP Server error types.
//!
//! Source: implementation_v3.md Phase 10

use crate::protocol::{JsonRpcError, ToolCallResult};

/// Errors that can occur during MCP server operations.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session already exists: {0}")]
    SessionAlreadyExists(String),

    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("Invalid faction: {0}. Must be 'custodes' or 'world_eaters'")]
    InvalidFaction(String),

    #[error("Invalid mission: {0}. Must be 1-6")]
    InvalidMission(u32),

    #[error("Invalid view mode: {0}. Must be 'public', 'player_0', 'player_1', or 'debug'")]
    InvalidViewMode(String),

    #[error("Invalid AI level: {0}. Must be 'greedy', 'one_ply', or 'negamax'")]
    InvalidAiLevel(String),

    #[error("Action index out of range: {index} (available: {available})")]
    ActionIndexOutOfRange { index: usize, available: usize },

    #[error("Game already ended")]
    GameAlreadyEnded,

    #[error("No legal actions available")]
    NoLegalActions,

    #[error("Engine error: {0}")]
    EngineError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Step limit exceeded after {0} steps")]
    StepLimitExceeded(u32),

    #[error("Invalid seed format: {0}")]
    InvalidSeed(String),
}

impl McpError {
    /// Convert to a JSON-RPC error for protocol-level errors.
    pub fn to_json_rpc_error(&self) -> JsonRpcError {
        match self {
            McpError::InvalidArguments(msg) => JsonRpcError::invalid_params(msg),
            McpError::ProtocolError(msg) => JsonRpcError::invalid_request(msg),
            McpError::IoError(e) => JsonRpcError::internal_error(&e.to_string()),
            _ => JsonRpcError::internal_error(&self.to_string()),
        }
    }

    /// Convert to a tool call error result for tool-level errors.
    pub fn to_tool_error(&self) -> ToolCallResult {
        ToolCallResult::error(self.to_string())
    }
}

impl From<serde_json::Error> for McpError {
    fn from(e: serde_json::Error) -> Self {
        McpError::SerializationError(e.to_string())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_not_found() {
        let err = McpError::SessionNotFound("abc123".to_string());
        assert!(err.to_string().contains("abc123"));
    }

    #[test]
    fn test_invalid_faction() {
        let err = McpError::InvalidFaction("orks".to_string());
        assert!(err.to_string().contains("orks"));
        assert!(err.to_string().contains("custodes"));
    }

    #[test]
    fn test_action_index_out_of_range() {
        let err = McpError::ActionIndexOutOfRange {
            index: 10,
            available: 5,
        };
        assert!(err.to_string().contains("10"));
        assert!(err.to_string().contains("5"));
    }

    #[test]
    fn test_to_json_rpc_error() {
        let err = McpError::InvalidArguments("bad param".to_string());
        let rpc_err = err.to_json_rpc_error();
        assert_eq!(rpc_err.code, -32602);
    }

    #[test]
    fn test_to_tool_error() {
        let err = McpError::GameAlreadyEnded;
        let result = err.to_tool_error();
        assert_eq!(result.is_error, Some(true));
        assert!(result.content[0].text.contains("Game already ended"));
    }

    #[test]
    fn test_from_serde_error() {
        let json_err = serde_json::from_str::<i32>("not a number").unwrap_err();
        let mcp_err: McpError = json_err.into();
        assert!(matches!(mcp_err, McpError::SerializationError(_)));
    }
}

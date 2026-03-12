//! MCP Protocol types - JSON-RPC 2.0 message format and MCP-specific types.
//!
//! Implements the Model Context Protocol (MCP) over JSON-RPC 2.0 for
//! stdio-based communication with external AI agents.
//!
//! Source: implementation_v3.md Phase 10, mcp_api.md

use serde::{Deserialize, Serialize};

// ============================================================================
// JSON-RPC 2.0 Base Types
// ============================================================================

/// A JSON-RPC 2.0 request message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    /// Check if this is a notification (no id = no response expected).
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

/// A JSON-RPC 2.0 success response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a success response.
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: Option<serde_json::Value>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    /// Standard JSON-RPC error: Parse error (-32700)
    pub fn parse_error(msg: &str) -> Self {
        Self {
            code: -32700,
            message: format!("Parse error: {}", msg),
            data: None,
        }
    }

    /// Standard JSON-RPC error: Invalid request (-32600)
    pub fn invalid_request(msg: &str) -> Self {
        Self {
            code: -32600,
            message: format!("Invalid request: {}", msg),
            data: None,
        }
    }

    /// Standard JSON-RPC error: Method not found (-32601)
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method),
            data: None,
        }
    }

    /// Standard JSON-RPC error: Invalid params (-32602)
    pub fn invalid_params(msg: &str) -> Self {
        Self {
            code: -32602,
            message: format!("Invalid params: {}", msg),
            data: None,
        }
    }

    /// Standard JSON-RPC error: Internal error (-32603)
    pub fn internal_error(msg: &str) -> Self {
        Self {
            code: -32603,
            message: format!("Internal error: {}", msg),
            data: None,
        }
    }
}

// ============================================================================
// MCP Protocol Types
// ============================================================================

/// MCP protocol version supported by this server.
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Server information for the initialize response.
pub const SERVER_NAME: &str = "wh40k-combat-patrol";
pub const SERVER_VERSION: &str = "0.1.0";

/// MCP Initialize request params.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub protocol_version: String,
    #[serde(default)]
    pub capabilities: ClientCapabilities,
    #[serde(default)]
    pub client_info: Option<ClientInfo>,
}

/// Client capabilities (currently minimal).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientCapabilities {}

/// Client identification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
}

/// MCP Initialize response result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

/// Server capabilities advertised during initialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
}

/// Tools capability marker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {}

/// Server identification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// MCP tools/list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListResult {
    pub tools: Vec<ToolDefinition>,
}

/// Definition of a single MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP tools/call request params.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallParams {
    pub name: String,
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
}

/// MCP tools/call response result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isError")]
    pub is_error: Option<bool>,
}

impl ToolCallResult {
    /// Create a successful text result.
    pub fn text(text: String) -> Self {
        Self {
            content: vec![ContentBlock::text(text)],
            is_error: None,
        }
    }

    /// Create a successful JSON result.
    pub fn json<T: Serialize>(value: &T) -> Result<Self, serde_json::Error> {
        let text = serde_json::to_string_pretty(value)?;
        Ok(Self {
            content: vec![ContentBlock::text(text)],
            is_error: None,
        })
    }

    /// Create an error result.
    pub fn error(message: String) -> Self {
        Self {
            content: vec![ContentBlock::text(message)],
            is_error: Some(true),
        }
    }
}

/// A content block in tool call results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

impl ContentBlock {
    pub fn text(text: String) -> Self {
        Self {
            content_type: "text".to_string(),
            text,
        }
    }
}

// ============================================================================
// Tool Argument Types
// ============================================================================

/// Arguments for game.create_session tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionArgs {
    /// Faction for player A: "custodes" or "world_eaters"
    pub faction_a: String,
    /// Faction for player B: "custodes" or "world_eaters"
    pub faction_b: String,
    /// Mission number (1-6)
    pub mission: u32,
    /// Optional random seed (hex string). If omitted, a random seed is generated.
    #[serde(default)]
    pub seed: Option<String>,
    /// Optional enhancement for player A
    #[serde(default)]
    pub enhancement_a: Option<String>,
    /// Optional enhancement for player B
    #[serde(default)]
    pub enhancement_b: Option<String>,
    /// Optional secondary objective for player A
    #[serde(default)]
    pub secondary_a: Option<String>,
    /// Optional secondary objective for player B
    #[serde(default)]
    pub secondary_b: Option<String>,
}

/// Arguments for game.load_scenario tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadScenarioArgs {
    /// Session ID to load state into. If omitted, creates a new session.
    #[serde(default)]
    pub session_id: Option<String>,
    /// Serialized GameState JSON.
    pub state_json: String,
}

/// Arguments for game.get_observation tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetObservationArgs {
    /// Session ID.
    pub session_id: String,
    /// View mode: "public", "player_0", "player_1", or "debug"
    #[serde(default = "default_view_mode")]
    pub view_mode: String,
}

fn default_view_mode() -> String {
    "public".to_string()
}

/// Arguments for game.list_legal_actions tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListLegalActionsArgs {
    /// Session ID.
    pub session_id: String,
    /// Optional: specific player perspective. Defaults to current decision owner.
    #[serde(default)]
    pub player: Option<u32>,
}

/// Arguments for game.apply_action tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyActionArgs {
    /// Session ID.
    pub session_id: String,
    /// Action index from the legal actions list.
    pub action_index: usize,
}

/// Arguments for game.step_until_decision tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepUntilDecisionArgs {
    /// Session ID.
    pub session_id: String,
    /// AI level to use for auto-stepping: "greedy", "one_ply", "negamax"
    #[serde(default = "default_ai_level")]
    pub ai_level: String,
    /// Maximum number of steps before returning (safety limit).
    #[serde(default = "default_max_steps")]
    pub max_steps: u32,
}

fn default_ai_level() -> String {
    "greedy".to_string()
}

fn default_max_steps() -> u32 {
    1000
}

/// Arguments for game.get_replay_log tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetReplayLogArgs {
    /// Session ID.
    pub session_id: String,
    /// Format: "json" or "summary"
    #[serde(default = "default_replay_format")]
    pub format: String,
    /// Optionally limit to last N entries.
    #[serde(default)]
    pub last_n: Option<usize>,
}

fn default_replay_format() -> String {
    "summary".to_string()
}

/// Arguments for game.get_score tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetScoreArgs {
    /// Session ID.
    pub session_id: String,
}

/// Arguments for game.reset tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetArgs {
    /// Session ID.
    pub session_id: String,
}

/// Arguments for game.end_session tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndSessionArgs {
    /// Session ID.
    pub session_id: String,
}

// ============================================================================
// Tool Definition Builder
// ============================================================================

/// Build the complete list of MCP tool definitions for this server.
pub fn build_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "game.create_session".to_string(),
            description: "Create a new Combat Patrol game session. Returns a session ID and initial game state.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "faction_a": {
                        "type": "string",
                        "description": "Faction for player A: 'custodes' or 'world_eaters'",
                        "enum": ["custodes", "world_eaters"]
                    },
                    "faction_b": {
                        "type": "string",
                        "description": "Faction for player B: 'custodes' or 'world_eaters'",
                        "enum": ["custodes", "world_eaters"]
                    },
                    "mission": {
                        "type": "integer",
                        "description": "Mission number (1-6)",
                        "minimum": 1,
                        "maximum": 6
                    },
                    "seed": {
                        "type": "string",
                        "description": "Optional hex seed for deterministic RNG (32 hex chars). Random if omitted."
                    },
                    "enhancement_a": {
                        "type": "string",
                        "description": "Optional enhancement for player A (faction-specific)"
                    },
                    "enhancement_b": {
                        "type": "string",
                        "description": "Optional enhancement for player B (faction-specific)"
                    },
                    "secondary_a": {
                        "type": "string",
                        "description": "Optional secondary objective for player A (faction-specific)"
                    },
                    "secondary_b": {
                        "type": "string",
                        "description": "Optional secondary objective for player B (faction-specific)"
                    }
                },
                "required": ["faction_a", "faction_b", "mission"]
            }),
        },
        ToolDefinition {
            name: "game.load_scenario".to_string(),
            description: "Load a game state from serialized JSON into a new or existing session.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Optional session ID. Creates new session if omitted."
                    },
                    "state_json": {
                        "type": "string",
                        "description": "Serialized GameState JSON"
                    }
                },
                "required": ["state_json"]
            }),
        },
        ToolDefinition {
            name: "game.get_observation".to_string(),
            description: "Get the current game state observation. Supports public, player-specific, and debug view modes.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID"
                    },
                    "view_mode": {
                        "type": "string",
                        "description": "View mode: 'public', 'player_0', 'player_1', or 'debug'",
                        "enum": ["public", "player_0", "player_1", "debug"],
                        "default": "public"
                    }
                },
                "required": ["session_id"]
            }),
        },
        ToolDefinition {
            name: "game.list_legal_actions".to_string(),
            description: "List all legal actions available at the current decision point. Returns action indices, labels, intents, and involved units.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID"
                    },
                    "player": {
                        "type": "integer",
                        "description": "Optional player perspective (0 or 1). Defaults to current decision owner."
                    }
                },
                "required": ["session_id"]
            }),
        },
        ToolDefinition {
            name: "game.apply_action".to_string(),
            description: "Apply a legal action by its index from list_legal_actions. The engine validates, executes, and resolves all resulting events.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID"
                    },
                    "action_index": {
                        "type": "integer",
                        "description": "Index of the action from list_legal_actions",
                        "minimum": 0
                    }
                },
                "required": ["session_id", "action_index"]
            }),
        },
        ToolDefinition {
            name: "game.step_until_decision".to_string(),
            description: "Auto-advance the game using AI until the next human decision point or game end. Useful for skipping through non-interactive phases.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID"
                    },
                    "ai_level": {
                        "type": "string",
                        "description": "AI level: 'greedy', 'one_ply', or 'negamax'",
                        "enum": ["greedy", "one_ply", "negamax"],
                        "default": "greedy"
                    },
                    "max_steps": {
                        "type": "integer",
                        "description": "Safety limit on number of steps (default 1000)",
                        "default": 1000
                    }
                },
                "required": ["session_id"]
            }),
        },
        ToolDefinition {
            name: "game.get_replay_log".to_string(),
            description: "Get the replay log for the current game session. Can return full JSON or human-readable summary.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID"
                    },
                    "format": {
                        "type": "string",
                        "description": "Output format: 'json' or 'summary'",
                        "enum": ["json", "summary"],
                        "default": "summary"
                    },
                    "last_n": {
                        "type": "integer",
                        "description": "Optionally limit to last N frames"
                    }
                },
                "required": ["session_id"]
            }),
        },
        ToolDefinition {
            name: "game.get_score".to_string(),
            description: "Get the current score and game status for both players.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID"
                    }
                },
                "required": ["session_id"]
            }),
        },
        ToolDefinition {
            name: "game.reset".to_string(),
            description: "Reset a session to its initial state, replaying the same scenario from the beginning with the same seed.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID"
                    }
                },
                "required": ["session_id"]
            }),
        },
        ToolDefinition {
            name: "game.end_session".to_string(),
            description: "End and clean up a game session, freeing all associated resources.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID"
                    }
                },
                "required": ["session_id"]
            }),
        },
    ]
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request_deserialize() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "tools/list");
        assert_eq!(req.id, Some(serde_json::json!(1)));
        assert!(!req.is_notification());
    }

    #[test]
    fn test_json_rpc_notification() {
        let json = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert!(req.is_notification());
    }

    #[test]
    fn test_json_rpc_response_success() {
        let resp = JsonRpcResponse::success(
            Some(serde_json::json!(1)),
            serde_json::json!({"status": "ok"}),
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"result\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_json_rpc_response_error() {
        let resp = JsonRpcResponse::error(
            Some(serde_json::json!(1)),
            JsonRpcError::method_not_found("foo"),
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"error\""));
        assert!(json.contains("-32601"));
    }

    #[test]
    fn test_tool_call_result_text() {
        let result = ToolCallResult::text("Hello".to_string());
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0].content_type, "text");
        assert_eq!(result.content[0].text, "Hello");
        assert!(result.is_error.is_none());
    }

    #[test]
    fn test_tool_call_result_error() {
        let result = ToolCallResult::error("Something went wrong".to_string());
        assert_eq!(result.is_error, Some(true));
    }

    #[test]
    fn test_create_session_args_deserialize() {
        let json = r#"{"faction_a":"custodes","faction_b":"world_eaters","mission":1}"#;
        let args: CreateSessionArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.faction_a, "custodes");
        assert_eq!(args.faction_b, "world_eaters");
        assert_eq!(args.mission, 1);
        assert!(args.seed.is_none());
    }

    #[test]
    fn test_tool_definitions_count() {
        let tools = build_tool_definitions();
        assert_eq!(tools.len(), 10);
    }

    #[test]
    fn test_tool_definitions_names() {
        let tools = build_tool_definitions();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"game.create_session"));
        assert!(names.contains(&"game.load_scenario"));
        assert!(names.contains(&"game.get_observation"));
        assert!(names.contains(&"game.list_legal_actions"));
        assert!(names.contains(&"game.apply_action"));
        assert!(names.contains(&"game.step_until_decision"));
        assert!(names.contains(&"game.get_replay_log"));
        assert!(names.contains(&"game.get_score"));
        assert!(names.contains(&"game.reset"));
        assert!(names.contains(&"game.end_session"));
    }

    #[test]
    fn test_initialize_result_serialize() {
        let result = InitializeResult {
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {}),
            },
            server_info: ServerInfo {
                name: SERVER_NAME.to_string(),
                version: SERVER_VERSION.to_string(),
            },
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("wh40k-combat-patrol"));
        assert!(json.contains("2024-11-05"));
    }

    #[test]
    fn test_tool_definitions_serializable() {
        let tools = build_tool_definitions();
        let result = ToolsListResult { tools };
        let json = serde_json::to_string_pretty(&result).unwrap();
        assert!(json.contains("game.create_session"));
        assert!(json.contains("inputSchema"));
    }
}

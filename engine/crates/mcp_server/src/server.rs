//! MCP Server - JSON-RPC 2.0 over stdio transport.
//!
//! Reads Content-Length framed JSON-RPC messages from stdin,
//! dispatches to tool handlers, and writes responses to stdout.
//!
//! Source: implementation_v3.md Phase 10, mcp_api.md

use std::io::{self, BufRead, Write};

use crate::error::McpError;
use crate::protocol::*;
use crate::session::SessionManager;
use crate::tools::dispatch_tool;

// ============================================================================
// MCP Server
// ============================================================================

/// The MCP server that manages sessions and handles JSON-RPC requests.
pub struct McpServer {
    sessions: SessionManager,
    initialized: bool,
}

impl McpServer {
    /// Create a new MCP server.
    pub fn new() -> Self {
        Self {
            sessions: SessionManager::new(),
            initialized: false,
        }
    }

    /// Run the server on stdin/stdout.
    pub fn run(&mut self) -> Result<(), McpError> {
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        let stdout = io::stdout();
        let mut writer = stdout.lock();

        tracing::info!("WH40K MCP Server starting (stdio transport)");

        loop {
            match read_message(&mut reader) {
                Ok(Some(message)) => {
                    let response = self.handle_message(&message);
                    if let Some(resp) = response {
                        write_message(&mut writer, &resp)?;
                    }
                }
                Ok(None) => {
                    // EOF - client disconnected
                    tracing::info!("Client disconnected (EOF)");
                    break;
                }
                Err(e) => {
                    tracing::error!("Failed to read message: {}", e);
                    let error_resp = JsonRpcResponse::error(
                        None,
                        JsonRpcError::parse_error(&e.to_string()),
                    );
                    write_message(&mut writer, &error_resp)?;
                }
            }
        }

        Ok(())
    }

    /// Handle a single JSON-RPC request and return an optional response.
    pub fn handle_message(&mut self, request: &JsonRpcRequest) -> Option<JsonRpcResponse> {
        tracing::debug!("Handling method: {}", request.method);

        match request.method.as_str() {
            "initialize" => Some(self.handle_initialize(request)),
            "notifications/initialized" => {
                // Notification - no response
                tracing::info!("Client initialized notification received");
                None
            }
            "tools/list" => Some(self.handle_tools_list(request)),
            "tools/call" => Some(self.handle_tools_call(request)),
            "ping" => Some(JsonRpcResponse::success(
                request.id.clone(),
                serde_json::json!({}),
            )),
            _ => {
                // Unknown method
                Some(JsonRpcResponse::error(
                    request.id.clone(),
                    JsonRpcError::method_not_found(&request.method),
                ))
            }
        }
    }

    /// Handle the initialize request.
    fn handle_initialize(&mut self, request: &JsonRpcRequest) -> JsonRpcResponse {
        self.initialized = true;

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

        match serde_json::to_value(&result) {
            Ok(val) => JsonRpcResponse::success(request.id.clone(), val),
            Err(e) => JsonRpcResponse::error(
                request.id.clone(),
                JsonRpcError::internal_error(&e.to_string()),
            ),
        }
    }

    /// Handle tools/list request.
    fn handle_tools_list(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        let tools = build_tool_definitions();
        let result = ToolsListResult { tools };

        match serde_json::to_value(&result) {
            Ok(val) => JsonRpcResponse::success(request.id.clone(), val),
            Err(e) => JsonRpcResponse::error(
                request.id.clone(),
                JsonRpcError::internal_error(&e.to_string()),
            ),
        }
    }

    /// Handle tools/call request.
    fn handle_tools_call(&mut self, request: &JsonRpcRequest) -> JsonRpcResponse {
        let params = match &request.params {
            Some(p) => p.clone(),
            None => {
                return JsonRpcResponse::error(
                    request.id.clone(),
                    JsonRpcError::invalid_params("Missing params"),
                );
            }
        };

        let call_params: ToolCallParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => {
                return JsonRpcResponse::error(
                    request.id.clone(),
                    JsonRpcError::invalid_params(&e.to_string()),
                );
            }
        };

        tracing::info!("Tool call: {}", call_params.name);

        let tool_result = dispatch_tool(
            &call_params.name,
            call_params.arguments,
            &mut self.sessions,
        );

        match serde_json::to_value(&tool_result) {
            Ok(val) => JsonRpcResponse::success(request.id.clone(), val),
            Err(e) => JsonRpcResponse::error(
                request.id.clone(),
                JsonRpcError::internal_error(&e.to_string()),
            ),
        }
    }

    /// Get a reference to the session manager (for testing).
    pub fn sessions(&self) -> &SessionManager {
        &self.sessions
    }

    /// Get a mutable reference to the session manager (for testing).
    pub fn sessions_mut(&mut self) -> &mut SessionManager {
        &mut self.sessions
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// stdio Transport
// ============================================================================

/// Read a Content-Length framed JSON-RPC message from a reader.
/// Returns None on EOF.
pub fn read_message<R: BufRead>(reader: &mut R) -> Result<Option<JsonRpcRequest>, McpError> {
    // Read headers until we find Content-Length
    let mut content_length: Option<usize> = None;

    loop {
        let mut header_line = String::new();
        let bytes_read = reader.read_line(&mut header_line)
            .map_err(McpError::IoError)?;

        if bytes_read == 0 {
            return Ok(None); // EOF
        }

        let trimmed = header_line.trim();

        if trimmed.is_empty() {
            // Empty line = end of headers
            break;
        }

        if let Some(len_str) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(
                len_str.trim().parse::<usize>()
                    .map_err(|e| McpError::ProtocolError(format!("Invalid Content-Length: {}", e)))?,
            );
        }
        // Ignore other headers (Content-Type, etc.)
    }

    let content_length = content_length.ok_or_else(|| {
        McpError::ProtocolError("Missing Content-Length header".to_string())
    })?;

    // Read the JSON body
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).map_err(McpError::IoError)?;

    let body_str = String::from_utf8(body)
        .map_err(|e| McpError::ProtocolError(format!("Invalid UTF-8 in body: {}", e)))?;

    let request: JsonRpcRequest = serde_json::from_str(&body_str)
        .map_err(|e| McpError::ProtocolError(format!("Invalid JSON-RPC: {}", e)))?;

    Ok(Some(request))
}

/// Write a Content-Length framed JSON-RPC message to a writer.
pub fn write_message<W: Write>(writer: &mut W, response: &JsonRpcResponse) -> Result<(), McpError> {
    let body = serde_json::to_string(response)
        .map_err(|e| McpError::SerializationError(e.to_string()))?;

    let header = format!("Content-Length: {}\r\n\r\n", body.len());

    writer.write_all(header.as_bytes()).map_err(McpError::IoError)?;
    writer.write_all(body.as_bytes()).map_err(McpError::IoError)?;
    writer.flush().map_err(McpError::IoError)?;

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = McpServer::new();
        assert_eq!(server.sessions().session_count(), 0);
        assert!(!server.initialized);
    }

    #[test]
    fn test_handle_initialize() {
        let mut server = McpServer::new();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "initialize".to_string(),
            params: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "1.0"}
            })),
        };

        let response = server.handle_message(&request).unwrap();
        assert!(response.result.is_some());
        assert!(response.error.is_none());
        assert!(server.initialized);

        let result = response.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert!(result["capabilities"]["tools"].is_object());
    }

    #[test]
    fn test_handle_tools_list() {
        let mut server = McpServer::new();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(2)),
            method: "tools/list".to_string(),
            params: None,
        };

        let response = server.handle_message(&request).unwrap();
        assert!(response.result.is_some());

        let result = response.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 10);
    }

    #[test]
    fn test_handle_tools_call() {
        let mut server = McpServer::new();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(3)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "game.create_session",
                "arguments": {
                    "faction_a": "custodes",
                    "faction_b": "world_eaters",
                    "mission": 1
                }
            })),
        };

        let response = server.handle_message(&request).unwrap();
        assert!(response.result.is_some());
        assert!(response.error.is_none());
        assert_eq!(server.sessions().session_count(), 1);
    }

    #[test]
    fn test_handle_unknown_method() {
        let mut server = McpServer::new();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(4)),
            method: "unknown/method".to_string(),
            params: None,
        };

        let response = server.handle_message(&request).unwrap();
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32601);
    }

    #[test]
    fn test_handle_notification() {
        let mut server = McpServer::new();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "notifications/initialized".to_string(),
            params: None,
        };

        let response = server.handle_message(&request);
        assert!(response.is_none()); // Notifications don't get responses
    }

    #[test]
    fn test_handle_ping() {
        let mut server = McpServer::new();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(5)),
            method: "ping".to_string(),
            params: None,
        };

        let response = server.handle_message(&request).unwrap();
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_read_message() {
        let input = "Content-Length: 46\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}";
        let mut reader = io::BufReader::new(input.as_bytes());

        let request = read_message(&mut reader).unwrap().unwrap();
        assert_eq!(request.method, "tools/list");
        assert_eq!(request.id, Some(serde_json::json!(1)));
    }

    #[test]
    fn test_read_message_eof() {
        let input = "";
        let mut reader = io::BufReader::new(input.as_bytes());

        let result = read_message(&mut reader).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_write_message() {
        let response = JsonRpcResponse::success(
            Some(serde_json::json!(1)),
            serde_json::json!({"status": "ok"}),
        );

        let mut output = Vec::new();
        write_message(&mut output, &response).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.starts_with("Content-Length:"));
        assert!(output_str.contains("\"result\""));
    }

    #[test]
    fn test_roundtrip_message() {
        let response = JsonRpcResponse::success(
            Some(serde_json::json!(42)),
            serde_json::json!({"hello": "world"}),
        );

        let mut buffer = Vec::new();
        write_message(&mut buffer, &response).unwrap();

        // Verify the output is valid Content-Length framing
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.starts_with("Content-Length: "));
        assert!(output.contains("\r\n\r\n"));
    }

    #[test]
    fn test_full_session_lifecycle() {
        let mut server = McpServer::new();

        // 1. Initialize
        let init_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "initialize".to_string(),
            params: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {}
            })),
        };
        server.handle_message(&init_req);

        // 2. Create session
        let create_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(2)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "game.create_session",
                "arguments": {
                    "faction_a": "custodes",
                    "faction_b": "world_eaters",
                    "mission": 1,
                    "seed": "deadbeef"
                }
            })),
        };
        let resp = server.handle_message(&create_req).unwrap();
        assert!(resp.result.is_some());

        // 3. Get score
        let score_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(3)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "game.get_score",
                "arguments": {
                    "session_id": "session_1"
                }
            })),
        };
        let resp = server.handle_message(&score_req).unwrap();
        assert!(resp.result.is_some());

        // 4. End session
        let end_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(4)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "game.end_session",
                "arguments": {
                    "session_id": "session_1"
                }
            })),
        };
        let resp = server.handle_message(&end_req).unwrap();
        assert!(resp.result.is_some());
        assert_eq!(server.sessions().session_count(), 0);
    }
}

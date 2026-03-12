//! WH40K Combat Patrol MCP Server binary.
//!
//! Runs the MCP server over stdio, exposing the authoritative game engine
//! to external AI agents via the Model Context Protocol.
//!
//! Usage:
//!   wh40k-mcp-server
//!
//! The server reads JSON-RPC 2.0 messages from stdin (Content-Length framed)
//! and writes responses to stdout. Logging goes to stderr.

use wh40k_mcp_server::server::McpServer;

fn main() {
    // Initialize tracing to stderr (stdout is used for MCP protocol)
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("wh40k_mcp_server=info".parse().unwrap()),
        )
        .init();

    tracing::info!("WH40K Combat Patrol MCP Server v{}", env!("CARGO_PKG_VERSION"));

    let mut server = McpServer::new();

    if let Err(e) = server.run() {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}

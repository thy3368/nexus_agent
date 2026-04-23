//! MCP tool adapters

use crate::error::Result;
use crate::tool::traits::tool::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;

/// Skeleton MCP tool caller.
pub struct McpTool;

impl McpTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for McpTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        "mcp_call"
    }

    fn description(&self) -> &str {
        "Call an MCP tool by server and tool name. This is currently a skeleton until an MCP runtime is wired into Nexus."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "server": {
                    "type": "string",
                    "description": "MCP server name"
                },
                "tool": {
                    "type": "string",
                    "description": "MCP tool name"
                },
                "raw_arguments": {
                    "type": "object",
                    "description": "Raw arguments passed to the MCP tool"
                }
            },
            "required": ["server", "tool", "raw_arguments"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        _ctx: &ToolContext,
        _config: &crate::config::Config,
    ) -> Result<ToolResult> {
        Ok(ToolResult::error("MCP runtime is not wired into Nexus yet".to_string())
            .with_metadata("server", args.get("server").cloned().unwrap_or(serde_json::Value::Null))
            .with_metadata("tool", args.get("tool").cloned().unwrap_or(serde_json::Value::Null))
            .with_metadata(
                "raw_arguments",
                args.get("raw_arguments")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            ))
    }
}

/// Skeleton MCP resource reader.
pub struct McpResourceTool;

impl McpResourceTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for McpResourceTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for McpResourceTool {
    fn name(&self) -> &str {
        "mcp_read_resource"
    }

    fn description(&self) -> &str {
        "Read an MCP resource by server and uri. This is currently a skeleton until an MCP runtime is wired into Nexus."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "server": {
                    "type": "string",
                    "description": "MCP server name"
                },
                "uri": {
                    "type": "string",
                    "description": "MCP resource URI"
                }
            },
            "required": ["server", "uri"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        _ctx: &ToolContext,
        _config: &crate::config::Config,
    ) -> Result<ToolResult> {
        Ok(ToolResult::error("MCP resource runtime is not wired into Nexus yet".to_string())
            .with_metadata("server", args.get("server").cloned().unwrap_or(serde_json::Value::Null))
            .with_metadata("uri", args.get("uri").cloned().unwrap_or(serde_json::Value::Null)))
    }
}

use crate::error::ToolError;
use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}




/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ToolResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
            metadata: HashMap::new(),
        }
    }

    pub fn error(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error.into()),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Tool context passed to tools during execution
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub working_dir: std::path::PathBuf,
    pub env_vars: HashMap<String, String>,
    pub current_working_dir: std::path::PathBuf,
    pub git_branch: Option<String>,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_default(),
            env_vars: HashMap::new(),
            current_working_dir: std::env::current_dir().unwrap_or_default(),
            git_branch: None,
        }
    }
}

/// Tool trait for implementing actions
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique tool name
    fn name(&self) -> &str;

    /// Human-readable description
    fn description(&self) -> &str;

    /// Parameter schema (JSON Schema format)
    fn parameters(&self) -> serde_json::Value;

    /// Whether tool is read-only (safe for plan mode)
    fn is_read_only(&self) -> bool {
        false
    }

    /// Execute the tool with given arguments
    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
        config: &crate::config::Config,
    ) -> crate::Result<ToolResult>;

    /// Validate arguments before execution
    fn validate_args(&self, args: &serde_json::Value) -> crate::Result<()> {
        // Basic validation - check required fields
        let schema = self.parameters();
        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            for field in required {
                if let Some(field_name) = field.as_str() {
                    if args.get(field_name).is_none() {
                        return Err(ToolError::InvalidArgs(format!(
                            "Missing required field: {}",
                            field_name
                        ))
                        .into());
                    }
                }
            }
        }
        Ok(())
    }

    /// Convert tool to definition for model
    fn to_definition(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name(),
            "description": self.description(),
            "parameters": self.parameters(),
        })
    }
}

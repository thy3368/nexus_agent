use crate::error::ToolError;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub read_only: bool,
    pub supports_parallel_calls: bool,
}

impl ToolDefinition {
    pub fn new(name: impl Into<String>, description: impl Into<String>, parameters: Value) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
            read_only: false,
            supports_parallel_calls: false,
        }
    }

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub fn supports_parallel_calls(mut self, supports_parallel_calls: bool) -> Self {
        self.supports_parallel_calls = supports_parallel_calls;
        self
    }

    pub fn as_model_definition(&self) -> Value {
        serde_json::json!({
            "name": self.name,
            "description": self.description,
            "parameters": self.parameters,
        })
    }
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub metadata: HashMap<String, Value>,
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

    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn log_preview(&self) -> String {
        let text = if self.success {
            &self.output
        } else {
            self.error.as_deref().unwrap_or(&self.output)
        };

        const MAX_PREVIEW_CHARS: usize = 2_000;
        if text.chars().count() <= MAX_PREVIEW_CHARS {
            text.to_string()
        } else {
            let mut preview = text.chars().take(MAX_PREVIEW_CHARS).collect::<String>();
            preview.push_str("...");
            preview
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolKind {
    Function,
}

#[derive(Debug, Clone)]
pub struct ToolInvocation {
    pub name: String,
    pub args: Value,
    pub call_id: Option<String>,
}

impl ToolInvocation {
    pub fn new(name: impl Into<String>, args: Value) -> Self {
        Self {
            name: name.into(),
            args,
            call_id: None,
        }
    }

    pub fn with_call_id(mut self, call_id: impl Into<String>) -> Self {
        self.call_id = Some(call_id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolHookPayload {
    pub tool_name: String,
    pub command: String,
}

pub type BoxToolFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// 参考下 /Users/hongyaotang/src/code_agent/codex/codex-rs/core/src/tools/registry.rs 的ToolHandler 补齐能力
/// Tool trait for implementing actions
/// 有哪些 /Users/hongyaotang/src/code_agent/codex/codex-rs/core/src/tools/下的 ToolHandler实现， 将它的能力迁移一价到 /Users/hongyaotang/src/code_agent/nexus_agent/src/tool/adapter
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Unique tool name
    fn name(&self) -> &str;

    /// Human-readable description
    fn description(&self) -> &str;

    /// Parameter schema (JSON Schema format)
    fn parameters(&self) -> Value;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    /// Whether tool is read-only (safe for plan mode)
    fn is_read_only(&self) -> bool {
        false
    }

    fn supports_parallel_calls(&self) -> bool {
        self.is_read_only()
    }

    fn is_mutating<'a>(&'a self, _invocation: &'a ToolInvocation) -> BoxToolFuture<'a, bool> {
        Box::pin(async move { !self.is_read_only() })
    }

    fn pre_tool_use_payload(&self, invocation: &ToolInvocation) -> Option<ToolHookPayload> {
        Some(ToolHookPayload {
            tool_name: self.name().to_string(),
            command: format_tool_command(self.name(), &invocation.args),
        })
    }

    fn post_tool_use_payload(
        &self,
        invocation: &ToolInvocation,
        result: &ToolResult,
    ) -> Option<ToolHookPayload> {
        let command = serde_json::json!({
            "input": invocation.args,
            "result": {
                "success": result.success,
                "output": result.output,
                "error": result.error,
                "metadata": result.metadata,
            }
        })
        .to_string();

        Some(ToolHookPayload {
            tool_name: self.name().to_string(),
            command,
        })
    }

    /// Execute the tool with given arguments
    async fn execute(
        &self,
        args: Value,
        ctx: &ToolContext,
        config: &crate::config::Config,
    ) -> crate::Result<ToolResult>;

    fn handle<'a>(
        &'a self,
        invocation: ToolInvocation,
        ctx: &'a ToolContext,
        config: &'a crate::config::Config,
    ) -> BoxToolFuture<'a, crate::Result<ToolResult>> {
        Box::pin(async move {
            if invocation.name != self.name() {
                return Err(ToolError::NotFound(invocation.name).into());
            }

            self.validate_args(&invocation.args)?;
            self.execute(invocation.args, ctx, config).await
        })
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(self.name(), self.description(), self.parameters())
            .read_only(self.is_read_only())
            .supports_parallel_calls(self.supports_parallel_calls())
    }

    /// Validate arguments before execution
    fn validate_args(&self, args: &Value) -> crate::Result<()> {
        if !args.is_object() {
            return Err(
                ToolError::InvalidArgs("tool arguments must be a JSON object".to_string()).into(),
            );
        }

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
    fn to_definition(&self) -> Value {
        self.definition().as_model_definition()
    }
}

fn format_tool_command(name: &str, args: &Value) -> String {
    if args.is_null() {
        name.to_string()
    } else {
        format!("{} {}", name, args)
    }
}

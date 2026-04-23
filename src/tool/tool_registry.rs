use crate::error::ToolError;
use crate::tool::traits::tool::{Tool, ToolContext, ToolResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool registry for managing available tools
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Box::new(tool));
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// Execute a tool by name
    pub async fn execute(
        &self,
        name: &str,
        args: serde_json::Value,
        ctx: &ToolContext,
        config: &crate::config::Config,
    ) -> crate::Result<ToolResult> {
        let tool = self
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;

        tool.validate_args(&args)?;
        tool.execute(args, ctx, config).await
    }

    /// List all registered tools
    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    /// Get tool definitions for model
    pub fn definitions(&self) -> Vec<serde_json::Value> {
        self.tools.values().map(|t| t.to_definition()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct TestTool;

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test_tool"
        }

        fn description(&self) -> &str {
            "A test tool"
        }

        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Test input"
                    }
                },
                "required": ["input"]
            })
        }

        async fn execute(
            &self,
            args: serde_json::Value,
            _ctx: &ToolContext,
            _config: &crate::config::Config,
        ) -> crate::Result<ToolResult> {
            let input = args["input"].as_str().unwrap_or("");
            Ok(ToolResult::success(format!("Processed: {}", input)))
        }
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool);

        assert!(registry.get("test_tool").is_some());
        assert!(registry.get("nonexistent").is_none());

        let ctx = ToolContext::default();
        let config = crate::config::Config::default();
        let result = registry
            .execute(
                "test_tool",
                serde_json::json!({"input": "hello"}),
                &ctx,
                &config,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("hello"));
    }

    #[tokio::test]
    async fn test_missing_required_field() {
        let tool = TestTool;
        let result = tool.validate_args(&serde_json::json!({}));
        assert!(result.is_err());
    }
}

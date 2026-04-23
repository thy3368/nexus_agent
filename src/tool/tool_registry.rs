use crate::error::ToolError;
use crate::tool::traits::tool::{Tool, ToolContext, ToolDefinition, ToolInvocation, ToolResult};
use std::collections::HashMap;
use std::sync::Arc;

/// Tool registry for managing available tools
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        let name = tool.name().to_string();
        if self.tools.insert(name.clone(), Arc::new(tool)).is_some() {
            tracing::warn!(tool = %name, "overwriting registered tool");
        }
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn handler(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).map(Arc::clone)
    }

    pub async fn is_mutating(&self, invocation: &ToolInvocation) -> crate::Result<bool> {
        let tool = self
            .handler(&invocation.name)
            .ok_or_else(|| ToolError::NotFound(invocation.name.clone()))?;
        Ok(tool.is_mutating(invocation).await)
    }

    /// Dispatch through the ToolHandler-style path so validation, hooks and execution share one invocation object.
    pub async fn dispatch(
        &self,
        invocation: ToolInvocation,
        ctx: &ToolContext,
        config: &crate::config::Config,
    ) -> crate::Result<ToolResult> {
        let tool = self
            .handler(&invocation.name)
            .ok_or_else(|| ToolError::NotFound(invocation.name.clone()))?;

        if let Some(payload) = tool.pre_tool_use_payload(&invocation) {
            tracing::debug!(tool = %payload.tool_name, command = %payload.command, "pre tool use");
        }

        let result = tool.handle(invocation.clone(), ctx, config).await?;

        if let Some(payload) = tool.post_tool_use_payload(&invocation, &result) {
            tracing::debug!(tool = %payload.tool_name, response = %payload.command, "post tool use");
        }

        Ok(result)
    }

    /// Execute a tool by name
    pub async fn execute(
        &self,
        name: &str,
        args: serde_json::Value,
        ctx: &ToolContext,
        config: &crate::config::Config,
    ) -> crate::Result<ToolResult> {
        self.dispatch(ToolInvocation::new(name, args), ctx, config)
            .await
    }

    /// List all registered tools
    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    pub fn definitions_with_metadata(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// Get tool definitions for model
    pub fn definitions(&self) -> Vec<serde_json::Value> {
        self.definitions_with_metadata()
            .into_iter()
            .map(|definition| definition.as_model_definition())
            .collect()
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

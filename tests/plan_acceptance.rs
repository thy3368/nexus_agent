use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use nexus_agent::agent::adapter::agent_react::AgentReAct;
use nexus_agent::agent::mode::AgentMode;
use nexus_agent::context::agent_context::AgentContext;
use nexus_agent::app::behavior::Agent;
use nexus_agent::config::Config;
use nexus_agent::llm::traits::ll_model::{
    LLMRequest, LLModel, LLMInfo, LLMReply, TokenUsage,
};
use nexus_agent::permissions::{PermissionLevel, PermissionManager};
use nexus_agent::tool::adapter::file_ops::FileListTool;
use nexus_agent::tool::adapter::plan::UpdatePlanTool;
use nexus_agent::tool::tool_registry::ToolRegistry;
use nexus_agent::tool::traits::tool_handler::{
    ToolContext, ToolDefinition, ToolHandler, ToolResult,
};

struct MockModel {
    responses: Vec<String>,
    call_count: Arc<Mutex<usize>>,
}

#[async_trait]
impl LLModel for MockModel {
    async fn complete(&self, _: &str, _: Option<&str>) -> nexus_agent::Result<LLMReply> {
        unimplemented!()
    }

    async fn do_chat(
        &self,
        _: &[LLMRequest],
        _: Option<&[ToolDefinition]>,
    ) -> nexus_agent::Result<LLMReply> {
        let mut count = self.call_count.lock().unwrap();
        let response = self.responses[*count].clone();
        *count += 1;

        Ok(LLMReply {
            content: response,
            model: "mock".to_string(),
            usage: TokenUsage::default(),
            tool_calls: None,
            finish_reason: Some("stop".to_string()),
        })
    }

    fn model_info(&self) -> LLMInfo {
        LLMInfo {
            provider: "mock".to_string(),
            model: "test".to_string(),
            max_tokens: 4096,
            supports_tools: false,
            supports_streaming: false,
        }
    }
}

struct FakeWorkTool;

#[async_trait]
impl ToolHandler for FakeWorkTool {
    fn name(&self) -> &str {
        "fake_work"
    }

    fn description(&self) -> &str {
        "Fake work tool"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "input": {"type": "string"}
            },
            "required": ["input"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        _ctx: &ToolContext,
        _config: &Config,
    ) -> nexus_agent::Result<ToolResult> {
        Ok(ToolResult::success(format!(
            "worked: {}",
            args["input"].as_str().unwrap_or_default()
        )))
    }
}

struct MutatingCounterTool {
    calls: Arc<Mutex<usize>>,
}

#[async_trait]
impl ToolHandler for MutatingCounterTool {
    fn name(&self) -> &str {
        "mutating_counter"
    }

    fn description(&self) -> &str {
        "Mutating counter tool"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }

    async fn execute(
        &self,
        _args: serde_json::Value,
        _ctx: &ToolContext,
        _config: &Config,
    ) -> nexus_agent::Result<ToolResult> {
        *self.calls.lock().unwrap() += 1;
        Ok(ToolResult::success("mutated"))
    }
}

fn permission_manager_for(tools: &[&str]) -> Arc<Mutex<PermissionManager>> {
    let mut manager = PermissionManager::default();
    for tool in tools {
        manager
            .set_permission((*tool).to_string(), PermissionLevel::Once)
            .unwrap();
    }
    Arc::new(Mutex::new(manager))
}

#[tokio::test]
async fn design_plan_acceptance_allows_read_only_exploration() {
    let model = Box::new(MockModel {
        responses: vec![
            r#"{"tool": "file_list", "args": {"dir_path": ".", "limit": 1}}"#.to_string(),
            "Here is the design.\n<proposed_plan>\n## Steps\n1. Add AgentMode\n2. Add update_plan\n3. Add tests\n</proposed_plan>".to_string(),
        ],
        call_count: Arc::new(Mutex::new(0)),
    });

    let mut tools = ToolRegistry::new();
    tools.register(FileListTool::new());

    let mut config = Config::default();
    config.safety.require_approval = false;

    let context = AgentContext::new(Vec::new(), None).await.unwrap();
    let mut agent = AgentReAct::new_with_mode(
        model,
        tools,
        config,
        context,
        permission_manager_for(&["file_list"]),
        AgentMode::Plan,
    )
    .await
    .unwrap();

    let result = agent
        .execute_task("Design plan support".to_string())
        .await
        .unwrap();

    assert!(result.success);
    assert_eq!(result.tool_calls, vec!["file_list"]);
    assert!(result.output.contains("Add AgentMode"));
    assert!(!result.output.contains("<proposed_plan>"));
}

#[tokio::test]
async fn design_plan_blocks_mutating_tools() {
    let calls = Arc::new(Mutex::new(0));
    let model = Box::new(MockModel {
        responses: vec![
            r#"{"tool": "mutating_counter", "args": {}}"#.to_string(),
            "<proposed_plan>\n## Steps\n1. Use read-only tools instead\n</proposed_plan>"
                .to_string(),
        ],
        call_count: Arc::new(Mutex::new(0)),
    });

    let mut tools = ToolRegistry::new();
    tools.register(MutatingCounterTool {
        calls: calls.clone(),
    });

    let mut config = Config::default();
    config.safety.require_approval = false;

    let context = AgentContext::new(Vec::new(), None).await.unwrap();
    let mut agent = AgentReAct::new_with_mode(
        model,
        tools,
        config,
        context,
        permission_manager_for(&["mutating_counter"]),
        AgentMode::Plan,
    )
    .await
    .unwrap();

    let result = agent
        .execute_task("Design safely".to_string())
        .await
        .unwrap();

    assert!(result.success);
    assert_eq!(*calls.lock().unwrap(), 0);
    assert_eq!(result.tool_calls, vec!["mutating_counter"]);
}

#[tokio::test]
async fn execute_plan_acceptance_uses_update_plan() {
    let model = Box::new(MockModel {
        responses: vec![
            r#"{"tool":"update_plan","args":{"explanation":"Starting","plan":[{"step":"Inspect","status":"in_progress"},{"step":"Validate","status":"pending"}]}}"#.to_string(),
            r#"{"tool":"fake_work","args":{"input":"step 1"}}"#.to_string(),
            r#"{"tool":"update_plan","args":{"plan":[{"step":"Inspect","status":"completed"},{"step":"Validate","status":"completed"}]}}"#.to_string(),
            "FINISH".to_string(),
        ],
        call_count: Arc::new(Mutex::new(0)),
    });

    let mut tools = ToolRegistry::new();
    tools.register(UpdatePlanTool::new());
    tools.register(FakeWorkTool);

    let mut config = Config::default();
    config.safety.require_approval = false;

    let context = AgentContext::new(Vec::new(), None).await.unwrap();
    let mut agent = AgentReAct::new_with_mode(
        model,
        tools,
        config,
        context,
        permission_manager_for(&["update_plan", "fake_work"]),
        AgentMode::Execute,
    )
    .await
    .unwrap();

    let result = agent
        .execute_task("Execute plan".to_string())
        .await
        .unwrap();

    assert!(result.success);
    assert_eq!(
        result.tool_calls,
        vec!["update_plan", "fake_work", "update_plan"]
    );
}

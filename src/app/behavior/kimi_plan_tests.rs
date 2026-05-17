//! Integration tests for Kimi provider in Plan Mode

use std::sync::{Arc, Mutex};

use crate::agent::adapter::agent_react::AgentReAct;
use crate::agent::mode::AgentMode;
use crate::agent::traits::Agent;
use crate::config::Config;
use crate::context::agent_context::AgentContext;
use crate::llm::adapter::kimi::KimiProvider;
use crate::llm::traits::ll_model::LLModel;
use crate::permissions::{PermissionLevel, PermissionManager};
use crate::tool::adapter::file_ops::{FileListTool, FileReadTool, FileWriteTool};
use crate::tool::adapter::git_ops::{GitCommitTool, GitDiffTool, GitStatusTool};
use crate::tool::adapter::plan::UpdatePlanTool;
use crate::tool::adapter::search_ops::CodebaseSearchTool;
use crate::tool::adapter::shell::ShellTool;
use crate::tool::adapter::web_ops::WebGetTool;
use crate::tool::tool_registry::ToolRegistry;

fn init_logging() {
    use tracing_subscriber::{fmt, EnvFilter};
    let _ = fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .try_init();
}

fn allow_tools(permission_manager: &Arc<Mutex<PermissionManager>>, tool_names: &[&str]) {
    let mut permissions = permission_manager.lock().unwrap();
    for tool_name in tool_names {
        permissions
            .set_permission((*tool_name).to_string(), PermissionLevel::Always)
            .unwrap();
    }
}

/// Test: Plan Mode with Kimi provider
///
/// Run with:
/// ```bash
/// KIMI_API_KEY="your-api-key" cargo test test_agent_kimi_plan_mode_proposes_plan_without_mutation -- --ignored --nocapture
/// ```
#[tokio::test]
#[ignore = "requires Kimi API credentials and live provider access"]
async fn test_agent_kimi_plan_mode_proposes_plan_without_mutation() {
    init_logging();

    let api_key = std::env::var("KIMI_API_KEY").expect("KIMI_API_KEY environment variable not set");

    let mut config = Config::load().unwrap_or_default();
    config.safety.require_approval = false;
    config.skills.project_skills = false;
    config.skills.user_skills = false;

    let kimi_provider = KimiProvider::new(
        api_key,
        Some("moonshot-v1-8k".to_string()),
        config.agent.llm_log_dir.clone(),
    );
    let model: Box<dyn LLModel> = Box::new(kimi_provider);

    let mut tools = ToolRegistry::new();
    tools.register(FileListTool::new());
    tools.register(FileReadTool::new());
    tools.register(FileWriteTool::new());
    tools.register(ShellTool::new());
    tools.register(GitStatusTool::new());
    tools.register(GitDiffTool::new());
    tools.register(GitCommitTool::new());
    tools.register(CodebaseSearchTool::new());
    tools.register(WebGetTool::new());
    tools.register(UpdatePlanTool::new());

    let permission_manager = Arc::new(Mutex::new(PermissionManager::new().unwrap()));
    allow_tools(
        &permission_manager,
        &[
            "file_list",
            "file_read",
            "git_status",
            "git_diff",
            "codebase_search",
            "web_get",
        ],
    );

    let context = AgentContext::new(Vec::new(), None)
        .await
        .expect("Failed to create agent context");
    let mut agent = AgentReAct::new_with_mode(
        model,
        tools,
        config,
        context,
        permission_manager,
        AgentMode::Plan,
    )
    .await
    .expect("Failed to create Plan Mode Kimi agent");

    let task =
        "请制定一个实现方案：在项目中新增一个简单的 CLI 参数校验功能。不要修改文件，只输出计划。";
    let result = agent
        .execute_task(task.to_string())
        .await
        .expect("Kimi Plan Mode call should succeed");

    println!("\nPlan Mode result success: {}", result.success);
    println!("Tools used: {:?}", result.tool_calls);
    println!("Output:\n{}", result.output);

    assert!(result.success);
    assert!(!result.output.trim().is_empty());
    assert!(!result.output.contains("<proposed_plan>"));
    assert!(!result.output.contains("</proposed_plan>"));
    assert!(
        result.output.contains("参数")
            || result.output.contains("校验")
            || result.output.to_lowercase().contains("cli")
    );
    assert!(!result.tool_calls.iter().any(|tool| tool == "update_plan"));
    assert!(!result
        .tool_calls
        .iter()
        .any(|tool| matches!(tool.as_str(), "file_write" | "shell_execute" | "git_commit")));
}

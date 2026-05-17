//! Integration tests for Claude provider

use std::sync::{Arc, Mutex};

use crate::agent::adapter::agent_react::AgentReAct;
use crate::agent::traits::Agent;
use crate::config::Config;
use crate::context::agent_context::AgentContext;
use crate::llm::adapter::claude::ClaudeProvider;
use crate::llm::traits::ll_model::LLModel;

use crate::permissions::PermissionManager;
use crate::skill::SkillManager;
use crate::tool::adapter::file_ops::{FileListTool, FileReadTool, FileWriteTool};
use crate::tool::adapter::git_ops::{GitCommitTool, GitDiffTool, GitStatusTool};
use crate::tool::adapter::plan::UpdatePlanTool;
use crate::tool::adapter::search_ops::CodebaseSearchTool;
use crate::tool::adapter::shell::ShellTool;
use crate::tool::adapter::web_ops::WebGetTool;
use crate::tool::tool_registry::ToolRegistry;

fn init_logging() {
    use tracing_subscriber::{fmt, EnvFilter};
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .init();
}

fn skill_fixture_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("skills")
}

fn task_fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("tasks")
        .join("claude_todo_task.md")
}

/// Test: Single task with Claude provider
///
/// Run with:
/// ```bash
/// ANTHROPIC_API_KEY="your-api-key" cargo test test_agent_with_claude_example -- --ignored --nocapture
/// ```
#[tokio::test]
async fn test_agent_with_claude_example() {
    init_logging();

    let api_key =
        std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY environment variable not set");
    let base_url =
        std::env::var("ANTHROPIC_BASE_URL").expect("ANTHROPIC_BASE_URL environment variable not set");

    let mut config = Config::load().unwrap_or_default();
    config.safety.require_approval = false;
    config.skills.project_skills = false;
    config.skills.user_skills = false;
    config.skills.roots = vec![skill_fixture_root()];

    let claude_provider = ClaudeProvider::new(
        api_key,
        base_url,
        Some("claude-sonnet-4-6".to_string()),
        config.agent.llm_log_dir.clone(),
    );
    let model: Box<dyn LLModel> = Box::new(claude_provider);

    let mut tools = ToolRegistry::new();
    tools.register(FileListTool::new());
    tools.register(FileReadTool::new());
    tools.register(FileWriteTool::new());
    tools.register(ShellTool::new());
    tools.register(GitStatusTool::new());
    tools.register(GitDiffTool::new());
    tools.register(GitCommitTool::new());
    tools.register(CodebaseSearchTool::new());
    tools.register(UpdatePlanTool::new());
    tools.register(WebGetTool::new());

    let permission_manager = Arc::new(Mutex::new(PermissionManager::new().unwrap()));
    let mut pm = permission_manager.lock().unwrap();
    pm.set_permission(
        "file_list".to_string(),
        crate::permissions::PermissionLevel::Always,
    )
    .unwrap();
    pm.set_permission(
        "file_read".to_string(),
        crate::permissions::PermissionLevel::Always,
    )
    .unwrap();
    pm.set_permission(
        "file_write".to_string(),
        crate::permissions::PermissionLevel::Always,
    )
    .unwrap();
    pm.set_permission(
        "shell_execute".to_string(),
        crate::permissions::PermissionLevel::Always,
    )
    .unwrap();
    pm.set_permission(
        "git_status".to_string(),
        crate::permissions::PermissionLevel::Always,
    )
    .unwrap();
    pm.set_permission(
        "git_diff".to_string(),
        crate::permissions::PermissionLevel::Always,
    )
    .unwrap();
    pm.set_permission(
        "git_commit".to_string(),
        crate::permissions::PermissionLevel::Always,
    )
    .unwrap();
    pm.set_permission(
        "codebase_search".to_string(),
        crate::permissions::PermissionLevel::Always,
    )
    .unwrap();
    pm.set_permission(
        "update_plan".to_string(),
        crate::permissions::PermissionLevel::Always,
    )
    .unwrap();
    pm.set_permission(
        "web_get".to_string(),
        crate::permissions::PermissionLevel::Always,
    )
    .unwrap();
    drop(pm);

    let skill_manager = Arc::new(SkillManager::new());
    skill_manager
        .load_for_config(&config, std::path::Path::new("."))
        .expect("Failed to load skills");

    let context = AgentContext::new(Vec::new(), Some(skill_manager))
        .await
        .expect("Failed to create agent context");
    let mut agent = AgentReAct::new(model, tools, config, context, permission_manager)
        .await
        .expect("Failed to create agent");

    let task = std::fs::read_to_string(task_fixture_path())
        .expect("Failed to read task fixture");

    match agent.execute_task(task).await {
        Ok(result) => {
            println!("\n✅ Task completed successfully!");
            println!("  Success: {}", result.success);
            println!("  Iterations: {}", result.iterations);
            println!("  Tools used: {:?}", result.tool_calls);
            println!("  Output:\n{}", result.output);
        }
        Err(e) => {
            eprintln!("\n❌ Task failed: {}", e);
        }
    }

    let system_prompt = agent
        .get_conversation_history()
        .iter()
        .find(|message| message.role == "system")
        .map(|message| message.content.as_str())
        .expect("system prompt should be present");
    assert!(system_prompt.contains("<skills_instructions>"));
}

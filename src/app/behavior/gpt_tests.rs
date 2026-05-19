//! Integration tests for GPT provider

use std::sync::{Arc, Mutex};

use crate::agent::adapter::agent_react::AgentReAct;
use crate::agent::traits::Agent;
use crate::config::Config;
use crate::context::agent_context::AgentContext;
use crate::llm::adapter::openai::OpenAIProvider;
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

/// Test: Single task with OpenAI provider
///
/// Run with:
/// ```bash
/// OPENAI_API_KEY="your-api-key" cargo test test_agent_with_gpt_example -- --ignored --nocapture
/// ```
#[tokio::test]
async fn test_agent_with_gpt_example() {
    init_logging();

    let api_key = match std::env::var("GPT_API_KEY") {
        Ok(v) => v,
        Err(_) => { println!("GPT_API_KEY not set, skipping"); return; }
    };
    let base_url = match std::env::var("GPT_BASE_URL") {
        Ok(v) => v,
        Err(_) => { println!("GPT_BASE_URL not set, skipping"); return; }
    };

    let mut config = Config::load().unwrap_or_default();
    config.safety.require_approval = false;
    config.skills.project_skills = false;
    config.skills.user_skills = false;
    config.skills.roots = vec![skill_fixture_root()];


    let openai_provider = OpenAIProvider::new(
        api_key.to_string(),
        Some(base_url),
        Some("gpt-5.4".to_string()),
        config.agent.llm_log_dir.clone(),
    );
    let model: Box<dyn LLModel> = Box::new(openai_provider);

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

    let task = std::fs::read_to_string(task_fixture_path()).expect("Failed to read task fixture");

    let system_prompt = agent
        .get_conversation_history()
        .iter()
        .find(|message| message.role == "system")
        .map(|message| message.content.as_str())
        .expect("system prompt should be present");
    assert!(system_prompt.contains("<skills_instructions>"));
}

/// Test: Minimal official OpenAI connectivity
///
/// Run with:
/// ```bash
/// OPENAI_API_KEY="your-api-key" cargo test test_openai_official_minimal -- --ignored --nocapture
/// ```
#[tokio::test]
#[ignore]
async fn test_openai_official_minimal() {
    let api_key = match std::env::var("GPT_API_KEY") {
        Ok(v) => v,
        Err(_) => {
            println!("OPENAI_API_KEY not set, skipping");
            return;
        }
    };

    let provider = OpenAIProvider::new(api_key, None, Some("gpt-5.4".to_string()), None);

    let reply = provider
        .complete("say hi", None)
        .await
        .expect("official OpenAI request should succeed");

    println!("reply model: {}", reply.model);
    println!("reply content: {}", reply.content);
    println!("usage: {:?}", reply.usage);

    assert!(
        !reply.content.trim().is_empty(),
        "reply content should not be empty"
    );
}

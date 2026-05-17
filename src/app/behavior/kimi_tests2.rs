//! Integration tests for Kimi provider

use std::sync::{Arc, Mutex};

use crate::agent::adapter::agent_react::AgentReAct;
use crate::agent::traits::Agent;
use crate::config::Config;
use crate::context::agent_context::AgentContext;
use crate::llm::adapter::kimi::KimiProvider;
use crate::llm::traits::ll_model::LLModel;

use crate::permissions::PermissionManager;
use crate::tool::adapter::file_ops::{FileListTool, FileReadTool, FileWriteTool};
use crate::tool::adapter::git_ops::{GitCommitTool, GitDiffTool, GitStatusTool};
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

/// Test: Multi-turn conversation with Kimi provider
///
/// Run with:
/// ```bash
/// KIMI_API_KEY="your-api-key" cargo test test_agent_kimi_multi_turn -- --ignored --nocapture
/// ```
#[tokio::test]
async fn test_agent_kimi_multi_turn() {
    init_logging();

    let api_key = std::env::var("KIMI_API_KEY").expect("KIMI_API_KEY environment variable not set");

    let mut config = Config::load().unwrap_or_default();
    config.safety.require_approval = false;

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

    let mut config = Config::default();
    config.safety.require_approval = false;

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
        "web_get".to_string(),
        crate::permissions::PermissionLevel::Always,
    )
    .unwrap();
    drop(pm);

    let context = AgentContext::new(Vec::new())
        .await
        .expect("Failed to create agent context");
    let mut agent = AgentReAct::new(model, tools, config, context, permission_manager)
        .await
        .expect("Failed to create agent");

    let tasks = vec!["你好，请介绍一下你自己", "列出当前目录的文件"];

    for (idx, task) in tasks.iter().enumerate() {
        println!("\n🔄 Task {}: {}", idx + 1, task);

        match agent.execute_task(task.to_string()).await {
            Ok(result) => {
                let preview = result.output.chars().take(100).collect::<String>();
                println!("✅ Result: {}", preview);
            }
            Err(e) => {
                eprintln!("❌ Error: {}", e);
            }
        }
    }

    let history = agent.get_conversation_history();
    println!("\n📊 Final conversation has {} messages", history.len());
}

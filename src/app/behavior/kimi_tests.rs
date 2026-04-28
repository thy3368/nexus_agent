//! Integration tests for Kimi provider

use std::sync::{Arc, Mutex};

use crate::agent::adapter::agent_react::AgentReAct;
use crate::agent::traits::Agent;
use crate::config::Config;
use crate::llm::adapter::kimi::KimiProvider;
use crate::llm::traits::language_model::LanguageModel;

use crate::permissions::PermissionManager;
use crate::skill::SkillManager;
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

/// Test: Single task with Kimi provider
///
/// Run with:
/// ```bash
/// KIMI_API_KEY="your-api-key" cargo test test_agent_with_kimi_example -- --ignored --nocapture
/// ```
#[tokio::test]
#[ignore]
async fn test_agent_with_kimi_example() {
    init_logging();

    let api_key = std::env::var("KIMI_API_KEY").expect("KIMI_API_KEY environment variable not set");

    let kimi_provider = KimiProvider::new(api_key, Some("moonshot-v1-8k".to_string()));
    let model: Box<dyn LanguageModel> = Box::new(kimi_provider);

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
    config.skills.project_skills = false;
    config.skills.user_skills = false;
    config.skills.roots = vec![skill_fixture_root()];

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

    let skill_manager = Arc::new(SkillManager::new());
    skill_manager
        .load_for_config(&config, std::path::Path::new("."))
        .expect("Failed to load skills");

    let mut agent = AgentReAct::new_with_skills(
        model,
        tools,
        config,
        Vec::new(),
        permission_manager,
        Some(skill_manager),
    )
    .await
    .expect("Failed to create agent");

    let task = "Use $simple-checklist to answer: 写个 Rust hello world 示例。不要创建文件或执行命令，只返回文本。";

    match agent.execute_task(task.to_string()).await {
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
    assert!(system_prompt.contains("<skill name=\"simple-checklist\""));
    assert!(system_prompt.contains("When this skill is active"));
}

/// Test: Multi-turn conversation with Kimi provider
///
/// Run with:
/// ```bash
/// KIMI_API_KEY="your-api-key" cargo test test_agent_kimi_multi_turn -- --ignored --nocapture
/// ```
#[tokio::test]
#[ignore]
async fn test_agent_kimi_multi_turn() {
    init_logging();

    let api_key = std::env::var("KIMI_API_KEY").expect("KIMI_API_KEY environment variable not set");

    let kimi_provider = KimiProvider::new(api_key, Some("moonshot-v1-8k".to_string()));
    let model: Box<dyn LanguageModel> = Box::new(kimi_provider);

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

    let mut agent = AgentReAct::new(model, tools, config, Vec::new(), permission_manager)
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

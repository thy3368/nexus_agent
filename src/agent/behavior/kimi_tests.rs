//! Integration tests for Kimi provider

use std::sync::{Arc, Mutex};

use crate::agent::adapter::agent_react::AgentBehaviorReAct;
use crate::agent::traits::Agent;
use crate::config::Config;
use crate::llm::adapter::kimi::KimiProvider;
use crate::llm::traits::language_model::LanguageModel;

use crate::permissions::PermissionManager;
use crate::tool::tool_registry::ToolRegistry;

fn init_logging() {
    use tracing_subscriber::{fmt, EnvFilter};
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .init();
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
    tools.register(crate::tool::adapter::file_ops::FileListTool::new());

    let mut config = Config::default();
    config.safety.require_approval = false;

    let permission_manager = Arc::new(Mutex::new(PermissionManager::new().unwrap()));
    permission_manager
        .lock()
        .unwrap()
        .set_permission(
            "file_list".to_string(),
            crate::permissions::PermissionLevel::Always,
        )
        .unwrap();

    let mut agent = AgentBehaviorReAct::new(model, tools, config, Vec::new(), permission_manager)
        .await
        .expect("Failed to create agent");

    let task = "列出当前目录中的所有文件";
    println!("\n📋 Task: {}", task);

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
    tools.register(crate::tool::adapter::file_ops::FileListTool::new());

    let mut config = Config::default();
    config.safety.require_approval = false;

    let permission_manager = Arc::new(Mutex::new(PermissionManager::new().unwrap()));
    permission_manager
        .lock()
        .unwrap()
        .set_permission(
            "file_list".to_string(),
            crate::permissions::PermissionLevel::Always,
        )
        .unwrap();

    let mut agent = AgentBehaviorReAct::new(model, tools, config, Vec::new(), permission_manager)
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

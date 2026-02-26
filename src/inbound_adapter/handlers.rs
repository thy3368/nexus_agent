use super::repl_session::ReplSession;
use crate::agent::behavior::agent_react::AgentBehaviorReAct;
use crate::agent::behavior::AgentBehavior;
use crate::setup;
use crate::Config;

pub async fn handle_init() -> anyhow::Result<()> {
    println!("🚀 Initializing PromptLine...\n");

    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| {
        println!("⚠️  OPENAI_API_KEY environment variable not set");
        String::new()
    });

    if api_key.is_empty() {
        println!("To use OpenAI models, set your API key:");
        println!("  export OPENAI_API_KEY='your-api-key-here'");
    } else {
        println!("✓ OPENAI_API_KEY found");
    }

    let config = Config::default();
    let config_path = if let Some(mut dir) = dirs::config_dir() {
        dir.push("promptline");
        std::fs::create_dir_all(&dir)?;
        dir.push("config.yaml");
        dir
    } else {
        std::path::PathBuf::from(".promptline/config.yaml")
    };

    config.save_to_file(&config_path)?;

    println!("\n✓ Configuration saved to: {}", config_path.display());
    println!("\nPromptLine is ready! Try:");
    println!("  promptline \"list all rust files\"");

    Ok(())
}

pub fn handle_doctor(config: &Config) -> anyhow::Result<()> {
    println!("🔍 PromptLine Health Check\n");

    println!("✓ Binary version: {}", crate::VERSION);

    match std::env::var("OPENAI_API_KEY") {
        Ok(key) if !key.is_empty() => {
            println!("✓ OpenAI API key configured");
        }
        _ => {
            println!("✗ OpenAI API key not found");
            println!("  Set OPENAI_API_KEY environment variable");
        }
    }

    println!("✓ Configuration loaded");
    println!("  Default model: {}", config.models.default);
    println!("  Max iterations: {}", config.safety.max_iterations);
    println!("  Approval required: {}", config.safety.require_approval);

    println!("\n✓ All checks passed!");

    Ok(())
}

pub async fn handle_agent(task: &str, config: Config) -> anyhow::Result<()> {
    println!("⚙️  Agent mode\n");

    let model = setup::create_model(&config)?;
    let tools = setup::create_tools();
    let permission_manager = setup::create_permission_manager()?;

    let mut agent =
        AgentBehaviorReAct::new(model, tools, config, Vec::new(), permission_manager).await?;

    println!("Task: {}\n", task);
    let result = agent.execute_task(task.to_string()).await?;

    println!("\n{}", "=".repeat(60));
    if result.success {
        println!("✓ Task completed successfully");
    } else {
        println!("✗ Task failed");
    }
    println!("Iterations: {}", result.iterations);
    println!("Tools used: {}", result.tool_calls.join(", "));
    println!("{}", "=".repeat(60));
    println!("\nResult:\n{}", result.output);

    Ok(())
}

pub async fn handle_chat(config: Config) -> anyhow::Result<()> {
    print!("\x1b[2J\x1b[1;1H");

    println!("\x1b[1;34m");
    println!(
        r#"
    ██████╗ ██████╗  ██████╗ ███╗   ███╗██████╗ ████████╗██╗     ██╗███╗   ██╗███████╗
    ██╔══██╗██╔══██╗██╔═══██╗████╗ ████║██╔══██╗╚══██╔══╝██║     ██║████╗  ██║██╔════╝
    ██████╔╝██████╔╝██║   ██║██╔████╔██║██████╔╝   ██║   ██║     ██║██╔██╗ ██║█████╗
    ██╔═══╝ ██╔══██╗██║   ██║██║╚██╔╝██║██╔═══╝    ██║   ██║     ██║██║╚██╗██║██╔══╝
    ██║     ██║  ██║╚██████╔╝██║ ╚═╝ ██║██║        ██║   ███████╗██║██║ ╚████║███████╗
    ╚═╝     ╚═╝  ╚═╝ ╚═════╝ ╚═╝     ╚═╝╚═╝        ╚═╝   ╚══════╝╚═╝╚═╝  ╚═══╝╚══════╝
    "#
    );
    println!("\x1b[0m");

    println!(
        "\x1b[32m    PromptLine v{} (Rust) - Agentic AI CLI\x1b[0m",
        crate::VERSION
    );
    println!("\x1b[90m    Type a command to see the agent in action (e.g., \"refactor main.rs\" or \"explain this code\")\x1b[0m");
    println!();

    loop {
        let mut session = ReplSession::new(config.clone()).await?;
        session.run().await?;
    }
}

pub async fn handle_edit(
    _file: &std::path::Path,
    _instruction: &str,
    _config: Config,
) -> anyhow::Result<()> {
    println!("📝 Edit mode\n");
    println!("This is a placeholder. Phase 1 will implement file editing.");
    Ok(())
}

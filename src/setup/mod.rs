use crate::permissions::PermissionManager;
use crate::skill::SkillManager;

use crate::config::Config;
use crate::llm::adapter::claude::ClaudeProvider;
use crate::llm::adapter::kimi::KimiProvider;
use crate::llm::adapter::minimax::MinimaxProvider;
use crate::llm::adapter::ollama::OllamaProvider;
use crate::llm::adapter::openai::OpenAIProvider;
use crate::llm::traits::ll_model::LLModel;
use crate::tool::adapter::{
    apply_patch, file_ops, git_ops, image_ops, mcp, plan, search_ops, shell, tool_search,
    unified_exec, web_ops,
};
use crate::tool::tool_registry::ToolRegistry;
use std::sync::{Arc, Mutex};

const OPENCODE_CONFIG_PATH: &str = "/Users/hongyaotang/.config/opencode/opencode.json";

pub fn create_model(config: &Config) -> anyhow::Result<Box<dyn LLModel>> {
    let provider = std::env::var("PROMPTLINE_PROVIDER").unwrap_or_else(|_| "openai".to_string());

    match provider.as_str() {
        "ollama" => {
            let api_key = std::env::var("OLLAMA_API_KEY").ok().or_else(|| {
                config
                    .models
                    .providers
                    .get("ollama")
                    .and_then(|p| p.api_key.clone())
            });

            let base_url = config
                .models
                .providers
                .get("ollama")
                .and_then(|p| p.base_url.clone());

            Ok(Box::new(OllamaProvider::new(
                base_url,
                api_key,
                Some(config.models.default.clone()),
                config.agent.llm_log_dir.clone(),
            )))
        }
        "kimi" => {
            let api_key = std::env::var("KIMI_API_KEY").ok().or_else(|| {
                config
                    .models
                    .providers
                    .get("kimi")
                    .and_then(|p| p.api_key.clone())
            });

            let api_key = api_key.ok_or_else(|| {
                anyhow::anyhow!("KIMI_API_KEY not set. You can set it via:\n1. Environment variable: KIMI_API_KEY\n2. Config file: ~/.promptline/config.yaml (under models.providers.kimi.api_key)")
            })?;

            Ok(Box::new(KimiProvider::new(
                api_key,
                Some(config.models.default.clone()),
                config.agent.llm_log_dir.clone(),
            )))
        }
        "claude" => {
            let api_key = std::env::var("ANTHROPIC_API_KEY").ok().or_else(|| {
                config
                    .models
                    .providers
                    .get("claude")
                    .and_then(|p| p.api_key.clone())
            });

            let api_key = api_key.ok_or_else(|| {
                anyhow::anyhow!("ANTHROPIC_API_KEY not set. You can set it via:\n1. Environment variable: ANTHROPIC_API_KEY\n2. Config file: ~/.promptline/config.yaml (under models.providers.claude.api_key)")
            })?;

            Ok(Box::new(ClaudeProvider::new(
                api_key,
                Some(config.models.default.clone()),
                config.agent.llm_log_dir.clone(),
            )))
        }
        "minimax" => {
            let api_key = std::env::var("MINIMAX_API_KEY")
                .ok()
                .or_else(|| read_opencode_provider_api_key("minimax"));

            let api_key = api_key.ok_or_else(|| {
                anyhow::anyhow!("MINIMAX_API_KEY not set. You can set it via:\n1. Environment variable: MINIMAX_API_KEY\n2. OpenCode config: ~/.config/opencode/opencode.json (provider.minimax.options.apiKey)")
            })?;

            Ok(Box::new(MinimaxProvider::new(
                api_key,
                Some(config.models.default.clone()),
                config.agent.llm_log_dir.clone(),
            )))
        }
        "openai" | _ => {
            let api_key = std::env::var("OPENAI_API_KEY").ok().or_else(|| {
                config
                    .models
                    .providers
                    .get("openai")
                    .and_then(|p| p.api_key.clone())
            });

            let api_key = api_key.ok_or_else(|| {
                anyhow::anyhow!("OPENAI_API_KEY not set. You can set it via:\n1. Environment variable: OPENAI_API_KEY\n2. Config file: ~/.promptline/config.yaml (under models.providers.openai.api_key)")
            })?;

            Ok(Box::new(OpenAIProvider::new(
                api_key,
                Some(config.models.default.clone()),
                config.agent.llm_log_dir.clone(),
            )))
        }
    }
}

fn read_opencode_provider_api_key(provider: &str) -> Option<String> {
    let content = std::fs::read_to_string(OPENCODE_CONFIG_PATH).ok()?;
    let config: serde_json::Value = serde_json::from_str(&content).ok()?;
    config
        .get("provider")?
        .get(provider)?
        .get("options")?
        .get("apiKey")?
        .as_str()
        .map(str::to_string)
}

pub fn create_tools() -> ToolRegistry {
    let mut tools = ToolRegistry::new();
    tools.register(file_ops::FileReadTool::new());
    tools.register(file_ops::FileWriteTool::new());
    tools.register(file_ops::FileListTool::new());
    tools.register(apply_patch::ApplyPatchTool::new());
    tools.register(shell::ShellTool::new());
    tools.register(git_ops::GitStatusTool::new());
    tools.register(git_ops::GitDiffTool::new());
    tools.register(git_ops::GitCommitTool::new());
    tools.register(web_ops::WebGetTool::new());
    tools.register(search_ops::CodebaseSearchTool::new());
    tools.register(image_ops::ViewImageTool::new());
    tools.register(mcp::McpTool::new());
    tools.register(mcp::McpResourceTool::new());
    tools.register(unified_exec::ExecCommandTool::new());
    tools.register(unified_exec::WriteStdinTool::new());
    tools.register(plan::UpdatePlanTool::new());
    tools.register(tool_search::ToolSearchTool::new(
        tools
            .definitions_with_metadata()
            .into_iter()
            .map(|definition| tool_search::ToolSearchEntry {
                name: definition.name,
                description: definition.description,
                parameters: definition.parameters,
                read_only: definition.read_only,
                supports_parallel_calls: definition.supports_parallel_calls,
            })
            .collect(),
    ));
    tools
}

pub fn create_permission_manager() -> anyhow::Result<Arc<Mutex<PermissionManager>>> {
    Ok(Arc::new(Mutex::new(PermissionManager::new()?)))
}

pub fn create_skill_manager(config: &Config) -> anyhow::Result<Arc<SkillManager>> {
    let manager = Arc::new(SkillManager::new());
    let cwd = std::env::current_dir()?;
    manager.load_for_config(config, &cwd)?;
    Ok(manager)
}

use crate::permissions::PermissionManager;

use crate::config::Config;
use crate::llm::adapter::ollama::OllamaProvider;
use crate::llm::adapter::openai::OpenAIProvider;
use crate::llm::traits::language_model::LanguageModel;
use crate::tools::adapter::{file_ops, git_ops, search_ops, shell, web_ops};
use crate::tools::tool_registry::ToolRegistry;
use std::sync::{Arc, Mutex};

pub fn create_model(config: &Config) -> anyhow::Result<Box<dyn LanguageModel>> {
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
            )))
        }
    }
}

pub fn create_tools() -> ToolRegistry {
    let mut tools = ToolRegistry::new();
    tools.register(file_ops::FileReadTool::new());
    tools.register(file_ops::FileWriteTool::new());
    tools.register(file_ops::FileListTool::new());
    tools.register(shell::ShellTool::new());
    tools.register(git_ops::GitStatusTool::new());
    tools.register(git_ops::GitDiffTool::new());
    tools.register(git_ops::GitCommitTool::new());
    tools.register(web_ops::WebGetTool::new());
    tools.register(search_ops::CodebaseSearchTool::new());
    tools
}

pub fn create_permission_manager() -> anyhow::Result<Arc<Mutex<PermissionManager>>> {
    Ok(Arc::new(Mutex::new(PermissionManager::new()?)))
}

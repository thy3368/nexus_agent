//! Context provider for git and project information

use crate::error::Result;

pub struct ContextProvider;

impl ContextProvider {
    /// Get current git branch
    pub async fn get_git_branch() -> Result<Option<String>> {
        match tokio::process::Command::new("git")
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("HEAD")
            .output()
            .await
        {
            Ok(output) if output.status.success() => Ok(Some(
                String::from_utf8_lossy(&output.stdout).trim().to_string(),
            )),
            _ => Ok(None),
        }
    }

    /// Get git branch synchronously
    pub fn get_git_branch_sync() -> Option<String> {
        match std::process::Command::new("git")
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("HEAD")
            .output()
        {
            Ok(output) if output.status.success() => {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
            _ => None,
        }
    }

    /// Get project context
    pub async fn get_project_context() -> Result<Option<String>> {
        match crate::context::ContextManager::new().await {
            Ok(context_manager) => match context_manager.load_project_context().await {
                Ok(context) => Ok(context),
                Err(e) => {
                    tracing::warn!("Failed to load project context: {}", e);
                    Ok(None)
                }
            },
            Err(e) => {
                tracing::warn!("Failed to load project context: {}", e);
                Ok(None)
            }
        }
    }

    /// Get project type
    pub async fn get_project_type() -> Result<String> {
        match crate::context::ContextManager::new().await {
            Ok(context_manager) => match context_manager.detect_project_type().await {
                Ok(project_type) => Ok(project_type),
                Err(e) => {
                    tracing::warn!("Failed to detect project type: {}", e);
                    Ok("Generic".to_string())
                }
            },
            Err(e) => {
                tracing::warn!("Failed to create context manager: {}", e);
                Ok("Generic".to_string())
            }
        }
    }
}

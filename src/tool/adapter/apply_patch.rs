//! Apply patch tool

use crate::error::{Result, ToolError};
use crate::tool::traits::tool_handler::{ToolHandler, ToolContext, ToolResult};
use async_trait::async_trait;
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use tokio::process::Command;

/// Patch-based file editing tool.
/// Usage: apply precise multi-line edits, additions, or deletions when a full file overwrite is too broad.
/// 使用场景：需要对文件做精确的多行修改、插入或删除时使用，适合替代整文件覆盖写入。
pub struct ApplyPatchTool;

impl ApplyPatchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ApplyPatchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for ApplyPatchTool {
    fn name(&self) -> &str {
        "apply_patch"
    }

    fn description(&self) -> &str {
        "Apply a unified patch to files in the current workspace. Prefer this over full file overwrites for precise edits."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "patch": {
                    "type": "string",
                    "description": "Unified patch content to apply"
                }
            },
            "required": ["patch"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
        _config: &crate::config::Config,
    ) -> Result<ToolResult> {
        let patch = args["patch"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("Missing patch".to_string()))?;
        let summary = PatchSummary::from_patch(patch)?;

        tracing::info!(
            changed_files = summary.changed_files.len(),
            "applying patch"
        );

        let strip_level = if patch.contains("\n--- a/") || patch.starts_with("--- a/") {
            "-p1"
        } else {
            "-p0"
        };

        let mut child = Command::new("patch")
            .arg(strip_level)
            .arg("--forward")
            .current_dir(&ctx.working_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to start patch: {}", e)))?;

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin
                .write_all(patch.as_bytes())
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write patch: {}", e)))?;
        }

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to apply patch: {}", e)))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let result = if output.status.success() {
            ToolResult::success(if stdout.trim().is_empty() {
                "Patch applied successfully".to_string()
            } else {
                stdout.clone()
            })
        } else {
            ToolResult::error(format!("Patch failed: {}", stderr))
        };

        Ok(result
            .with_metadata("changed_files", serde_json::json!(summary.changed_files))
            .with_metadata("added", serde_json::json!(summary.added))
            .with_metadata("modified", serde_json::json!(summary.modified))
            .with_metadata("deleted", serde_json::json!(summary.deleted))
            .with_metadata("stdout", serde_json::json!(stdout))
            .with_metadata("stderr", serde_json::json!(stderr)))
    }
}

#[derive(Debug, Default)]
struct PatchSummary {
    changed_files: Vec<PathBuf>,
    added: usize,
    modified: usize,
    deleted: usize,
}

impl PatchSummary {
    fn from_patch(patch: &str) -> Result<Self> {
        let mut changed_files = BTreeSet::new();
        let mut plus_paths = BTreeSet::new();
        let mut minus_paths = BTreeSet::new();

        for line in patch.lines() {
            if let Some(path) = line.strip_prefix("+++ ") {
                let path = normalize_patch_path(path)?;
                if path != PathBuf::from("/dev/null") {
                    plus_paths.insert(path.clone());
                    changed_files.insert(path);
                }
            } else if let Some(path) = line.strip_prefix("--- ") {
                let path = normalize_patch_path(path)?;
                if path != PathBuf::from("/dev/null") {
                    minus_paths.insert(path.clone());
                    changed_files.insert(path);
                }
            }
        }

        let mut added = 0;
        let mut modified = 0;
        let mut deleted = 0;
        for path in &changed_files {
            let has_plus = plus_paths.contains(path);
            let has_minus = minus_paths.contains(path);
            match (has_minus, has_plus) {
                (false, true) => added += 1,
                (true, false) => deleted += 1,
                (true, true) => modified += 1,
                (false, false) => {}
            }
        }

        Ok(Self {
            changed_files: changed_files.into_iter().collect(),
            added,
            modified,
            deleted,
        })
    }
}

fn normalize_patch_path(path: &str) -> Result<PathBuf> {
    let path = path.split_whitespace().next().unwrap_or(path);
    if path == "/dev/null" {
        return Ok(PathBuf::from(path));
    }

    let normalized = path
        .strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path);
    let path_buf = PathBuf::from(normalized);
    validate_workspace_relative_path(&path_buf)?;
    Ok(path_buf)
}

fn validate_workspace_relative_path(path: &Path) -> Result<()> {
    if path.is_absolute() {
        return Err(ToolError::InvalidArgs(format!(
            "Absolute patch path is not allowed: {}",
            path.display()
        ))
        .into());
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(ToolError::InvalidArgs(format!(
            "Patch path escapes workspace: {}",
            path.display()
        ))
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_patch_summary_tracks_modified_file() {
        let summary =
            PatchSummary::from_patch("--- a/file.txt\n+++ b/file.txt\n@@ -1 +1 @@\n-old\n+new\n")
                .unwrap();

        assert_eq!(summary.changed_files, vec![PathBuf::from("file.txt")]);
        assert_eq!(summary.modified, 1);
        assert_eq!(summary.added, 0);
        assert_eq!(summary.deleted, 0);
    }

    #[test]
    fn test_patch_summary_rejects_escape_path() {
        let result = PatchSummary::from_patch("--- a/../../etc/passwd\n+++ b/../../etc/passwd\n");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_apply_patch() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("file.txt"), "old\n").unwrap();

        let tool = ApplyPatchTool::new();
        let ctx = ToolContext {
            working_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        let config = crate::config::Config::default();

        let patch = "--- file.txt\n+++ file.txt\n@@ -1 +1 @@\n-old\n+new\n";
        let result = tool
            .execute(serde_json::json!({"patch": patch}), &ctx, &config)
            .await
            .unwrap();

        assert!(result.success, "{:?}", result.error);
        assert_eq!(
            std::fs::read_to_string(temp_dir.path().join("file.txt")).unwrap(),
            "new\n"
        );
    }
}

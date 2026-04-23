//! Image viewing tool

use crate::error::{Result, ToolError};
use crate::tool::traits::tool_handler::{ToolHandler, ToolContext, ToolResult};
use async_trait::async_trait;
use base64::Engine;
use std::path::{Path, PathBuf};

/// Read a local image and return a data URL for multimodal callers.
/// Usage: inspect screenshots, diagrams, or other local image assets from the workspace.
/// 使用场景：需要让模型查看本地截图、图表或图片资源时使用。
pub struct ViewImageTool;

impl ViewImageTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ViewImageTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for ViewImageTool {
    fn name(&self) -> &str {
        "view_image"
    }

    fn description(&self) -> &str {
        "Load a local image file and return a data URL plus MIME metadata. Supports png, jpg/jpeg, gif, webp, and bmp."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the image file"
                },
                "detail": {
                    "type": "string",
                    "enum": ["auto", "low", "high", "original"],
                    "description": "Requested image detail level for downstream model consumers"
                }
            },
            "required": ["path"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
        _config: &crate::config::Config,
    ) -> Result<ToolResult> {
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("Missing path".to_string()))?;
        let detail = args
            .get("detail")
            .and_then(|value| value.as_str())
            .unwrap_or("auto");
        let path = resolve_path(ctx, path_str);
        let mime_type = image_mime_type(&path).ok_or_else(|| {
            ToolError::InvalidArgs(format!("Unsupported image type: {}", path.display()))
        })?;

        if !path.exists() {
            return Ok(ToolResult::error(format!(
                "Image not found: {}",
                path.display()
            )));
        }
        let metadata = tokio::fs::metadata(&path).await?;
        if !metadata.is_file() {
            return Ok(ToolResult::error(format!("Not a file: {}", path.display())));
        }

        let bytes = tokio::fs::read(&path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read image: {}", e)))?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let data_url = format!("data:{};base64,{}", mime_type, encoded);

        Ok(ToolResult::success(data_url)
            .with_metadata("path", serde_json::json!(path))
            .with_metadata("mime_type", serde_json::json!(mime_type))
            .with_metadata("bytes", serde_json::json!(metadata.len()))
            .with_metadata("detail", serde_json::json!(detail)))
    }
}

fn resolve_path(ctx: &ToolContext, path_str: &str) -> PathBuf {
    let path = PathBuf::from(path_str);
    if path.is_absolute() {
        path
    } else {
        ctx.working_dir.join(path)
    }
}

fn image_mime_type(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "bmp" => Some("image/bmp"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_view_image_png() {
        let temp_dir = TempDir::new().unwrap();
        let image_path = temp_dir.path().join("image.png");
        std::fs::write(&image_path, [137, 80, 78, 71, 13, 10, 26, 10]).unwrap();

        let tool = ViewImageTool::new();
        let ctx = ToolContext::default();
        let config = crate::config::Config::default();
        let result = tool
            .execute(
                serde_json::json!({"path": image_path.to_str().unwrap(), "detail": "original"}),
                &ctx,
                &config,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.starts_with("data:image/png;base64,"));
        assert_eq!(result.metadata["mime_type"], "image/png");
    }

    #[tokio::test]
    async fn test_view_image_rejects_unknown_extension() {
        let temp_dir = TempDir::new().unwrap();
        let image_path = temp_dir.path().join("image.txt");
        std::fs::write(&image_path, "not image").unwrap();

        let tool = ViewImageTool::new();
        let ctx = ToolContext::default();
        let config = crate::config::Config::default();
        let result = tool
            .execute(
                serde_json::json!({"path": image_path.to_str().unwrap()}),
                &ctx,
                &config,
            )
            .await;

        assert!(result.is_err());
    }
}

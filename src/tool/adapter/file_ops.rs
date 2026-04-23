//! File operation tools

use crate::error::{Result, ToolError};
use crate::tool::traits::tool::{Tool, ToolContext, ToolResult};
use crate::util::diff::display_diff;
use async_trait::async_trait;
use dialoguer::Confirm;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

fn resolve_path(ctx: &ToolContext, path_str: &str) -> PathBuf {
    let path = PathBuf::from(path_str);
    if path.is_absolute() {
        path
    } else {
        ctx.working_dir.join(path)
    }
}

fn is_likely_binary(bytes: &[u8]) -> bool {
    bytes.contains(&0)
}

/// File read tool.
/// Usage: inspect a text file or a bounded byte range without mutating the workspace.
/// 使用场景：读取文本文件内容，或按偏移与长度读取局部内容，适合只读检查。
pub struct FileReadTool;

impl FileReadTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read a file with optional offset and limit. Returns content plus metadata describing truncation and bytes read."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                },
                "offset": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Byte offset to start reading from"
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Maximum number of bytes to read"
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
        let offset = args
            .get("offset")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as usize;
        let limit = args
            .get("limit")
            .and_then(|value| value.as_u64())
            .unwrap_or(64 * 1024) as usize;
        let path = resolve_path(ctx, path_str);

        tracing::info!(path = %path.display(), offset, limit, "reading file");

        if !path.exists() {
            return Ok(ToolResult::error(format!(
                "File not found: {}",
                path.display()
            )));
        }

        let metadata = tokio::fs::metadata(&path).await?;
        if !metadata.is_file() {
            return Ok(ToolResult::error(format!("Not a file: {}", path.display())));
        }

        let mut file = tokio::fs::File::open(&path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to open file: {}", e)))?;
        let mut probe = vec![0u8; 8 * 1024];
        let probe_bytes = file
            .read(&mut probe)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;
        probe.truncate(probe_bytes);

        if is_likely_binary(&probe) {
            return Ok(ToolResult::error(format!(
                "Binary file is not supported: {}",
                path.display()
            ))
            .with_metadata("path", serde_json::json!(path))
            .with_metadata("size", serde_json::json!(metadata.len())));
        }

        let file_len = metadata.len() as usize;
        let start = offset.min(file_len);
        file.seek(std::io::SeekFrom::Start(start as u64))
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to seek file: {}", e)))?;

        let mut limited = vec![0u8; limit.min(file_len.saturating_sub(start))];
        let bytes_read = file
            .read(&mut limited)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;
        limited.truncate(bytes_read);
        let content = String::from_utf8_lossy(&limited).to_string();
        let truncated = start.saturating_add(bytes_read) < file_len;

        Ok(ToolResult::success(content)
            .with_metadata("path", serde_json::json!(path))
            .with_metadata("size", serde_json::json!(metadata.len()))
            .with_metadata("bytes_read", serde_json::json!(bytes_read))
            .with_metadata("offset", serde_json::json!(start))
            .with_metadata("limit", serde_json::json!(limit))
            .with_metadata("truncated", serde_json::json!(truncated)))
    }
}

/// File write tool.
/// Usage: create or replace a whole file when exact patch editing is not needed.
/// 使用场景：需要新建文件或整文件重写时使用；如果只是局部改动，优先用 apply_patch。
pub struct FileWriteTool;

impl FileWriteTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write content to a file. Creates the file if it doesn't exist, or overwrites if it does. Parent directories must exist."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
        config: &crate::config::Config,
    ) -> Result<ToolResult> {
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("Missing path".to_string()))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("Missing content".to_string()))?;
        let path = resolve_path(ctx, path_str);

        tracing::info!(path = %path.display(), "writing file");

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    ToolError::ExecutionFailed(format!(
                        "Failed to create parent directory {}: {}",
                        parent.display(),
                        e
                    ))
                })?;
            }
        }

        if path.exists() {
            let original_content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
            display_diff(path_str, &original_content, content);

            if config.safety.require_diff_preview {
                let confirmation = Confirm::new()
                    .with_prompt("Apply these changes?")
                    .default(false)
                    .interact()?;

                if !confirmation {
                    return Ok(ToolResult::error("User denied file write.".to_string()));
                }
            }
        }

        tokio::fs::write(&path, content)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write file: {}", e)))?;

        Ok(ToolResult::success(format!(
            "Successfully wrote {} bytes to {}",
            content.len(),
            path.display()
        ))
        .with_metadata("path", serde_json::json!(path))
        .with_metadata("bytes_written", serde_json::json!(content.len())))
    }
}

#[derive(Debug, Clone)]
struct ListedEntry {
    path: PathBuf,
    file_type: &'static str,
    size: u64,
    depth: usize,
}

/// File list tool.
/// Usage: explore directory contents with pagination and optional shallow recursion.
/// 使用场景：浏览目录结构、分页查看文件列表，或做浅层递归探索时使用。
pub struct FileListTool;

impl FileListTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileListTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileListTool {
    fn name(&self) -> &str {
        "file_list"
    }

    fn description(&self) -> &str {
        "List files in a directory with optional dir_path, offset, limit, and depth for paginated directory exploration."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "dir_path": {
                    "type": "string",
                    "description": "Path to the directory to list (defaults to current directory)"
                },
                "path": {
                    "type": "string",
                    "description": "Alias for dir_path"
                },
                "offset": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Number of entries to skip"
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Maximum number of entries to return"
                },
                "depth": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Maximum recursion depth"
                }
            }
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
        let path_str = args
            .get("dir_path")
            .or_else(|| args.get("path"))
            .and_then(|value| value.as_str())
            .unwrap_or(".");
        let offset = args
            .get("offset")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as usize;
        let limit = args
            .get("limit")
            .and_then(|value| value.as_u64())
            .unwrap_or(100) as usize;
        let depth = args
            .get("depth")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as usize;
        let path = resolve_path(ctx, path_str);

        tracing::info!(path = %path.display(), offset, limit, depth, "listing directory");

        if !path.exists() {
            return Ok(ToolResult::error(format!(
                "Directory not found: {}",
                path.display()
            )));
        }
        if !path.is_dir() {
            return Ok(ToolResult::error(format!(
                "Not a directory: {}",
                path.display()
            )));
        }

        let mut entries = Vec::new();
        collect_entries(&path, &path, 0, depth, &mut entries).await?;
        entries.sort_by(|left, right| left.path.cmp(&right.path));

        let total_entries = entries.len();
        let paged_entries = entries
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect::<Vec<_>>();
        let has_more = offset.saturating_add(paged_entries.len()) < total_entries;

        let output = if paged_entries.is_empty() {
            "Directory is empty".to_string()
        } else {
            format!(
                "Found {} items (showing {}-{}):\n{}",
                total_entries,
                offset.saturating_add(1),
                offset + paged_entries.len(),
                paged_entries
                    .iter()
                    .map(format_entry_line)
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };

        Ok(ToolResult::success(output)
            .with_metadata("path", serde_json::json!(path))
            .with_metadata("offset", serde_json::json!(offset))
            .with_metadata("limit", serde_json::json!(limit))
            .with_metadata("depth", serde_json::json!(depth))
            .with_metadata("total_entries", serde_json::json!(total_entries))
            .with_metadata("has_more", serde_json::json!(has_more)))
    }
}

async fn collect_entries(
    root: &Path,
    current: &Path,
    current_depth: usize,
    max_depth: usize,
    entries: &mut Vec<ListedEntry>,
) -> Result<()> {
    let mut read_dir = tokio::fs::read_dir(current).await.map_err(|e| {
        ToolError::ExecutionFailed(format!(
            "Failed to read directory {}: {}",
            current.display(),
            e
        ))
    })?;

    while let Some(entry) = read_dir.next_entry().await? {
        let path = entry.path();
        let metadata = tokio::fs::symlink_metadata(&path).await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to stat {}: {}", path.display(), e))
        })?;
        let file_type = if metadata.file_type().is_symlink() {
            "link"
        } else if metadata.is_dir() {
            "dir"
        } else {
            "file"
        };

        entries.push(ListedEntry {
            path: path.strip_prefix(root).unwrap_or(&path).to_path_buf(),
            file_type,
            size: metadata.len(),
            depth: current_depth,
        });

        if metadata.is_dir() && !metadata.file_type().is_symlink() && current_depth < max_depth {
            Box::pin(collect_entries(
                root,
                &path,
                current_depth + 1,
                max_depth,
                entries,
            ))
            .await?;
        }
    }

    Ok(())
}

fn format_entry_line(entry: &ListedEntry) -> String {
    format!(
        "{:<10} {:<10} {:<5} {}",
        entry.file_type,
        entry.size,
        entry.depth,
        entry.path.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_read() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "Hello, World!").unwrap();

        let tool = FileReadTool::new();
        let ctx = ToolContext::default();
        let config = crate::config::Config::default();

        let result = tool
            .execute(
                serde_json::json!({"path": file_path.to_str().unwrap()}),
                &ctx,
                &config,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output, "Hello, World!");
    }

    #[tokio::test]
    async fn test_file_read_with_limit() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "Hello, World!").unwrap();

        let tool = FileReadTool::new();
        let ctx = ToolContext::default();
        let config = crate::config::Config::default();

        let result = tool
            .execute(
                serde_json::json!({"path": file_path.to_str().unwrap(), "offset": 7, "limit": 5}),
                &ctx,
                &config,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output, "World");
    }

    #[tokio::test]
    async fn test_file_write() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("output.txt");

        let tool = FileWriteTool::new();
        let ctx = ToolContext::default();
        let config = crate::config::Config::default();

        let result = tool
            .execute(
                serde_json::json!({
                    "path": file_path.to_str().unwrap(),
                    "content": "Test content"
                }),
                &ctx,
                &config,
            )
            .await
            .unwrap();

        assert!(result.success);
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Test content");
    }

    #[tokio::test]
    async fn test_file_list() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("file1.txt"), "").unwrap();
        std::fs::write(temp_dir.path().join("file2.txt"), "").unwrap();

        let tool = FileListTool::new();
        let ctx = ToolContext::default();
        let config = crate::config::Config::default();

        let result = tool
            .execute(
                serde_json::json!({"dir_path": temp_dir.path().to_str().unwrap()}),
                &ctx,
                &config,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("file1.txt"));
        assert!(result.output.contains("file2.txt"));
    }

    #[tokio::test]
    async fn test_file_list_with_depth() {
        let temp_dir = TempDir::new().unwrap();
        let nested = temp_dir.path().join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("deep.txt"), "hello").unwrap();

        let tool = FileListTool::new();
        let ctx = ToolContext::default();
        let config = crate::config::Config::default();

        let shallow = tool
            .execute(
                serde_json::json!({"dir_path": temp_dir.path().to_str().unwrap(), "depth": 0}),
                &ctx,
                &config,
            )
            .await
            .unwrap();
        assert!(!shallow.output.contains("deep.txt"));

        let deep = tool
            .execute(
                serde_json::json!({"dir_path": temp_dir.path().to_str().unwrap(), "depth": 2}),
                &ctx,
                &config,
            )
            .await
            .unwrap();
        assert!(deep.output.contains("deep.txt"));
    }
}

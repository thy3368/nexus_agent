//! Codebase search tool

use crate::error::{Result, ToolError};
use crate::tool::traits::tool::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use regex::Regex;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug, Clone)]
struct SearchOptions {
    pattern: String,
    path: PathBuf,
    glob: Option<String>,
    include: Vec<String>,
    exclude: Vec<String>,
    limit: usize,
    case_sensitive: bool,
}

/// Codebase Search tool
pub struct CodebaseSearchTool;

impl CodebaseSearchTool {
    pub fn new() -> Self {
        Self
    }

    fn parse_options(&self, args: &serde_json::Value, ctx: &ToolContext) -> Result<SearchOptions> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("Missing search pattern".to_string()))?
            .to_string();
        Regex::new(&pattern)
            .map_err(|e| ToolError::InvalidArgs(format!("Invalid regex pattern: {}", e)))?;

        let path = args
            .get("path")
            .and_then(|value| value.as_str())
            .map(PathBuf::from)
            .map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    ctx.working_dir.join(path)
                }
            })
            .unwrap_or_else(|| ctx.working_dir.clone());
        let glob = args
            .get("glob")
            .and_then(|value| value.as_str())
            .map(ToString::to_string);
        let include = read_string_array(args, "include");
        let exclude = read_string_array(args, "exclude");
        let limit = args
            .get("limit")
            .and_then(|value| value.as_u64())
            .unwrap_or(200) as usize;
        let case_sensitive = args
            .get("case_sensitive")
            .and_then(|value| value.as_bool())
            .unwrap_or(true);

        Ok(SearchOptions {
            pattern,
            path,
            glob,
            include,
            exclude,
            limit,
            case_sensitive,
        })
    }

    async fn search_with_powershell(
        &self,
        options: &SearchOptions,
        ctx: &ToolContext,
    ) -> Result<ToolResult> {
        let script = build_powershell_script(options);
        let output = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-Command")
            .arg(script)
            .current_dir(&ctx.working_dir)
            .output()
            .await?;

        build_search_result(output, options.limit)
    }
}

impl Default for CodebaseSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CodebaseSearchTool {
    fn name(&self) -> &str {
        "codebase_search"
    }

    fn description(&self) -> &str {
        "Search file contents with regex plus optional path, glob, include/exclude filters, limit, and case sensitivity."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regular expression pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search within. Defaults to current working directory."
                },
                "glob": {
                    "type": "string",
                    "description": "Single glob filter to include matching files"
                },
                "include": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Additional globs to include"
                },
                "exclude": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Globs to exclude"
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Maximum number of matching lines to return"
                },
                "case_sensitive": {
                    "type": "boolean",
                    "description": "Whether the regex search is case-sensitive"
                }
            },
            "required": ["pattern"]
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
        let options = self.parse_options(&args, ctx)?;
        tracing::info!(pattern = %options.pattern, path = %options.path.display(), limit = options.limit, "searching codebase");

        if Command::new("rg").arg("--version").output().await.is_ok() {
            let mut command = Command::new("rg");
            command
                .arg("--line-number")
                .arg("--with-filename")
                .arg("--color=never");
            if !options.case_sensitive {
                command.arg("-i");
            }
            if let Some(glob) = &options.glob {
                command.arg("--glob").arg(glob);
            }
            for include in &options.include {
                command.arg("--glob").arg(include);
            }
            for exclude in &options.exclude {
                command.arg("--glob").arg(format!("!{}", exclude));
            }
            command
                .arg("--max-count")
                .arg(options.limit.to_string())
                .arg(&options.pattern)
                .arg(&options.path)
                .current_dir(&ctx.working_dir);
            return build_search_result(command.output().await?, options.limit);
        }

        if Command::new("grep").arg("--version").output().await.is_ok() {
            let mut command = Command::new("grep");
            command.arg("-n").arg("-r");
            if !options.case_sensitive {
                command.arg("-i");
            }
            command.arg("-m").arg(options.limit.to_string());
            for include in combined_includes(&options) {
                command.arg(format!("--include={}", include));
            }
            for exclude in &options.exclude {
                command.arg(format!("--exclude={}", exclude));
            }
            command
                .arg(&options.pattern)
                .arg(&options.path)
                .current_dir(&ctx.working_dir);
            return build_search_result(command.output().await?, options.limit);
        }

        self.search_with_powershell(&options, ctx).await
    }
}

fn read_string_array(args: &serde_json::Value, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(ToString::to_string))
        .collect()
}

fn combined_includes(options: &SearchOptions) -> Vec<String> {
    let mut includes = Vec::new();
    if let Some(glob) = &options.glob {
        includes.push(glob.clone());
    }
    includes.extend(options.include.clone());
    includes
}

fn build_powershell_script(options: &SearchOptions) -> String {
    let includes = combined_includes(options)
        .into_iter()
        .map(|glob| format!("'{}'", glob.replace('"', "`\"")))
        .collect::<Vec<_>>()
        .join(", ");
    let excludes = options
        .exclude
        .iter()
        .map(|glob| format!("'{}'", glob.replace('"', "`\"")))
        .collect::<Vec<_>>()
        .join(", ");
    let case_flag = if options.case_sensitive { "" } else { "(?i)" };

    format!(
        r#"
$includes = @({includes})
$excludes = @({excludes})
$files = Get-ChildItem -Path '{path}' -Recurse -File
if ($includes.Count -gt 0) {{
  $files = $files | Where-Object {{
    $name = $_.FullName
    $includes | Where-Object {{ $name -like $_ }}
  }}
}}
if ($excludes.Count -gt 0) {{
  $files = $files | Where-Object {{
    $name = $_.FullName
    -not ($excludes | Where-Object {{ $name -like $_ }})
  }}
}}
$files | Select-String -Pattern '{case_flag}{pattern}' | Select-Object -First {limit} | ForEach-Object {{ "$($_.Path):$($_.LineNumber):$($_.Line)" }}
"#,
        includes = includes,
        excludes = excludes,
        path = options.path.display().to_string().replace('"', "`\""),
        case_flag = case_flag,
        pattern = options.pattern.replace('"', "`\""),
        limit = options.limit,
    )
}

fn build_search_result(output: std::process::Output, limit: usize) -> Result<ToolResult> {
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if output.status.success() || output.status.code() == Some(1) {
        let matches = stdout.lines().count();
        Ok(ToolResult::success(stdout)
            .with_metadata("match_count", serde_json::json!(matches))
            .with_metadata("limit", serde_json::json!(limit)))
    } else {
        Err(ToolError::ExecutionFailed(format!("Codebase search failed: {}", stderr)).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_codebase_search_success() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        fs::write(dir_path.join("file1.txt"), "hello world").unwrap();
        fs::write(
            dir_path.join("file2.rs"),
            "fn main() {\n    println!(\"hello\");\n}",
        )
        .unwrap();

        let tool = CodebaseSearchTool::new();
        let ctx = ToolContext {
            working_dir: dir_path.to_path_buf(),
            ..Default::default()
        };
        let config = crate::config::Config::default();

        let result = tool
            .execute(
                serde_json::json!({"pattern": "hello", "limit": 10}),
                &ctx,
                &config,
            )
            .await;
        if let Err(e) = result {
            if e.to_string().contains("program not found") {
                eprintln!("Skipping test: ripgrep not installed");
                return;
            }
            panic!("Unexpected error: {}", e);
        }
        let result = result.unwrap();
        assert!(result.success);
        assert!(result.output.contains("file1.txt") || result.output.contains("hello"));
    }

    #[tokio::test]
    async fn test_codebase_search_no_match() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();
        fs::write(dir_path.join("file1.txt"), "foo bar").unwrap();

        let tool = CodebaseSearchTool::new();
        let ctx = ToolContext {
            working_dir: dir_path.to_path_buf(),
            ..Default::default()
        };
        let config = crate::config::Config::default();

        let result = tool
            .execute(serde_json::json!({"pattern": "nonexistent"}), &ctx, &config)
            .await;
        if let Err(e) = result {
            if e.to_string().contains("program not found") {
                eprintln!("Skipping test: ripgrep not installed");
                return;
            }
            panic!("Unexpected error: {}", e);
        }
        let result = result.unwrap();
        assert!(result.success);
        assert!(result.output.is_empty() || !result.output.contains("nonexistent"));
    }

    #[tokio::test]
    async fn test_codebase_search_invalid_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let tool = CodebaseSearchTool::new();
        let ctx = ToolContext {
            working_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        let config = crate::config::Config::default();

        let result = tool
            .execute(serde_json::json!({"pattern": "["}), &ctx, &config)
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid regex pattern"));
    }
}

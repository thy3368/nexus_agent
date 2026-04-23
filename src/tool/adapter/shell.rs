//! Shell command execution tool

use crate::error::{Result, ToolError};
use crate::tool::traits::tool::{BoxToolFuture, Tool, ToolContext, ToolInvocation, ToolResult};
use async_trait::async_trait;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GitCommandAccess {
    ReadOnly,
    Mutating,
    Unknown,
}

#[derive(Debug, Clone)]
struct ShellExecParams {
    command: String,
    workdir: PathBuf,
    timeout_secs: u64,
    login: bool,
}

/// Shell command execution tool.
/// Usage: run workspace commands when no narrower purpose-built tool exists.
/// 使用场景：需要运行构建、测试或项目命令，且没有更专用工具可用时使用。
pub struct ShellTool {
    timeout_secs: u64,
}

impl ShellTool {
    pub fn new() -> Self {
        Self { timeout_secs: 30 }
    }

    pub fn with_timeout(timeout_secs: u64) -> Self {
        Self { timeout_secs }
    }

    fn to_exec_params(
        &self,
        args: &serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ShellExecParams> {
        let command = args
            .get("command")
            .or_else(|| args.get("cmd"))
            .and_then(|value| value.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("Missing command".to_string()))?
            .to_string();

        let workdir = args
            .get("workdir")
            .and_then(|value| value.as_str())
            .map(|path| {
                let path = PathBuf::from(path);
                if path.is_absolute() {
                    path
                } else {
                    ctx.working_dir.join(path)
                }
            })
            .unwrap_or_else(|| ctx.working_dir.clone());

        let timeout_secs = args
            .get("timeout_ms")
            .and_then(|value| value.as_u64())
            .map(|ms| ms.div_ceil(1_000).max(1))
            .unwrap_or(self.timeout_secs);

        let login = args
            .get("login")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);

        Ok(ShellExecParams {
            command,
            workdir,
            timeout_secs,
            login,
        })
    }
}

impl Default for ShellTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell_execute"
    }

    fn description(&self) -> &str {
        "Execute a shell command with optional workdir, timeout_ms, login shell, and justification. Returns stdout/stderr/exit metadata."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "cmd": {
                    "type": "string",
                    "description": "Alias for command"
                },
                "workdir": {
                    "type": "string",
                    "description": "Directory to execute the command in. Relative paths resolve from the tool working directory."
                },
                "timeout_ms": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Execution timeout in milliseconds"
                },
                "login": {
                    "type": "boolean",
                    "description": "Use a login shell when supported"
                },
                "justification": {
                    "type": "string",
                    "description": "Optional reason for running the command"
                }
            },
            "required": ["command"]
        })
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_mutating<'a>(&'a self, invocation: &'a ToolInvocation) -> BoxToolFuture<'a, bool> {
        Box::pin(async move {
            let command = invocation
                .args
                .get("command")
                .or_else(|| invocation.args.get("cmd"))
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            !is_known_read_only_command(command)
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
        _config: &crate::config::Config,
    ) -> Result<ToolResult> {
        let params = self.to_exec_params(&args, ctx)?;

        tracing::info!(command = %params.command, workdir = %params.workdir.display(), "executing shell command");

        let (shell, shell_arg) = if cfg!(target_os = "windows") {
            ("cmd", "/C")
        } else if params.login {
            ("sh", "-lc")
        } else {
            ("sh", "-c")
        };

        let started = Instant::now();
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(params.timeout_secs),
            Command::new(shell)
                .arg(shell_arg)
                .arg(&params.command)
                .current_dir(&params.workdir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await
        .map_err(|_| ToolError::Timeout)??;

        let duration_ms = started.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        let result = if output.status.success() {
            ToolResult::success(stdout.clone())
        } else {
            ToolResult::error(format!(
                "Command failed with exit code {}: {}",
                exit_code, stderr
            ))
        };

        Ok(result
            .with_metadata("exit_code", serde_json::json!(exit_code))
            .with_metadata("stdout", serde_json::json!(stdout))
            .with_metadata("stderr", serde_json::json!(stderr))
            .with_metadata("duration_ms", serde_json::json!(duration_ms))
            .with_metadata("workdir", serde_json::json!(params.workdir)))
    }
}

fn is_known_read_only_command(command: &str) -> bool {
    let trimmed = command.trim();
    if trimmed.is_empty()
        || contains_shell_metacharacters(trimmed)
        || contains_write_operators(trimmed)
    {
        return false;
    }

    let tokens = tokenize_command(trimmed);
    let Some(first) = tokens.first().map(String::as_str) else {
        return false;
    };

    match first {
        "cat" | "cd" | "find" | "grep" | "head" | "ls" | "pwd" | "rg" | "tail" | "tree" | "wc" => {
            true
        }
        "git" => classify_git_command(&tokens) == GitCommandAccess::ReadOnly,
        _ => false,
    }
}

fn contains_shell_metacharacters(command: &str) -> bool {
    command.contains('|')
        || command.contains('&')
        || command.contains(';')
        || command.contains('`')
        || command.contains('$')
        || command.contains('(')
        || command.contains(')')
        || command.contains('<')
        || command.contains('\n')
}

fn contains_write_operators(command: &str) -> bool {
    command.contains('>')
        || command.contains(" rm ")
        || command.starts_with("rm ")
        || command.contains(" mv ")
        || command.starts_with("mv ")
        || command.contains(" cp ")
        || command.starts_with("cp ")
}

fn tokenize_command(command: &str) -> Vec<String> {
    command
        .split_whitespace()
        .map(ToString::to_string)
        .collect()
}

fn classify_git_command(tokens: &[String]) -> GitCommandAccess {
    let Some(subcommand) = tokens.get(1).map(String::as_str) else {
        return GitCommandAccess::Unknown;
    };

    match subcommand {
        "status" | "diff" | "log" | "show" | "rev-parse" | "describe" | "remote" | "ls-files"
        | "blame" | "grep" | "cat-file" | "branch" | "tag" => {
            if tokens.iter().skip(2).any(|token| git_mutating_flag(token)) {
                GitCommandAccess::Mutating
            } else {
                GitCommandAccess::ReadOnly
            }
        }
        _ => GitCommandAccess::Mutating,
    }
}

fn git_mutating_flag(token: &str) -> bool {
    matches!(
        token,
        "-d" | "-D"
            | "-m"
            | "-M"
            | "--delete"
            | "--move"
            | "--copy"
            | "--edit-description"
            | "--unset-upstream"
            | "--set-upstream-to"
            | "-f"
            | "--force"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shell_execute_success() {
        let tool = ShellTool::new();
        let ctx = ToolContext::default();
        let config = crate::config::Config::default();

        let result = tool
            .execute(serde_json::json!({"command": "echo hello"}), &ctx, &config)
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("hello"));
        assert!(result.metadata.contains_key("duration_ms"));
    }

    #[tokio::test]
    async fn test_shell_execute_failure() {
        let tool = ShellTool::new();
        let ctx = ToolContext::default();
        let config = crate::config::Config::default();

        let result = tool
            .execute(
                serde_json::json!({"command": "nonexistent_command_xyz"}),
                &ctx,
                &config,
            )
            .await
            .unwrap();

        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_shell_timeout() {
        let tool = ShellTool::with_timeout(2);
        let ctx = ToolContext::default();
        let config = crate::config::Config::default();

        let command = if cfg!(target_os = "windows") {
            "ping -n 6 127.0.0.1 > nul"
        } else {
            "sleep 5"
        };

        let result = tool
            .execute(serde_json::json!({"command": command}), &ctx, &config)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_shell_read_only_classification() {
        let tool = ShellTool::new();
        let read = ToolInvocation::new("shell_execute", serde_json::json!({"command": "ls -la"}));
        let write = ToolInvocation::new("shell_execute", serde_json::json!({"command": "rm file"}));

        assert!(!tool.is_mutating(&read).await);
        assert!(tool.is_mutating(&write).await);
    }
}

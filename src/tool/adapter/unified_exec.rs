//! Unified command execution tools

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
struct ExecCommandArgs {
    command: String,
    workdir: PathBuf,
    timeout_ms: u64,
    max_output_tokens: Option<usize>,
    session_id: Option<String>,
    yield_time_ms: Option<u64>,
    tty: bool,
}

/// Codex-style unified exec command adapter.
/// Usage: run structured command execution requests that may later evolve into session-based exec flows.
/// 使用场景：需要以统一参数结构执行命令，且后续可能扩展为会话式 exec 流程时使用。
pub struct ExecCommandTool {
    default_timeout_ms: u64,
}

impl ExecCommandTool {
    pub fn new() -> Self {
        Self {
            default_timeout_ms: 30_000,
        }
    }

    fn parse_args(&self, args: &serde_json::Value, ctx: &ToolContext) -> Result<ExecCommandArgs> {
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
        let timeout_ms = args
            .get("timeout_ms")
            .and_then(|value| value.as_u64())
            .unwrap_or(self.default_timeout_ms);
        let max_output_tokens = args
            .get("max_output_tokens")
            .and_then(|value| value.as_u64())
            .map(|value| value as usize);
        let session_id = args
            .get("session_id")
            .and_then(|value| value.as_str())
            .map(ToString::to_string);
        let yield_time_ms = args.get("yield_time_ms").and_then(|value| value.as_u64());
        let tty = args
            .get("tty")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);

        Ok(ExecCommandArgs {
            command,
            workdir,
            timeout_ms,
            max_output_tokens,
            session_id,
            yield_time_ms,
            tty,
        })
    }
}

impl Default for ExecCommandTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ExecCommandTool {
    fn name(&self) -> &str {
        "exec_command"
    }

    fn description(&self) -> &str {
        "Execute a command through the unified exec interface. Current implementation is non-interactive; session_id/stdin continuation are reserved for a future process manager."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Command to execute"
                },
                "cmd": {
                    "type": "string",
                    "description": "Alias for command"
                },
                "workdir": {
                    "type": "string",
                    "description": "Working directory for command execution"
                },
                "timeout_ms": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Execution timeout in milliseconds"
                },
                "session_id": {
                    "type": "string",
                    "description": "Reserved for future long-running process sessions"
                },
                "yield_time_ms": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Reserved for future yielding process sessions"
                },
                "max_output_tokens": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Approximate maximum output tokens to return"
                },
                "tty": {
                    "type": "boolean",
                    "description": "Reserved for future pseudo-terminal execution"
                }
            },
            "required": ["command"]
        })
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
        let args = self.parse_args(&args, ctx)?;
        if args.session_id.is_some() || args.yield_time_ms.is_some() || args.tty {
            return Ok(ToolResult::error(
                "Interactive exec sessions are not wired into Nexus yet".to_string(),
            )
            .with_metadata("session_id", serde_json::json!(args.session_id))
            .with_metadata("yield_time_ms", serde_json::json!(args.yield_time_ms))
            .with_metadata("tty", serde_json::json!(args.tty)));
        }

        let (shell, shell_arg) = if cfg!(target_os = "windows") {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let started = Instant::now();
        let output = tokio::time::timeout(
            std::time::Duration::from_millis(args.timeout_ms),
            Command::new(shell)
                .arg(shell_arg)
                .arg(&args.command)
                .current_dir(&args.workdir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await
        .map_err(|_| ToolError::Timeout)??;

        let duration_ms = started.elapsed().as_millis() as u64;
        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = truncate_output(
            String::from_utf8_lossy(&output.stdout).to_string(),
            args.max_output_tokens,
        );
        let stderr = truncate_output(
            String::from_utf8_lossy(&output.stderr).to_string(),
            args.max_output_tokens,
        );
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
            .with_metadata("workdir", serde_json::json!(args.workdir)))
    }
}

/// Reserved stdin continuation tool for the future process manager.
/// Usage: keep the write-to-session API shape stable until interactive exec support is implemented.
/// 使用场景：为未来交互式命令会话保留 stdin 写入口，当前仅作为占位接口。
pub struct WriteStdinTool;

impl WriteStdinTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WriteStdinTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WriteStdinTool {
    fn name(&self) -> &str {
        "write_stdin"
    }

    fn description(&self) -> &str {
        "Write stdin to an existing exec session. Currently returns unsupported until a process manager is wired into Nexus."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "Existing exec session id"
                },
                "input": {
                    "type": "string",
                    "description": "Input to write to stdin"
                }
            },
            "required": ["session_id", "input"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        _ctx: &ToolContext,
        _config: &crate::config::Config,
    ) -> Result<ToolResult> {
        Ok(
            ToolResult::error("Exec stdin sessions are not wired into Nexus yet".to_string())
                .with_metadata(
                    "session_id",
                    args.get("session_id")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                ),
        )
    }
}

fn truncate_output(output: String, max_output_tokens: Option<usize>) -> String {
    let Some(max_output_tokens) = max_output_tokens else {
        return output;
    };
    let max_chars = max_output_tokens.saturating_mul(4);
    if output.chars().count() <= max_chars {
        output
    } else {
        let mut truncated = output.chars().take(max_chars).collect::<String>();
        truncated.push_str("\n[output truncated]");
        truncated
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
    async fn test_exec_command() {
        let tool = ExecCommandTool::new();
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
    async fn test_exec_command_rejects_session_mode() {
        let tool = ExecCommandTool::new();
        let ctx = ToolContext::default();
        let config = crate::config::Config::default();

        let result = tool
            .execute(
                serde_json::json!({"command": "echo hello", "session_id": "abc"}),
                &ctx,
                &config,
            )
            .await
            .unwrap();

        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_write_stdin_unsupported() {
        let tool = WriteStdinTool::new();
        let ctx = ToolContext::default();
        let config = crate::config::Config::default();

        let result = tool
            .execute(
                serde_json::json!({"session_id": "abc", "input": "hello"}),
                &ctx,
                &config,
            )
            .await
            .unwrap();

        assert!(!result.success);
    }
}

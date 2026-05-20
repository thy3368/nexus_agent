//! Tool executor for executing tools with permission and safety checks

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::agent::mode::AgentMode;
use crate::config::Config;
use crate::error::Result;
use crate::formatter::ResponseFormatter;
use crate::permissions::{PermissionLevel, PermissionManager};
use crate::safety::SafetyValidator;
use crate::tool::tool_registry::ToolRegistry;
use crate::context::context_provider::ContextProvider;
use crate::tool::traits::tool_definition::{ToolContext, ToolInvocation};
use crate::util::json_log::{build_log_prefix, log_file_path, write_json_log};

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

pub struct ToolExecutor {
    pub tools: ToolRegistry,
    permission_manager: Arc<Mutex<PermissionManager>>,
    safety_validator: SafetyValidator,
    formatter: ResponseFormatter,
    mode: AgentMode,
    log_dir: Option<PathBuf>,
}

impl ToolExecutor {
    pub fn new(
        tools: ToolRegistry,
        permission_manager: Arc<Mutex<PermissionManager>>,
        safety_validator: SafetyValidator,
    ) -> Self {
        Self::new_with_mode(
            tools,
            permission_manager,
            safety_validator,
            AgentMode::Execute,
            None,
        )
    }

    pub fn new_with_mode(
        tools: ToolRegistry,
        permission_manager: Arc<Mutex<PermissionManager>>,
        safety_validator: SafetyValidator,
        mode: AgentMode,
        log_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            tools,
            permission_manager,
            safety_validator,
            formatter: ResponseFormatter::new(),
            mode,
            log_dir,
        }
    }

    pub async fn execute(
        &self,
        tool_call: ToolCall,
        config: &Config,
        session_id: &str,
    ) -> Result<ToolExecutionResult> {
        tracing::debug!("[TOOL EXEC] Input name={}", tool_call.name);
        tracing::debug!("[TOOL EXEC] Input args={}", tool_call.args);

        if let Some(log_dir) = &self.log_dir {
            let prefix = build_log_prefix("tool", &tool_call.name, session_id);
            let path = log_file_path(log_dir, &prefix, "call");
            let payload = serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "session_id": session_id,
                "tool": tool_call.name,
                "args": tool_call.args,
            });
            if let Err(err) = write_json_log(path.clone(), payload).await {
                tracing::warn!(path = %path.display(), error = %err, "Failed to write tool call log");
            }
        }

        let result = self.do_execute(tool_call.clone(), config).await;

        match &result {
            Ok(exec_result) => {
                tracing::debug!("[TOOL EXEC] Output success={}", exec_result.success);
                tracing::debug!("[TOOL EXEC] Output output={}", exec_result.output);
                if let Some(ref err) = exec_result.error {
                    tracing::debug!("[TOOL EXEC] Output error={}", err);
                }

                if let Some(log_dir) = &self.log_dir {
                    let prefix = build_log_prefix("tool", &tool_call.name, session_id);
                    let path = log_file_path(log_dir, &prefix, "result");
                    let payload = serde_json::json!({
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "session_id": session_id,
                        "tool": tool_call.name,
                        "success": exec_result.success,
                        "output": exec_result.output,
                        "error": exec_result.error,
                    });
                    if let Err(err) = write_json_log(path.clone(), payload).await {
                        tracing::warn!(path = %path.display(), error = %err, "Failed to write tool result log");
                    }
                }
            }
            Err(e) => {
                tracing::debug!("[TOOL EXEC] Failed: {}", e);

                if let Some(log_dir) = &self.log_dir {
                    let prefix = build_log_prefix("tool", &tool_call.name, session_id);
                    let path = log_file_path(log_dir, &prefix, "error");
                    let payload = serde_json::json!({
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "session_id": session_id,
                        "tool": tool_call.name,
                        "error": e.to_string(),
                    });
                    if let Err(err) = write_json_log(path.clone(), payload).await {
                        tracing::warn!(path = %path.display(), error = %err, "Failed to write tool error log");
                    }
                }
            }
        }

        result
    }
    /// Execute a tool call with permission and safety checks.
    ///
    /// The executor owns cross-cutting concerns only: permissions, safety validation,
    /// runtime context assembly and user-facing formatting. Actual argument validation,
    /// mutability classification and execution are delegated into the registry's
    /// ToolHandler-style dispatch path.
    pub async fn do_execute(
        &self,
        tool_call: ToolCall,
        config: &Config,
    ) -> Result<ToolExecutionResult> {
        let invocation = ToolInvocation::new(tool_call.name.clone(), tool_call.args.clone());

        if self.mode.is_plan() && tool_call.name == "update_plan" {
            return Ok(ToolExecutionResult {
                success: false,
                output: "update_plan is a TODO/checklist tool and is not allowed in Plan mode"
                    .to_string(),
                error: Some("Tool is not allowed in Plan mode".to_string()),
            });
        }

        let is_mutating = self.tools.is_mutating(&invocation).await?;
        tracing::debug!(tool = %tool_call.name, is_mutating, "tool mutability classified");

        if self.mode.is_plan() && is_mutating {
            return Ok(ToolExecutionResult {
                success: false,
                output: format!(
                    "Tool '{}' is not allowed in Plan mode because it may modify state.",
                    tool_call.name
                ),
                error: Some("Mutation blocked in Plan mode".to_string()),
            });
        }

        // Permission is checked before dispatch so potentially mutating tools cannot run
        // without explicit approval.
        let permission_level = {
            let pm = self.permission_manager.lock().unwrap();
            pm.check_permission(&tool_call.name)
        };

        match permission_level {
            PermissionLevel::Never => {
                return Err(crate::error::ToolError::PermissionDenied(tool_call.name).into());
            }
            PermissionLevel::Ask => {
                let allowed = {
                    let mut pm = self.permission_manager.lock().unwrap();
                    pm.prompt_for_permission(&tool_call.name)
                        .map_err(|e| crate::error::PromptLineError::Other(e.to_string()))?
                };

                if !allowed {
                    return Ok(ToolExecutionResult {
                        success: false,
                        output: "Permission denied.".to_string(),
                        error: None,
                    });
                }
            }
            PermissionLevel::Once | PermissionLevel::Always => {}
        }

        let command_str = format!("{} {}", tool_call.name, tool_call.args);
        match self.safety_validator.validate_command(&command_str) {
            crate::safety::ValidationResult::Denied(reason) => {
                return Err(crate::error::PromptLineError::Safety(reason));
            }
            crate::safety::ValidationResult::RequiresApproval => {}
            crate::safety::ValidationResult::Allowed => {}
        }

        let mut ctx = ToolContext::default();
        if let Some(branch) = ContextProvider::get_git_branch_sync() {
            ctx.git_branch = Some(branch);
        }

        let result = self.tools.dispatch(invocation, &ctx, config).await?;

        let result_text = if result.success {
            &result.output
        } else {
            result.error.as_ref().unwrap_or(&result.output)
        };

        let formatted_output = self
            .formatter
            .format_tool_result(&tool_call.name, result_text);
        print!("{}", formatted_output);
        use std::io::Write;
        std::io::stdout().flush().ok();

        Ok(ToolExecutionResult {
            success: result.success,
            output: result_text.clone(),
            error: result.error,
        })
    }

    /// Format file write output for display
    pub fn format_file_write_output(&self, path: &str, content: &str) {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("txt");

        println!("\n\x1b[1;32mWritten to {}:\x1b[0m", path);
        println!("```{}", ext);
        println!("{}", content);
        println!("```\n");
    }
}

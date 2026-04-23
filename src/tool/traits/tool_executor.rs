//! Tool executor for executing tools with permission and safety checks

use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::error::Result;
use crate::formatter::ResponseFormatter;
use crate::permissions::{PermissionLevel, PermissionManager};
use crate::safety::SafetyValidator;
use crate::tool::tool_registry::ToolRegistry;
use crate::tool::traits::context_provider::ContextProvider;
use crate::tool::traits::tool_handler::{ToolContext, ToolInvocation};

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
}

impl ToolExecutor {
    pub fn new(
        tools: ToolRegistry,
        permission_manager: Arc<Mutex<PermissionManager>>,
        safety_validator: SafetyValidator,
    ) -> Self {
        Self {
            tools,
            permission_manager,
            safety_validator,
            formatter: ResponseFormatter::new(),
        }
    }

    pub async fn execute(
        &self,
        tool_call: ToolCall,
        config: &Config,
    ) -> Result<ToolExecutionResult> {
        tracing::debug!("[TOOL EXEC] Input name={}", tool_call.name);
        tracing::debug!("[TOOL EXEC] Input args={}", tool_call.args);

        let result = self.do_execute(tool_call, config).await;

        match &result {
            Ok(exec_result) => {
                tracing::debug!("[TOOL EXEC] Output success={}", exec_result.success);
                tracing::debug!("[TOOL EXEC] Output output={}", exec_result.output);
                if let Some(ref err) = exec_result.error {
                    tracing::debug!("[TOOL EXEC] Output error={}", err);
                }
            }
            Err(e) => {
                tracing::debug!("[TOOL EXEC] Failed: {}", e);
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

        let is_mutating = self.tools.is_mutating(&invocation).await?;
        tracing::debug!(tool = %tool_call.name, is_mutating, "tool mutability classified");

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

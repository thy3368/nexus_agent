//! Tool executor for executing tools with permission and safety checks

use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::error::Result;
use crate::formatter::ResponseFormatter;
use crate::permissions::{PermissionLevel, PermissionManager};
use crate::safety::SafetyValidator;
use crate::tools::{ToolContext, ToolRegistry};

use super::context_provider::ContextProvider;

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

    /// Execute a tool call with permission and safety checks
    pub async fn execute(&self, tool_call: ToolCall, config: &Config) -> Result<ToolExecutionResult> {
        tracing::info!("Executing tool: {}", tool_call.name);

        // Check permission
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
            PermissionLevel::Once | PermissionLevel::Always => {
                // Permission already granted
            }
        }

        // Validate command
        let command_str = format!("{} {}", tool_call.name, tool_call.args);
        match self.safety_validator.validate_command(&command_str) {
            crate::safety::ValidationResult::Denied(reason) => {
                return Err(crate::error::PromptLineError::Safety(reason));
            }
            crate::safety::ValidationResult::RequiresApproval => {
                // Already handled by permission check
            }
            crate::safety::ValidationResult::Allowed => {
                tracing::debug!("Command is allowed by safety validator");
            }
        }

        // Build tool context
        let mut ctx = ToolContext::default();
        if let Some(branch) = ContextProvider::get_git_branch_sync() {
            ctx.git_branch = Some(branch);
        }

        // Execute the tool
        let result = self.tools.execute(&tool_call.name, tool_call.args.clone(), &ctx, config).await?;

        // Format and display result
        let result_text = if result.success {
            &result.output
        } else {
            result.error.as_ref().unwrap_or(&result.output)
        };

        let formatted_output = self.formatter.format_tool_result(&tool_call.name, result_text);
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

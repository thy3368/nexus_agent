mod apply_patch;
mod shell;
mod tool_call_reg;

pub mod types;

use serde_json::Map;
use serde_json::Value;
use std::fmt;
use std::path::PathBuf;

pub use apply_patch::ApplyPatchJsonAdapter;
pub use apply_patch::apply_patch_call_schema;
pub use apply_patch::handle_apply_patch_call;
pub use apply_patch::tool_definition as apply_patch_tool_definition;
pub use shell::ShellCommandJsonAdapter;
pub use shell::handle_shell_command_call;
pub use shell::shell_command_arguments_schema;
pub use shell::shell_command_call_schema;
pub use shell::tool_definition as shell_command_tool_definition;
pub use tool_call_reg::ToolCallRegistry;
pub use tool_call_reg::handle_tool_call;
pub use tool_call_reg::tool_definition;
pub use tool_call_reg::tool_definitions;
pub use types::ToolDefinition;


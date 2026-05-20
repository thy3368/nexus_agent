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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonAdapterError {
    InvalidRequest(String),
}

impl fmt::Display for JsonAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for JsonAdapterError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonToolCallType {
    FunctionCall,
    CustomToolCall,
}

impl JsonToolCallType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FunctionCall => "function_call",
            Self::CustomToolCall => "custom_tool_call",
        }
    }
}

pub trait JsonToolAdapter {
    fn handle_call(&self, request: Value) -> Result<Value, JsonAdapterError>;
}

pub(crate) fn validate_tool_call_envelope<'a>(
    request: &'a Value,
    expected_type: JsonToolCallType,
    expected_tool_name: &str,
    label: &str,
) -> Result<&'a Map<String, Value>, JsonAdapterError> {
    let object = request.as_object().ok_or_else(|| {
        JsonAdapterError::InvalidRequest(format!("{label} JSON call must be an object"))
    })?;

    let request_type = object
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or(expected_type.as_str());
    if request_type != expected_type.as_str() {
        return Err(JsonAdapterError::InvalidRequest(format!(
            "unsupported call type `{request_type}`; expected `{}`",
            expected_type.as_str()
        )));
    }

    let tool_name = object
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(expected_tool_name);
    if tool_name != expected_tool_name {
        return Err(JsonAdapterError::InvalidRequest(format!(
            "unsupported tool `{tool_name}`; expected `{expected_tool_name}`"
        )));
    }

    Ok(object)
}

pub(crate) fn read_workdir(object: &Map<String, Value>) -> Option<PathBuf> {
    object
        .get("workdir")
        .or_else(|| object.get("working_directory"))
        .or_else(|| object.get("cwd"))
        .and_then(Value::as_str)
        .map(PathBuf::from)
}

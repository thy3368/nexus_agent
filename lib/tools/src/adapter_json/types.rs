use std::fmt;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub call_type: String,
    pub schema: Value,
}

impl ToolDefinition {
    pub fn new(name: impl Into<String>, call_type: impl Into<String>, schema: Value) -> Self {
        Self {
            name: name.into(),
            call_type: call_type.into(),
            schema,
        }
    }
}



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


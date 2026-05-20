use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

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

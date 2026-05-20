use crate::tools_adapter_json::JsonAdapterError;
use crate::tools_adapter_json::JsonToolCallType;
use crate::tools_adapter_json::ToolDefinition;
use crate::tools_adapter_json::apply_patch;
use crate::tools_adapter_json::shell;
use serde_json::Value;

#[derive(Clone)]
struct ToolHandlerEntry {
    call_type: JsonToolCallType,
    name: String,
    definition: fn() -> ToolDefinition,
    handle_call: fn(Value) -> Result<Value, JsonAdapterError>,
}

impl ToolHandlerEntry {
    fn new(
        call_type: JsonToolCallType,
        name: impl Into<String>,
        definition: fn() -> ToolDefinition,
        handle_call: fn(Value) -> Result<Value, JsonAdapterError>,
    ) -> Self {
        Self {
            call_type,
            name: name.into(),
            definition,
            handle_call,
        }
    }

    fn matches(&self, request_type: &str, tool_name: &str) -> bool {
        self.call_type.as_str() == request_type && self.name == tool_name
    }
}

fn default_tool_handlers() -> Vec<ToolHandlerEntry> {
    vec![
        ToolHandlerEntry::new(
        JsonToolCallType::FunctionCall,
        "shell_command",
        shell::tool_definition,
        shell::handle_shell_command_call,
        ),
        ToolHandlerEntry::new(
        JsonToolCallType::CustomToolCall,
        "apply_patch",
        apply_patch::tool_definition,
        apply_patch::handle_apply_patch_call,
        ),
    ]
}

#[derive(Clone)]
pub struct ToolCallRegistry {
    handlers: Vec<ToolHandlerEntry>,
}

impl Default for ToolCallRegistry {
    fn default() -> Self {
        Self::new(default_tool_handlers())
    }
}

impl ToolCallRegistry {
    fn new(handlers: Vec<ToolHandlerEntry>) -> Self {
        Self { handlers }
    }

    pub fn add_tool(
        &mut self,
        call_type: JsonToolCallType,
        name: impl Into<String>,
        definition: fn() -> ToolDefinition,
        handle_call: fn(Value) -> Result<Value, JsonAdapterError>,
    ) -> &mut Self {
        self.handlers
            .push(ToolHandlerEntry::new(call_type, name, definition, handle_call));
        self
    }

    pub fn handle_tool_call(&self, request: Value) -> Result<Value, JsonAdapterError> {
        let object = request.as_object().ok_or_else(|| {
            JsonAdapterError::InvalidRequest("JSON tool call must be an object".to_string())
        })?;

        let request_type = object
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| JsonAdapterError::InvalidRequest("missing `type`".to_string()))?;
        let tool_name = object
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| JsonAdapterError::InvalidRequest("missing `name`".to_string()))?;

        for entry in &self.handlers {
            if entry.matches(request_type, tool_name) {
                return (entry.handle_call)(request);
            }
        }

        Err(JsonAdapterError::InvalidRequest(format!(
            "unsupported tool call `{request_type}:{tool_name}`"
        )))
    }

    pub fn tool_definitions(&self) -> Vec<ToolDefinition> {
        self.handlers
            .iter()
            .map(|entry| (entry.definition)())
            .collect()
    }

    pub fn tool_definition(&self, tool_name: &str) -> Option<ToolDefinition> {
        self.tool_definitions()
            .into_iter()
            .find(|definition| definition.name == tool_name)
    }
}

pub fn handle_tool_call(request: Value) -> Result<Value, JsonAdapterError> {
    ToolCallRegistry::default().handle_tool_call(request)
}

pub fn tool_definitions() -> Vec<ToolDefinition> {
    ToolCallRegistry::default().tool_definitions()
}

pub fn tool_definition(tool_name: &str) -> Option<ToolDefinition> {
    ToolCallRegistry::default().tool_definition(tool_name)
}

#[cfg(test)]
mod tests {
    use super::ToolCallRegistry;
    use crate::tools_adapter_json::JsonAdapterError;
    use crate::tools_adapter_json::JsonToolCallType;
    use crate::tools_adapter_json::ToolDefinition;
    use serde_json::json;
    use serde_json::Value;

    #[test]
    fn tool_definitions_expose_consistent_metadata() {
        let definitions = ToolCallRegistry::default().tool_definitions();

        assert_eq!(definitions.len(), 2);
        assert!(definitions.iter().any(|definition| {
            definition.name == "shell_command" && definition.call_type == "function_call"
        }));
        assert!(definitions.iter().any(|definition| {
            definition.name == "apply_patch" && definition.call_type == "custom_tool_call"
        }));
    }

    #[test]
    fn tool_definition_exposes_queryable_schema() {
        let definition = ToolCallRegistry::default()
            .tool_definition("shell_command")
            .expect("shell definition");

        assert_eq!(definition.name, "shell_command");
        assert_eq!(definition.call_type, "function_call");
        assert_eq!(definition.schema["properties"]["name"]["const"], "shell_command");
    }

    #[test]
    fn handle_tool_call_dispatches_shell_calls() {
        let response = ToolCallRegistry::default()
            .handle_tool_call(json!({
                "type": "function_call",
                "name": "shell_command",
                "call_id": "dispatch-shell",
                "arguments": {
                    "command": if cfg!(windows) { "echo dispatch" } else { "printf dispatch" }
                }
            }))
            .expect("dispatch should succeed");

        assert_eq!(response["type"], "function_call_output");
        assert_eq!(response["call_id"], "dispatch-shell");
    }

    #[test]
    fn handle_tool_call_dispatches_apply_patch_calls() {
        let root = std::env::temp_dir().join("json_dispatch_apply_patch");
        std::fs::create_dir_all(&root).expect("create dispatch dir");

        let response = ToolCallRegistry::default()
            .handle_tool_call(json!({
                "type": "custom_tool_call",
                "name": "apply_patch",
                "call_id": "dispatch-patch",
                "input": "*** Begin Patch\n*** Add File: dispatched.txt\n+ok\n*** End Patch\n",
                "workdir": root
            }))
            .expect("dispatch should succeed");

        assert_eq!(response["type"], "custom_tool_call_output");
        assert_eq!(response["call_id"], "dispatch-patch");
        let _ = std::fs::remove_dir_all(
            response["metadata"]["changed_files"][0]
                .as_str()
                .map(std::path::PathBuf::from)
                .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
                .expect("parent dir"),
        );
    }

    #[test]
    fn handle_tool_call_rejects_unknown_tool() {
        let error = ToolCallRegistry::default()
            .handle_tool_call(json!({
                "type": "function_call",
                "name": "unknown_tool",
                "call_id": "dispatch-bad",
                "arguments": {}
            }))
            .expect_err("unknown tool should fail");

        assert_eq!(
            error.to_string(),
            "unsupported tool call `function_call:unknown_tool`"
        );
    }

    #[test]
    fn add_tool_registers_and_dispatches_custom_handler() {
        fn echo_tool_definition() -> ToolDefinition {
            ToolDefinition::new(
                "echo_json",
                "function_call",
                json!({
                    "type": "object",
                    "properties": {
                        "type": { "type": "string", "const": "function_call" },
                        "name": { "type": "string", "const": "echo_json" },
                        "call_id": { "type": "string" },
                        "arguments": {
                            "type": "object",
                            "properties": {
                                "message": { "type": "string" }
                            },
                            "required": ["message"]
                        }
                    },
                    "required": ["type", "name", "call_id", "arguments"]
                }),
            )
        }

        fn handle_echo_tool(request: Value) -> Result<Value, JsonAdapterError> {
            let object = request.as_object().ok_or_else(|| {
                JsonAdapterError::InvalidRequest("JSON tool call must be an object".to_string())
            })?;
            let call_id = object
                .get("call_id")
                .and_then(Value::as_str)
                .ok_or_else(|| JsonAdapterError::InvalidRequest("missing `call_id`".to_string()))?;
            let message = object
                .get("arguments")
                .and_then(Value::as_object)
                .and_then(|arguments| arguments.get("message"))
                .and_then(Value::as_str)
                .ok_or_else(|| JsonAdapterError::InvalidRequest("missing `message`".to_string()))?;

            Ok(json!({
                "type": "function_call_output",
                "call_id": call_id,
                "output": [{
                    "type": "input_text",
                    "text": message
                }],
                "success": true
            }))
        }

        let mut registry = ToolCallRegistry::default();
        registry.add_tool(
            JsonToolCallType::FunctionCall,
            "echo_json",
            echo_tool_definition,
            handle_echo_tool,
        );

        let definition = registry
            .tool_definition("echo_json")
            .expect("custom tool definition");
        assert_eq!(definition.name, "echo_json");

        let response = registry
            .handle_tool_call(json!({
                "type": "function_call",
                "name": "echo_json",
                "call_id": "dispatch-echo",
                "arguments": {
                    "message": "hello"
                }
            }))
            .expect("custom dispatch should succeed");

        assert_eq!(response["type"], "function_call_output");
        assert_eq!(response["call_id"], "dispatch-echo");
        assert_eq!(response["output"][0]["text"], "hello");
    }
}

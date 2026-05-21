use crate::core::apply_patch::parse_patch;
use crate::core::apply_patch::ApplyPatchExecutor;
use crate::core::apply_patch::LocalApplyPatchExecutor;

use crate::adapter_json::ToolDefinition;

use crate::adapter_json::types::{
    validate_tool_call_envelope, JsonAdapterError, JsonToolAdapter, JsonToolCallType,
};
use serde_json::json;
use serde_json::{Map, Value};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default)]
pub struct ApplyPatchJsonAdapter;

impl JsonToolAdapter for ApplyPatchJsonAdapter {
    fn handle_call(&self, request: Value) -> Result<Value, JsonAdapterError> {
        handle_apply_patch_call(request)
    }
}

pub fn tool_definition() -> ToolDefinition {
    ToolDefinition::new(
        "apply_patch",
        JsonToolCallType::CustomToolCall.as_str(),
        apply_patch_call_schema(),
    )
}

pub(crate) fn read_workdir(object: &Map<String, Value>) -> Option<PathBuf> {
    object
        .get("workdir")
        .or_else(|| object.get("working_directory"))
        .or_else(|| object.get("cwd"))
        .and_then(Value::as_str)
        .map(PathBuf::from)
}

pub fn handle_apply_patch_call(request: Value) -> Result<Value, JsonAdapterError> {
    let request = ApplyPatchJsonCall::from_value(request)?;
    let action = parse_patch(&request.input, &request.workdir);

    let response = match action {
        Ok(action) => match LocalApplyPatchExecutor::new().execute(&action) {
            Ok(summary) => json!({
                "type": "custom_tool_call_output",
                "call_id": request.call_id,
                "output": [{
                    "type": "input_text",
                    "text": format_success_message(&summary.changed_paths)
                }],
                "success": true,
                "metadata": {
                    "changed_files": summary.changed_paths
                }
            }),
            Err(error) => json!({
                "type": "custom_tool_call_output",
                "call_id": request.call_id,
                "output": [{
                    "type": "input_text",
                    "text": error.to_string()
                }],
                "success": false
            }),
        },
        Err(error) => json!({
            "type": "custom_tool_call_output",
            "call_id": request.call_id,
            "output": [{
                "type": "input_text",
                "text": error.to_string()
            }],
            "success": false
        }),
    };

    Ok(response)
}

pub fn apply_patch_call_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "type": {
                "type": "string",
                "const": "custom_tool_call",
                "description": "Codex-style JSON tool call type for freeform core."
            },
            "name": {
                "type": "string",
                "const": "apply_patch",
                "description": "Tool name."
            },
            "call_id": {
                "type": "string",
                "description": "Opaque call id supplied by the caller."
            },
            "input": {
                "type": "string",
                "description": "Raw apply_patch text content."
            },
            "workdir": {
                "type": "string",
                "description": "Optional working directory base path for resolving patch file paths. Also supports `working_directory` or `cwd`."
            },
            "working_directory": {
                "type": "string",
                "description": "Alias for `workdir`."
            },
            "cwd": {
                "type": "string",
                "description": "Alias for `workdir`."
            }
        },
        "required": ["type", "name", "call_id", "input"]
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApplyPatchJsonCall {
    call_id: String,
    input: String,
    workdir: PathBuf,
}

impl ApplyPatchJsonCall {
    fn from_value(request: Value) -> Result<Self, JsonAdapterError> {
        let object = validate_tool_call_envelope(
            &request,
            JsonToolCallType::CustomToolCall,
            "apply_patch",
            "apply_patch",
        )?;

        let call_id = object
            .get("call_id")
            .and_then(Value::as_str)
            .ok_or_else(|| JsonAdapterError::InvalidRequest("missing `call_id`".to_string()))?
            .to_string();

        let input = object
            .get("input")
            .and_then(Value::as_str)
            .ok_or_else(|| JsonAdapterError::InvalidRequest("missing `input`".to_string()))?
            .to_string();

        let workdir = read_workdir(object)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        Ok(Self {
            call_id,
            input,
            workdir,
        })
    }
}

fn format_success_message(changed_paths: &[PathBuf]) -> String {
    let mut paths = changed_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    paths.sort();

    match paths.len() {
        0 => "Patch applied successfully with no file changes".to_string(),
        1 => format!("Patch applied successfully to 1 file: {}", paths[0]),
        count => format!(
            "Patch applied successfully to {count} files: {}",
            paths.join(", ")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::apply_patch_call_schema;
    use super::handle_apply_patch_call;
    use crate::adapter_json::types::JsonAdapterError;
    use serde_json::json;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn handle_apply_patch_call_executes_codex_style_custom_tool_call() {
        let root = create_temp_test_dir("json_apply_patch_success");
        let patch = "\
*** Begin Patch
*** Add File: hello.txt
+hello
*** End Patch
";

        let response = handle_apply_patch_call(json!({
            "type": "custom_tool_call",
            "name": "apply_patch",
            "call_id": "call-1",
            "input": patch,
            "workdir": root
        }))
        .expect("adapter should respond");

        assert_eq!(response["type"], "custom_tool_call_output");
        assert_eq!(response["call_id"], "call-1");
        assert_eq!(response["success"], true);
        assert!(
            response["output"][0]["text"]
                .as_str()
                .expect("text output")
                .contains("Patch applied successfully")
        );

        cleanup_temp_test_dir(&root_path_from_json(&response));
    }

    #[test]
    fn handle_apply_patch_call_returns_failure_output_for_invalid_patch() {
        let response = handle_apply_patch_call(json!({
            "type": "custom_tool_call",
            "name": "apply_patch",
            "call_id": "call-2",
            "input": "*** Add File: broken.txt\n+oops\n*** End Patch\n"
        }))
        .expect("adapter should respond");

        assert_eq!(response["type"], "custom_tool_call_output");
        assert_eq!(response["call_id"], "call-2");
        assert_eq!(response["success"], false);
        assert!(
            response["output"][0]["text"]
                .as_str()
                .expect("text output")
                .contains("failed to parse apply_patch")
        );
    }

    #[test]
    fn handle_apply_patch_call_rejects_wrong_tool_name() {
        let error = handle_apply_patch_call(json!({
            "type": "custom_tool_call",
            "name": "not_apply_patch",
            "call_id": "call-3",
            "input": "noop"
        }))
        .expect_err("wrong tool should be rejected");

        assert_eq!(
            error,
            JsonAdapterError::InvalidRequest(
                "unsupported tool `not_apply_patch`; expected `apply_patch`".to_string()
            )
        );
    }

    #[test]
    fn apply_patch_call_schema_describes_codex_custom_tool_call_wrapper() {
        let schema = apply_patch_call_schema();

        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["type"]["const"], "custom_tool_call");
        assert_eq!(schema["properties"]["name"]["const"], "apply_patch");
        assert_eq!(
            schema["required"],
            json!(["type", "name", "call_id", "input"])
        );
    }

    fn root_path_from_json(response: &serde_json::Value) -> PathBuf {
        response["metadata"]["changed_files"][0]
            .as_str()
            .map(PathBuf::from)
            .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
            .expect("changed file parent")
    }

    fn create_temp_test_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}_{nanos}"));
        std::fs::create_dir_all(&path).expect("create temp test dir");
        path
    }

    fn cleanup_temp_test_dir(path: &PathBuf) {
        let _ = std::fs::remove_dir_all(path);
    }
}

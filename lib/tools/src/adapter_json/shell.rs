use crate::core::shell::LocalShellExecutor;
use crate::core::shell::ShellExecutor;
use crate::core::shell::ShellOutput;
use crate::core::shell::ShellProgram;
use crate::core::shell::ShellRequest;

use crate::adapter_json::types::{
    validate_tool_call_envelope, JsonAdapterError, JsonToolAdapter, JsonToolCallType,
};
use crate::adapter_json::ToolDefinition;
use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default)]
pub struct ShellCommandJsonAdapter;

impl JsonToolAdapter for ShellCommandJsonAdapter {
    fn handle_call(&self, request: Value) -> Result<Value, JsonAdapterError> {
        handle_shell_command_call(request)
    }
}

pub fn tool_definition() -> ToolDefinition {
    ToolDefinition::new(
        "shell_command",
        JsonToolCallType::FunctionCall.as_str(),
        shell_command_call_schema(),
    )
}

pub fn handle_shell_command_call(request: Value) -> Result<Value, JsonAdapterError> {
    let request = ShellJsonCall::from_value(request)?;
    let shell_request = request.to_shell_request()?;

    let response = match LocalShellExecutor::new().execute(&shell_request) {
        Ok(output) => {
            let text = format_shell_output_text(&output);
            json!({
                "type": "function_call_output",
                "call_id": request.call_id,
                "output": [{
                    "type": "input_text",
                    "text": text
                }],
                "success": output.success(),
                "metadata": {
                    "stdout": output.stdout,
                    "stderr": output.stderr,
                    "exit_code": output.status_code,
                    "workdir": shell_request.workdir
                }
            })
        }
        Err(error) => json!({
            "type": "function_call_output",
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

pub fn shell_command_arguments_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "command": {
                "type": "string",
                "description": "Shell command to execute."
            },
            "cmd": {
                "type": "string",
                "description": "Alias for `command`."
            },
            "workdir": {
                "type": "string",
                "description": "Working directory for the command. Also supports `working_directory` or `cwd`."
            },
            "working_directory": {
                "type": "string",
                "description": "Alias for `workdir`."
            },
            "cwd": {
                "type": "string",
                "description": "Alias for `workdir`."
            },
            "login": {
                "type": "boolean",
                "description": "Whether to run the command as a login shell."
            },
            "shell": {
                "type": "string",
                "description": "Optional shell program hint, for example `bash`, `zsh`, `sh`, `pwsh`, or `cmd`."
            },
            "env": {
                "type": "object",
                "description": "Optional environment variables to inject into the command.",
                "additionalProperties": {
                    "type": "string"
                }
            }
        },
        "required": ["command"]
    })
}

pub fn shell_command_call_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "type": {
                "type": "string",
                "const": "function_call",
                "description": "Codex-style JSON tool call type for structured core."
            },
            "name": {
                "type": "string",
                "const": "shell_command",
                "description": "Tool name."
            },
            "call_id": {
                "type": "string",
                "description": "Opaque call id supplied by the caller."
            },
            "arguments": {
                "description": "Tool arguments, either as a JSON string or a JSON object.",
                "oneOf": [
                    {
                        "type": "string",
                        "description": "Serialized JSON object matching the shell arguments schema."
                    },
                    shell_command_arguments_schema()
                ]
            }
        },
        "required": ["type", "name", "call_id", "arguments"]
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ShellJsonCall {
    call_id: String,
    arguments: Value,
}

impl ShellJsonCall {
    fn from_value(request: Value) -> Result<Self, JsonAdapterError> {
        let object = validate_tool_call_envelope(
            &request,
            JsonToolCallType::FunctionCall,
            "shell_command",
            "shell",
        )?;

        let call_id = object
            .get("call_id")
            .and_then(Value::as_str)
            .ok_or_else(|| JsonAdapterError::InvalidRequest("missing `call_id`".to_string()))?
            .to_string();

        let arguments = match object.get("arguments") {
            Some(Value::String(arguments)) => serde_json::from_str(arguments).map_err(|error| {
                JsonAdapterError::InvalidRequest(format!(
                    "failed to parse `arguments` JSON: {error}"
                ))
            })?,
            Some(Value::Object(arguments)) => Value::Object(arguments.clone()),
            Some(_) => {
                return Err(JsonAdapterError::InvalidRequest(
                    "`arguments` must be a JSON string or object".to_string(),
                ));
            }
            None => {
                return Err(JsonAdapterError::InvalidRequest(
                    "missing `arguments`".to_string(),
                ));
            }
        };

        Ok(Self { call_id, arguments })
    }

    fn to_shell_request(&self) -> Result<ShellRequest, JsonAdapterError> {
        let object = self.arguments.as_object().ok_or_else(|| {
            JsonAdapterError::InvalidRequest("shell arguments must be an object".to_string())
        })?;

        let command = object
            .get("command")
            .or_else(|| object.get("cmd"))
            .and_then(Value::as_str)
            .ok_or_else(|| JsonAdapterError::InvalidRequest("missing `command`".to_string()))?;

        let mut request = ShellRequest::new(command);

        if let Some(workdir) = read_workdir(object) {
            request = request.with_workdir(workdir);
        }

        if let Some(login) = object.get("login").and_then(Value::as_bool) {
            request = request.with_login(login);
        }

        if let Some(shell) = object
            .get("shell")
            .and_then(Value::as_str)
            .and_then(shell_program_from_hint)
        {
            request = request.with_shell(shell);
        }

        if let Some(env) = object.get("env").and_then(Value::as_object) {
            for (key, value) in env {
                if let Some(value) = value.as_str() {
                    request = request.with_env(key.clone(), value.to_string());
                }
            }
        }

        Ok(request)
    }
}

fn read_workdir(object: &Map<String, Value>) -> Option<PathBuf> {
    object
        .get("workdir")
        .or_else(|| object.get("working_directory"))
        .or_else(|| object.get("cwd"))
        .and_then(Value::as_str)
        .map(PathBuf::from)
}

fn shell_program_from_hint(hint: &str) -> Option<ShellProgram> {
    let shell = hint.rsplit(['/', '\\']).next()?.to_ascii_lowercase();
    match shell.as_str() {
        "sh" => Some(ShellProgram::Sh),
        "bash" => Some(ShellProgram::Bash),
        "zsh" => Some(ShellProgram::Zsh),
        "powershell" | "powershell.exe" | "pwsh" | "pwsh.exe" => Some(ShellProgram::PowerShell),
        "cmd" | "cmd.exe" => Some(ShellProgram::Cmd),
        other if !other.is_empty() => Some(ShellProgram::Custom(other.to_string())),
        _ => None,
    }
}

fn format_shell_output_text(output: &ShellOutput) -> String {
    if !output.stdout.trim().is_empty() {
        output.stdout.clone()
    } else if !output.stderr.trim().is_empty() {
        output.stderr.clone()
    } else if output.success() {
        "Command completed with no output".to_string()
    } else {
        "Command failed with no output".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::handle_shell_command_call;
    use super::shell_command_arguments_schema;
    use super::shell_command_call_schema;
    use crate::adapter_json::types::JsonAdapterError;
    use serde_json::json;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn handle_shell_command_call_executes_codex_style_function_call() {
        let root = create_temp_test_dir("json_shell_success");
        let response = handle_shell_command_call(json!({
            "type": "function_call",
            "name": "shell_command",
            "call_id": "call-shell-1",
            "arguments": serde_json::to_string(&json!({
                "command": "pwd",
                "workdir": root
            })).expect("serialize args")
        }))
        .expect("adapter should respond");

        assert_eq!(response["type"], "function_call_output");
        assert_eq!(response["call_id"], "call-shell-1");
        assert_eq!(response["success"], true);
        assert_eq!(response["metadata"]["workdir"].as_str(), root.to_str());
        assert!(
            response["output"][0]["text"]
                .as_str()
                .expect("text output")
                .contains(root.to_str().expect("root str"))
        );

        cleanup_temp_test_dir(&root);
    }

    #[test]
    fn handle_shell_command_call_supports_object_arguments_and_env() {
        let response = handle_shell_command_call(json!({
            "type": "function_call",
            "name": "shell_command",
            "call_id": "call-shell-2",
            "arguments": {
                "command": if cfg!(windows) { "echo %HELLO_JSON%" } else { "echo $HELLO_JSON" },
                "env": {
                    "HELLO_JSON": "from-adapter"
                }
            }
        }))
        .expect("adapter should respond");

        assert_eq!(response["type"], "function_call_output");
        assert_eq!(response["call_id"], "call-shell-2");
        assert_eq!(response["success"], true);
        assert!(
            response["output"][0]["text"]
                .as_str()
                .expect("text output")
                .contains("from-adapter")
        );
    }

    #[test]
    fn handle_shell_command_call_rejects_wrong_tool_name() {
        let error = handle_shell_command_call(json!({
            "type": "function_call",
            "name": "not_shell_command",
            "call_id": "call-shell-3",
            "arguments": "{}"
        }))
        .expect_err("wrong tool should be rejected");

        assert_eq!(
            error,
            JsonAdapterError::InvalidRequest(
                "unsupported tool `not_shell_command`; expected `shell_command`".to_string()
            )
        );
    }

    #[test]
    fn handle_shell_command_call_rejects_invalid_arguments_json() {
        let error = handle_shell_command_call(json!({
            "type": "function_call",
            "name": "shell_command",
            "call_id": "call-shell-4",
            "arguments": "{not-json"
        }))
        .expect_err("invalid arguments should be rejected");

        assert!(
            error
                .to_string()
                .contains("failed to parse `arguments` JSON")
        );
    }

    #[test]
    fn shell_command_arguments_schema_describes_supported_fields() {
        let schema = shell_command_arguments_schema();

        assert_eq!(schema["type"], "object");
        assert_eq!(schema["required"], json!(["command"]));
        assert!(schema["properties"]["env"]["additionalProperties"].is_object());
        assert_eq!(schema["properties"]["login"]["type"], "boolean");
    }

    #[test]
    fn shell_command_call_schema_describes_codex_function_call_wrapper() {
        let schema = shell_command_call_schema();

        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["type"]["const"], "function_call");
        assert_eq!(schema["properties"]["name"]["const"], "shell_command");
        assert_eq!(
            schema["required"],
            json!(["type", "name", "call_id", "arguments"])
        );
        assert_eq!(
            schema["properties"]["arguments"]["oneOf"][1]["type"],
            "object"
        );
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

//! OpenAI API provider implementation

use crate::error::{ModelError, Result};
use crate::llm::traits::ll_model::{
    LLMInfo, LLMReply, LLMRequest, LLModel, TokenUsage, ToolCall,
};
use crate::tool::traits::tool_definition::ToolDefinition;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const DEFAULT_OPENAI_API_BASE: &str = "https://api.openai.com/v1";
const DEFAULT_OPENAI_MODEL: &str = "gpt-5-mini";

pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    api_base: String,
    model: String,
    temperature: f32,
    max_tokens: usize,
    llm_log_dir: Option<PathBuf>,
}

impl OpenAIProvider {
    pub fn new(
        api_key: String,
        base_url: Option<String>,
        model: Option<String>,
        llm_log_dir: Option<PathBuf>,
    ) -> Self {
        let client = Client::builder()
            .no_proxy()
            .build()
            .expect("failed to build OpenAI HTTP client");

        Self {
            client,
            api_key,
            api_base: base_url.unwrap_or_else(|| DEFAULT_OPENAI_API_BASE.to_string()),
            model: model.unwrap_or_else(|| DEFAULT_OPENAI_MODEL.to_string()),
            temperature: 0.2,
            max_tokens: 4096,
            llm_log_dir,
        }
    }

    pub fn with_params(mut self, temperature: f32, max_tokens: usize) -> Self {
        self.temperature = temperature;
        self.max_tokens = max_tokens;
        self
    }

    fn responses_endpoint(&self) -> String {
        format!("{}/responses", self.api_base.trim_end_matches('/'))
    }

    fn should_send_temperature(&self) -> bool {
        !self.model.starts_with("gpt-5")
    }

    fn build_request_body(
        &self,
        messages: &[LLMRequest],
        available_tools: Option<&[ToolDefinition]>,
    ) -> Value {
        let mut request = json!({
            "model": self.model,
            "input": self.convert_messages(messages),
            "max_output_tokens": self.max_tokens,
        });

        if self.should_send_temperature() {
            request["temperature"] = json!(self.temperature);
        }

        if let Some(available_tools) = Self::convert_available_tools(available_tools) {
            request["tools"] = Value::Array(available_tools);
        }

        request
    }

    fn convert_messages(&self, messages: &[LLMRequest]) -> Vec<Value> {
        messages
            .iter()
            .flat_map(Self::convert_message_items)
            .collect()
    }

    fn convert_message_items(message: &LLMRequest) -> Vec<Value> {
        match message.role.as_str() {
            "tool" => vec![json!({
                "type": "function_call_output",
                "call_id": message.tool_call_id.clone().unwrap_or_default(),
                "output": message.content,
            })],
            "assistant" if message.tool_calls.is_some() => {
                let mut items = Vec::new();
                if !message.content.trim().is_empty() {
                    items.push(Self::message_item(message));
                }
                if let Some(tool_calls) = &message.tool_calls {
                    for tool_call in tool_calls {
                        items.push(json!({
                            "type": "function_call",
                            "call_id": tool_call.id,
                            "name": tool_call.name,
                            "arguments": tool_call.arguments.to_string(),
                        }));
                    }
                }
                items
            }
            _ => vec![Self::message_item(message)],
        }
    }

    fn message_item(message: &LLMRequest) -> Value {
        let mut item = json!({
            "type": "message",
            "role": message.role,
            "content": message.content,
        });

        if let Some(name) = &message.name {
            item["name"] = json!(name);
        }

        item
    }

    fn convert_available_tools(available_tools: Option<&[ToolDefinition]>) -> Option<Vec<Value>> {
        available_tools.map(|items| {
            items
                .iter()
                .map(|tool| {
                    json!({
                        "type": "function",
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters,
                        "strict": true,
                    })
                })
                .collect()
        })
    }

    fn parse_response(&self, response: Value) -> Result<LLMReply> {
        if let Some(error) = response.get("error") {
            return Err(ModelError::Api(error.to_string()).into());
        }

        let output = response
            .get("output")
            .and_then(Value::as_array)
            .ok_or_else(|| ModelError::InvalidResponse("missing `output` array".to_string()))?;

        let content = response
            .get("output_text")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .unwrap_or_else(|| collect_output_text(output));

        let tool_calls = collect_function_calls(output);
        let usage = response
            .get("usage")
            .map(parse_usage)
            .unwrap_or_default();

        Ok(LLMReply {
            content,
            model: response
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or(&self.model)
                .to_string(),
            usage,
            tool_calls,
            finish_reason: response
                .get("status")
                .and_then(Value::as_str)
                .map(ToString::to_string),
        })
    }
}

#[async_trait]
impl LLModel for OpenAIProvider {
    async fn do_chat(
        &self,
        messages: &[LLMRequest],
        available_tools: Option<&[ToolDefinition]>,
    ) -> Result<LLMReply> {
        let request = self.build_request_body(messages, available_tools);

        let response = self
            .client
            .post(self.responses_endpoint())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ModelError::Request(e))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ModelError::Api(format!("OpenAI API error: {}", error_text)).into());
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| ModelError::Request(e))?;

        self.parse_response(response_json)
    }

    fn model_info(&self) -> LLMInfo {
        LLMInfo {
            provider: "openai".to_string(),
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            supports_tools: true,
            supports_streaming: false,
        }
    }

    fn llm_log_dir(&self) -> Option<&Path> {
        self.llm_log_dir.as_deref()
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_streaming(&self) -> bool {
        false
    }
}

fn collect_output_text(output: &[Value]) -> String {
    let mut parts = Vec::new();

    for item in output {
        if item.get("type").and_then(Value::as_str) != Some("message") {
            continue;
        }

        let Some(content) = item.get("content").and_then(Value::as_array) else {
            continue;
        };

        for content_item in content {
            match content_item.get("type").and_then(Value::as_str) {
                Some("output_text") => {
                    if let Some(text) = content_item.get("text").and_then(Value::as_str) {
                        parts.push(text.to_string());
                    }
                }
                Some("refusal") => {
                    if let Some(text) = content_item.get("refusal").and_then(Value::as_str) {
                        parts.push(text.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    parts.join("\n")
}

fn collect_function_calls(output: &[Value]) -> Option<Vec<ToolCall>> {
    let calls = output
        .iter()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("function_call"))
        .map(|item| ToolCall {
            id: item
                .get("call_id")
                .and_then(Value::as_str)
                .or_else(|| item.get("id").and_then(Value::as_str))
                .unwrap_or_default()
                .to_string(),
            name: item
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            arguments: item
                .get("arguments")
                .and_then(Value::as_str)
                .and_then(|arguments| serde_json::from_str(arguments).ok())
                .unwrap_or_else(|| {
                    item.get("arguments")
                        .cloned()
                        .unwrap_or_else(|| Value::String(String::new()))
                }),
        })
        .collect::<Vec<_>>();

    if calls.is_empty() {
        None
    } else {
        Some(calls)
    }
}

fn parse_usage(usage: &Value) -> TokenUsage {
    TokenUsage {
        prompt_tokens: usage
            .get("input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        completion_tokens: usage
            .get("output_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        total_tokens: usage
            .get("total_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_provider_creation() {
        let provider = OpenAIProvider::new(
            "test-key".to_string(),
            None,
            Some("gpt-5-mini".to_string()),
            None,
        );
        let info = provider.model_info();

        assert_eq!(info.provider, "openai");
        assert_eq!(info.model, "gpt-5-mini");
        assert!(info.supports_tools);
    }

    #[test]
    fn test_message_conversion_preserves_tool_outputs() {
        let provider = OpenAIProvider::new("test-key".to_string(), None, None, None);
        let messages = vec![
            LLMRequest::user("Hello"),
            LLMRequest::assistant_with_tool_calls(
                String::new(),
                vec![ToolCall {
                    id: "call-1".to_string(),
                    name: "shell_command".to_string(),
                    arguments: json!({"command": "printf ok"}),
                }],
            ),
            LLMRequest::tool("ok", "call-1", "shell_command"),
        ];

        let converted = provider.convert_messages(&messages);

        assert_eq!(converted[0]["type"], "message");
        assert_eq!(converted[1]["type"], "function_call");
        assert_eq!(converted[1]["call_id"], "call-1");
        assert_eq!(converted[2]["type"], "function_call_output");
        assert_eq!(converted[2]["call_id"], "call-1");
    }

    #[test]
    fn test_parse_response_extracts_text_and_tool_calls() {
        let provider = OpenAIProvider::new("test-key".to_string(), None, None, None);
        let reply = provider
            .parse_response(json!({
                "model": "gpt-5-mini",
                "status": "completed",
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 5,
                    "total_tokens": 15
                },
                "output": [
                    {
                        "type": "function_call",
                        "call_id": "call-1",
                        "name": "shell_command",
                        "arguments": "{\"command\":\"printf ok\"}"
                    },
                    {
                        "type": "message",
                        "role": "assistant",
                        "content": [
                            {
                                "type": "output_text",
                                "text": "Final answer"
                            }
                        ]
                    }
                ]
            }))
            .expect("response should parse");

        assert_eq!(reply.content, "Final answer");
        assert_eq!(reply.usage.total_tokens, 15);
        assert_eq!(reply.tool_calls.expect("tool calls")[0].name, "shell_command");
    }

    #[test]
    fn test_gpt5_requests_skip_temperature() {
        let provider = OpenAIProvider::new(
            "test-key".to_string(),
            None,
            Some("gpt-5-mini".to_string()),
            None,
        );

        assert!(!provider.should_send_temperature());
    }

    #[test]
    fn test_non_gpt5_requests_keep_temperature() {
        let provider = OpenAIProvider::new(
            "test-key".to_string(),
            None,
            Some("gpt-4.1-mini".to_string()),
            None,
        );

        assert!(provider.should_send_temperature());
    }

    #[test]
    fn test_do_chat_with_tool() {
        let provider = OpenAIProvider::new("test-key".to_string(), None, None, None);
        let messages = vec![LLMRequest::user("show current directory")];
        let available_tools = vec![ToolDefinition::new(
            "shell_command",
            "Run a shell command",
            json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string" }
                },
                "required": ["command"]
            }),
        )];

        let body = provider.build_request_body(&messages, Some(&available_tools));
        assert_eq!(body["model"], "gpt-5-mini");
        assert_eq!(body["input"][0]["type"], "message");
        assert_eq!(body["input"][0]["role"], "user");
        assert_eq!(body["input"][0]["content"], "show current directory");
        assert_eq!(body["tools"][0]["type"], "function");
        assert_eq!(body["tools"][0]["name"], "shell_command");
        assert_eq!(body["tools"][0]["strict"], true);
        assert!(body.get("temperature").is_none());

        let reply = provider
            .parse_response(json!({
                "model": "gpt-5-mini",
                "status": "completed",
                "usage": {
                    "input_tokens": 24,
                    "output_tokens": 7,
                    "total_tokens": 31
                },
                "output": [
                    {
                        "type": "function_call",
                        "call_id": "call-1",
                        "name": "shell_command",
                        "arguments": "{\"command\":\"pwd\"}"
                    }
                ]
            }))
            .expect("tool response should parse");

        assert_eq!(reply.model, "gpt-5-mini");
        assert_eq!(reply.usage.total_tokens, 31);
        assert_eq!(reply.content, "");
        assert_eq!(
            reply.tool_calls,
            Some(vec![ToolCall {
                id: "call-1".to_string(),
                name: "shell_command".to_string(),
                arguments: json!({"command": "pwd"}),
            }])
        );
    }
}

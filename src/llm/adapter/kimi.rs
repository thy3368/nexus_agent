use crate::error::{ModelError, Result};
use crate::llm::traits::ll_model::{
    LLMRequest, LLModel, LLMInfo, LLMReply, TokenUsage, ToolCall,
};
use crate::tool::traits::tool_handler::ToolDefinition;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};

pub struct KimiProvider {
    api_key: String,
    model: String,
    temperature: f32,
    max_tokens: usize,
    client: Client,
    llm_log_dir: Option<PathBuf>,
}

impl KimiProvider {
    pub fn new(api_key: String, model: Option<String>, llm_log_dir: Option<PathBuf>) -> Self {
        let client = Client::builder()
            .no_proxy()
            .build()
            .expect("failed to build Kimi HTTP client");

        Self {
            api_key,
            model: model.unwrap_or_else(|| "moonshot-v1".to_string()),
            temperature: 0.3,
            max_tokens: 4096,
            client,
            llm_log_dir,
        }
    }

    pub fn with_params(mut self, temperature: f32, max_tokens: usize) -> Self {
        self.temperature = temperature;
        self.max_tokens = max_tokens;
        self
    }

    async fn request_chat(
        &self,
        messages: &[LLMRequest],
        tools: Option<Vec<KimiTool>>,
    ) -> Result<LLMReply> {
        let url = "https://api.moonshot.cn/v1/chat/completions";

        let kimi_messages: Vec<KimiMessage> = messages
            .iter()
            .map(|msg| KimiMessage {
                role: msg.role.clone(),
                content: Some(msg.content.clone()),
                tool_calls: None,
            })
            .collect();

        let request = KimiRequest {
            model: self.model.clone(),
            messages: kimi_messages,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            tools,
        };

        let response = self
            .client
            .post(url)
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
            return Err(ModelError::Api(format!("Kimi API error: {}", error_text)).into());
        }

        let kimi_resp: KimiResponse = response.json().await.map_err(|e| ModelError::Request(e))?;

        let choice = kimi_resp
            .choices
            .first()
            .ok_or_else(|| ModelError::InvalidResponse("No choices in response".to_string()))?;

        let tool_calls = choice.message.tool_calls.as_ref().map(|calls| {
            calls
                .iter()
                .map(|call| ToolCall {
                    id: call.id.clone(),
                    name: call.function.name.clone(),
                    arguments: serde_json::from_str(&call.function.arguments).unwrap_or_else(
                        |_| json!({"raw_arguments": call.function.arguments.clone()}),
                    ),
                })
                .collect()
        });

        Ok(LLMReply {
            content: choice.message.content.clone().unwrap_or_default(),
            model: kimi_resp.model,
            usage: TokenUsage {
                prompt_tokens: kimi_resp.usage.prompt_tokens,
                completion_tokens: kimi_resp.usage.completion_tokens,
                total_tokens: kimi_resp.usage.total_tokens,
            },
            tool_calls,
            finish_reason: Some(choice.finish_reason.clone()),
        })
    }
}

#[derive(Debug, Serialize)]
struct KimiRequest {
    model: String,
    messages: Vec<KimiMessage>,
    temperature: f32,
    max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<KimiTool>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct KimiTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: KimiToolFunction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct KimiToolFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct KimiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<KimiToolCall>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct KimiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: KimiToolCallFunction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct KimiToolCallFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct KimiResponse {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<KimiChoice>,
    usage: KimiUsage,
}

#[derive(Debug, Deserialize)]
struct KimiChoice {
    index: i32,
    message: KimiMessage,
    finish_reason: String,
}

#[derive(Debug, Deserialize)]
struct KimiUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[async_trait]
impl LLModel for KimiProvider {
    async fn complete(&self, prompt: &str, system_prompt: Option<&str>) -> Result<LLMReply> {
        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(LLMRequest::system(sys));
        }
        messages.push(LLMRequest::user(prompt));
        self.do_chat(&messages, None).await
    }

    async fn do_chat(
        &self,
        messages: &[LLMRequest],
        tools: Option<&[ToolDefinition]>,
    ) -> Result<LLMReply> {
        let kimi_tools = tools.map(|tools| {
            tools
                .iter()
                .map(|tool| KimiTool {
                    tool_type: "function".to_string(),
                    function: KimiToolFunction {
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        parameters: tool.parameters.clone(),
                    },
                })
                .collect()
        });

        self.request_chat(messages, kimi_tools).await
    }

    fn llm_log_dir(&self) -> Option<&Path> {
        self.llm_log_dir.as_deref()
    }

    fn model_info(&self) -> LLMInfo {
        LLMInfo {
            provider: "kimi".to_string(),
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            supports_tools: true,
            supports_streaming: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kimi_provider_creation() {
        let provider = KimiProvider::new(
            "test-key".to_string(),
            Some("moonshot-v1".to_string()),
            None,
        );
        let info = provider.model_info();

        assert_eq!(info.provider, "kimi");
        assert_eq!(info.model, "moonshot-v1");
        assert!(info.supports_tools);
    }

    #[test]
    fn test_kimi_provider_with_params() {
        let provider = KimiProvider::new("test-key".to_string(), None, None).with_params(0.5, 2048);

        assert_eq!(provider.temperature, 0.5);
        assert_eq!(provider.max_tokens, 2048);
    }

    #[test]
    fn test_kimi_provider_default_model() {
        let provider = KimiProvider::new("test-key".to_string(), None, None);
        let info = provider.model_info();

        assert_eq!(info.model, "moonshot-v1");
        assert_eq!(info.max_tokens, 4096);
    }

    #[tokio::test]
    async fn test_kimi_message_conversion() {
        let messages = vec![
            LLMRequest::system("You are a helpful assistant"),
            LLMRequest::user("Hello"),
            LLMRequest::assistant("Hi there!"),
        ];

        let kimi_messages: Vec<KimiMessage> = messages
            .iter()
            .map(|msg| KimiMessage {
                role: msg.role.clone(),
                content: Some(msg.content.clone()),
                tool_calls: None,
            })
            .collect();

        assert_eq!(kimi_messages.len(), 3);
        assert_eq!(kimi_messages[0].role, "system");
        assert_eq!(kimi_messages[1].role, "user");
        assert_eq!(kimi_messages[2].role, "assistant");
    }

    #[tokio::test]
    #[ignore] // Run with: KIMI_API_KEY="your-key" cargo test test_kimi_chat_real -- --ignored --nocapture
    async fn test_kimi_chat_real() {
        let api_key = std::env::var("KIMI_API_KEY")
            .expect("KIMI_API_KEY environment variable not set. Run: KIMI_API_KEY='your-key' cargo test test_kimi_chat_real -- --ignored --nocapture");

        let provider = KimiProvider::new(api_key, Some("moonshot-v1".to_string()), None);

        let messages = vec![LLMRequest::user("你好，请用一句话介绍你自己")];

        let response = provider.do_chat(&messages, None).await;

        match response {
            Ok(resp) => {
                println!("\n✓ Kimi API call successful!");
                println!("  Model: {}", resp.model);
                println!("  Content: {}", resp.content);
                println!(
                    "  Tokens - Prompt: {}, Completion: {}, Total: {}",
                    resp.usage.prompt_tokens, resp.usage.completion_tokens, resp.usage.total_tokens
                );
                println!("  Finish reason: {:?}", resp.finish_reason);
                assert!(!resp.content.is_empty());
            }
            Err(e) => {
                panic!("Kimi API call failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore] // Run with: KIMI_API_KEY="your-key" cargo test test_kimi_complete_real -- --ignored --nocapture
    async fn test_kimi_complete_real() {
        let api_key = std::env::var("KIMI_API_KEY")
            .expect("KIMI_API_KEY environment variable not set. Run: KIMI_API_KEY='your-key' cargo test test_kimi_complete_real -- --ignored --nocapture");

        let provider = KimiProvider::new(api_key, Some("moonshot-v1-8k".to_string()), None);

        let response = provider
            .complete(
                "写一个Rust的Hello World程序",
                Some("你是一个有帮助的编程助手"),
            )
            .await;

        match response {
            Ok(resp) => {
                println!("\n✓ Kimi complete call successful!");
                println!("  Content length: {}", resp.content.len());
                println!(
                    "  First 100 chars: {}",
                    &resp.content[..resp.content.len().min(100)]
                );
                assert!(!resp.content.is_empty());
            }
            Err(e) => {
                panic!("Kimi complete call failed: {:?}", e);
            }
        }
    }
}

use crate::error::{ModelError, Result};
use crate::llm::traits::ll_model::{
    LLMRequest, LLModel, LLMInfo, LLMReply, TokenUsage,
};
use crate::tool::traits::tool_handler::ToolDefinition;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::path::{Path, PathBuf};

pub struct ClaudeProvider {
    api_key: String,
    base_url: String,
    model: String,
    temperature: f32,
    max_tokens: usize,
    client: Client,
    llm_log_dir: Option<PathBuf>,
}

impl ClaudeProvider {
    pub fn new(
        api_key: String,
        base_url: String,
        model: Option<String>,
        llm_log_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            api_key,
            base_url,
            model: model.unwrap_or_else(|| "claude-sonnet-4-6".to_string()),
            temperature: 0.3,
            max_tokens: 4096,
            client: Client::new(),
            llm_log_dir,
        }
    }

    pub fn with_params(mut self, temperature: f32, max_tokens: usize) -> Self {
        self.temperature = temperature;
        self.max_tokens = max_tokens;
        self
    }

    async fn request_chat(&self, messages: &[LLMRequest]) -> Result<LLMReply> {
        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));

        let mut system_messages = Vec::new();
        let mut chat_messages = Vec::new();

        for msg in messages {
            match msg.role.as_str() {
                "system" => system_messages.push(msg.content.clone()),
                "assistant" => chat_messages.push(json!({
                    "role": "assistant",
                    "content": msg.content,
                })),
                _ => chat_messages.push(json!({
                    "role": "user",
                    "content": msg.content,
                })),
            }
        }

        let request_body = json!({
            "model": self.model,
            "system": system_messages.join("\n\n"),
            "messages": chat_messages,
            "temperature": self.temperature,
            "max_tokens": self.max_tokens,
        });

        let response = self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request_body)
            .send()
            .await
            .map_err(ModelError::Request)?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ModelError::Api(format!("Claude API error: {}", error_text)).into());
        }

        let response_json: serde_json::Value = response.json().await.map_err(ModelError::Request)?;
        let text = response_json["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|block| {
                (block["type"].as_str() == Some("text"))
                    .then(|| block["text"].as_str().unwrap_or_default())
            })
            .collect::<Vec<_>>()
            .join("\n");

        let usage = TokenUsage {
            prompt_tokens: response_json["usage"]["input_tokens"].as_u64().unwrap_or(0) as usize,
            completion_tokens: response_json["usage"]["output_tokens"].as_u64().unwrap_or(0)
                as usize,
            total_tokens: response_json["usage"]["input_tokens"].as_u64().unwrap_or(0) as usize
                + response_json["usage"]["output_tokens"].as_u64().unwrap_or(0) as usize,
        };

        Ok(LLMReply {
            content: text,
            model: response_json["model"]
                .as_str()
                .unwrap_or(&self.model)
                .to_string(),
            usage,
            tool_calls: None,
            finish_reason: response_json["stop_reason"].as_str().map(|s| s.to_string()),
        })
    }
}

#[async_trait]
impl LLModel for ClaudeProvider {
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
        _tools: Option<&[ToolDefinition]>,
    ) -> Result<LLMReply> {
        self.request_chat(messages).await
    }

    fn llm_log_dir(&self) -> Option<&Path> {
        self.llm_log_dir.as_deref()
    }

    fn model_info(&self) -> LLMInfo {
        LLMInfo {
            provider: "claude".to_string(),
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            supports_tools: false,
            supports_streaming: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_provider_creation() {
        let provider = ClaudeProvider::new(
            "test-key".to_string(),
            "https://api.anthropic.com".to_string(),
            Some("claude-sonnet-4-6".to_string()),
            None,
        );
        let info = provider.model_info();

        assert_eq!(info.provider, "claude");
        assert_eq!(info.model, "claude-sonnet-4-6");
        assert_eq!(info.max_tokens, 4096);
    }

    #[test]
    fn test_claude_provider_with_params() {
        let provider = ClaudeProvider::new(
            "test-key".to_string(),
            "https://api.anthropic.com".to_string(),
            None,
            None,
        )
        .with_params(0.5, 2048);

        assert_eq!(provider.temperature, 0.5);
        assert_eq!(provider.max_tokens, 2048);
    }
}

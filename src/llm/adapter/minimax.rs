use crate::error::{ModelError, Result};
use crate::llm::traits::ll_model::{
    LLMRequest, LLModel, LLMInfo, LLMReply, TokenUsage,
};
use crate::tool::traits::tool_handler::ToolDefinition;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::path::{Path, PathBuf};

pub struct MinimaxProvider {
    api_key: String,
    model: String,
    temperature: f32,
    max_tokens: usize,
    client: Client,
    llm_log_dir: Option<PathBuf>,
}

impl MinimaxProvider {
    pub fn new(api_key: String, model: Option<String>, llm_log_dir: Option<PathBuf>) -> Self {
        Self {
            api_key,
            model: model.unwrap_or_else(|| "MiniMax-M1-80k".to_string()),
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
        let url = "https://api.minimax.chat/v1/text/chatcompletion_v2";
        let minimax_messages: Vec<_> = messages
            .iter()
            .map(|msg| {
                json!({
                    "sender_type": match msg.role.as_str() {
                        "assistant" => "BOT",
                        _ => "USER",
                    },
                    "text": msg.content,
                })
            })
            .collect();

        let request_body = json!({
            "model": self.model,
            "messages": minimax_messages,
            "temperature": self.temperature,
            "max_tokens": self.max_tokens,
        });

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await
            .map_err(ModelError::Request)?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ModelError::Api(format!("MiniMax API error: {}", error_text)).into());
        }

        let response_json: serde_json::Value = response.json().await.map_err(ModelError::Request)?;
        let choice = response_json["choices"]
            .as_array()
            .and_then(|choices| choices.first())
            .ok_or_else(|| ModelError::InvalidResponse("No choices in response".to_string()))?;

        let content = choice["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        let usage = response_json
            .get("usage")
            .map(|usage| TokenUsage {
                prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as usize,
                completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as usize,
                total_tokens: usage["total_tokens"].as_u64().unwrap_or(0) as usize,
            })
            .unwrap_or_default();

        Ok(LLMReply {
            content,
            model: self.model.clone(),
            usage,
            tool_calls: None,
            finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
        })
    }
}

#[async_trait]
impl LLModel for MinimaxProvider {
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
            provider: "minimax".to_string(),
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
    fn test_minimax_provider_creation() {
        let provider = MinimaxProvider::new(
            "test-key".to_string(),
            Some("MiniMax-M1-80k".to_string()),
            None,
        );
        let info = provider.model_info();

        assert_eq!(info.provider, "minimax");
        assert_eq!(info.model, "MiniMax-M1-80k");
        assert_eq!(info.max_tokens, 4096);
    }

    #[test]
    fn test_minimax_provider_with_params() {
        let provider = MinimaxProvider::new("test-key".to_string(), None, None).with_params(0.5, 2048);

        assert_eq!(provider.temperature, 0.5);
        assert_eq!(provider.max_tokens, 2048);
    }
}

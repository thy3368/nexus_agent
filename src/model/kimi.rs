use async_trait::async_trait;
use crate::error::{Result, ModelError};
use crate::model::{LanguageModel, ModelResponse, MMessage, ModelInfo, ToolDefinition, TokenUsage};
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct KimiProvider {
    api_key: String,
    model: String,
    temperature: f32,
    max_tokens: usize,
    client: Client,
}

impl KimiProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            api_key,
            model: model.unwrap_or_else(|| "moonshot-v1".to_string()),
            temperature: 0.3,
            max_tokens: 4096,
            client: Client::new(),
        }
    }

    pub fn with_params(mut self, temperature: f32, max_tokens: usize) -> Self {
        self.temperature = temperature;
        self.max_tokens = max_tokens;
        self
    }
}

#[derive(Debug, Serialize)]
struct KimiRequest {
    model: String,
    messages: Vec<KimiMessage>,
    temperature: f32,
    max_tokens: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct KimiMessage {
    role: String,
    content: String,
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
impl LanguageModel for KimiProvider {
    async fn complete(&self, prompt: &str, system_prompt: Option<&str>) -> Result<ModelResponse> {
        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(MMessage::system(sys));
        }
        messages.push(MMessage::user(prompt));
        self.chat(&messages).await
    }

    async fn chat(&self, messages: &[MMessage]) -> Result<ModelResponse> {
        let url = "https://api.moonshot.cn/v1/chat/completions";

        let kimi_messages: Vec<KimiMessage> = messages
            .iter()
            .map(|msg| KimiMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            })
            .collect();

        let request = KimiRequest {
            model: self.model.clone(),
            messages: kimi_messages,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        };

        tracing::info!(
            "Kimi Chat: URL={}, Model={}, Messages={}",
            url,
            self.model,
            messages.len()
        );

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

        let kimi_resp: KimiResponse = response
            .json()
            .await
            .map_err(|e| ModelError::Request(e))?;

        let choice = kimi_resp
            .choices
            .first()
            .ok_or_else(|| ModelError::InvalidResponse("No choices in response".to_string()))?;

        Ok(ModelResponse {
            content: choice.message.content.clone(),
            model: kimi_resp.model,
            usage: TokenUsage {
                prompt_tokens: kimi_resp.usage.prompt_tokens,
                completion_tokens: kimi_resp.usage.completion_tokens,
                total_tokens: kimi_resp.usage.total_tokens,
            },
            tool_calls: None,
            finish_reason: Some(choice.finish_reason.clone()),
        })
    }

    async fn chat_with_tools(
        &self,
        messages: &[MMessage],
        _tools: &[ToolDefinition],
    ) -> Result<ModelResponse> {
        // For MVP, use regular chat without tool support
        self.chat(messages).await
    }

    fn model_info(&self) -> ModelInfo {
        ModelInfo {
            provider: "kimi".to_string(),
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
    fn test_kimi_provider_creation() {
        let provider = KimiProvider::new("test-key".to_string(), Some("moonshot-v1".to_string()));
        let info = provider.model_info();

        assert_eq!(info.provider, "kimi");
        assert_eq!(info.model, "moonshot-v1");
        assert!(!info.supports_tools);
    }

    #[test]
    fn test_kimi_provider_with_params() {
        let provider = KimiProvider::new("test-key".to_string(), None)
            .with_params(0.5, 2048);

        assert_eq!(provider.temperature, 0.5);
        assert_eq!(provider.max_tokens, 2048);
    }

    #[test]
    fn test_kimi_provider_default_model() {
        let provider = KimiProvider::new("test-key".to_string(), None);
        let info = provider.model_info();

        assert_eq!(info.model, "moonshot-v1");
        assert_eq!(info.max_tokens, 4096);
    }

    #[tokio::test]
    async fn test_kimi_message_conversion() {
        let messages = vec![
            MMessage::system("You are a helpful assistant"),
            MMessage::user("Hello"),
            MMessage::assistant("Hi there!"),
        ];

        let kimi_messages: Vec<KimiMessage> = messages
            .iter()
            .map(|msg| KimiMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
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

        let provider = KimiProvider::new(api_key, Some("moonshot-v1".to_string()));

        let messages = vec![
            MMessage::user("你好，请用一句话介绍你自己"),
        ];

        let response = provider.chat(&messages).await;

        match response {
            Ok(resp) => {
                println!("\n✓ Kimi API call successful!");
                println!("  Model: {}", resp.model);
                println!("  Content: {}", resp.content);
                println!("  Tokens - Prompt: {}, Completion: {}, Total: {}",
                    resp.usage.prompt_tokens,
                    resp.usage.completion_tokens,
                    resp.usage.total_tokens
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

        let provider = KimiProvider::new(api_key, Some("moonshot-v1-8k".to_string()));

        let response = provider.complete(
            "写一个Rust的Hello World程序",
            Some("你是一个有帮助的编程助手")
        ).await;

        match response {
            Ok(resp) => {
                println!("\n✓ Kimi complete call successful!");
                println!("  Content length: {}", resp.content.len());
                println!("  First 100 chars: {}", &resp.content[..resp.content.len().min(100)]);
                assert!(!resp.content.is_empty());
            }
            Err(e) => {
                panic!("Kimi complete call failed: {:?}", e);
            }
        }
    }
}

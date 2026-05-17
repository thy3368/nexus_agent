// use crate::model::{ModelInfo, ToolDefinition};
use crate::tool::traits::tool_handler::ToolDefinition;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Model information
#[derive(Debug, Clone)]
pub struct LLmInfo {
    pub provider: String,
    pub model: String,
    pub max_tokens: usize,
    pub supports_tools: bool,
    pub supports_streaming: bool,
}

/// Message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub role: String,
    pub content: String,
}

impl LLMRequest {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// Token usage information
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// Tool call from model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Model response
#[derive(Debug, Clone)]
pub struct LLMReply {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub finish_reason: Option<String>,
}

/// Language model provider trait
#[async_trait]
pub trait LLModel: Send + Sync {
    /// Generate a completion for a prompt
    async fn complete(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
    ) -> crate::Result<LLMReply>;

    /// Generate a chat completion
    async fn do_chat(&self, messages: &[LLMRequest]) -> crate::Result<LLMReply>;

    async fn chat(&self, messages: &[LLMRequest]) -> crate::Result<LLMReply> {
        tracing::debug!(
            "\n[LLM CHAT] === Input Messages (count: {}) ===",
            messages.len()
        );
        for (i, msg) in messages.iter().enumerate() {
            tracing::debug!("[LLM CHAT] Message[{}] role={}", i, msg.role);
            tracing::debug!("[LLM CHAT] Message[{}] content:\n{}", i, msg.content);
        }
        tracing::debug!("[LLM CHAT] === End Input ===\n");

        //todo 我想把 每一次  request/reply 打印到一个单独的log文件中

        let result = self.do_chat(messages).await;

        match &result {
            Ok(reply) => {
                tracing::debug!("\n[LLM CHAT] === Output (model: {}) ===", reply.model);
                tracing::debug!(
                    "[LLM CHAT] content (len={}):\n---START---\n{}\n---END---",
                    reply.content.len(),
                    reply.content
                );
                if let Some(tool_calls) = &reply.tool_calls {
                    tracing::debug!("[LLM CHAT] tool_calls: {:?}", tool_calls);
                }
                tracing::debug!("[LLM CHAT] === End Output ===\n");
            }
            Err(e) => {
                tracing::error!("[LLM CHAT] === Error: {} ===", e);
            }
        }

        result
    }
    /// Generate a chat completion with tool support
    async fn chat_with_tools(
        &self,
        messages: &[LLMRequest],
        tools: &[ToolDefinition],
    ) -> crate::Result<LLMReply>;

    /// Get model information
    fn model_info(&self) -> LLmInfo;

    /// Estimate token count for text
    fn estimate_tokens(&self, text: &str) -> usize {
        // Rough estimate: 1 token ≈ 4 characters
        (text.len() + 3) / 4
    }

    /// Check if model supports tool calling
    fn supports_tools(&self) -> bool {
        false
    }

    /// Check if model supports streaming
    fn supports_streaming(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = LLMRequest::user("Hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");

        let sys = LLMRequest::system("System prompt");
        assert_eq!(sys.role, "system");
    }
}

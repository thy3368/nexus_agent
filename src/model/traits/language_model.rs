use crate::model::{ModelInfo, ToolDefinition};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
}

impl AgentMessage {
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
pub struct ModelReply {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub finish_reason: Option<String>,
}

/// Language model provider trait
#[async_trait]
pub trait LanguageModel: Send + Sync {
    /// Generate a completion for a prompt
    async fn complete(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
    ) -> crate::Result<ModelReply>;

    /// Generate a chat completion
    async fn chat(&self, messages: &[AgentMessage]) -> crate::Result<ModelReply>;

    /// Generate a chat completion with tool support
    async fn chat_with_tools(
        &self,
        messages: &[AgentMessage],
        tools: &[ToolDefinition],
    ) -> crate::Result<ModelReply>;

    /// Get model information
    fn model_info(&self) -> ModelInfo;

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
        let msg = AgentMessage::user("Hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");

        let sys = AgentMessage::system("System prompt");
        assert_eq!(sys.role, "system");
    }
}

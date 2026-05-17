// use crate::model::{ModelInfo, ToolDefinition};
use crate::tool::traits::tool_handler::ToolDefinition;
use async_trait::async_trait;
use chrono::Utc;
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};

/// Model information
#[derive(Debug, Clone)]
pub struct LLMInfo {
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
#[derive(Debug, Clone, Default, Serialize)]
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

fn sanitize_log_segment(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '-',
        })
        .collect();

    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

fn build_log_file_prefix(info: &LLMInfo) -> String {
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    let provider = sanitize_log_segment(&info.provider);
    let model = sanitize_log_segment(&info.model);
    let suffix: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();

    format!("{}-{}-{}-{}", timestamp, provider, model, suffix)
}

fn log_file_path(dir: &Path, prefix: &str, kind: &str) -> PathBuf {
    dir.join(format!("{}-{}.json", prefix, kind))
}

async fn write_llm_log(path: PathBuf, payload: serde_json::Value) -> crate::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let content = serde_json::to_string_pretty(&payload)?;
    tokio::fs::write(path, content).await?;
    Ok(())
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
    async fn do_chat(
        &self,
        messages: &[LLMRequest],
        tools: Option<&[ToolDefinition]>,
    ) -> crate::Result<LLMReply>;

    fn llm_log_dir(&self) -> Option<&Path> {
        None
    }

    async fn chat(
        &self,
        messages: &[LLMRequest],
        tools: Option<&[ToolDefinition]>,
        session_id: Option<&str>,
    ) -> crate::Result<LLMReply> {
        let sid = session_id.unwrap_or("N/A");
        tracing::debug!(
            "\n[LLM CHAT] [{}] === Input Messages (count: {}) ===",
            sid,
            messages.len()
        );

        for (i, msg) in messages.iter().enumerate() {
            tracing::debug!("[LLM CHAT] [{}] Message[{}] role={}", sid, i, msg.role);
            tracing::debug!("[LLM CHAT] [{}] Message[{}] content:\n{}", sid, i, msg.content);
        }
        if let Some(tools) = tools {
            let tool_names: Vec<_> = tools.iter().map(|tool| tool.name.as_str()).collect();
            tracing::debug!("[LLM CHAT] [{}] tools (count={}): {:?}", sid, tools.len(), tool_names);
        }
        tracing::debug!("[LLM CHAT] [{}] === End Input ===\n", sid);

        let model_info = self.model_info();
        let log_prefix = self.llm_log_dir().map(|_| build_log_file_prefix(&model_info));

        if let (Some(log_dir), Some(prefix)) = (self.llm_log_dir(), log_prefix.as_deref()) {
            let request_path = log_file_path(log_dir, prefix, "request");
            let request_payload = json!({
                "timestamp": Utc::now().to_rfc3339(),
                "provider": model_info.provider,
                "model": model_info.model,
                "message_count": messages.len(),
                "messages": messages,
                "tool_count": tools.map_or(0, |items| items.len()),
                "tool_names": tools.map(|items| items.iter().map(|tool| tool.name.clone()).collect::<Vec<_>>()).unwrap_or_default(),
            });

            if let Err(err) = write_llm_log(request_path.clone(), request_payload).await {
                tracing::warn!(path = %request_path.display(), error = %err, "Failed to write LLM request log");
            }
        }

        let result = self.do_chat(messages, tools).await;

        match &result {
            Ok(reply) => {
                tracing::debug!("\n[LLM CHAT] [{}] === Output (model: {}) ===", sid, reply.model);
                tracing::debug!(
                    "[LLM CHAT] [{}] content (len={}):\n---START---\n{}\n---END---",
                    sid,
                    reply.content.len(),
                    reply.content
                );
                if let Some(tool_calls) = &reply.tool_calls {
                    tracing::debug!("[LLM CHAT] [{}] tool_calls: {:?}", sid, tool_calls);
                }
                tracing::debug!("[LLM CHAT] [{}] === End Output ===\n", sid);

                if let (Some(log_dir), Some(prefix)) = (self.llm_log_dir(), log_prefix.as_deref()) {
                    let reply_path = log_file_path(log_dir, prefix, "reply");
                    let reply_payload = json!({
                        "timestamp": Utc::now().to_rfc3339(),
                        "provider": model_info.provider,
                        "model": reply.model,
                        "content": reply.content,
                        "usage": reply.usage,
                        "tool_calls": reply.tool_calls,
                        "finish_reason": reply.finish_reason,
                    });

                    if let Err(err) = write_llm_log(reply_path.clone(), reply_payload).await {
                        tracing::warn!(path = %reply_path.display(), error = %err, "Failed to write LLM reply log");
                    }
                }
            }
            Err(e) => {
                tracing::error!("[LLM CHAT] [{}] === Error: {} ===", sid, e);

                if let (Some(log_dir), Some(prefix)) = (self.llm_log_dir(), log_prefix.as_deref()) {
                    let error_path = log_file_path(log_dir, prefix, "error");
                    let error_payload = json!({
                        "timestamp": Utc::now().to_rfc3339(),
                        "provider": model_info.provider,
                        "model": model_info.model,
                        "error": e.to_string(),
                    });

                    if let Err(err) = write_llm_log(error_path.clone(), error_payload).await {
                        tracing::warn!(path = %error_path.display(), error = %err, "Failed to write LLM error log");
                    }
                }
            }
        }

        result
    }

    /// Get model information
    fn model_info(&self) -> LLMInfo;

    /// Estimate token count for text
    fn estimate_tokens(&self, text: &str) -> usize {
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

// use crate::model::{ModelInfo, ToolDefinition};
use crate::tool::traits::tool_definition::ToolDefinition;
use crate::util::json_log::{build_log_prefix, log_file_path, write_json_log};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;

#[derive(Debug, Clone)]
/// LLM 提供方与模型能力信息。
///
/// 这类元数据主要用于：
/// - 运行时能力判断，例如是否支持 tools / streaming
/// - 日志落盘时记录 provider / model
/// - 上层根据 token 上限做策略选择
pub struct LLMInfo {
    /// 模型提供方标识，例如 `openai`、`kimi`、`claude`。
    pub provider: String,
    /// 具体模型名，例如 `gpt-4.1`、`moonshot-v1-8k`。
    pub model: String,
    /// 单次请求允许的最大输出 token 上限。
    pub max_tokens: usize,
    /// 该模型是否支持原生 tool calling。
    pub supports_tools: bool,
    /// 该模型是否支持流式输出。
    pub supports_streaming: bool,
}

/// 一条发送给或来自 LLM 的对话消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    /// 消息角色，通常为 `system`、`user` 或 `assistant`。
    pub role: String,
    /// 消息正文。
    pub content: String,
    /// 发送方名称；部分 provider 对 `tool` 等消息支持该字段。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 当角色为 `tool` 时，对应的 tool call id。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// 当 assistant 返回原生 tool calls 时，记录其结构化信息。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl LLMRequest {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn assistant_with_tool_calls(
        content: impl Into<String>,
        tool_calls: Vec<ToolCall>,
    ) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: Some(tool_calls),
        }
    }

    pub fn tool(
        content: impl Into<String>,
        tool_call_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            role: "tool".to_string(),
            content: content.into(),
            name: Some(name.into()),
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: None,
        }
    }
}

/// 一次 LLM 调用的 token 使用统计。
#[derive(Debug, Clone, Default, Serialize)]
pub struct TokenUsage {
    /// 输入消息消耗的 token 数。
    pub prompt_tokens: usize,
    /// 模型输出消耗的 token 数。
    pub completion_tokens: usize,
    /// 总 token 数，通常等于 prompt + completion。
    pub total_tokens: usize,
}

/// 模型返回的一次工具调用请求。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    /// 模型生成的 tool call 唯一标识。
    pub id: String,
    /// 要调用的工具名。
    pub name: String,
    /// 工具参数，保留为 JSON 以兼容不同 provider 的结构。
    pub arguments: serde_json::Value,
}

/// 一次 LLM 调用的标准化响应。
#[derive(Debug, Clone)]
pub struct LLMReply {
    /// 模型返回的文本内容。
    pub content: String,
    /// 实际参与响应的模型名。
    pub model: String,
    /// 该次请求的 token 统计。
    pub usage: TokenUsage,
    /// 若模型触发了工具调用，则记录其标准化结果。
    pub tool_calls: Option<Vec<ToolCall>>,
    /// 提供方返回的停止原因，例如 `stop`、`tool_calls`。
    pub finish_reason: Option<String>,
}

/// Language model provider trait
#[async_trait]
pub trait LLModel: Send + Sync {
    /// Generate a chat completion
    async fn do_chat(
        &self,
        messages: &[LLMRequest],
        available_tools: Option<&[ToolDefinition]>,
    ) -> crate::Result<LLMReply>;

    fn llm_log_dir(&self) -> Option<&Path> {
        None
    }

    async fn chat(
        &self,
        messages: &[LLMRequest],
        available_tools: Option<&[ToolDefinition]>,
        session_id: &str,
    ) -> crate::Result<LLMReply> {
        let sid = session_id;
        tracing::debug!(
            "\n[LLM CHAT] [{}] === Input Messages (count: {}) ===",
            sid,
            messages.len()
        );

        for (i, msg) in messages.iter().enumerate() {
            tracing::debug!("[LLM CHAT] [{}] Message[{}] role={}", sid, i, msg.role);
            tracing::debug!(
                "[LLM CHAT] [{}] Message[{}] content:\n{}",
                sid,
                i,
                msg.content
            );
        }
        if let Some(available_tools) = available_tools {
            let tool_names: Vec<_> = available_tools
                .iter()
                .map(|tool| tool.name.as_str())
                .collect();
            tracing::debug!(
                "[LLM CHAT] [{}] core (count={}): {:?}",
                sid,
                available_tools.len(),
                tool_names
            );
        }
        tracing::debug!("[LLM CHAT] [{}] === End Input ===\n", sid);

        let model_info = self.model_info();
        let log_prefix = self
            .llm_log_dir()
            .map(|_| build_log_prefix(&model_info.provider, &model_info.model, session_id));

        if let (Some(log_dir), Some(prefix)) = (self.llm_log_dir(), log_prefix.as_deref()) {
            let request_path = log_file_path(log_dir, prefix, "request");
            let request_payload = json!({
                "timestamp": Utc::now().to_rfc3339(),
                "provider": model_info.provider,
                "model": model_info.model,
                "message_count": messages.len(),
                "messages": messages,
                "tool_count": available_tools.map_or(0, |items| items.len()),
                "tool_names": available_tools.map(|items| items.iter().map(|tool| tool.name.clone()).collect::<Vec<_>>()).unwrap_or_default(),
            });

            if let Err(err) = write_json_log(request_path.clone(), request_payload).await {
                tracing::warn!(path = %request_path.display(), error = %err, "Failed to write LLM request log");
            }
        }

        let result = self.do_chat(messages, available_tools).await;

        match &result {
            Ok(reply) => {
                tracing::debug!(
                    "\n[LLM CHAT] [{}] === Output (model: {}) ===",
                    sid,
                    reply.model
                );
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

                    if let Err(err) = write_json_log(reply_path.clone(), reply_payload).await {
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

                    if let Err(err) = write_json_log(error_path.clone(), error_payload).await {
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
        assert!(msg.tool_calls.is_none());

        let sys = LLMRequest::system("System prompt");
        assert_eq!(sys.role, "system");

        let tool_call = ToolCall {
            id: "call-1".to_string(),
            name: "shell_command".to_string(),
            arguments: serde_json::json!({"command": "printf ok"}),
        };
        let assistant = LLMRequest::assistant_with_tool_calls("", vec![tool_call.clone()]);
        assert_eq!(assistant.role, "assistant");
        assert_eq!(assistant.tool_calls, Some(vec![tool_call.clone()]));

        let tool = LLMRequest::tool("ok", "call-1", "shell_command");
        assert_eq!(tool.role, "tool");
        assert_eq!(tool.tool_call_id.as_deref(), Some("call-1"));
        assert_eq!(tool.name.as_deref(), Some("shell_command"));
    }
}

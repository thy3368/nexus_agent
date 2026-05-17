use crate::llm::traits::ll_model::LLMRequest;
use serde::{Deserialize, Serialize};

/// Agent execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    /// 任务是否成功完成。
    pub success: bool,
    /// Agent 最终输出的内容。
    pub output: String,
    /// ReACT 循环执行的次数。
    pub iterations: usize,
    /// 执行过程中调用的工具列表。
    pub tool_calls: Vec<String>,
}

/// Agent behavior trait - defines Use Case layer interface
#[async_trait::async_trait]
pub trait Agent: Send + Sync {
    /// Execute a task using ReACT loop
    async fn execute_task(&mut self, task: String) -> crate::Result<AgentResult>;

    /// Get conversation history (read-only)
    fn get_conversation_history(&self) -> &[LLMRequest];

    /// Clear conversation history
    fn clear_conversation_history(&mut self);

    /// Add message to conversation history
    fn add_to_history(&mut self, message: LLMRequest);
}

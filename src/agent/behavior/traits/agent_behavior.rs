use serde::{Deserialize, Serialize};
use crate::model::traits::language_model::AgentMessage;

/// Agent execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub success: bool,
    pub output: String,
    pub iterations: usize,
    pub tool_calls: Vec<String>,
}

/// Agent behavior trait - defines Use Case layer interface
#[async_trait::async_trait]
pub trait AgentBehavior: Send + Sync {
    /// Execute a task using ReACT loop
    async fn execute_task(&mut self, task: String) -> crate::Result<AgentResult>;

    /// Format response for display
    fn format_response(&self, content: &str) -> String;

    /// Get conversation history (read-only)
    fn get_conversation_history(&self) -> &[AgentMessage];

    /// Clear conversation history
    fn clear_conversation_history(&mut self);

    /// Add message to conversation history
    fn add_to_history(&mut self, message: AgentMessage);
}

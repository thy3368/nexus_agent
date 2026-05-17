use crate::config::Config;
use crate::formatter::ResponseFormatter;
use crate::llm::traits::ll_model::{LLMRequest, LLModel};
use crate::tool::traits::prompt_builder::SystemPromptBuilder;
use crate::tool::traits::tool_executor::ToolExecutor;
use kameo::Actor;

#[derive(Actor)]
pub struct AgentRalph {
    model: Box<dyn LLModel>,
    tool_executor: ToolExecutor,
    prompt_builder: SystemPromptBuilder,
    config: Config,
    formatter: ResponseFormatter,
    iteration_count: usize,
    conversation_history: Vec<LLMRequest>,
}

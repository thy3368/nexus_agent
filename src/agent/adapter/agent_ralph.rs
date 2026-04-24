use crate::config::Config;
use crate::formatter::ResponseFormatter;
use crate::llm::traits::language_model::{AgentMessage, LanguageModel};
use crate::tool::traits::prompt_builder::SystemPromptBuilder;
use crate::tool::traits::tool_executor::ToolExecutor;
use kameo::Actor;

#[derive(Actor)]
pub struct AgentRalph {
    model: Box<dyn LanguageModel>,
    tool_executor: ToolExecutor,
    prompt_builder: SystemPromptBuilder,
    config: Config,
    formatter: ResponseFormatter,
    iteration_count: usize,
    conversation_history: Vec<AgentMessage>,
}

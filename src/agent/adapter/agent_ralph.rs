use crate::agent::behavior::prompt_builder::SystemPromptBuilder;
use crate::agent::behavior::tool_executor::ToolExecutor;
use crate::formatter::ResponseFormatter;
use kameo::Actor;
use crate::config::Config;
use crate::llm::traits::language_model::{AgentMessage, LanguageModel};

#[derive(Actor)]
pub struct AgentBehaviorRalph {
    model: Box<dyn LanguageModel>,
    tool_executor: ToolExecutor,
    prompt_builder: SystemPromptBuilder,
    config: Config,
    formatter: ResponseFormatter,
    iteration_count: usize,
    conversation_history: Vec<AgentMessage>,
}

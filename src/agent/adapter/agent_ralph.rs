use crate::config::Config;
use crate::context::agent_context::AgentContext;
use crate::formatter::ResponseFormatter;
use crate::llm::traits::ll_model::LLModel;
use crate::tool::traits::tool_executor::ToolExecutor;
use kameo::Actor;

#[derive(Actor)]
pub struct AgentRalph {
    model: Box<dyn LLModel>,
    tool_executor: ToolExecutor,
    context: AgentContext,
    config: Config,
    formatter: ResponseFormatter,
    iteration_count: usize,
}

use std::sync::Arc;
use crate::agent::mode::AgentMode;
use crate::config::Config;
use crate::context::prompt_builder::SystemPromptBuilder;
use crate::error::Result;
use crate::llm::traits::ll_model::LLMRequest;
use crate::skill::model::SkillPromptContext;
use crate::skill::SkillManager;
use crate::tool::tool_registry::ToolRegistry;
use crate::tool::traits::tool_executor::ToolExecutor;

pub struct AgentContext {



  /// todo   skill_manager: Option<Arc<SkillManager>>,

    prompt_builder: SystemPromptBuilder,

    /// 流水账
    conversation_history: Vec<LLMRequest>,
    iteration_count: usize,
    task: String,
}

impl AgentContext {
    pub async fn new(conversation_history: Vec<LLMRequest>) -> Result<Self> {
        let prompt_builder = SystemPromptBuilder::new().await?;
        Ok(Self {
            prompt_builder,
            conversation_history,
            task: String::new(),
            iteration_count: 0,
        })
    }

    pub async fn initialize(
        &mut self,
        config: &Config,
        tools: &ToolRegistry,
        skill_context: Option<&SkillPromptContext>,
        mode: AgentMode,
    ) -> Result<()> {
        let system_prompt = self
            .prompt_builder
            .build_with_skills_and_mode(config, tools, skill_context, mode)
            .await?;
        self.push(LLMRequest::system(system_prompt));
        self.push(LLMRequest::user(self.task.clone()));
        Ok(())
    }

    pub fn set_task(&mut self, task: String) {
        self.task = task;
    }

    pub fn task(&self) -> &str {
        &self.task
    }

    pub fn reset_iterations(&mut self) {
        self.iteration_count = 0;
    }

    pub fn increment_iterations(&mut self) {
        self.iteration_count += 1;
    }

    pub fn iteration_count(&self) -> usize {
        self.iteration_count
    }

    pub fn history(&self) -> &[LLMRequest] {
        &self.conversation_history
    }

    pub fn push(&mut self, message: LLMRequest) {
        self.conversation_history.push(message);
    }

    pub fn clear(&mut self) {
        self.conversation_history.clear();
        self.task.clear();
        self.iteration_count = 0;
    }
}

//! Domain layer: Pure Agent business logic without framework dependencies

use std::sync::{Arc, Mutex};

use crate::agent::traits::agent_trait::{Agent, AgentResult};
use crate::config::Config;
use crate::error::{AgentError, Result};
use crate::formatter::ResponseFormatter;
use crate::loading::LoadingIndicator;
use crate::permissions::PermissionManager;
use kameo::Actor;
use serde::{Deserialize, Serialize};

use crate::llm::traits::language_model::{AgentMessage, LanguageModel, ModelReply};
use crate::tool::tool_registry::ToolRegistry;
use crate::tool::traits::prompt_builder::SystemPromptBuilder;
use crate::tool::traits::tool_executor::ToolExecutor;
use crate::tool::traits::tool_parser::{ModelResponseParser, ParsedResponse};

/// Core Agent domain entity - orchestrates LLM interactions and tool execution
#[derive(Actor)]
pub struct AgentReAct {
    model: Box<dyn LanguageModel>,
    tool_executor: ToolExecutor,
    prompt_builder: SystemPromptBuilder,
    config: Config,
    formatter: ResponseFormatter,
    iteration_count: usize,
    conversation_history: Vec<AgentMessage>,
}

impl AgentReAct {
    /// Create a new agent
    pub async fn new(
        model: Box<dyn LanguageModel>,
        tools: ToolRegistry,
        config: Config,
        conversation_history: Vec<AgentMessage>,
        permission_manager: Arc<Mutex<PermissionManager>>,
    ) -> Result<Self> {
        let safety_validator = crate::safety::SafetyValidator::new(config.clone())?;
        let prompt_builder = SystemPromptBuilder::new().await?;
        let formatter = ResponseFormatter::new();
        let tool_executor = ToolExecutor::new(tools, permission_manager, safety_validator);

        Ok(Self {
            model,
            tool_executor,
            prompt_builder,
            config,
            formatter,
            iteration_count: 0,
            conversation_history,
        })
    }

    /// Initialize conversation with system prompt and task
    async fn init_prompt(&mut self, task: &str) -> Result<()> {
        let system_prompt = self
            .prompt_builder
            .build(&self.config, &self.tool_executor.tools)
            .await?;
        self.conversation_history
            .push(AgentMessage::system(system_prompt));
        self.conversation_history.push(AgentMessage::user(task));
        Ok(())
    }

    /// Check if iteration limit exceeded
    fn check_iteration_limit(&self) -> Result<()> {
        if self.iteration_count > self.config.safety.max_iterations {
            return Err(AgentError::MaxIterationsExceeded.into());
        }
        Ok(())
    }

    /// Get model response with loading indicator
    async fn call_model(&self) -> Result<ModelReply> {
        let reply = self.model.chat(&self.conversation_history).await?;

        Ok(reply)
    }

    /// Create agent result
    fn create_result(&self, success: bool, output: String, tool_calls: Vec<String>) -> AgentResult {
        AgentResult {
            success,
            output,
            iterations: self.iteration_count,
            tool_calls,
        }
    }
}

#[async_trait::async_trait]
impl Agent for AgentReAct {
    /// Run the agent on a task using ReACT (Reasoning, Acting, Observing) loop
    async fn execute_task(&mut self, task: String) -> Result<AgentResult> {
        let task_preview: String = task.chars().take(80).collect::<String>();

        self.iteration_count = 0;
        self.init_prompt(&task).await?;

        let mut tool_calls = Vec::new();

        loop {
            self.iteration_count += 1;
            self.check_iteration_limit()?;

            let response = self.call_model().await?;

            match ModelResponseParser::parse(&response.content) {
                ParsedResponse::ToolCall(tool_call) => {
                    let result = self
                        .tool_executor
                        .execute(tool_call.clone(), &self.config)
                        .await?;

                    tool_calls.push(tool_call.name.clone());

                    if tool_call.name == "file_write" && result.success {
                        if let Some(content) =
                            tool_call.args.get("content").and_then(|c| c.as_str())
                        {
                            let path = tool_call
                                .args
                                .get("path")
                                .and_then(|p| p.as_str())
                                .unwrap_or("unknown");
                            self.tool_executor.format_file_write_output(path, content);
                        }
                    }

                    let observation =
                        format!("Tool '{}' result: {}", tool_call.name, result.output);
                    self.conversation_history
                        .push(AgentMessage::user(observation));
                }
                ParsedResponse::Complete => {
                    tracing::info!(
                        iterations = self.iteration_count,
                        tools_used = tool_calls.len(),
                        "Task completed successfully"
                    );
                    self.conversation_history
                        .push(AgentMessage::assistant(response.content.clone()));
                    return Ok(self.create_result(true, response.content, tool_calls));
                }
                ParsedResponse::Incomplete(_) => {
                    let preview: String = response.content.chars().take(150).collect::<String>();
                    tracing::warn!(
                        response_preview = %preview,
                        "Incomplete response - requesting tool call or FINISH"
                    );
                    let prompt = format!(
                        "You said: \"{}\"\n\nPlease either:\n1. Use a tool to complete the task, OR\n2. Say FINISH if the task is done.",
                        response.content.chars().take(200).collect::<String>()
                    );
                    self.conversation_history.push(AgentMessage::user(prompt));
                }
            }
        }
    }

    /// Format a response using the formatter
    fn format_response(&self, content: &str) -> String {
        self.formatter.format_response(content)
    }

    /// Get conversation history (read-only)
    fn get_conversation_history(&self) -> &[AgentMessage] {
        &self.conversation_history
    }

    /// Clear conversation history
    fn clear_conversation_history(&mut self) {
        self.conversation_history.clear();
    }

    /// Add message to conversation history
    fn add_to_history(&mut self, message: AgentMessage) {
        self.conversation_history.push(message);
    }
}

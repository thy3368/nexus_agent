//! Domain layer: Pure Agent business logic without framework dependencies

use std::sync::{Arc, Mutex};
use rand::{distributions::Alphanumeric, Rng};

use crate::agent::mode::AgentMode;
use crate::agent::traits::agent_trait::{Agent, AgentResult};
use crate::config::Config;
use crate::error::{AgentError, Result};
use crate::permissions::PermissionManager;
use kameo::Actor;

use crate::context::agent_context::AgentContext;
use crate::llm::traits::ll_model::{LLMRequest, LLModel};
use crate::tool::tool_registry::ToolRegistry;
use crate::tool::traits::tool_executor::ToolExecutor;
use crate::tool::traits::tool_parser::{ModelResponseParser, ParsedResponse};

/// Core Agent domain entity - orchestrates LLM interactions and tool execution
#[derive(Actor)]
pub struct AgentReAct {
    model: Box<dyn LLModel>,
    tool_executor: ToolExecutor,
    context: AgentContext,
    config: Config,
    mode: AgentMode,
}

impl AgentReAct {
    /// Create a new agent
    pub async fn new(
        model: Box<dyn LLModel>,
        tools: ToolRegistry,
        config: Config,
        context: AgentContext,
        permission_manager: Arc<Mutex<PermissionManager>>,
    ) -> Result<Self> {
        Self::new_with_mode(
            model,
            tools,
            config,
            context,
            permission_manager,
            AgentMode::Execute,
        )
        .await
    }

    pub async fn new_with_mode(
        model: Box<dyn LLModel>,
        tools: ToolRegistry,
        config: Config,
        context: AgentContext,
        permission_manager: Arc<Mutex<PermissionManager>>,
        mode: AgentMode,
    ) -> Result<Self> {
        let safety_validator = crate::safety::SafetyValidator::new(config.clone())?;
        let tool_executor =
            ToolExecutor::new_with_mode(tools, permission_manager, safety_validator, mode);

        Ok(Self {
            model,
            tool_executor,
            context,
            config,
            mode,
        })
    }

    /// Create agent result
    fn create_result(&self, success: bool, output: String, tool_calls: Vec<String>) -> AgentResult {
        AgentResult {
            success,
            output,
            iterations: self.context.iteration_count(),
            tool_calls,
        }
    }
}

#[async_trait::async_trait]
impl Agent for AgentReAct {
    /// Run the agent on a task using ReACT (Reasoning, Acting, Observing) loop
    async fn execute_task(&mut self, task: String) -> Result<AgentResult> {
        // Generate session ID for this task execution
        let session_id: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        self.context.set_task(task);
        self.context.reset_iterations();
        let skill_context = self.context.render_skill_context(&self.config);
        self.context
            .initialize(
                &self.config,
                &self.tool_executor.tools,
                skill_context.as_ref(),
                self.mode,
            )
            .await;

        let mut tool_calls = Vec::new();

        loop {
            self.context.increment_iterations();
            if self.context.iteration_count() > self.config.safety.max_iterations {
                return Err(AgentError::MaxIterationsExceeded.into());
            }

            let response = self.model.chat(self.context.history(), None, Some(&session_id)).await?;

            match ModelResponseParser::parse_with_mode(&response.content, self.mode) {
                ParsedResponse::ToolCall(tool_call) => {
                    let call_result = self
                        .tool_executor
                        .execute(tool_call.clone(), &self.config)
                        .await?;

                    tool_calls.push(tool_call.name.clone());

                    if tool_call.name == "file_write" && call_result.success {
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
                        format!("Tool '{}' result: {}", tool_call.name, call_result.output);
                    self.context.push(LLMRequest::user(observation));
                }
                ParsedResponse::ProposedPlan(plan_text) => {
                    tracing::info!(
                        iterations = self.context.iteration_count(),
                        tools_used = tool_calls.len(),
                        "Plan proposed successfully"
                    );
                    self.context
                        .push(LLMRequest::assistant(response.content.clone()));
                    return Ok(self.create_result(true, plan_text, tool_calls));
                }
                ParsedResponse::Complete => {
                    tracing::info!(
                        iterations = self.context.iteration_count(),
                        tools_used = tool_calls.len(),
                        "Task completed successfully"
                    );
                    self.context
                        .push(LLMRequest::assistant(response.content.clone()));
                    return Ok(self.create_result(true, response.content, tool_calls));
                }
                ParsedResponse::Incomplete(_) => {
                    let preview: String = response.content.chars().take(150).collect::<String>();
                    tracing::warn!(
                        response_preview = %preview,
                        "Incomplete response - requesting tool call or FINISH"
                    );
                    let prompt = if self.mode.is_plan() {
                        format!(
                            "You said: \"{}\"\n\nPlease either use a read-only tool to continue planning, or finalize with a complete <proposed_plan>...</proposed_plan> block.",
                            response.content.chars().take(200).collect::<String>()
                        )
                    } else {
                        format!(
                            "You said: \"{}\"\n\nThis task is not complete yet. Continue by calling a tool, and use update_plan first if the task has multiple steps. Do not say FINISH until you have completed the full task and already provided the final deliverable.",
                            response.content.chars().take(200).collect::<String>()
                        )
                    };
                    self.context.push(LLMRequest::user(prompt));
                }
            }
        }
    }

    /// Get conversation history (read-only)
    fn get_conversation_history(&self) -> &[LLMRequest] {
        self.context.history()
    }

    /// Clear conversation history
    fn clear_conversation_history(&mut self) {
        self.context.clear();
    }

    /// Add message to conversation history
    fn add_to_history(&mut self, message: LLMRequest) {
        self.context.push(message);
    }
}

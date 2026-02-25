//! Domain layer: Pure Agent business logic without framework dependencies

use std::sync::{Arc, Mutex};

use crate::agent::behavior::traits::agent_behavior::{AgentBehavior, AgentResult};
use crate::config::Config;
use crate::error::{AgentError, Result};
use crate::formatter::ResponseFormatter;
use crate::loading::LoadingIndicator;
use crate::model::traits::language_model::{AgentMessage, LanguageModel, ModelReply};
use crate::permissions::PermissionManager;
use crate::tools::ToolRegistry;
use kameo::Actor;
use serde::{Deserialize, Serialize};

use super::prompt_builder::SystemPromptBuilder;
use super::tool_executor::ToolExecutor;
use super::tool_parser::{ModelResponseParser, ParsedResponse};

/// Core Agent domain entity - orchestrates LLM interactions and tool execution
#[derive(Actor)]
pub struct AgentBehaviorImpl {
    model: Box<dyn LanguageModel>,
    tool_executor: ToolExecutor,
    prompt_builder: SystemPromptBuilder,
    config: Config,
    formatter: ResponseFormatter,
    iteration_count: usize,
    conversation_history: Vec<AgentMessage>,
}

impl AgentBehaviorImpl {
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
    async fn initialize_conversation(&mut self, task: &str) -> Result<()> {
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
    async fn get_model_response(&self) -> Result<ModelReply> {
        let mut loading = LoadingIndicator::new();
        loading.start();
        //todo 用 tracing  优化日志打印 打印 chat 入参 方便高度和审记
        for (i, msg) in self.conversation_history.iter().enumerate() {
            tracing::debug!("[{}] {}: {}", i, msg.role, msg.content);
        }
        let reply = self.model.chat(&self.conversation_history).await?;
        loading.stop().await;
        tracing::info!("Reply content: {:?}", reply.content);
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
impl AgentBehavior for AgentBehaviorImpl {
    /// Run the agent on a task using ReACT (Reasoning, Acting, Observing) loop
    async fn execute_task(&mut self, task: String) -> Result<AgentResult> {
        tracing::info!("Starting agent run for task: {}", task);

        self.iteration_count = 0;
        self.initialize_conversation(&task).await?;

        let mut tool_calls = Vec::new();

        // ReACT loop: Reason → Act → Observe → repeat
        loop {
            self.iteration_count += 1;
            self.check_iteration_limit()?;

            tracing::debug!("Agent iteration: {}", self.iteration_count);

            // REASON: Get model response with reasoning
            let response = self.get_model_response().await?;

            // ACT: Parse and handle model response
            match ModelResponseParser::parse(&response.content) {
                ParsedResponse::ToolCall(tool_call) => {
                    let result = self
                        .tool_executor
                        .execute(tool_call.clone(), &self.config)
                        .await?;

                    tool_calls.push(tool_call.name.clone());

                    // Special handling for file_write to show user what was written
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

                    // OBSERVE: Add tool result to conversation history
                    let observation =
                        format!("Tool '{}' result: {}", tool_call.name, result.output);
                    self.conversation_history
                        .push(AgentMessage::user(observation));
                }
                ParsedResponse::Complete => {
                    tracing::info!("Task complete detected!");
                    self.conversation_history
                        .push(AgentMessage::assistant(response.content.clone()));
                    return Ok(self.create_result(true, response.content, tool_calls));
                }
                ParsedResponse::Incomplete(_) => {
                    // No valid tool call found and no FINISH - model gave free-form response
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use super::*;
    use crate::config::Config;
    use crate::model::traits::language_model::TokenUsage;
    use crate::model::{ModelInfo, ToolDefinition};
    use crate::permissions::PermissionManager;
    use crate::tools::ToolRegistry;

    struct MockModel {
        responses: Vec<String>,
        call_count: Arc<Mutex<usize>>,
    }

    #[async_trait]
    impl LanguageModel for MockModel {
        async fn complete(&self, _: &str, _: Option<&str>) -> crate::error::Result<ModelReply> {
            unimplemented!()
        }

        async fn chat(&self, _: &[AgentMessage]) -> crate::error::Result<ModelReply> {
            let mut count = self.call_count.lock().unwrap();
            let response = self.responses[*count].clone();
            *count += 1;

            Ok(ModelReply {
                content: response,
                model: "mock".to_string(),
                usage: TokenUsage::default(),
                tool_calls: None,
                finish_reason: Some("stop".to_string()),
            })
        }

        async fn chat_with_tools(
            &self,
            messages: &[AgentMessage],
            _: &[ToolDefinition],
        ) -> crate::error::Result<ModelReply> {
            self.chat(messages).await
        }

        fn model_info(&self) -> ModelInfo {
            ModelInfo {
                provider: "mock".to_string(),
                model: "test".to_string(),
                max_tokens: 4096,
                supports_tools: false,
                supports_streaming: false,
            }
        }
    }

    #[tokio::test]
    async fn test_agent_simple_task() {
        let model = Box::new(MockModel {
            responses: vec![
                "I will list the files. {\"tool\": \"file_list\", \"args\": {}}".to_string(),
                "FINISH".to_string(),
            ],
            call_count: Arc::new(Mutex::new(0)),
        });

        let mut tools = ToolRegistry::new();
        tools.register(crate::tools::file_ops::FileListTool::new());

        let mut config = Config::default();
        config.safety.require_approval = false;
        let permission_manager = Arc::new(Mutex::new(PermissionManager::new().unwrap()));
        permission_manager
            .lock()
            .unwrap()
            .set_permission(
                "file_list".to_string(),
                crate::permissions::PermissionLevel::Always,
            )
            .unwrap();
        let mut agent =
            AgentBehaviorImpl::new(model, tools, config, Vec::new(), permission_manager)
                .await
                .unwrap();

        let result = agent
            .execute_task("List the files and size in m".to_string())
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.iterations, 2);
        assert_eq!(result.tool_calls.len(), 1);
    }
}

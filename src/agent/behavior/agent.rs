//! Domain layer: Pure Agent business logic without framework dependencies

use std::sync::{Arc, Mutex};

use kameo::Actor;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::error::{AgentError, Result};
use crate::formatter::ResponseFormatter;
use crate::loading::LoadingIndicator;
use crate::model::{LanguageModel, MMessage};
use crate::permissions::PermissionManager;
use crate::tools::ToolRegistry;

use super::context_provider::ContextProvider;
use super::prompt_builder::SystemPromptBuilder;
use super::tool_executor::ToolExecutor;
use super::tool_parser::{ParsedToolCall, ToolCallParser};

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
    async fn execute_task(&mut self, task: String) -> Result<AgentResult>;

    /// Format response for display
    fn format_response(&self, content: &str) -> String;

    /// Get conversation history (read-only)
    fn get_conversation_history(&self) -> &[MMessage];

    /// Clear conversation history
    fn clear_conversation_history(&mut self);

    /// Add message to conversation history
    fn add_to_history(&mut self, message: MMessage);
}

/// Core Agent domain entity - orchestrates LLM interactions and tool execution
#[derive(Actor)]
pub struct AgentBehaviorImpl {
    model: Box<dyn LanguageModel>,
    tool_executor: ToolExecutor,
    prompt_builder: SystemPromptBuilder,
    config: Config,
    formatter: ResponseFormatter,
    iteration_count: usize,
    conversation_history: Vec<MMessage>,
}

impl AgentBehaviorImpl {
    /// Create a new agent
    pub async fn new(
        model: Box<dyn LanguageModel>,
        tools: ToolRegistry,
        config: Config,
        conversation_history: Vec<MMessage>,
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
        let system_prompt = self.prompt_builder.build(&self.config, &self.tool_executor.tools).await?;
        self.conversation_history.push(MMessage::system(system_prompt));
        self.conversation_history.push(MMessage::user(task));
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
    async fn get_model_response(&self) -> Result<crate::model::ModelResponse> {
        let mut loading = LoadingIndicator::new();
        loading.start();
        let response = self.model.chat(&self.conversation_history).await?;
        loading.stop().await;
        tracing::info!("Response content: {:?}", response.content);
        Ok(response)
    }

    /// Handle tool call execution and update conversation
    async fn handle_tool_call(
        &mut self,
        tool_call: &ParsedToolCall,
        tool_calls: &mut Vec<String>,
    ) -> Result<(bool, String)> {
        let result = self.tool_executor.execute(tool_call.clone(), &self.config).await?;

        // Special handling for file_write
        if tool_call.name == "file_write" && result.success {
            if let Some(content) = tool_call.args.get("content").and_then(|c| c.as_str()) {
                let path = tool_call.args
                    .get("path")
                    .and_then(|p| p.as_str())
                    .unwrap_or("unknown");
                self.tool_executor.format_file_write_output(path, content);
            }
        }

        tool_calls.push(tool_call.name.clone());

        // Add result to conversation
        let observation = format!("Tool '{}' result: {}", tool_call.name, result.output);
        self.conversation_history.push(MMessage::assistant(observation));

        Ok((result.success, result.output))
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
    /// Run the agent on a task
    async fn execute_task(&mut self, task: String) -> Result<AgentResult> {
        tracing::info!("Starting agent run for task: {}", task);

        self.iteration_count = 0;
        self.initialize_conversation(&task).await?;

        let mut tool_calls = Vec::new();

        // ReACT loop
        loop {
            self.iteration_count += 1;
            self.check_iteration_limit()?;

            tracing::debug!("Agent iteration: {}", self.iteration_count);

            let response = self.get_model_response().await?;

            // Check if task is complete
            if ToolCallParser::is_complete(&response.content) {
                tracing::info!("Task complete detected!");
                // Add final response to history before returning
                self.conversation_history.push(MMessage::assistant(response.content.clone()));
                return Ok(self.create_result(true, response.content, tool_calls));
            }

            // ACT: Parse and execute tool calls
            if let Some(tool_call) = ToolCallParser::parse(&response.content) {
                let (success, output) = self.handle_tool_call(&tool_call, &mut tool_calls).await?;

                if !success {
                    return Ok(self.create_result(false, output, tool_calls));
                }
            } else {
                // No tool call found, add response to history
                self.conversation_history.push(MMessage::assistant(response.content));
            }
        }
    }

    /// Format a response using the formatter
    fn format_response(&self, content: &str) -> String {
        self.formatter.format_response(content)
    }

    /// Get conversation history (read-only)
    fn get_conversation_history(&self) -> &[MMessage] {
        &self.conversation_history
    }

    /// Clear conversation history
    fn clear_conversation_history(&mut self) {
        self.conversation_history.clear();
    }

    /// Add message to conversation history
    fn add_to_history(&mut self, message: MMessage) {
        self.conversation_history.push(message);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use super::*;
    use crate::config::Config;
    use crate::model::{ModelInfo, ModelResponse, TokenUsage};
    use crate::permissions::PermissionManager;
    use crate::tools::ToolRegistry;

    struct MockModel {
        responses: Vec<String>,
        call_count: Arc<Mutex<usize>>,
    }

    #[async_trait]
    impl crate::model::LanguageModel for MockModel {
        async fn complete(&self, _: &str, _: Option<&str>) -> crate::error::Result<ModelResponse> {
            unimplemented!()
        }

        async fn chat(&self, _: &[crate::model::MMessage]) -> crate::error::Result<ModelResponse> {
            let mut count = self.call_count.lock().unwrap();
            let response = self.responses[*count].clone();
            *count += 1;

            Ok(ModelResponse {
                content: response,
                model: "mock".to_string(),
                usage: TokenUsage::default(),
                tool_calls: None,
                finish_reason: Some("stop".to_string()),
            })
        }

        async fn chat_with_tools(
            &self,
            messages: &[crate::model::MMessage],
            _: &[crate::model::ToolDefinition],
        ) -> crate::error::Result<ModelResponse> {
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
            .set_permission("file_list".to_string(), crate::permissions::PermissionLevel::Always)
            .unwrap();
        let mut agent =
            AgentBehaviorImpl::new(model, tools, config, Vec::new(), permission_manager).await.unwrap();

        let result = agent.execute_task("List the files and size in m".to_string()).await.unwrap();

        assert!(result.success);
        assert_eq!(result.iterations, 2);
        assert_eq!(result.tool_calls.len(), 1);
    }
}

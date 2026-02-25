//! Domain layer: Pure Agent business logic without framework dependencies

use std::sync::{Arc, Mutex};

use kameo::Actor;
use serde::{Deserialize, Serialize};
use crate::agent::behavior::traits::agent_behavior::{AgentBehavior, AgentResult};
use crate::config::Config;
use crate::error::{AgentError, Result};
use crate::formatter::ResponseFormatter;
use crate::loading::LoadingIndicator;
use crate::model::traits::language_model::{LanguageModel, MMessage, ModelResponse};
use crate::permissions::PermissionManager;
use crate::tools::ToolRegistry;

use super::prompt_builder::SystemPromptBuilder;
use super::tool_executor::ToolExecutor;
use super::tool_parser::ToolCallParser;



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
    async fn get_model_response(&self) -> Result<ModelResponse> {
        let mut loading = LoadingIndicator::new();
        loading.start();
        let response = self.model.chat(&self.conversation_history).await?;
        loading.stop().await;
        tracing::info!("Response content: {:?}", response.content);
        Ok(response)
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

            // ACT: Try to parse and execute tool call FIRST (before checking FINISH)
            if let Some(tool_call) = ToolCallParser::parse(&response.content) {
                let result = self.tool_executor.execute(tool_call.clone(), &self.config).await?;

                tool_calls.push(tool_call.name.clone());

                // Special handling for file_write to show user what was written
                if tool_call.name == "file_write" && result.success {
                    if let Some(content) = tool_call.args.get("content").and_then(|c| c.as_str()) {
                        let path = tool_call.args
                            .get("path")
                            .and_then(|p| p.as_str())
                            .unwrap_or("unknown");
                        self.tool_executor.format_file_write_output(path, content);
                    }
                }

                // OBSERVE: Add tool result to conversation history
                // This lets the model see the outcome and decide next action
                let observation = format!("Tool '{}' result: {}", tool_call.name, result.output);
                self.conversation_history.push(MMessage::user(observation));

                // Continue loop - model will reason about the result
            } else if ToolCallParser::is_complete(&response.content) {
                // Check if task is complete (model says FINISH) - only if no tool call
                tracing::info!("Task complete detected!");
                // Add final response to history before returning
                self.conversation_history.push(MMessage::assistant(response.content.clone()));
                return Ok(self.create_result(true, response.content, tool_calls));
            } else {
                // No valid tool call found - treat as free-form response
                // Add to history and continue (model may refine or try again)
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
    use crate::model::{ModelInfo, ToolDefinition};
    use crate::model::traits::language_model::TokenUsage;
    use crate::permissions::PermissionManager;
    use crate::tools::ToolRegistry;

    struct MockModel {
        responses: Vec<String>,
        call_count: Arc<Mutex<usize>>,
    }

    #[async_trait]
    impl LanguageModel for MockModel {
        async fn complete(&self, _: &str, _: Option<&str>) -> crate::error::Result<ModelResponse> {
            unimplemented!()
        }

        async fn chat(&self, _: &[MMessage]) -> crate::error::Result<ModelResponse> {
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
            messages: &[MMessage],
            _: &[ToolDefinition],
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

    /// Example: Using AgentBehaviorImpl with Kimi LLM
    ///
    /// This example demonstrates how to:
    /// 1. Create a KimiProvider with API key
    /// 2. Initialize AgentBehaviorImpl with the Kimi model
    /// 3. Execute a task using the ReACT loop
    ///
    /// To run this example:
    /// ```bash
    /// KIMI_API_KEY="your-api-key" cargo test test_agent_with_kimi_example -- --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore]
    async fn test_agent_with_kimi_example() {
        // Step 1: Get Kimi API key from environment
        let api_key = std::env::var("KIMI_API_KEY")
            .expect("KIMI_API_KEY environment variable not set");

        // Step 2: Create Kimi provider
        let kimi_provider = crate::model::kimi::KimiProvider::new(
            api_key,
            Some("moonshot-v1-8k".to_string()),
        );
        let model: Box<dyn LanguageModel> = Box::new(kimi_provider);

        // Step 3: Create tool registry and register tools
        let mut tools = ToolRegistry::new();
        // Register available tools (example: file operations)
        tools.register(crate::tools::file_ops::FileListTool::new());

        // Step 4: Create configuration
        let mut config = Config::default();
        config.safety.require_approval = false;

        // Step 5: Create permission manager and set permissions
        let permission_manager = Arc::new(Mutex::new(PermissionManager::new().unwrap()));
        permission_manager
            .lock()
            .unwrap()
            .set_permission("file_list".to_string(), crate::permissions::PermissionLevel::Always)
            .unwrap();

        // Step 6: Initialize AgentBehaviorImpl with Kimi
        let mut agent = AgentBehaviorImpl::new(
            model,
            tools,
            config,
            Vec::new(),  // empty conversation history
            permission_manager,
        )
        .await
        .expect("Failed to create agent");

        // Step 7: Execute a task
        let task = "列出当前目录中的所有文件";
        println!("\n📋 Task: {}", task);

        match agent.execute_task(task.to_string()).await {
            Ok(result) => {
                println!("\n✅ Task completed successfully!");
                println!("  Success: {}", result.success);
                println!("  Iterations: {}", result.iterations);
                println!("  Tools used: {:?}", result.tool_calls);
                println!("  Output:\n{}", result.output);
            }
            Err(e) => {
                eprintln!("\n❌ Task failed: {}", e);
            }
        }

        // Step 8: Access conversation history
        let history = agent.get_conversation_history();
        println!("\n📝 Conversation history ({} messages):", history.len());
        for (i, msg) in history.iter().enumerate() {
            println!("  [{}] {}: {}", i, msg.role, msg.content.chars().take(50).collect::<String>());
        }
    }

    /// Example: Using AgentBehaviorImpl with Kimi for multi-turn conversation
    ///
    /// This example shows how to maintain conversation state across multiple tasks
    ///
    /// To run this example:
    /// ```bash
    /// KIMI_API_KEY="your-api-key" cargo test test_agent_kimi_multi_turn -- --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore]
    async fn test_agent_kimi_multi_turn() {
        let api_key = std::env::var("KIMI_API_KEY")
            .expect("KIMI_API_KEY environment variable not set");

        let kimi_provider = crate::model::kimi::KimiProvider::new(
            api_key,
            Some("moonshot-v1-8k".to_string()),
        );
        let model: Box<dyn LanguageModel> = Box::new(kimi_provider);

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

        let mut agent = AgentBehaviorImpl::new(
            model,
            tools,
            config,
            Vec::new(),
            permission_manager,
        )
        .await
        .expect("Failed to create agent");

        // Execute multiple tasks in sequence
        let tasks = vec![
            "你好，请介绍一下你自己",
            "列出当前目录的文件",
        ];

        for (idx, task) in tasks.iter().enumerate() {
            println!("\n🔄 Task {}: {}", idx + 1, task);

            match agent.execute_task(task.to_string()).await {
                Ok(result) => {
                    println!("✅ Result: {}", &result.output[..result.output.len().min(100)]);
                }
                Err(e) => {
                    eprintln!("❌ Error: {}", e);
                }
            }
        }

        // Show final conversation history
        let history = agent.get_conversation_history();
        println!("\n📊 Final conversation has {} messages", history.len());
    }
}

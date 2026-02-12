//! Actix Actor adapter for Agent (Interface/Adapter layer)
//! This module provides Actor capabilities for the domain Agent

use kameo::message::{Context, Message};
use serde::{Deserialize, Serialize};

use crate::agent::domain::agent::{Agent, AgentResult};
use crate::error;

/// Message for executing a task asynchronously via Actor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunTaskCmd(pub String);

/// Handler for asynchronous task execution
impl Message<RunTaskCmd> for Agent {
    type Reply = crate::Result<AgentResult>;

    async fn handle(
        &mut self,
        msg: RunTaskCmd,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.execute_task(msg.0).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use kameo::actor::Spawn;

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
        async fn complete(&self, _: &str, _: Option<&str>) -> error::Result<ModelResponse> {
            unimplemented!()
        }

        async fn chat(&self, _: &[crate::model::MMessage]) -> error::Result<ModelResponse> {
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
        ) -> error::Result<ModelResponse> {
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
    async fn test_run_task_cmd_handler() {
        let model = Box::new(MockModel {
            responses: vec!["FINISH".to_string()],
            call_count: Arc::new(Mutex::new(0)),
        });

        let tools = ToolRegistry::new();
        let mut config = Config::default();
        config.safety.require_approval = false;

        let permission_manager = Arc::new(Mutex::new(PermissionManager::new().unwrap()));

        let agent = Agent::new(model, tools, config, Vec::new(), permission_manager).await.unwrap();

        // Spawn the actor using kameo's spawn method
        let actor_ref = Agent::spawn(agent);

        // Send message and await response
        let result = actor_ref.ask(RunTaskCmd("test task".to_string())).await.unwrap();

        assert!(result.success);
        assert_eq!(result.iterations, 1);
    }

    #[tokio::test]
    async fn test_run_task_cmd_multiple_messages() {
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

        let agent = Agent::new(model, tools, config, Vec::new(), permission_manager).await.unwrap();

        // Spawn the actor using kameo's spawn method
        let actor_ref = Agent::spawn(agent);

        // Send message and await response
        let result = actor_ref.ask(RunTaskCmd("List the files and size in".to_string())).await.unwrap();

        println!("msg1 result: {:?}", result);

        assert!(result.success);
    }
}

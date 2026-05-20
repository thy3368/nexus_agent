use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::ToolError;
use crate::tool::traits::tool_definition::{ToolContext, ToolMeta, ToolResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanItemArg {
    pub step: String,
    pub status: StepStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpdatePlanArgs {
    #[serde(default)]
    pub explanation: Option<String>,
    pub plan: Vec<PlanItemArg>,
}

pub struct UpdatePlanTool;

impl UpdatePlanTool {
    pub fn new() -> Self {
        Self
    }

    fn parse_args(&self, args: Value) -> crate::Result<UpdatePlanArgs> {
        let parsed: UpdatePlanArgs =
            serde_json::from_value(args).map_err(|err| ToolError::InvalidArgs(err.to_string()))?;

        if parsed.plan.is_empty() {
            return Err(ToolError::InvalidArgs("plan must not be empty".to_string()).into());
        }

        let mut in_progress_count = 0;
        for item in &parsed.plan {
            if item.step.trim().is_empty() {
                return Err(
                    ToolError::InvalidArgs("plan steps must not be empty".to_string()).into(),
                );
            }
            if item.status == StepStatus::InProgress {
                in_progress_count += 1;
            }
        }

        if in_progress_count > 1 {
            return Err(ToolError::InvalidArgs(
                "at most one plan step can be in_progress".to_string(),
            )
            .into());
        }

        Ok(parsed)
    }
}

impl Default for UpdatePlanTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolMeta for UpdatePlanTool {
    fn name(&self) -> &str {
        "update_plan"
    }

    fn description(&self) -> &str {
        "Updates the task plan. Provide an optional explanation and a list of plan items, each with a step and status. At most one step can be in_progress at a time."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "explanation": {
                    "type": "string",
                    "description": "Optional explanation for why the plan changed"
                },
                "plan": {
                    "type": "array",
                    "description": "The list of plan steps",
                    "items": {
                        "type": "object",
                        "properties": {
                            "step": {
                                "type": "string",
                                "description": "A concise plan step"
                            },
                            "status": {
                                "type": "string",
                                "description": "One of: pending, in_progress, completed",
                                "enum": ["pending", "in_progress", "completed"]
                            }
                        },
                        "required": ["step", "status"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["plan"],
            "additionalProperties": false
        })
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn execute(
        &self,
        args: Value,
        _ctx: &ToolContext,
        _config: &crate::config::Config,
    ) -> crate::Result<ToolResult> {
        let parsed = self.parse_args(args)?;
        let plan_value = serde_json::to_value(&parsed.plan)
            .map_err(|err| ToolError::ExecutionFailed(err.to_string()))?;
        let mut result = ToolResult::success("Plan updated").with_metadata("plan", plan_value);

        if let Some(explanation) = parsed.explanation {
            result = result.with_metadata("explanation", Value::String(explanation));
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn valid_update_plan_returns_plan_updated() {
        let tool = UpdatePlanTool::new();
        let result = tool
            .execute(
                serde_json::json!({
                    "explanation": "Starting",
                    "plan": [
                        {"step": "Inspect code", "status": "completed"},
                        {"step": "Add plan mode", "status": "in_progress"},
                        {"step": "Add tests", "status": "pending"}
                    ]
                }),
                &ToolContext::default(),
                &crate::config::Config::default(),
            )
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output, "Plan updated");
        assert!(result.metadata.contains_key("plan"));
        assert!(result.metadata.contains_key("explanation"));
    }

    #[test]
    fn missing_plan_fails_validation() {
        let tool = UpdatePlanTool::new();
        assert!(tool.validate_args(&serde_json::json!({})).is_err());
    }

    #[tokio::test]
    async fn invalid_status_fails() {
        let tool = UpdatePlanTool::new();
        let result = tool
            .execute(
                serde_json::json!({
                    "plan": [{"step": "Inspect", "status": "started"}]
                }),
                &ToolContext::default(),
                &crate::config::Config::default(),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn empty_step_fails() {
        let tool = UpdatePlanTool::new();
        let result = tool
            .execute(
                serde_json::json!({
                    "plan": [{"step": "", "status": "pending"}]
                }),
                &ToolContext::default(),
                &crate::config::Config::default(),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn multiple_in_progress_steps_fail() {
        let tool = UpdatePlanTool::new();
        let result = tool
            .execute(
                serde_json::json!({
                    "plan": [
                        {"step": "One", "status": "in_progress"},
                        {"step": "Two", "status": "in_progress"}
                    ]
                }),
                &ToolContext::default(),
                &crate::config::Config::default(),
            )
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn update_plan_is_not_read_only() {
        assert!(!UpdatePlanTool::new().is_read_only());
    }
}

//! Tool discovery search

use crate::error::{Result, ToolError};
use crate::tool::traits::tool::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct ToolSearchEntry {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub read_only: bool,
    pub supports_parallel_calls: bool,
}

/// Tool search adapter. The registry owns the actual tool set; this adapter owns ranking and formatting.
/// Usage: discover the right registered tool by capability instead of guessing tool names.
/// 使用场景：不知道该调用哪个工具时，先按能力关键词搜索已注册工具。
pub struct ToolSearchTool {
    entries: Vec<ToolSearchEntry>,
}

impl ToolSearchTool {
    pub fn new(entries: Vec<ToolSearchEntry>) -> Self {
        Self { entries }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }
}

impl Default for ToolSearchTool {
    fn default() -> Self {
        Self::empty()
    }
}

#[async_trait]
impl Tool for ToolSearchTool {
    fn name(&self) -> &str {
        "tool_search"
    }

    fn description(&self) -> &str {
        "Search available tools by name, description, and parameter schema. This searches tools, not source files."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query describing the tool capability to find"
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Maximum number of tools to return"
                }
            },
            "required": ["query"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        _ctx: &ToolContext,
        _config: &crate::config::Config,
    ) -> Result<ToolResult> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("Missing query".to_string()))?;
        let limit = args
            .get("limit")
            .and_then(|value| value.as_u64())
            .unwrap_or(10) as usize;
        let terms = query
            .split_whitespace()
            .map(|term| term.to_ascii_lowercase())
            .collect::<Vec<_>>();

        let mut matches = self
            .entries
            .iter()
            .filter_map(|entry| {
                let score = score_entry(entry, &terms);
                (score > 0).then_some((score, entry))
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| left.1.name.cmp(&right.1.name))
        });

        let selected = matches.into_iter().take(limit).collect::<Vec<_>>();
        let output = if selected.is_empty() {
            "No matching tools found".to_string()
        } else {
            selected
                .iter()
                .map(|(score, entry)| {
                    format!(
                        "{} (score: {}, read_only: {}, parallel: {})\n  {}",
                        entry.name,
                        score,
                        entry.read_only,
                        entry.supports_parallel_calls,
                        entry.description
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let tools = selected
            .iter()
            .map(|(score, entry)| {
                serde_json::json!({
                    "name": entry.name,
                    "description": entry.description,
                    "parameters": entry.parameters,
                    "read_only": entry.read_only,
                    "supports_parallel_calls": entry.supports_parallel_calls,
                    "score": score,
                })
            })
            .collect::<Vec<_>>();

        Ok(ToolResult::success(output)
            .with_metadata("query", serde_json::json!(query))
            .with_metadata("tools", serde_json::json!(tools)))
    }
}

fn score_entry(entry: &ToolSearchEntry, terms: &[String]) -> usize {
    let searchable =
        format!("{} {} {}", entry.name, entry.description, entry.parameters).to_ascii_lowercase();

    terms
        .iter()
        .map(|term| {
            if entry.name.to_ascii_lowercase().contains(term) {
                5
            } else if searchable.contains(term) {
                1
            } else {
                0
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_search() {
        let tool = ToolSearchTool::new(vec![ToolSearchEntry {
            name: "file_read".to_string(),
            description: "Read files".to_string(),
            parameters: serde_json::json!({"path": "string"}),
            read_only: true,
            supports_parallel_calls: true,
        }]);
        let ctx = ToolContext::default();
        let config = crate::config::Config::default();

        let result = tool
            .execute(serde_json::json!({"query": "read file"}), &ctx, &config)
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("file_read"));
    }
}

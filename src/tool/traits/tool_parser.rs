//! Tool call parser for extracting tool invocations from model responses

use crate::tool::traits::tool_executor::ToolCall;
use serde_json;

#[derive(Debug, Clone)]
pub enum ParsedResponse {
    /// Model wants to call a tool
    ToolCall(ToolCall),
    /// Task is complete, model responded with FINISH
    Complete,
    /// Neither tool call nor FINISH - model gave free-form response
    Incomplete(String),
}

pub struct ModelResponseParser;

impl ModelResponseParser {
    /// Parse model response into a unified enum
    pub fn parse(content: &str) -> ParsedResponse {
        // Check for tool call first (takes priority)
        if let Some(start) = content.find('{') {
            if let Some(end) = content.rfind('}') {
                let json_str = &content[start..=end];
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let (Some(tool), Some(args)) = (
                        value.get("tool").and_then(|v| v.as_str()),
                        value.get("args"),
                    ) {
                        return ParsedResponse::ToolCall(ToolCall {
                            name: tool.to_string(),
                            args: args.clone(),
                        });
                    }
                }
            }
        }

        // Check for FINISH (mutually exclusive with tool call)
        if content.trim().ends_with("FINISH") || content.contains("task is complete") {
            return ParsedResponse::Complete;
        }

        // Neither tool call nor FINISH,不应该出现
        ParsedResponse::Incomplete(content.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_call() {
        let content = r#"I will list files. {"tool": "file_list", "args": {}}"#;
        match ModelResponseParser::parse(content) {
            ParsedResponse::ToolCall(call) => {
                assert_eq!(call.name, "file_list");
            }
            _ => panic!("Expected ToolCall"),
        }
    }

    #[test]
    fn test_parse_complete() {
        let content = "Done. FINISH";
        match ModelResponseParser::parse(content) {
            ParsedResponse::Complete => {}
            _ => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_parse_incomplete() {
        let content = "Still working...";
        match ModelResponseParser::parse(content) {
            ParsedResponse::Incomplete(s) => {
                assert_eq!(s, "Still working...");
            }
            _ => panic!("Expected Incomplete"),
        }
    }

    #[test]
    fn test_tool_call_and_finish_prefers_tool_call() {
        let content = r#"{"tool": "file_list", "args": {}}FINISH"#;
        match ModelResponseParser::parse(content) {
            ParsedResponse::ToolCall(call) => {
                assert_eq!(call.name, "file_list");
            }
            ParsedResponse::Complete => panic!("Should be ToolCall, not Complete"),
            ParsedResponse::Incomplete(_) => panic!("Should be ToolCall, not Incomplete"),
        }
    }
}

//! Tool call parser for extracting tool invocations from model responses

use serde_json;

#[derive(Debug, Clone)]
pub struct ParsedToolCall {
    pub name: String,
    pub args: serde_json::Value,
}

pub struct ToolCallParser;

impl ToolCallParser {
    /// Parse tool call from model response
    pub fn parse(content: &str) -> Option<ParsedToolCall> {
        if let Some(start) = content.find('{') {
            if let Some(end) = content.rfind('}') {
                let json_str = &content[start..=end];
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let (Some(tool), Some(args)) =
                        (value.get("tool").and_then(|v| v.as_str()), value.get("args"))
                    {
                        return Some(ParsedToolCall {
                            name: tool.to_string(),
                            args: args.clone(),
                        });
                    }
                }
            }
        }
        None
    }

    /// Check if response indicates task completion
    pub fn is_complete(content: &str) -> bool {
        content.trim().ends_with("FINISH") || content.contains("task is complete")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_call() {
        let content = r#"I will list files. {"tool": "file_list", "args": {}}"#;
        let parsed = ToolCallParser::parse(content);
        assert!(parsed.is_some());
        let call = parsed.unwrap();
        assert_eq!(call.name, "file_list");
    }

    #[test]
    fn test_is_complete() {
        assert!(ToolCallParser::is_complete("Done. FINISH"));
        assert!(!ToolCallParser::is_complete("Still working..."));
    }
}

//! Language model provider interface

use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod gemini;
pub mod openai;
pub mod ollama;

pub mod kimi;

pub mod traits;


/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}





/// Model information
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub provider: String,
    pub model: String,
    pub max_tokens: usize,
    pub supports_tools: bool,
    pub supports_streaming: bool,
}

// PromptLine - An Agentic AI-Powered CLI for Intelligent Code Assistance
//
// Library entry point exposing public API

pub mod agent;
pub mod commands;
pub mod config;
pub mod context;
pub mod error;
pub mod formatter;
pub mod loading;
pub mod permissions;
pub mod prompt;
pub mod repl;
pub mod safety;
pub mod setup;
pub mod tool;
pub mod util;

pub mod inbound_adapter;
pub mod llm;

// // Re-export commonly used types
// pub use config::Config;
pub use error::{PromptLineError, Result};
// pub use tools::{Tool, ToolResult};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Prelude for convenient imports
// pub mod prelude {
//     pub use crate::config::Config;
//     pub use crate::error::{PromptLineError, Result};
//     // pub use crate::tools::{Tool, ToolResult};
// }

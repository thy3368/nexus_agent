//! CLI interface

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "promptline")]
#[command(version, about = "An Agentic AI-Powered CLI for Intelligent Code Assistance", long_about = None)]
pub struct Cli {
    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Model provider to use (openai, anthropic, local)
    #[arg(short = 'p', long)]
    pub provider: Option<String>,

    /// Specific model to use (e.g., gpt-4, gpt-3.5-turbo)
    #[arg(short = 'm', long)]
    pub model: Option<String>,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Auto-approve all actions (use with caution!)
    #[arg(long)]
    pub auto_approve: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Direct task to execute (if no subcommand provided)
    #[arg(value_name = "TASK")]
    pub task: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run agent in execution mode
    Agent {
        /// Task to execute
        task: String,
    },

    /// Design an implementation plan without mutating files
    #[command(name = "design-plan", alias = "plan")]
    DesignPlan {
        /// Task to plan
        task: String,
    },

    /// Execute an implementation plan
    #[command(name = "execute-plan")]
    ExecutePlan {
        /// Plan or task to execute
        task: String,
    },

    /// Start interactive chat mode
    Chat,

    /// Edit a file with AI assistance
    Edit {
        /// File to edit
        file: PathBuf,

        /// Description of changes
        instruction: String,
    },

    /// Initialize configuration
    Init,

    /// Check installation and configuration
    Doctor,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_design_plan_command() {
        let cli = Cli::try_parse_from(["promptline", "design-plan", "add auth"]).unwrap();
        match cli.command {
            Some(Commands::DesignPlan { task }) => assert_eq!(task, "add auth"),
            _ => panic!("expected design-plan command"),
        }
    }

    #[test]
    fn parses_plan_alias() {
        let cli = Cli::try_parse_from(["promptline", "plan", "add auth"]).unwrap();
        match cli.command {
            Some(Commands::DesignPlan { task }) => assert_eq!(task, "add auth"),
            _ => panic!("expected plan alias"),
        }
    }

    #[test]
    fn parses_execute_plan_command() {
        let cli = Cli::try_parse_from(["promptline", "execute-plan", "implement plan"]).unwrap();
        match cli.command {
            Some(Commands::ExecutePlan { task }) => assert_eq!(task, "implement plan"),
            _ => panic!("expected execute-plan command"),
        }
    }
}

//! Slash command handling
//!
//! Provides commands for configuration and control

use crate::config::Config;
use crate::permissions::PermissionManager;
use crate::skill::SkillManager;
use anyhow::Result;
use std::sync::{Arc, Mutex};

/// Command execution result
pub struct CommandOutput {
    pub message: String,
    pub action: CommandAction,
}

/// Action to be taken by the main loop
#[derive(Debug, PartialEq)]
pub enum CommandAction {
    None,
    Quit,
    ClearHistory,
    ReloadAgent,
    ReloadSkills,
    DesignPlan(String),
}

impl CommandOutput {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            action: CommandAction::None,
        }
    }

    pub fn with_action(mut self, action: CommandAction) -> Self {
        self.action = action;
        self
    }
}

/// Slash command types
#[derive(Debug, Clone, PartialEq)]
pub enum SlashCommand {
    Help,
    Settings,
    Clear,
    Status,
    Model(Option<Vec<String>>),       // Optional args
    Permissions(Option<Vec<String>>), // Optional args
    Skills(Option<Vec<String>>),
    Plan(Option<Vec<String>>),
    Quit,
    Version,
}

/// Command handler
pub struct CommandHandler {
    config: Config,
    permissions: Arc<Mutex<PermissionManager>>,
    skill_manager: Option<Arc<SkillManager>>,
}

impl CommandHandler {
    /// Create a new command handler
    pub fn new(config: Config, permissions: Arc<Mutex<PermissionManager>>) -> Self {
        Self {
            config,
            permissions,
            skill_manager: None,
        }
    }

    pub fn new_with_skills(
        config: Config,
        permissions: Arc<Mutex<PermissionManager>>,
        skill_manager: Option<Arc<SkillManager>>,
    ) -> Self {
        Self {
            config,
            permissions,
            skill_manager,
        }
    }

    /// Parse a slash command from input
    pub fn parse(input: &str) -> Option<SlashCommand> {
        let trimmed = input.trim();
        if !trimmed.starts_with('/') {
            return None;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let command = parts[0].to_lowercase();
        let args = if parts.len() > 1 {
            Some(parts[1..].iter().map(|s| s.to_string()).collect())
        } else {
            None
        };

        match command.as_str() {
            "/help" | "/h" => Some(SlashCommand::Help),
            "/settings" | "/config" => Some(SlashCommand::Settings),
            "/clear" | "/new" => Some(SlashCommand::Clear),
            "/status" => Some(SlashCommand::Status),
            "/model" => Some(SlashCommand::Model(args)),
            "/permissions" | "/perms" => Some(SlashCommand::Permissions(args)),
            "/skills" | "/skill" => Some(SlashCommand::Skills(args)),
            "/plan" => Some(SlashCommand::Plan(args)),
            "/quit" | "/exit" | "/q" => Some(SlashCommand::Quit),
            "/version" | "/v" => Some(SlashCommand::Version),
            _ => None,
        }
    }

    /// Execute a slash command
    pub fn execute(&mut self, command: SlashCommand) -> Result<CommandOutput> {
        match command {
            SlashCommand::Help => Ok(CommandOutput::new(self.help())),
            SlashCommand::Settings => Ok(CommandOutput::new(self.settings())),
            SlashCommand::Clear => {
                Ok(CommandOutput::new("Session cleared.").with_action(CommandAction::ClearHistory))
            }
            SlashCommand::Status => Ok(CommandOutput::new(self.status())),
            SlashCommand::Model(args) => self.handle_model(args),
            SlashCommand::Permissions(args) => {
                Ok(CommandOutput::new(self.handle_permissions(args)?))
            }
            SlashCommand::Skills(args) => self.handle_skills(args),
            SlashCommand::Plan(args) => self.handle_plan(args),
            SlashCommand::Quit => {
                Ok(CommandOutput::new("Goodbye! 👋").with_action(CommandAction::Quit))
            }
            SlashCommand::Version => Ok(CommandOutput::new(format!(
                "PromptLine v{}",
                crate::VERSION
            ))),
        }
    }

    /// Show help message
    fn help(&self) -> String {
        r#"
⚙️  PromptLine Commands

Available slash commands:
  /help         Show this help message
  /settings     Configure permissions and preferences
  /clear        Start new session (clear history)
  /status       Show current configuration
  /model        Show model information
  /permissions  Manage tool permissions
  /skills       List and reload available skills
  /plan <task>  Design a read-only implementation plan
  /quit         Exit PromptLine
  /version      Show version info

Aliases:
  /h → /help
  /q → /quit
  /v → /version
  /perms → /permissions
  /skill → /skills
"#
        .to_string()
    }

    /// Show settings
    fn settings(&self) -> String {
        let perms = self.permissions.lock().unwrap();
        let all_perms = perms.get_all_permissions();
        let mut output = String::from("\n⚙️  PromptLine Settings\n\nPermissions:\n");

        if all_perms.is_empty() {
            output.push_str("  (No custom permissions set)\n");
        } else {
            for (tool, level) in all_perms {
                output.push_str(&format!("  • {}: {:?}\n", tool, level));
            }
        }

        output.push_str(&format!("\nProvider: {}\n", self.config.models.default));
        output.push_str("\nType /help for available commands\n");

        output
    }

    /// Show status
    fn status(&self) -> String {
        format!(
            "\n⚙️  Status\n\nProvider: {}\nVersion: {}\n",
            self.config.models.default,
            crate::VERSION
        )
    }

    /// Handle model command
    fn handle_model(&mut self, args: Option<Vec<String>>) -> Result<CommandOutput> {
        if let Some(args) = args {
            if args.is_empty() {
                return Ok(CommandOutput::new(self.model_info()));
            }

            let subcommand = args[0].to_lowercase();
            match subcommand.as_str() {
                "set" => {
                    // /model set <provider> [model]
                    if args.len() < 2 {
                        return Ok(CommandOutput::new("Usage: /model set <provider> [model]\nExample: /model set openai gpt-4"));
                    }
                    let provider = args[1]
                        .to_lowercase()
                        .trim_matches(|c| c == '<' || c == '>' || c == '[' || c == ']')
                        .to_string();
                    let model = if args.len() > 2 {
                        args[2]
                            .clone()
                            .trim_matches(|c| c == '<' || c == '>' || c == '[' || c == ']')
                            .to_string()
                    } else {
                        // Default models if not specified
                        match provider.as_str() {
                            "openai" => "gpt-3.5-turbo".to_string(),
                            "ollama" => "llama2".to_string(),
                            "gemini" => "gemini-pro".to_string(),
                            "claude" => "claude-3-5-sonnet-latest".to_string(),
                            "minimax" => "MiniMax-M1-80k".to_string(),
                            _ => "default".to_string(),
                        }
                    };

                    // Update config
                    self.config.models.default = model.clone();
                    // We might need to store the active provider separately if it's not just inferred from the model
                    // For now, we assume the environment variable PROMPTLINE_PROVIDER drives it,
                    // OR we update the config to explicitly store the active provider.
                    // The current Config struct doesn't have an "active_provider" field, it just has "default" model.
                    // But main.rs uses PROMPTLINE_PROVIDER env var.
                    // We should probably update main.rs to look at config first.

                    // Let's assume we set the env var for the current process so main.rs picks it up?
                    // Or better, we persist it in the config if we add a field.
                    // For now, let's just set the default model.

                    // To switch provider, we really need to know which provider "gpt-4" belongs to.
                    // Let's update the config to have an `active_provider` field?
                    // Or just rely on the user setting it.

                    // Actually, let's set the env var for this process
                    std::env::set_var("PROMPTLINE_PROVIDER", &provider);

                    // Save config
                    if let Ok(path) = self.get_config_path() {
                        let _ = self.config.save_to_file(&path);
                    }

                    return Ok(CommandOutput::new(format!(
                        "✓ Switched to {} ({})",
                        provider, model
                    ))
                    .with_action(CommandAction::ReloadAgent));
                }
                "config" => {
                    // /model config <provider> key <value>
                    // /model config <provider> url <value>
                    if args.len() < 4 {
                        return Ok(CommandOutput::new(
                            "Usage: /model config <provider> <key|url> <value>",
                        ));
                    }
                    let provider_name = args[1].to_lowercase();
                    let setting = args[2].to_lowercase();
                    let value = args[3].clone();

                    let provider_config = self
                        .config
                        .models
                        .providers
                        .entry(provider_name.clone())
                        .or_insert_with(|| crate::config::ProviderConfig {
                            api_key: None,
                            models: vec![],
                            default_params: crate::config::ModelParams::default(),
                            base_url: None,
                        });

                    match setting.as_str() {
                        "key" | "api_key" => {
                            provider_config.api_key = Some(value);
                        }
                        "url" | "base_url" | "endpoint" => {
                            provider_config.base_url = Some(value);
                        }
                        _ => return Ok(CommandOutput::new("Invalid setting. Use 'key' or 'url'.")),
                    }

                    // Save config
                    if let Ok(path) = self.get_config_path() {
                        let _ = self.config.save_to_file(&path);
                    }

                    return Ok(CommandOutput::new(format!(
                        "✓ Updated {} configuration",
                        provider_name
                    ))
                    .with_action(CommandAction::ReloadAgent));
                }
                _ => {
                    return Ok(CommandOutput::new(format!(
                        "Unknown subcommand: {}",
                        subcommand
                    )))
                }
            }
        }

        Ok(CommandOutput::new(self.model_info()))
    }

    fn get_config_path(&self) -> Result<std::path::PathBuf> {
        // Try to find where to save
        if let Some(mut dir) = dirs::config_dir() {
            dir.push("promptline");
            std::fs::create_dir_all(&dir)?;
            dir.push("config.yaml");
            Ok(dir)
        } else {
            Ok(std::path::PathBuf::from(".promptline/config.yaml"))
        }
    }

    /// Show model info
    fn model_info(&self) -> String {
        let provider =
            std::env::var("PROMPTLINE_PROVIDER").unwrap_or_else(|_| "openai".to_string());
        format!(
            "\n🤖 Model Information\n\nProvider: {}\nDefault Model: {}\n\nUsage:\n  /model set <provider> [model]\n  /model config <provider> key <value>\n  /model config <provider> url <value>\n",
            provider,
            self.config.models.default
        )
    }

    fn handle_plan(&self, args: Option<Vec<String>>) -> Result<CommandOutput> {
        let Some(args) = args else {
            return Ok(CommandOutput::new("Usage: /plan <task>"));
        };

        if args.is_empty() {
            return Ok(CommandOutput::new("Usage: /plan <task>"));
        }

        let task = args.join(" ");
        Ok(
            CommandOutput::new(format!("Designing a read-only plan for: {}", task))
                .with_action(CommandAction::DesignPlan(task)),
        )
    }

    fn handle_skills(&self, args: Option<Vec<String>>) -> Result<CommandOutput> {
        let Some(manager) = &self.skill_manager else {
            return Ok(CommandOutput::new(
                "Skills are unavailable in this session.",
            ));
        };

        let subcommand = args
            .as_ref()
            .and_then(|args| args.first())
            .map(|arg| arg.to_lowercase())
            .unwrap_or_else(|| "list".to_string());

        match subcommand.as_str() {
            "list" => Ok(CommandOutput::new(self.skills_list(manager))),
            "errors" => Ok(CommandOutput::new(self.skills_errors(manager))),
            "reload" => {
                Ok(CommandOutput::new("Reloading skills...")
                    .with_action(CommandAction::ReloadSkills))
            }
            other => Ok(CommandOutput::new(format!(
                "Unknown skills subcommand: {}\nUsage: /skills [list|errors|reload]",
                other
            ))),
        }
    }

    fn skills_list(&self, manager: &SkillManager) -> String {
        let skills = manager.skills();
        if skills.is_empty() {
            return "\nNo skills loaded. Add SKILL.md directories under .promptline/skills or configure skills.roots.".to_string();
        }

        let mut output = String::from("\nSkills\n\n");
        for skill in skills {
            output.push_str(&format!(
                "  • {} [{}]\n    {}\n    {}\n",
                skill.name,
                skill.scope,
                skill.display_description(),
                skill.skill_md_path.display()
            ));
        }
        output
    }

    fn skills_errors(&self, manager: &SkillManager) -> String {
        let errors = manager.errors();
        if errors.is_empty() {
            return "\nNo skill load errors.".to_string();
        }

        let mut output = String::from("\nSkill load errors\n\n");
        for error in errors {
            output.push_str(&format!(
                "  • {}\n    {}\n",
                error.path.display(),
                error.message
            ));
        }
        output
    }

    /// Handle permissions command
    fn handle_permissions(&self, args: Option<Vec<String>>) -> Result<String> {
        if let Some(args) = args {
            if args.len() >= 2 {
                let tool = &args[0];
                let level_str = &args[1].to_lowercase();

                let level = match level_str.as_str() {
                    "allow" | "always" => crate::permissions::PermissionLevel::Always,
                    "deny" | "never" | "block" => crate::permissions::PermissionLevel::Never,
                    "ask" | "prompt" => crate::permissions::PermissionLevel::Ask,
                    "once" => crate::permissions::PermissionLevel::Once,
                    _ => {
                        return Ok(format!(
                            "Invalid permission level: {}. Use always, never, ask, or once.",
                            level_str
                        ))
                    }
                };

                let mut perms = self.permissions.lock().unwrap();
                perms.set_permission(tool.clone(), level.clone())?;

                return Ok(format!("✓ Set permission for '{}' to {:?}", tool, level));
            }
        }

        // If no args or invalid args, show info
        Ok(self.permissions_info())
    }

    /// Show permissions info
    fn permissions_info(&self) -> String {
        let perms = self.permissions.lock().unwrap();
        let all_perms = perms.get_all_permissions();
        let mut output = String::from("\n🔐 Tool Permissions\n\n");

        if all_perms.is_empty() {
            output.push_str("No custom permissions set. All tools will prompt for permission.\n");
        } else {
            for (tool, level) in all_perms {
                let icon = match level {
                    crate::permissions::PermissionLevel::Always => "✓",
                    crate::permissions::PermissionLevel::Never => "✗",
                    _ => "?",
                };
                output.push_str(&format!("  {} {}: {:?}\n", icon, tool, level));
            }
        }

        output.push_str("\nUsage: /permissions [tool] [level]\n");
        output.push_str("Levels: always, never, ask, once\n");

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_commands() {
        assert_eq!(CommandHandler::parse("/help"), Some(SlashCommand::Help));
        assert_eq!(CommandHandler::parse("/quit"), Some(SlashCommand::Quit));
        assert_eq!(CommandHandler::parse("/h"), Some(SlashCommand::Help));
        assert_eq!(
            CommandHandler::parse("/plan add auth"),
            Some(SlashCommand::Plan(Some(vec![
                "add".to_string(),
                "auth".to_string()
            ])))
        );
        assert_eq!(CommandHandler::parse("not a command"), None);
    }

    #[test]
    fn test_plan_command_action() {
        let permissions = Arc::new(Mutex::new(PermissionManager::new().unwrap()));
        let mut handler = CommandHandler::new(Config::default(), permissions);
        let output = handler
            .execute(SlashCommand::Plan(Some(vec![
                "add".to_string(),
                "auth".to_string(),
            ])))
            .unwrap();

        assert_eq!(
            output.action,
            CommandAction::DesignPlan("add auth".to_string())
        );
    }

    #[test]
    fn help_mentions_plan_command() {
        let permissions = Arc::new(Mutex::new(PermissionManager::new().unwrap()));
        let handler = CommandHandler::new(Config::default(), permissions);
        assert!(handler.help().contains("/plan <task>"));
    }
}

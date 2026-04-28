//! REPL session management for interactive chat mode

use std::io::{self, Write};

use crate::agent::adapter::agent_react::AgentReAct;
use crate::agent::mode::AgentMode;
use crate::app::behavior::Agent;
use crate::commands::CommandHandler;
use crate::config::Config;
use crate::repl::ReplHelper;
use crate::setup;
use crate::skill::SkillManager;
use std::sync::Arc;

pub struct ReplSession {
    agent: AgentReAct,
    command_handler: CommandHandler,
    config: Config,
    skill_manager: Arc<SkillManager>,
}

impl ReplSession {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let model = setup::create_model(&config)?;
        let tools = setup::create_tools();
        let permission_manager = setup::create_permission_manager()?;
        let skill_manager = setup::create_skill_manager(&config)?;

        let agent = AgentReAct::new_with_skills(
            model,
            tools,
            config.clone(),
            Vec::new(),
            permission_manager.clone(),
            Some(skill_manager.clone()),
        )
        .await?;

        let command_handler = CommandHandler::new_with_skills(
            config.clone(),
            permission_manager,
            Some(skill_manager.clone()),
        );

        Ok(Self {
            agent,
            command_handler,
            config,
            skill_manager,
        })
    }

    async fn run_design_plan(&self, task: String) -> anyhow::Result<()> {
        let model = setup::create_model(&self.config)?;
        let tools = setup::create_tools();
        let permission_manager = setup::create_permission_manager()?;
        let mut agent = AgentReAct::new_with_skills_and_mode(
            model,
            tools,
            self.config.clone(),
            Vec::new(),
            permission_manager,
            Some(self.skill_manager.clone()),
            AgentMode::Plan,
        )
        .await?;

        match agent.execute_task(task).await {
            Ok(result) => {
                if !result.output.is_empty() {
                    println!("{}\n", agent.format_response(&result.output));
                }
            }
            Err(e) => {
                eprintln!("\n\x1b[1;31mError:\x1b[0m {}\n", e);
            }
        }

        Ok(())
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let helper = ReplHelper::new();
        let repl_config = rustyline::Config::builder()
            .completion_type(rustyline::CompletionType::List)
            .edit_mode(rustyline::EditMode::Emacs)
            .build();

        let mut editor = rustyline::Editor::with_config(repl_config)?;
        editor.set_helper(Some(helper));

        let history_path = dirs::home_dir().map(|d| d.join(".promptline").join("history.txt"));

        if let Some(path) = &history_path {
            if path.exists() {
                let _ = editor.load_history(path);
            }
        }

        loop {
            let readline = editor.readline("→ ~ ");

            match readline {
                Ok(line) => {
                    let input = line.trim();
                    editor.add_history_entry(input)?;

                    if input.is_empty() {
                        continue;
                    }

                    if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                        println!("\n👋 Goodbye!");
                        break;
                    }

                    if let Some(command) = CommandHandler::parse(input) {
                        match self.command_handler.execute(command) {
                            Ok(output) => {
                                println!("{}", output.message);
                                match output.action {
                                    crate::commands::CommandAction::Quit => return Ok(()),
                                    crate::commands::CommandAction::ClearHistory => {
                                        self.agent.clear_conversation_history();
                                        println!("✓ Session cleared");
                                    }
                                    crate::commands::CommandAction::ReloadAgent => {
                                        println!("↻ Reloading agent...");
                                        return Ok(());
                                    }
                                    crate::commands::CommandAction::ReloadSkills => {
                                        let cwd = std::env::current_dir()?;
                                        let outcome =
                                            self.skill_manager.reload(&self.config, &cwd)?;
                                        println!(
                                            "✓ Reloaded {} skills ({} errors)",
                                            outcome.skills.len(),
                                            outcome.errors.len()
                                        );
                                    }
                                    crate::commands::CommandAction::DesignPlan(task) => {
                                        self.run_design_plan(task).await?;
                                    }
                                    crate::commands::CommandAction::None => {}
                                }
                                continue;
                            }
                            Err(e) => {
                                eprintln!("\x1b[1;31mError:\x1b[0m {}", e);
                                continue;
                            }
                        }
                    } else if input.starts_with('/') {
                        println!("\x1b[1;33mUnknown command:\x1b[0m {}", input);
                        println!("Type /help for available commands");
                        continue;
                    }

                    print!("\n\x1b[1;34mPromptLine:\x1b[0m ");
                    io::stdout().flush()?;

                    match self.agent.execute_task(input.to_string()).await {
                        Ok(result) => {
                            let response_content = &result.output;

                            if !response_content.is_empty() && response_content != "FINISH" {
                                let formatted = self.agent.format_response(response_content);
                                println!("{}\n", formatted);
                            }
                        }
                        Err(e) => {
                            eprintln!("\n\x1b[1;31mError:\x1b[0m {}\n", e);
                        }
                    }
                }
                Err(rustyline::error::ReadlineError::Interrupted) => {
                    println!("^C");
                    break;
                }
                Err(rustyline::error::ReadlineError::Eof) => {
                    println!("^D");
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    break;
                }
            }
        }

        if let Some(path) = &history_path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let _ = editor.save_history(path);
        }

        Ok(())
    }
}

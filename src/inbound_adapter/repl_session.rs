//! REPL session management for interactive chat mode

use std::io::{self, Write};

use crate::agent::behavior::agent::{AgentBehavior, AgentBehaviorImpl};
use crate::commands::CommandHandler;
use crate::Config;
use crate::repl::ReplHelper;
use crate::setup;

pub struct ReplSession {
    agent: AgentBehaviorImpl,
    command_handler: CommandHandler,
}

impl ReplSession {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let model = setup::create_model(&config)?;
        let tools = setup::create_tools();
        let permission_manager = setup::create_permission_manager()?;

        let agent = AgentBehaviorImpl::new(model, tools, config.clone(), Vec::new(), permission_manager.clone()).await?;

        let command_handler = CommandHandler::new(config, permission_manager);

        Ok(Self {
            agent,
            command_handler,
        })
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

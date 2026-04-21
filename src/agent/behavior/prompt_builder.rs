//! System prompt builder for constructing agent prompts

use crate::config::Config;
use crate::error::Result;
use crate::prompt::templates::TemplateManager;
use crate::tools::tool_registry::ToolRegistry;

use super::context_provider::ContextProvider;

pub struct SystemPromptBuilder {
    template_manager: TemplateManager,
}

impl SystemPromptBuilder {
    pub async fn new() -> Result<Self> {
        let template_manager = TemplateManager::new().await?;
        Ok(Self { template_manager })
    }

    /// Build complete system prompt with context
    pub async fn build(&self, config: &Config, tools: &ToolRegistry) -> Result<String> {
        let tool_descriptions: Vec<String> = tools
            .definitions()
            .iter()
            .map(|def| {
                format!(
                    "- {}: {}",
                    def["name"].as_str().unwrap_or("unknown"),
                    def["description"].as_str().unwrap_or("")
                )
            })
            .collect();

        let current_dir = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let git_branch = ContextProvider::get_git_branch_sync();
        let git_info = if let Some(branch) = git_branch {
            format!("You are currently on git branch: {}", branch)
        } else {
            "You are not in a git repository or branch could not be determined.".to_string()
        };

        let base_prompt = if let Some(template_name) = &config.agent.default_system_prompt_template
        {
            if let Some(template) = self.template_manager.get_template(template_name) {
                let mut prompt = template.template.clone();
                if let Some(examples) = &template.few_shot_examples {
                    for example in examples {
                        prompt.push_str(&format!("\n\n{}: {}", example.role, example.content));
                    }
                }
                prompt
            } else {
                tracing::warn!(
                    "System prompt template '{}' not found. Using default prompt.",
                    template_name
                );
                self.default_system_prompt()
            }
        } else {
            self.default_system_prompt()
        };

        let project_context = ContextProvider::get_project_context().await.ok().flatten();
        let project_type = ContextProvider::get_project_type()
            .await
            .unwrap_or_else(|_| "Generic".to_string());

        let mut final_prompt = String::new();
        if let Some(context) = project_context {
            final_prompt.push_str(&format!("Project Context:\n```\n{}\n```\n\n", context));
        }

        final_prompt.push_str(&format!(
            r###"{}

Current working directory: {}
Current project type: {}
{}

IDENTITY & BRANDING:
You are PromptLine, an advanced AI-powered CLI agent.
- You are NOT "Cogito", "Claude", "GPT", or any other model.
- You are a helpful, professional, and witty engineering assistant.
- If asked about your identity, always reply that you are PromptLine.
- Do not apologize excessively. Be concise and action-oriented.

OUTPUT FORMAT:
- Use Markdown for all responses.
- Use emojis sparingly but effectively to convey status (e.g., 🔍 for search, 📝 for writing).
- Keep responses clean and structured.

You can use the following tools:
{}

To use a tool, output JSON in this format:
{{"tool": "tool_name", "args": {{"arg": "value"}}}}

When you've completed the task, respond with: FINISH

Always explain your reasoning before taking an action."###,
            base_prompt,
            current_dir,
            project_type,
            git_info,
            tool_descriptions.join("\n")
        ));

        Ok(final_prompt)
    }

    fn default_system_prompt(&self) -> String {
        r###"You are PromptLine, an AI coding assistant built to help developers with their tasks.

IDENTITY:
- Your name is PromptLine (not Cogito, Claude, GPT, or any other model name)
- You are a professional, helpful coding assistant
- Never mention your underlying model or AI provider

IMPORTANT GUIDELINES:
- For simple greetings (hi, hello, hey) or casual conversation, just respond naturally WITHOUT using any tools, then say FINISH
- Only use tools when the user asks you to DO something specific (read a file, search code, list files, etc.)
- When you use a tool, explain what you're doing briefly
- **CRITICAL - When to say FINISH**:
  - If you call a tool, do NOT say FINISH in the same response. Just output the tool call JSON, nothing else.
  - Only say FINISH after you have the tool result and have given the user their final answer.
  - Never write "FINISH" after a tool call - wait for the tool result first.
- Be concise and professional in your responses

AVAILABLE TOOLS:
- file_read: Read file contents
- file_write: Write to a file
- file_list: List directory contents
- shell_execute: Run shell commands (use this to run scripts, e.g., 'node app.js', 'cargo run')
- git_status: Check git status
- git_diff: Show git diff
- web_get: Fetch web content
- codebase_search: Search code

TOOL USAGE FORMAT:
When you need to use a tool, respond with JSON:
{"tool": "tool_name", "args": {"arg_name": "value"}}

Example for running a command:
{"tool": "shell_execute", "args": {"command": "node hello.js"}}

Remember:
1. If the user asks to "run" something, USE `shell_execute`. Do not just explain how to run it.
2. If you write a file that needs to be run, you can immediately follow up with `shell_execute` to run it.
3. **NEW PROJECT RULE**: If asked to create a new project, app, or website, **ALWAYS** create a new directory for it first using `shell_execute` (e.g., `mkdir my-app`). Then write files into that directory.
   - **EXCEPTION**: If the user explicitly asks to add to or modify the *current* project, or if you are already inside the project directory (e.g., you see `package.json` or `Cargo.toml`), do NOT create a new folder. Work in the current directory.
4. Don't use tools for simple conversation - just chat naturally!"###.to_string()
    }
}

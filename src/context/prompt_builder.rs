//! System prompt builder for constructing agent prompts

use crate::agent::mode::AgentMode;
use crate::config::Config;
use crate::error::Result;
use crate::prompt::templates::TemplateManager;
use crate::skill::model::SkillPromptContext;
use crate::tool::tool_registry::ToolRegistry;

use crate::context::context_provider::ContextProvider;

const DEFAULT_SYSTEM_PROMPT: &str = include_str!("../../prompts/system_prompt.md");
const EXECUTE_MODE_PROMPT: &str = include_str!("../../prompts/execute_mode.md");
const PLAN_MODE_PROMPT: &str = include_str!("../../prompts/plan_mode.md");
const SYSTEM_PROMPT_WRAPPER: &str = include_str!("../../prompts/system_prompt_wrapper.md");

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
        self.build_with_mode(config, tools, AgentMode::Execute)
            .await
    }

    pub async fn build_with_mode(
        &self,
        config: &Config,
        tools: &ToolRegistry,
        mode: AgentMode,
    ) -> Result<String> {
        self.build_with_skills_and_mode(config, tools, None, mode)
            .await
    }

    /// Build complete system prompt with optional skill context
    pub async fn build_with_skills(
        &self,
        config: &Config,
        tools: &ToolRegistry,
        skill_context: Option<&SkillPromptContext>,
    ) -> Result<String> {
        self.build_with_skills_and_mode(config, tools, skill_context, AgentMode::Execute)
            .await
    }

    pub async fn build_with_skills_and_mode(
        &self,
        config: &Config,
        tools: &ToolRegistry,
        skill_context: Option<&SkillPromptContext>,
        mode: AgentMode,
    ) -> Result<String> {
        let tool_descriptions: Vec<String> = tools
            .definitions_with_metadata()
            .into_iter()
            .filter(|def| !mode.is_plan() || def.read_only)
            .map(|def| {
                let params = serde_json::to_string_pretty(&def.parameters).unwrap_or_default();
                format!(
                    "- {}: {}\n  Parameters: {}",
                    def.name, def.description, params
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

        let mut skills_section = String::new();
        if let Some(context) = skill_context {
            if let Some(block) = &context.available_skills_block {
                skills_section.push_str(block);
                skills_section.push('\n');
            }
            for block in &context.explicit_skill_blocks {
                skills_section.push_str(block);
                skills_section.push('\n');
            }
            if !context.warnings.is_empty() {
                skills_section.push_str("<skill_warnings>\n");
                for warning in &context.warnings {
                    skills_section.push_str("- ");
                    skills_section.push_str(warning);
                    skills_section.push('\n');
                }
                skills_section.push_str("</skill_warnings>\n\n");
            }
        }

        let mode_instructions = match mode {
            AgentMode::Execute => EXECUTE_MODE_PROMPT,
            AgentMode::Plan => PLAN_MODE_PROMPT,
        };

        let mut final_prompt = String::new();
        if let Some(context) = project_context {
            final_prompt.push_str(&format!("Project Context:\n```\n{}\n```\n\n", context));
        }

        let wrapper = SYSTEM_PROMPT_WRAPPER
            .replace("{{base_prompt}}", &base_prompt)
            .replace("{{current_dir}}", &current_dir)
            .replace("{{project_type}}", &project_type)
            .replace("{{git_info}}", &git_info)
            .replace("{{skills_section}}", &skills_section)
            .replace("{{mode_instructions}}", mode_instructions)
            .replace("{{tool_descriptions}}", &tool_descriptions.join("\n"));

        final_prompt.push_str(&wrapper);

        Ok(final_prompt)
    }

    fn default_system_prompt(&self) -> String {
        DEFAULT_SYSTEM_PROMPT.to_string()
    }
}

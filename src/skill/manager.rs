use crate::config::Config;
use crate::skill::injection::{build_explicit_skill_injections, render_skill_injection};
use crate::skill::loader::load_skills_from_roots;
use crate::skill::model::{
    SkillError, SkillLoadOutcome, SkillPromptContext, SkillRoot, SkillScope,
};
use crate::skill::render::render_available_skills;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[derive(Debug, Default)]
pub struct SkillManager {
    cache: RwLock<Option<Arc<SkillLoadOutcome>>>,
}

impl SkillManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_for_config(&self, config: &Config, cwd: &Path) -> Result<Arc<SkillLoadOutcome>> {
        let roots = skill_roots_for_config(config, cwd);
        let outcome = if config.skills.enabled {
            load_skills_from_roots(&roots, &config.skills.disabled)
        } else {
            SkillLoadOutcome::default()
        };
        let outcome = Arc::new(outcome);
        *self.cache.write().unwrap() = Some(outcome.clone());
        Ok(outcome)
    }

    pub fn reload(&self, config: &Config, cwd: &Path) -> Result<Arc<SkillLoadOutcome>> {
        self.load_for_config(config, cwd)
    }

    pub fn cached(&self) -> Option<Arc<SkillLoadOutcome>> {
        self.cache.read().unwrap().clone()
    }

    pub fn render_prompt_context(&self, config: &Config, user_input: &str) -> SkillPromptContext {
        let Some(outcome) = self.cached() else {
            return SkillPromptContext::default();
        };

        let available_skills_block =
            render_available_skills(&outcome, config.skills.metadata_char_budget);
        let (injections, mut warnings) = build_explicit_skill_injections(
            user_input,
            &outcome,
            config.skills.injection_char_budget,
        );
        let explicit_skill_blocks = injections.iter().map(render_skill_injection).collect();

        if !outcome.errors.is_empty() {
            warnings.push(format!(
                "{} skill file(s) failed to load. Use /skills errors for details.",
                outcome.errors.len()
            ));
        }

        SkillPromptContext {
            available_skills_block,
            explicit_skill_blocks,
            warnings,
        }
    }

    pub fn skills(&self) -> Vec<crate::skill::model::SkillMetadata> {
        self.cached()
            .map(|outcome| outcome.skills.clone())
            .unwrap_or_default()
    }

    pub fn errors(&self) -> Vec<SkillError> {
        self.cached()
            .map(|outcome| outcome.errors.clone())
            .unwrap_or_default()
    }
}

pub fn skill_roots_for_config(config: &Config, cwd: &Path) -> Vec<SkillRoot> {
    let mut roots = Vec::new();

    if config.skills.project_skills {
        roots.push(SkillRoot {
            path: cwd.join(".promptline").join("skills"),
            scope: SkillScope::Project,
        });
    }

    for root in &config.skills.roots {
        roots.push(SkillRoot {
            path: expand_home(root),
            scope: SkillScope::Config,
        });
    }

    if config.skills.user_skills {
        if let Some(home) = dirs::home_dir() {
            roots.push(SkillRoot {
                path: home.join(".promptline").join("skills"),
                scope: SkillScope::User,
            });
        }
    }

    roots
}

fn expand_home(path: &Path) -> PathBuf {
    let Some(path_str) = path.to_str() else {
        return path.to_path_buf();
    };
    if path_str == "~" {
        return dirs::home_dir().unwrap_or_else(|| path.to_path_buf());
    }
    if let Some(rest) = path_str.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    path.to_path_buf()
}

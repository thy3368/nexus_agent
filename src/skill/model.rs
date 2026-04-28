use std::collections::HashSet;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkillScope {
    Project,
    Config,
    User,
    Bundled,
}

impl SkillScope {
    pub fn priority(self) -> usize {
        match self {
            SkillScope::Project => 0,
            SkillScope::Config => 1,
            SkillScope::User => 2,
            SkillScope::Bundled => 3,
        }
    }
}

impl fmt::Display for SkillScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkillScope::Project => write!(f, "project"),
            SkillScope::Config => write!(f, "config"),
            SkillScope::User => write!(f, "user"),
            SkillScope::Bundled => write!(f, "bundled"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillRoot {
    pub path: PathBuf,
    pub scope: SkillScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub short_description: Option<String>,
    pub skill_dir: PathBuf,
    pub skill_md_path: PathBuf,
    pub scope: SkillScope,
}

impl SkillMetadata {
    pub fn display_description(&self) -> &str {
        self.short_description
            .as_deref()
            .unwrap_or(self.description.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillDocument {
    pub metadata: SkillMetadata,
    pub body: String,
    pub raw_content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillError {
    pub path: PathBuf,
    pub message: String,
}

impl SkillError {
    pub fn new(path: PathBuf, message: impl Into<String>) -> Self {
        Self {
            path,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillLoadOutcome {
    pub skills: Vec<SkillMetadata>,
    pub errors: Vec<SkillError>,
    pub disabled_paths: HashSet<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillPromptContext {
    pub available_skills_block: Option<String>,
    pub explicit_skill_blocks: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillInjection {
    pub name: String,
    pub path: PathBuf,
    pub contents: String,
}

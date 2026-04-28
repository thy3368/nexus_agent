use crate::skill::model::{SkillMetadata, SkillScope};
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const MAX_NAME_LEN: usize = 64;
pub const MAX_DESCRIPTION_LEN: usize = 1024;
pub const MAX_SHORT_DESCRIPTION_LEN: usize = 1024;

#[derive(Debug, Deserialize)]
pub struct SkillFrontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub metadata: Option<SkillFrontmatterMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct SkillFrontmatterMetadata {
    #[serde(rename = "short-description")]
    pub short_description: Option<String>,
}

pub fn extract_frontmatter(markdown: &str) -> Result<(&str, &str)> {
    let markdown = markdown.strip_prefix('\u{feff}').unwrap_or(markdown);
    let Some(rest) = markdown.strip_prefix("---") else {
        return Err(anyhow!("SKILL.md must start with YAML frontmatter"));
    };

    let rest = rest
        .strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'))
        .ok_or_else(|| anyhow!("SKILL.md frontmatter opening delimiter must be on its own line"))?;

    let delimiter = rest
        .find("\n---")
        .ok_or_else(|| anyhow!("SKILL.md frontmatter is missing closing delimiter"))?;
    let frontmatter = &rest[..delimiter];
    let after_delimiter = &rest[delimiter + 4..];
    let body = after_delimiter
        .strip_prefix("\r\n")
        .or_else(|| after_delimiter.strip_prefix('\n'))
        .unwrap_or(after_delimiter);

    Ok((frontmatter, body))
}

pub fn parse_frontmatter(markdown: &str) -> Result<(SkillFrontmatter, &str)> {
    let (frontmatter, body) = extract_frontmatter(markdown)?;
    let parsed = serde_yaml::from_str(frontmatter)?;
    Ok((parsed, body))
}

pub fn validate_skill_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("skill name is required"));
    }
    if name.len() > MAX_NAME_LEN {
        return Err(anyhow!("skill name exceeds {} characters", MAX_NAME_LEN));
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphanumeric() {
        return Err(anyhow!(
            "skill name must start with an ASCII letter or digit"
        ));
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(anyhow!(
            "skill name may only contain ASCII letters, digits, underscores, and hyphens"
        ));
    }

    Ok(())
}

fn validate_single_line(field: &str, value: &str, max_len: usize) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow!("{} is required", field));
    }
    if value.len() > max_len {
        return Err(anyhow!("{} exceeds {} characters", field, max_len));
    }
    Ok(value.split_whitespace().collect::<Vec<_>>().join(" "))
}

pub fn metadata_from_frontmatter(
    frontmatter: SkillFrontmatter,
    skill_dir: PathBuf,
    skill_md_path: PathBuf,
    scope: SkillScope,
) -> Result<SkillMetadata> {
    let name = frontmatter
        .name
        .ok_or_else(|| anyhow!("skill frontmatter is missing name"))?;
    let name = validate_single_line("skill name", &name, MAX_NAME_LEN)?;
    validate_skill_name(&name)?;

    let description = frontmatter
        .description
        .ok_or_else(|| anyhow!("skill frontmatter is missing description"))?;
    let description = validate_single_line("skill description", &description, MAX_DESCRIPTION_LEN)?;

    let short_description = frontmatter
        .metadata
        .and_then(|m| m.short_description)
        .map(|value| {
            validate_single_line("skill short-description", &value, MAX_SHORT_DESCRIPTION_LEN)
        })
        .transpose()?;

    Ok(SkillMetadata {
        name,
        description,
        short_description,
        skill_dir,
        skill_md_path,
        scope,
    })
}

pub fn parse_skill_metadata(
    content: &str,
    skill_md_path: &Path,
    scope: SkillScope,
) -> Result<(SkillMetadata, String)> {
    let (frontmatter, body) = parse_frontmatter(content)?;
    let skill_dir = skill_md_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("SKILL.md has no parent directory"))?;
    let metadata =
        metadata_from_frontmatter(frontmatter, skill_dir, skill_md_path.to_path_buf(), scope)?;
    Ok((metadata, body.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_frontmatter() {
        let markdown = "---\nname: simple-checklist\ndescription: Adds checklist\nmetadata:\n  short-description: Tiny checklist\n---\n\nBody";
        let (frontmatter, body) = parse_frontmatter(markdown).unwrap();
        assert_eq!(frontmatter.name.as_deref(), Some("simple-checklist"));
        assert_eq!(frontmatter.description.as_deref(), Some("Adds checklist"));
        assert_eq!(
            frontmatter
                .metadata
                .and_then(|metadata| metadata.short_description)
                .as_deref(),
            Some("Tiny checklist")
        );
        assert_eq!(body, "\nBody");
    }

    #[test]
    fn rejects_missing_frontmatter() {
        assert!(parse_frontmatter("name: nope").is_err());
    }

    #[test]
    fn rejects_invalid_name() {
        assert!(validate_skill_name("bad name").is_err());
        assert!(validate_skill_name("-bad").is_err());
        assert!(validate_skill_name("good-name_1").is_ok());
    }

    #[test]
    fn rejects_missing_required_fields() {
        let content = "---\nname: test\n---\nbody";
        let (frontmatter, _) = parse_frontmatter(content).unwrap();
        let result = metadata_from_frontmatter(
            frontmatter,
            PathBuf::from("skill"),
            PathBuf::from("skill/SKILL.md"),
            SkillScope::Project,
        );
        assert!(result.is_err());
    }
}

use crate::skill::matching::extract_skill_mentions;
use crate::skill::model::{SkillInjection, SkillLoadOutcome, SkillMetadata};
use std::collections::HashMap;
use std::fs;

pub fn build_explicit_skill_injections(
    input: &str,
    outcome: &SkillLoadOutcome,
    char_budget: usize,
) -> (Vec<SkillInjection>, Vec<String>) {
    let mentions = extract_skill_mentions(input);
    if mentions.is_empty() || char_budget == 0 {
        return (Vec::new(), Vec::new());
    }

    let by_name = skills_by_name(&outcome.skills);
    let mut injections = Vec::new();
    let mut warnings = Vec::new();
    let mut used_chars = 0usize;

    for mention in mentions {
        let Some(matches) = by_name.get(mention.as_str()) else {
            continue;
        };
        if matches.len() > 1 {
            warnings.push(format!(
                "Skill mention ${} is ambiguous; {} enabled skills have that name.",
                mention,
                matches.len()
            ));
            continue;
        }

        let skill = matches[0];
        match fs::read_to_string(&skill.skill_md_path) {
            Ok(mut contents) => {
                let remaining = char_budget.saturating_sub(used_chars);
                if remaining == 0 {
                    warnings.push(format!(
                        "Skill ${} was omitted because the skill injection budget was exhausted.",
                        mention
                    ));
                    continue;
                }
                if contents.len() > remaining {
                    contents.truncate(remaining);
                    warnings.push(format!(
                        "Skill ${} was truncated to fit the skill injection budget.",
                        mention
                    ));
                }
                used_chars += contents.len();
                injections.push(SkillInjection {
                    name: skill.name.clone(),
                    path: skill.skill_md_path.clone(),
                    contents,
                });
            }
            Err(error) => warnings.push(format!(
                "Failed to read skill ${} at {}: {}",
                mention,
                skill.skill_md_path.display(),
                error
            )),
        }
    }

    (injections, warnings)
}

pub fn render_skill_injection(injection: &SkillInjection) -> String {
    format!(
        "<skill name=\"{}\" path=\"{}\">\n{}\n</skill>\n",
        injection.name,
        injection.path.display(),
        injection.contents
    )
}

fn skills_by_name(skills: &[SkillMetadata]) -> HashMap<&str, Vec<&SkillMetadata>> {
    let mut by_name: HashMap<&str, Vec<&SkillMetadata>> = HashMap::new();
    for skill in skills {
        by_name.entry(skill.name.as_str()).or_default().push(skill);
    }
    by_name
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::model::{SkillLoadOutcome, SkillMetadata, SkillScope};
    use std::fs;
    use tempfile::tempdir;

    fn metadata(name: &str, path: std::path::PathBuf) -> SkillMetadata {
        SkillMetadata {
            name: name.to_string(),
            description: "desc".to_string(),
            short_description: None,
            skill_dir: path.parent().unwrap().to_path_buf(),
            skill_md_path: path,
            scope: SkillScope::Project,
        }
    }

    #[test]
    fn injects_explicit_skill() {
        let temp = tempdir().unwrap();
        let skill_path = temp.path().join("SKILL.md");
        fs::write(
            &skill_path,
            "---\nname: simple\ndescription: desc\n---\nbody",
        )
        .unwrap();
        let outcome = SkillLoadOutcome {
            skills: vec![metadata("simple", skill_path)],
            ..Default::default()
        };

        let (injections, warnings) = build_explicit_skill_injections("Use $simple", &outcome, 1000);
        assert!(warnings.is_empty());
        assert_eq!(injections.len(), 1);
        assert!(render_skill_injection(&injections[0]).contains("<skill name=\"simple\""));
    }

    #[test]
    fn avoids_ambiguous_skill_name() {
        let temp = tempdir().unwrap();
        let first = temp.path().join("one/SKILL.md");
        let second = temp.path().join("two/SKILL.md");
        fs::create_dir_all(first.parent().unwrap()).unwrap();
        fs::create_dir_all(second.parent().unwrap()).unwrap();
        fs::write(&first, "body").unwrap();
        fs::write(&second, "body").unwrap();
        let outcome = SkillLoadOutcome {
            skills: vec![metadata("same", first), metadata("same", second)],
            ..Default::default()
        };

        let (injections, warnings) = build_explicit_skill_injections("Use $same", &outcome, 1000);
        assert!(injections.is_empty());
        assert_eq!(warnings.len(), 1);
    }
}

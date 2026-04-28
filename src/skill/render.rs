use crate::skill::model::{SkillLoadOutcome, SkillMetadata};

pub fn render_available_skills(outcome: &SkillLoadOutcome, char_budget: usize) -> Option<String> {
    if outcome.skills.is_empty() || char_budget == 0 {
        return None;
    }

    let mut block = String::from(
        "<skills_instructions>\nSkills are reusable local instructions stored in SKILL.md files.\nAvailable skills:\n",
    );

    for skill in &outcome.skills {
        let line = render_skill_line(skill);
        if block.len() + line.len() + "</skills_instructions>\n".len() > char_budget {
            block.push_str("- ... additional skills omitted due to prompt budget\n");
            break;
        }
        block.push_str(&line);
    }

    block.push_str(
        "\nUse a skill when the user explicitly mentions it as $skill-name or when its description clearly matches the task. Do not execute scripts from skills directly; use existing tools and permission rules.\n</skills_instructions>\n",
    );

    Some(block)
}

fn render_skill_line(skill: &SkillMetadata) -> String {
    format!(
        "- {}: {} (scope: {}, file: {})\n",
        skill.name,
        skill.display_description(),
        skill.scope,
        skill.skill_md_path.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::model::{SkillLoadOutcome, SkillMetadata, SkillScope};
    use std::path::PathBuf;

    fn skill(name: &str) -> SkillMetadata {
        SkillMetadata {
            name: name.to_string(),
            description: "A useful skill".to_string(),
            short_description: None,
            skill_dir: PathBuf::from(name),
            skill_md_path: PathBuf::from(name).join("SKILL.md"),
            scope: SkillScope::Project,
        }
    }

    #[test]
    fn renders_available_skills() {
        let outcome = SkillLoadOutcome {
            skills: vec![skill("simple")],
            ..Default::default()
        };
        let rendered = render_available_skills(&outcome, 8000).unwrap();
        assert!(rendered.contains("<skills_instructions>"));
        assert!(rendered.contains("simple"));
    }

    #[test]
    fn respects_budget() {
        let outcome = SkillLoadOutcome {
            skills: vec![skill("one"), skill("two")],
            ..Default::default()
        };
        let rendered = render_available_skills(&outcome, 180).unwrap();
        assert!(rendered.contains("omitted") || !rendered.contains("two"));
    }
}

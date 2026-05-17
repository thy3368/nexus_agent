use nexus_agent::config::Config;
use nexus_agent::skill::manager::skill_roots_for_config;
use nexus_agent::skill::SkillManager;

#[test]
fn loads_fixture_skill_and_renders_available_block() {
    let mut config = Config::default();
    config.skills.project_skills = false;
    config.skills.user_skills = false;
    config.skills.roots = vec![fixture_root()];

    let manager = SkillManager::new();
    let outcome = manager
        .load_for_config(&config, std::path::Path::new("."))
        .unwrap();

    assert_eq!(outcome.skills.len(), 1);
    assert_eq!(outcome.skills[0].name, "simple-checklist");

    let context = manager.render_prompt_context(&config, "hello");
    let available = context.available_skills_block.unwrap();
    assert!(available.contains("<skills_instructions>"));
    assert!(available.contains("simple-checklist"));
    assert!(available.contains("SKILL.md"));
    assert!(context.explicit_skill_blocks.is_empty());
}

#[test]
fn explicit_mention_injects_full_skill_body() {
    let mut config = Config::default();
    config.skills.project_skills = false;
    config.skills.user_skills = false;
    config.skills.roots = vec![fixture_root()];

    let manager = SkillManager::new();
    manager
        .load_for_config(&config, std::path::Path::new("."))
        .unwrap();

    let context = manager.render_prompt_context(&config, "Use $simple-checklist to answer.");
    assert_eq!(context.explicit_skill_blocks.len(), 1);
    assert!(context.explicit_skill_blocks[0].contains("<skill name=\"simple-checklist\""));
    assert!(context.explicit_skill_blocks[0].contains("When this skill is active"));
}

#[test]
fn config_roots_are_used() {
    let mut config = Config::default();
    config.skills.project_skills = false;
    config.skills.user_skills = false;
    config.skills.roots = vec![fixture_root()];

    let roots = skill_roots_for_config(&config, std::path::Path::new("."));
    assert_eq!(roots.len(), 1);
    assert!(roots[0].path.ends_with("tests/fixtures/skills"));
}

fn fixture_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("skills")
}

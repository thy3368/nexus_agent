use crate::skill::model::{SkillError, SkillLoadOutcome, SkillMetadata, SkillRoot};
use crate::skill::parsing::parse_skill_metadata;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const SKILL_FILENAME: &str = "SKILL.md";
const MAX_SCAN_DEPTH: usize = 6;
const MAX_SKILL_DIRS_PER_ROOT: usize = 2000;

pub fn load_skills_from_roots(roots: &[SkillRoot], disabled_names: &[String]) -> SkillLoadOutcome {
    let mut skills = Vec::new();
    let mut errors = Vec::new();
    let disabled_name_set: HashSet<&str> = disabled_names.iter().map(String::as_str).collect();

    for root in roots {
        if !root.path.exists() {
            continue;
        }

        let mut visited_dirs = 0usize;
        discover_skill_files(&root.path, 0, &mut visited_dirs, &mut |skill_md_path| {
            match load_skill_metadata(&skill_md_path, root) {
                Ok(metadata) => {
                    if !disabled_name_set.contains(metadata.name.as_str()) {
                        skills.push(metadata);
                    }
                }
                Err(message) => errors.push(SkillError::new(skill_md_path, message)),
            }
        });
    }

    sort_and_dedupe(&mut skills);

    SkillLoadOutcome {
        skills,
        errors,
        disabled_paths: HashSet::new(),
    }
}

fn discover_skill_files(
    root: &Path,
    depth: usize,
    visited_dirs: &mut usize,
    on_skill: &mut impl FnMut(PathBuf),
) {
    if depth > MAX_SCAN_DEPTH || *visited_dirs >= MAX_SKILL_DIRS_PER_ROOT {
        return;
    }

    *visited_dirs += 1;

    let skill_md = root.join(SKILL_FILENAME);
    if skill_md.is_file() {
        on_skill(skill_md);
        return;
    }

    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || is_hidden(&path) {
            continue;
        }
        discover_skill_files(&path, depth + 1, visited_dirs, on_skill);
        if *visited_dirs >= MAX_SKILL_DIRS_PER_ROOT {
            break;
        }
    }
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

fn load_skill_metadata(skill_md_path: &Path, root: &SkillRoot) -> Result<SkillMetadata, String> {
    let content = fs::read_to_string(skill_md_path).map_err(|error| error.to_string())?;
    let canonical_path = skill_md_path
        .canonicalize()
        .unwrap_or_else(|_| skill_md_path.to_path_buf());
    parse_skill_metadata(&content, &canonical_path, root.scope)
        .map(|(metadata, _)| metadata)
        .map_err(|error| error.to_string())
}

fn sort_and_dedupe(skills: &mut Vec<SkillMetadata>) {
    skills.sort_by(|left, right| {
        left.scope
            .priority()
            .cmp(&right.scope.priority())
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.skill_md_path.cmp(&right.skill_md_path))
    });

    let mut seen_paths = HashSet::new();
    skills.retain(|skill| seen_paths.insert(skill.skill_md_path.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::model::SkillScope;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn loads_skill_from_root() {
        let temp = tempdir().unwrap();
        let skill_dir = temp.path().join("simple");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: simple\ndescription: Simple skill\n---\nbody",
        )
        .unwrap();

        let outcome = load_skills_from_roots(
            &[SkillRoot {
                path: temp.path().to_path_buf(),
                scope: SkillScope::Project,
            }],
            &[],
        );

        assert_eq!(outcome.skills.len(), 1);
        assert_eq!(outcome.skills[0].name, "simple");
        assert!(outcome.errors.is_empty());
    }

    #[test]
    fn collects_invalid_skill_errors() {
        let temp = tempdir().unwrap();
        let skill_dir = temp.path().join("bad");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "not frontmatter").unwrap();

        let outcome = load_skills_from_roots(
            &[SkillRoot {
                path: temp.path().to_path_buf(),
                scope: SkillScope::Project,
            }],
            &[],
        );

        assert!(outcome.skills.is_empty());
        assert_eq!(outcome.errors.len(), 1);
    }
}

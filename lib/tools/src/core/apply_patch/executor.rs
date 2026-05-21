use crate::core::apply_patch::ApplyPatchAction;
use crate::core::apply_patch::ApplyPatchError;
use crate::core::apply_patch::ApplyPatchFileChange;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

pub trait ApplyPatchExecutor {
    fn execute(
        &self,
        action: &ApplyPatchAction,
    ) -> Result<ApplyPatchExecutionSummary, ApplyPatchError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyPatchExecutionSummary {
    pub changed_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalApplyPatchExecutor;

impl LocalApplyPatchExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl ApplyPatchExecutor for LocalApplyPatchExecutor {
    fn execute(
        &self,
        action: &ApplyPatchAction,
    ) -> Result<ApplyPatchExecutionSummary, ApplyPatchError> {
        let mut changed_paths = Vec::with_capacity(action.changes().len());

        for (path, change) in action.changes() {
            match change {
                ApplyPatchFileChange::Add { content } => {
                    let resolved = action.resolve_path(path);
                    if resolved.exists() {
                        return Err(ApplyPatchError::Conflict(format!(
                            "file already exists: {}",
                            path.display()
                        )));
                    }
                    ensure_parent_dir(&resolved)?;
                    fs::write(&resolved, content)?;
                    changed_paths.push(resolved);
                }
                ApplyPatchFileChange::Delete { .. } => {
                    let resolved = action.resolve_path(path);
                    if !resolved.exists() {
                        return Err(ApplyPatchError::Conflict(format!(
                            "file does not exist: {}",
                            path.display()
                        )));
                    }
                    fs::remove_file(&resolved)?;
                    changed_paths.push(resolved);
                }
                ApplyPatchFileChange::Update {
                    unified_diff,
                    move_path,
                    new_content,
                } => {
                    let source = action.resolve_path(path);
                    if !source.exists() {
                        return Err(ApplyPatchError::Conflict(format!(
                            "file does not exist: {}",
                            path.display()
                        )));
                    }

                    let original = fs::read_to_string(&source)?;
                    let updated = match new_content {
                        Some(content) => content.clone(),
                        None => apply_unified_diff(&original, unified_diff)?,
                    };

                    let destination = move_path
                        .as_ref()
                        .map(|path| action.resolve_path(path))
                        .unwrap_or_else(|| source.clone());

                    if destination != source && destination.exists() {
                        return Err(ApplyPatchError::Conflict(format!(
                            "move target already exists: {}",
                            destination.display()
                        )));
                    }

                    ensure_parent_dir(&destination)?;
                    fs::write(&destination, updated)?;
                    if destination != source {
                        fs::remove_file(&source)?;
                    }
                    changed_paths.push(destination);
                }
            }
        }

        Ok(ApplyPatchExecutionSummary { changed_paths })
    }
}

fn ensure_parent_dir(path: &Path) -> Result<(), ApplyPatchError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn apply_unified_diff(original: &str, diff: &str) -> Result<String, ApplyPatchError> {
    if diff.trim().is_empty() {
        return Ok(original.to_string());
    }

    let chunks = parse_diff_chunks(diff)?;
    let (mut lines, mut trailing_newline) = split_lines(original);
    let mut cursor = 0usize;

    for chunk in chunks {
        if chunk.old_lines.is_empty() {
            lines.splice(cursor..cursor, chunk.new_lines.clone());
            cursor += chunk.new_lines.len();
        } else {
            let relative = find_subsequence(&lines[cursor..], &chunk.old_lines).ok_or_else(|| {
                ApplyPatchError::Conflict(format!(
                    "failed to match update hunk: {:?}",
                    chunk.old_lines
                ))
            })?;
            let start = cursor + relative;
            let end = start + chunk.old_lines.len();
            lines.splice(start..end, chunk.new_lines.clone());
            cursor = start + chunk.new_lines.len();
        }

        if chunk.end_of_file {
            trailing_newline = false;
        }
    }

    Ok(join_lines(&lines, trailing_newline))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DiffChunk {
    old_lines: Vec<String>,
    new_lines: Vec<String>,
    end_of_file: bool,
}

fn parse_diff_chunks(diff: &str) -> Result<Vec<DiffChunk>, ApplyPatchError> {
    let mut chunks = Vec::new();
    let mut current: Option<DiffChunk> = None;

    for line in diff.lines() {
        if line == "@@" || line.starts_with("@@ ") {
            if let Some(chunk) = current.take() {
                chunks.push(chunk);
            }
            current = Some(DiffChunk {
                old_lines: Vec::new(),
                new_lines: Vec::new(),
                end_of_file: false,
            });
            continue;
        }

        let Some(chunk) = current.as_mut() else {
            return Err(ApplyPatchError::Parse(
                "update diff must start with `@@`".to_string(),
            ));
        };

        if line == "*** End of File" {
            chunk.end_of_file = true;
            continue;
        }

        let (kind, content) = line.split_at(1);
        match kind {
            "+" => chunk.new_lines.push(content.to_string()),
            "-" => chunk.old_lines.push(content.to_string()),
            " " => {
                chunk.old_lines.push(content.to_string());
                chunk.new_lines.push(content.to_string());
            }
            _ => {
                return Err(ApplyPatchError::Parse(format!(
                    "invalid diff line: {line}"
                )));
            }
        }
    }

    if let Some(chunk) = current.take() {
        chunks.push(chunk);
    }

    if chunks.is_empty() {
        return Err(ApplyPatchError::Parse(
            "update diff must contain at least one `@@` chunk".to_string(),
        ));
    }

    Ok(chunks)
}

fn split_lines(content: &str) -> (Vec<String>, bool) {
    let trailing_newline = content.ends_with('\n');
    let lines = content.lines().map(ToString::to_string).collect();
    (lines, trailing_newline)
}

fn join_lines(lines: &[String], trailing_newline: bool) -> String {
    if lines.is_empty() {
        return String::new();
    }

    let mut content = lines.join("\n");
    if trailing_newline {
        content.push('\n');
    }
    content
}

fn find_subsequence(haystack: &[String], needle: &[String]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }

    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

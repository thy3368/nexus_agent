use crate::tools::apply_patch::ApplyPatchAction;
use crate::tools::apply_patch::ApplyPatchError;
use crate::tools::apply_patch::ApplyPatchFileChange;
use std::path::PathBuf;

const BEGIN_PATCH: &str = "*** Begin Patch";
const END_PATCH: &str = "*** End Patch";
const ADD_FILE: &str = "*** Add File: ";
const DELETE_FILE: &str = "*** Delete File: ";
const UPDATE_FILE: &str = "*** Update File: ";
const MOVE_TO: &str = "*** Move to: ";

pub fn parse_patch(
    input: &str,
    cwd: impl Into<PathBuf>,
) -> Result<ApplyPatchAction, ApplyPatchError> {
    let lines: Vec<&str> = input.lines().collect();
    if lines.first().copied() != Some(BEGIN_PATCH) {
        return Err(ApplyPatchError::Parse(
            "missing `*** Begin Patch` header".to_string(),
        ));
    }
    if lines.last().copied() != Some(END_PATCH) {
        return Err(ApplyPatchError::Parse(
            "missing `*** End Patch` footer".to_string(),
        ));
    }

    let mut action = ApplyPatchAction::new(cwd);
    let mut index = 1;
    while index < lines.len() - 1 {
        let line = lines[index];
        if let Some(path) = line.strip_prefix(ADD_FILE) {
            index += 1;
            let mut content_lines = Vec::new();
            while index < lines.len() - 1 && !is_hunk_header(lines[index]) {
                let current = lines[index];
                let Some(content) = current.strip_prefix('+') else {
                    return Err(ApplyPatchError::Parse(format!(
                        "invalid add line: {current}"
                    )));
                };
                content_lines.push(content);
                index += 1;
            }
            if content_lines.is_empty() {
                return Err(ApplyPatchError::Parse(format!(
                    "add file hunk must contain content: {path}"
                )));
            }
            action.insert_change(
                path,
                ApplyPatchFileChange::Add {
                    content: format!("{}\n", content_lines.join("\n")),
                },
            );
            continue;
        }

        if let Some(path) = line.strip_prefix(DELETE_FILE) {
            action.insert_change(
                path,
                ApplyPatchFileChange::Delete {
                    content: String::new(),
                },
            );
            index += 1;
            continue;
        }

        if let Some(path) = line.strip_prefix(UPDATE_FILE) {
            index += 1;
            let move_path = if index < lines.len() - 1 {
                lines[index].strip_prefix(MOVE_TO).map(PathBuf::from)
            } else {
                None
            };
            if move_path.is_some() {
                index += 1;
            }

            let mut diff_lines = Vec::new();
            while index < lines.len() - 1 && !is_hunk_header(lines[index]) {
                let current = lines[index];
                if is_update_line(current) {
                    diff_lines.push(current);
                    index += 1;
                } else {
                    return Err(ApplyPatchError::Parse(format!(
                        "invalid update line: {current}"
                    )));
                }
            }

            let unified_diff = if diff_lines.is_empty() {
                String::new()
            } else {
                format!("{}\n", diff_lines.join("\n"))
            };
            action.insert_change(
                path,
                ApplyPatchFileChange::Update {
                    unified_diff,
                    move_path,
                    new_content: None,
                },
            );
            continue;
        }

        return Err(ApplyPatchError::Parse(format!(
            "unexpected patch line: {line}"
        )));
    }

    Ok(action)
}

fn is_hunk_header(line: &str) -> bool {
    line.starts_with(ADD_FILE) || line.starts_with(DELETE_FILE) || line.starts_with(UPDATE_FILE)
}

fn is_update_line(line: &str) -> bool {
    line == "@@"
        || line.starts_with("@@ ")
        || line.starts_with('+')
        || line.starts_with('-')
        || line.starts_with(' ')
        || line == "*** End of File"
}

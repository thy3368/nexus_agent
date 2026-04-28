use std::collections::HashSet;

pub fn extract_skill_mentions(input: &str) -> Vec<String> {
    let mut mentions = Vec::new();
    let mut seen = HashSet::new();
    let bytes = input.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'$' {
            index += 1;
            continue;
        }

        let start = index + 1;
        if start >= bytes.len() || !bytes[start].is_ascii_alphanumeric() {
            index += 1;
            continue;
        }

        let mut end = start + 1;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'-')
        {
            end += 1;
        }

        let mention = &input[start..end];
        if !is_common_env_var(mention) && seen.insert(mention.to_string()) {
            mentions.push(mention.to_string());
        }
        index = end;
    }

    mentions
}

fn is_common_env_var(name: &str) -> bool {
    matches!(
        name,
        "HOME"
            | "PATH"
            | "USER"
            | "SHELL"
            | "PWD"
            | "OLDPWD"
            | "TMPDIR"
            | "TERM"
            | "EDITOR"
            | "LANG"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_skill_mentions() {
        assert_eq!(
            extract_skill_mentions("Use $simple-checklist and $other_1."),
            vec!["simple-checklist", "other_1"]
        );
    }

    #[test]
    fn ignores_common_env_vars_and_duplicates() {
        assert_eq!(
            extract_skill_mentions("echo $PATH then use $skill and $skill"),
            vec!["skill"]
        );
    }
}

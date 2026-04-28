pub const PROPOSED_PLAN_OPEN: &str = "<proposed_plan>";
pub const PROPOSED_PLAN_CLOSE: &str = "</proposed_plan>";

pub fn contains_complete_proposed_plan(text: &str) -> bool {
    let Some(open_start) = text.find(PROPOSED_PLAN_OPEN) else {
        return false;
    };

    text[open_start + PROPOSED_PLAN_OPEN.len()..].contains(PROPOSED_PLAN_CLOSE)
}

pub fn extract_proposed_plan_text(text: &str) -> Option<String> {
    let open_start = text.find(PROPOSED_PLAN_OPEN)?;
    let content_start = open_start + PROPOSED_PLAN_OPEN.len();
    let remaining = &text[content_start..];
    let content = match remaining.find(PROPOSED_PLAN_CLOSE) {
        Some(close_start) => &remaining[..close_start],
        None => remaining,
    };

    Some(content.trim().to_string())
}

pub fn strip_proposed_plan_blocks(text: &str) -> String {
    let mut output = String::new();
    let mut remaining = text;

    while let Some(open_start) = remaining.find(PROPOSED_PLAN_OPEN) {
        output.push_str(&remaining[..open_start]);
        let after_open = &remaining[open_start + PROPOSED_PLAN_OPEN.len()..];

        if let Some(close_start) = after_open.find(PROPOSED_PLAN_CLOSE) {
            remaining = &after_open[close_start + PROPOSED_PLAN_CLOSE.len()..];
        } else {
            remaining = "";
            break;
        }
    }

    output.push_str(remaining);
    output.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_proposed_plan_text() {
        let text = "Before\n<proposed_plan>\n- Step 1\n- Step 2\n</proposed_plan>\nAfter";
        assert_eq!(
            extract_proposed_plan_text(text),
            Some("- Step 1\n- Step 2".to_string())
        );
    }

    #[test]
    fn strips_proposed_plan_blocks() {
        let text = "Before\n<proposed_plan>\n- Step 1\n</proposed_plan>\nAfter";
        assert_eq!(strip_proposed_plan_blocks(text), "Before\n\nAfter");
    }

    #[test]
    fn detects_complete_proposed_plan() {
        assert!(contains_complete_proposed_plan(
            "<proposed_plan>plan</proposed_plan>"
        ));
        assert!(!contains_complete_proposed_plan("<proposed_plan>plan"));
        assert!(!contains_complete_proposed_plan("plan"));
    }

    #[test]
    fn returns_none_without_plan_block() {
        assert_eq!(extract_proposed_plan_text("plain text"), None);
    }

    #[test]
    fn extracts_unclosed_plan_block_to_end() {
        assert_eq!(
            extract_proposed_plan_text("text <proposed_plan>unfinished"),
            Some("unfinished".to_string())
        );
    }
}

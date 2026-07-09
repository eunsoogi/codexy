use super::completion_handoff_pending_worktree_text::{has_false_value, phrase_has_boundaries};

pub(super) fn has_false_actionable_error_evidence(text: &str) -> bool {
    has_false_label_value(text, "actionable error")
}

pub(super) fn has_false_label_value(text: &str, label: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(label) {
        let start = offset + index;
        let end = start + label.len();
        if phrase_has_boundaries(text, start, end) {
            let suffix = text[end..].trim_start();
            if let Some(value) = suffix.strip_prefix([':', '=', '-', '?']) {
                if has_false_value(value.trim_start()) {
                    return true;
                }
            }
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

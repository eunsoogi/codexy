use super::completion_handoff_pending_worktree_text::{has_false_value, phrase_has_boundaries};

pub(super) fn has_false_actionable_error_evidence(text: &str) -> bool {
    has_false_label_value(text, "actionable error")
        || has_false_label_phrase(text, "actionable error")
        || find_phrase(text, "missing actionable error")
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

pub(super) fn has_false_label_phrase(text: &str, label: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(label) {
        let start = offset + index;
        let end = start + label.len();
        if phrase_has_boundaries(text, start, end) {
            let suffix = text[end..].trim_start();
            if suffix.starts_with("is missing")
                || suffix.starts_with("missing")
                || suffix.starts_with("is absent")
                || suffix.starts_with("absent")
                || suffix.starts_with("is unknown")
                || suffix.starts_with("unknown")
            {
                return true;
            }
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

fn find_phrase(text: &str, phrase: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let start = offset + index;
        let end = start + phrase.len();
        if phrase_has_boundaries(text, start, end) {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

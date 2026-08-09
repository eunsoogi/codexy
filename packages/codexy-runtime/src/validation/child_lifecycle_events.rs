pub(super) struct ActiveLine {
    pub(super) text: String,
    pub(super) packaged_terminal: bool,
}

pub(super) fn active_lines(evidence: &str) -> Vec<ActiveLine> {
    let text = evidence.to_ascii_lowercase();
    let mut lines = Vec::new();
    let mut start = 0;
    for fragment in text.split_inclusive('\n') {
        if super::sentinel_handoff::active_result_line(&text, start) {
            if let Some(text) = super::readiness_context::active_line(fragment) {
                let text = text.to_owned();
                let packaged_terminal = super::sentinel_handoff::packaged_terminal_result(&text);
                lines.push(ActiveLine {
                    text,
                    packaged_terminal,
                });
            }
        }
        start += fragment.len();
    }
    lines
}

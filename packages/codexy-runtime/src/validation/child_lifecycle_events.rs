pub(super) struct ActiveLine {
    pub(super) text: String,
}

pub(super) fn active_lines(evidence: &str) -> Vec<ActiveLine> {
    let text = super::readiness_context::current_text(evidence).to_ascii_lowercase();
    let mut lines = Vec::new();
    for fragment in text.split_inclusive('\n') {
        if let Some(text) = super::readiness_context::active_line(fragment) {
            lines.push(ActiveLine {
                text: text.to_owned(),
            });
        }
    }
    lines
}

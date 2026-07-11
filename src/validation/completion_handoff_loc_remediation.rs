pub(super) fn check(handoff: &str) -> Vec<String> {
    let text = handoff.to_ascii_lowercase();
    if !mentions_loc_evidence(&text) {
        return Vec::new();
    }
    if [
        "blank-line deletion",
        "multiline collapse",
        "formatting-only",
    ]
    .iter()
    .any(|marker| text.contains(marker))
    {
        return vec![
            "formatting-only LOC remediation cannot satisfy completion readiness evidence".into(),
        ];
    }
    if has_structural_evidence(&text) || has_not_applicable_evidence(&text) {
        return Vec::new();
    }
    vec![
        "LOC remediation evidence must name a structural boundary, responsibility, or real duplication removal".into(),
    ]
}

fn mentions_loc_evidence(text: &str) -> bool {
    text.contains("--check-touched-loc")
        || text.contains("loc remediation")
        || text.contains("touched loc")
}

fn has_structural_evidence(text: &str) -> bool {
    [
        "helper extraction",
        "module splitting",
        "test-target splitting",
        "responsibility separation",
        "real duplication removal",
    ]
    .iter()
    .any(|marker| has_positive_marker(text, marker))
}

fn has_positive_marker(text: &str, marker: &str) -> bool {
    let mut search_start = 0;
    while let Some(relative_index) = text[search_start..].find(marker) {
        let marker_index = search_start + relative_index;
        let prefix = text[..marker_index].trim_end();
        if !is_quoted(prefix) && !is_negated(prefix) {
            return true;
        }
        search_start = marker_index + marker.len();
    }
    false
}

fn is_quoted(prefix: &str) -> bool {
    matches!(prefix.chars().next_back(), Some('"' | '\'' | '`'))
}

fn is_negated(prefix: &str) -> bool {
    prefix
        .rsplit_once(['.', '!', '?', ';', '\n'])
        .map_or(prefix, |(_, sentence)| sentence)
        .split_whitespace()
        .next_back()
        .is_some_and(|word| matches!(word, "not" | "no" | "without"))
}

fn has_not_applicable_evidence(text: &str) -> bool {
    text.contains("loc remediation: not applicable") && text.contains("no touched file")
}

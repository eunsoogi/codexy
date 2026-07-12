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
    evidence_clauses(text).any(|clause| {
        !is_stale_clause(clause)
            && has_file_boundary(clause)
            && structural_markers()
                .iter()
                .any(|marker| has_positive_marker(clause, marker))
    })
}

fn evidence_clauses(text: &str) -> impl Iterator<Item = &str> {
    text.split(['\n', ';'])
        .flat_map(|segment| segment.split(". "))
        .map(str::trim)
        .filter(|segment| {
            !segment.is_empty() && segment.to_ascii_lowercase().contains("loc remediation")
        })
}

const fn structural_markers() -> &'static [&'static str] {
    &[
        "helper extraction",
        "module splitting",
        "test-target splitting",
        "responsibility separation",
        "real duplication removal",
    ]
}

fn has_file_boundary(clause: &str) -> bool {
    clause.split_whitespace().any(|word| {
        let word = word.trim_matches(|character: char| {
            matches!(character, '"' | '\'' | '`' | '(' | ')' | ',' | ':' | '.')
        });
        ["src/", "tests/", "plugins/", "scripts/"]
            .iter()
            .any(|prefix| word.starts_with(prefix))
            && word.contains('.')
    })
}

fn is_stale_clause(clause: &str) -> bool {
    clause.split_whitespace().any(|word| {
        matches!(
            word.trim_matches(|character: char| !character.is_ascii_alphabetic())
                .to_ascii_lowercase()
                .as_str(),
            "previous" | "stale" | "historical" | "earlier"
        )
    })
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
        .rev()
        .take(5)
        .map(|word| word.trim_matches(|character: char| !character.is_ascii_alphabetic()))
        .any(|word| matches!(word, "not" | "no" | "without"))
}

fn has_not_applicable_evidence(text: &str) -> bool {
    text.contains("loc remediation: not applicable") && text.contains("no touched file")
}

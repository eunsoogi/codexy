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
            && has_current_lane_scope(clause)
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
        .filter(|segment| !segment.is_empty() && has_evidence_label(segment))
}

fn has_evidence_label(clause: &str) -> bool {
    let clause = clause.trim_start().to_ascii_lowercase();
    clause.starts_with("loc remediation:") || clause.starts_with("touched loc:")
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

fn has_current_lane_scope(clause: &str) -> bool {
    ![
        "fallback lane",
        "fallback-lane",
        "fallback child lane",
        "fallback route",
        "other lane",
        "another lane",
        "separate lane",
        "different lane",
        "other branch",
        "another branch",
    ]
    .iter()
    .any(|foreign_scope| clause.contains(foreign_scope))
}

fn has_positive_marker(text: &str, marker: &str) -> bool {
    let mut search_start = 0;
    while let Some(relative_index) = text[search_start..].find(marker) {
        let marker_index = search_start + relative_index;
        let prefix = text[..marker_index].trim_end();
        let suffix = &text[marker_index + marker.len()..];
        if !is_quoted(prefix)
            && !has_marker_negation(prefix, suffix)
            && !has_marker_example(prefix, suffix)
            && !is_tentative(prefix, suffix)
        {
            return true;
        }
        search_start = marker_index + marker.len();
    }
    false
}

fn has_marker_negation(prefix: &str, suffix: &str) -> bool {
    is_negated(prefix) || has_postposed_negation(suffix)
}

fn has_postposed_negation(suffix: &str) -> bool {
    let words = evidence_words(suffix).collect::<Vec<_>>();
    words.iter().take(6).enumerate().any(|(index, word)| {
        matches!(*word, "not" | "no")
            || (*word == "without"
                && !(words.get(index + 1) == Some(&"changing")
                    && words.get(index + 2) == Some(&"behavior")))
    })
}

fn has_marker_example(prefix: &str, suffix: &str) -> bool {
    is_example(prefix) || suffix.contains("as an example only")
}

fn is_quoted(prefix: &str) -> bool {
    matches!(prefix.chars().next_back(), Some('"' | '\'' | '`'))
        || prefix.chars().filter(|character| *character == '"').count() % 2 == 1
}

fn is_negated(prefix: &str) -> bool {
    evidence_words(prefix).any(|word| matches!(word, "not" | "no" | "without"))
}

fn is_example(prefix: &str) -> bool {
    prefix.to_ascii_lowercase().contains("for example")
        || evidence_words(prefix).any(|word| word == "example")
}

fn is_tentative(prefix: &str, suffix: &str) -> bool {
    evidence_words(prefix)
        .chain(evidence_words(suffix))
        .any(|word| {
            matches!(
                word,
                "considered"
                    | "plan"
                    | "planned"
                    | "intend"
                    | "intended"
                    | "would"
                    | "could"
                    | "might"
            )
        })
}

fn evidence_words(text: &str) -> impl Iterator<Item = &str> {
    text.split_whitespace()
        .map(|word| word.trim_matches(|character: char| !character.is_ascii_alphabetic()))
        .filter(|word| !word.is_empty())
}

fn has_not_applicable_evidence(text: &str) -> bool {
    text.contains("loc remediation: not applicable") && text.contains("no touched file")
}

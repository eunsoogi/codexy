mod no_remediation;

const STRUCTURAL_MARKERS: &[&str] = &[
    "helper extraction",
    "module splitting",
    "test-target splitting",
    "responsibility separation",
    "real duplication removal",
];
const COSMETIC_MARKERS: &[&str] = &[
    "blank-line deletion",
    "multiline collapse",
    "formatting-only",
];

pub(super) fn check(handoff: &str) -> Vec<String> {
    let text = handoff.to_ascii_lowercase();
    if !mentions_loc_evidence(&text) {
        return Vec::new();
    }
    let affirmative_cosmetic = has_affirmative_cosmetic_remediation(&text);
    if (has_structural_evidence(&text) || no_remediation::has_evidence(&text))
        && !affirmative_cosmetic
    {
        return Vec::new();
    }
    if affirmative_cosmetic || has_cosmetic_marker(&text) {
        return vec![
            "formatting-only LOC remediation cannot satisfy completion readiness evidence".into(),
        ];
    }
    vec![
        "LOC remediation evidence must name a structural boundary, responsibility, or real duplication removal".into(),
    ]
}

fn mentions_loc_evidence(text: &str) -> bool {
    ["--check-touched-loc", "loc remediation", "touched loc"]
        .iter()
        .any(|marker| text.contains(marker))
}

fn has_structural_evidence(text: &str) -> bool {
    evidence_clauses(text).any(|clause| {
        !is_stale_clause(clause)
            && has_current_lane_scope(clause)
            && has_file_boundary(clause)
            && STRUCTURAL_MARKERS
                .iter()
                .any(|marker| has_positive_marker(clause, marker))
    })
}

fn has_affirmative_cosmetic_remediation(text: &str) -> bool {
    text.lines().flat_map(|line| line.split(';')).any(|clause| {
        !is_stale_clause(clause)
            && has_current_lane_scope(clause)
            && COSMETIC_MARKERS
                .iter()
                .any(|marker| has_positive_marker(clause, marker))
    })
}

fn has_cosmetic_marker(text: &str) -> bool {
    COSMETIC_MARKERS.iter().any(|marker| text.contains(marker))
}

fn evidence_clauses(text: &str) -> impl Iterator<Item = &str> {
    text.split(['\n', ';'])
        .flat_map(|segment| segment.split(". "))
        .map(str::trim)
        .filter(|segment| !segment.is_empty() && has_evidence_label(segment))
}

fn has_evidence_label(clause: &str) -> bool {
    let trimmed = clause.trim_start();
    let clause = match trimmed.chars().next() {
        Some('-' | '*' | '+') if trimmed[1..].starts_with(char::is_whitespace) => {
            trimmed[1..].trim_start()
        }
        _ => trimmed,
    }
    .to_ascii_lowercase();
    clause.starts_with("loc remediation:") || clause.starts_with("touched loc:")
}

fn has_file_boundary(clause: &str) -> bool {
    clause.split_whitespace().any(|word| {
        let word = word.trim_matches(|character: char| {
            matches!(character, '"' | '\'' | '`' | '(' | ')' | ',' | ':' | '.')
        });
        word.contains('.')
            && matches!(
                word.split('/').next(),
                Some("src" | "tests" | "plugins" | "scripts")
            )
    })
}

fn is_stale_clause(clause: &str) -> bool {
    evidence_words(clause)
        .any(|word| matches!(word, "previous" | "stale" | "historical" | "earlier"))
}

fn has_current_lane_scope(clause: &str) -> bool {
    let words = evidence_words(clause).collect::<Vec<_>>();
    !clause.contains("fallback-lane")
        && !words.windows(2).any(|words| {
            matches!(
                words,
                ["fallback", "lane" | "child" | "route"]
                    | [
                        "other" | "another" | "separate" | "different",
                        "lane" | "branch"
                    ]
            )
        })
}

fn has_positive_marker(text: &str, marker: &str) -> bool {
    let mut search_start = 0;
    while let Some(relative_index) = text[search_start..].find(marker) {
        let marker_index = search_start + relative_index;
        let prefix = text[..marker_index].trim_end();
        let suffix = &text[marker_index + marker.len()..];
        if !is_quoted(prefix, suffix)
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

fn is_quoted(prefix: &str, suffix: &str) -> bool {
    has_unclosed_straight_quote(prefix, suffix, '"')
        || has_unclosed_straight_quote(prefix, suffix, '\'')
        || has_unclosed_straight_quote(prefix, suffix, '`')
        || has_unclosed_quote(prefix, '“', '”')
        || has_unclosed_quote(prefix, '‘', '’')
}

fn has_unclosed_straight_quote(prefix: &str, suffix: &str, quote: char) -> bool {
    let chars = prefix.chars().collect::<Vec<_>>();
    chars
        .iter()
        .enumerate()
        .fold(false, |open, (index, character)| {
            let contraction = quote == '\''
                && index > 0
                && chars[index - 1].is_alphanumeric()
                && ((!open && chars[index - 1] == 's')
                    || (open
                        && chars[index - 1] == 's'
                        && (chars[index + 1..]
                            .iter()
                            .all(|character| character.is_whitespace())
                            || suffix.contains(quote)
                            || quote_opened_after_said(&chars, index, quote)))
                    || chars
                        .get(index + 1)
                        .is_some_and(|next| next.is_alphanumeric()));
            if *character == quote && !contraction {
                !open
            } else {
                open
            }
        })
}

fn quote_opened_after_said(chars: &[char], index: usize, quote: char) -> bool {
    chars[..index]
        .iter()
        .rposition(|character| *character == quote)
        .is_some_and(|opening| {
            chars[..opening]
                .iter()
                .collect::<String>()
                .trim_end()
                .ends_with("said")
        })
}

fn has_unclosed_quote(prefix: &str, opening: char, closing: char) -> bool {
    let mut open = false;
    for character in prefix.chars() {
        if character == opening {
            open = true;
        } else if character == closing {
            open = false;
        }
    }
    open
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

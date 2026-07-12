pub(super) fn luna_policy_clauses(segment: &str) -> impl Iterator<Item = String> {
    normalize_clause(segment)
        .split(" but ")
        .flat_map(|clause| clause.split(" and "))
        .map(str::to_owned)
        .collect::<Vec<_>>()
        .into_iter()
}

pub(super) fn has_luna_default_assignment(segment: &str) -> bool {
    let normalized = normalize_clause(segment);
    has_word(&normalized, "luna")
        && normalized.contains("blanket default")
        && [
            "be", "use", "make", "set", "assign", "route", "serve", "serves", "keep", "remain",
        ]
        .iter()
        .any(|verb| has_word(&normalized, verb))
}

pub(super) fn luna_blanket_default_is_negated(segment: &str) -> bool {
    let normalized = normalize_clause(segment);
    let Some(default_index) = normalized.find("blanket default") else {
        return false;
    };
    normalized[..default_index]
        .split_whitespace()
        .any(|word| matches!(word, "not" | "never"))
}

fn normalize_clause(segment: &str) -> String {
    segment
        .to_ascii_lowercase()
        .replace("n't", " not")
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn has_word(text: &str, expected: &str) -> bool {
    text.split_whitespace().any(|word| word == expected)
}

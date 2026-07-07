pub(super) fn has_affirmed_phrase(text: &str, phrase: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let start = offset + index;
        let end = start + phrase.len();
        if phrase_has_boundaries(text, start, end)
            && !has_unchecked_checklist_marker_before(text, start)
            && !is_locally_negated(&text[..start])
            && !has_non_claim_heading_suffix(&text[end..])
            && !has_non_claim_label_value(&text[end..])
        {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

fn has_unchecked_checklist_marker_before(text: &str, start: usize) -> bool {
    let prefix = text[..start].trim_end_matches([' ', '\t']);
    prefix.ends_with("- [ ]") || prefix.ends_with("* [ ]")
}

pub(super) fn has_non_claim_phrase_label(text: &str, phrase: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let start = offset + index;
        let end = start + phrase.len();
        if phrase_has_boundaries(text, start, end) && has_non_claim_label_value(&text[end..]) {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

fn has_non_claim_label_value(suffix: &str) -> bool {
    if label_value(suffix).is_some_and(|value| {
        value
            .strip_prefix("no blockers")
            .is_some_and(|rest| is_boundary(rest.chars().next()))
    }) {
        return false;
    }
    super::codex_review_handoff::has_negative_label_value(suffix)
        || [
            "not verified",
            "not yet verified",
            "not currently verified",
            "not confirmed",
            "not yet confirmed",
            "not currently confirmed",
            "not checked",
            "not run",
            "not yet",
            "not currently",
            "not clean",
            "not yet clean",
            "not currently clean",
            "not pushed",
            "not yet pushed",
            "not currently pushed",
            "not synced",
            "not yet synced",
            "not currently synced",
            "unverified",
            "unconfirmed",
            "unchecked",
            "missing",
            "unknown",
            "pending",
            "dirty",
            "blocked",
            "waiting",
            "deferred",
            "n/a",
            "na",
            "none",
            "no",
        ]
        .iter()
        .any(|phrase| label_value_starts_with(suffix, phrase))
}

fn has_non_claim_heading_suffix(suffix: &str) -> bool {
    let suffix = suffix.trim_start_matches([' ', '\t']);
    ["blocker", "blockers", "blocked", "pending", "waiting"]
        .iter()
        .any(|phrase| {
            suffix
                .strip_prefix(phrase)
                .is_some_and(|rest| is_boundary(rest.chars().next()))
        })
}

fn label_value_starts_with(suffix: &str, phrase: &str) -> bool {
    let Some(value) = label_value(suffix) else {
        return false;
    };
    value
        .strip_prefix(phrase)
        .is_some_and(|rest| is_boundary(rest.chars().next()))
}

fn label_value(suffix: &str) -> Option<&str> {
    let suffix = suffix.trim_start_matches([' ', '\t']);
    let value = suffix
        .strip_prefix(':')
        .or_else(|| suffix.strip_prefix('?'))?;
    Some(value.trim_start_matches([' ', '\t', '\n', '\r', '-', '*']))
}

fn phrase_has_boundaries(text: &str, start: usize, end: usize) -> bool {
    is_boundary(text[..start].chars().next_back()) && is_boundary(text[end..].chars().next())
}

fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| {
        !character.is_ascii_alphanumeric() && character != '-' && character != '_'
    })
}

fn is_locally_negated(prefix: &str) -> bool {
    let clause_start = last_clause_boundary(prefix).map_or(0, |index| index);
    prefix[clause_start..]
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '\'')
        .filter(|word| !word.is_empty())
        .rev()
        .take(4)
        .any(|word| {
            matches!(
                word,
                "no" | "not"
                    | "never"
                    | "without"
                    | "isn't"
                    | "wasn't"
                    | "hasn't"
                    | "haven't"
                    | "aren't"
                    | "don't"
                    | "doesn't"
                    | "didn't"
                    | "won't"
                    | "can't"
                    | "cannot"
            )
        })
}

fn last_clause_boundary(text: &str) -> Option<usize> {
    text.char_indices()
        .filter(|(_, character)| matches!(character, '.' | '!' | '?' | ';' | ':' | ',' | '\n'))
        .map(|(index, character)| index + character.len_utf8())
        .last()
}

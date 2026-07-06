const READY_PHRASES: &str = "merge-ready|merge-readiness|merge readiness|merge ready|ready to merge|ready for merge|ready for parent handoff|pr-ready|pr-readiness|pr readiness|pr ready|pull-request-ready|pull request ready|codex review passed|codex review completed|codex review complete|codex review approved";
const OVERRIDE_PHRASES: &str = "maintainer override: yes|maintainer override: granted|maintainer accepted proceeding without codex review|maintainer accepted proceeding without full codex review|maintainer explicitly accepted proceeding without codex review|maintainer explicitly accepted proceeding without full codex review";

pub(super) fn claims_ready(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    READY_PHRASES
        .split('|')
        .any(|phrase| has_affirmed_phrase(&text, phrase))
}
pub(super) fn claims_completion(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    [
        "codex review passed",
        "codex review completed",
        "codex review complete",
        "codex review approved",
    ]
    .iter()
    .any(|phrase| has_affirmed_phrase(&text, phrase))
}
pub(super) fn states_override(handoff: &str) -> bool {
    handoff.lines().any(|line| {
        let line = line.trim_start();
        let text = line.to_ascii_lowercase();
        let unordered = matches!(line.as_bytes().first(), Some(b'-' | b'*' | b'+'))
            && line[1..].trim_start().starts_with("[ ]");
        let ordered = line.split_once(['.', ')']).is_some_and(|(number, rest)| {
            !number.is_empty()
                && number.chars().all(|character| character.is_ascii_digit())
                && rest.trim_start().starts_with("[ ]")
        });
        !unordered
            && !ordered
            && OVERRIDE_PHRASES
                .split('|')
                .any(|phrase| has_affirmed_phrase(&text, phrase))
    })
}
fn has_affirmed_phrase(text: &str, phrase: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let start = offset + index;
        let end = start + phrase.len();
        if is_boundary(text[..start].chars().next_back())
            && is_boundary(text[end..].chars().next())
            && !is_locally_negated(&text[..start])
            && !has_blocking_label_value(&text[end..])
            && !has_negative_label_value(&text[end..])
        {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}
fn has_blocking_label_value(suffix: &str) -> bool {
    let suffix = suffix.trim_start_matches([' ', '\t']);
    let Some((label, value)) = suffix.split_once(':') else {
        return false;
    };
    let value = value.trim_start_matches([' ', '\t', '\n', '\r', '-', '*']);
    match label.trim() {
        "blocker" | "blockers" => {
            !starts_with_any(value, &["none", "no", "no blocker", "no blockers", "clear"])
        }
        "status" => {
            !starts_with_any(
                value,
                &["ready", "complete", "completed", "passed", "clean"],
            ) && (has_negative_label_value(suffix)
                || starts_with_any(
                    value,
                    &[
                        "blocked",
                        "blocking",
                        "waiting",
                        "pending",
                        "unresolved",
                        "incomplete",
                        "not complete",
                        "not yet complete",
                    ],
                ))
        }
        _ => false,
    }
}
fn starts_with_any(value: &str, phrases: &[&str]) -> bool {
    phrases
        .iter()
        .any(|phrase| value.strip_prefix(phrase).is_some_and(starts_with_boundary))
}
pub(super) fn has_negative_label_value(suffix: &str) -> bool {
    let Some(value) = label_value(suffix) else {
        return false;
    };
    [
        "not ready",
        "not yet ready",
        "not currently ready",
        "isn't ready",
        "isn't yet ready",
        "isn't currently ready",
        "aren't ready",
        "aren't yet ready",
        "aren't currently ready",
        "false",
        "not requested",
        "isn't requested",
        "aren't requested",
        "not applicable",
        "isn't applicable",
        "aren't applicable",
    ]
    .iter()
    .any(|phrase| value.strip_prefix(phrase).is_some_and(starts_with_boundary))
        || value
            .strip_prefix("no")
            .is_some_and(starts_with_standalone_label_boundary)
}
fn label_value(suffix: &str) -> Option<&str> {
    let suffix = suffix.trim_start_matches([' ', '\t']);
    let value = suffix
        .strip_prefix(':')
        .or_else(|| suffix.strip_prefix('?'))?;
    Some(value.trim_start_matches([' ', '\t', '\n', '\r', '-', '*']))
}
fn starts_with_boundary(rest: &str) -> bool {
    is_boundary(rest.chars().next())
}
fn starts_with_standalone_label_boundary(rest: &str) -> bool {
    rest.is_empty()
        || rest
            .chars()
            .next()
            .is_some_and(|character| matches!(character, '.' | ';' | ',' | '\n' | '\r'))
}
fn is_locally_negated(prefix: &str) -> bool {
    let clause_start = last_clause_boundary(prefix).map_or(0, |index| index);
    let clause = &prefix[clause_start..];
    clause
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
    let mut boundary = None;
    for (index, character) in text.char_indices() {
        let end = index + character.len_utf8();
        if matches!(character, '.' | '!' | '?' | ';' | ':' | ',' | '\n')
            || is_dash_separator(text, index, character)
        {
            boundary = Some(end);
        }
    }
    boundary
}
fn is_dash_separator(text: &str, index: usize, character: char) -> bool {
    if matches!(character, '–' | '—') {
        return true;
    }
    if character != '-' {
        return false;
    }
    let previous = text[..index].chars().next_back();
    let next = text[index + character.len_utf8()..].chars().next();
    previous.is_some_and(char::is_whitespace)
        && next.is_some_and(|character| character.is_whitespace() || character == '-')
}
fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| !character.is_ascii_alphanumeric())
}

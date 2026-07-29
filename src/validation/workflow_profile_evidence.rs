use super::child_lane_classification_boundaries::lane_boundary;
use super::child_lane_classification_control::normalize_metadata_prefix;
use super::child_lane_ownership_phrases::metadata_key;

const STRICT_SIGNALS: [&str; 7] = [
    "destructive",
    "security",
    "secret",
    "secrets",
    "permission",
    "release",
    "publication",
];

pub(super) fn current_active_lines(evidence: &str) -> Vec<String> {
    let (mut fence, mut lines) = (None, Vec::new());
    for raw in evidence.lines() {
        let line = normalize_metadata_prefix(raw);
        if let Some((marker, length, tail)) = fence_marker(line) {
            if fence.is_none() {
                fence = Some((marker, length));
            } else if fence.is_some_and(|(open, minimum)| {
                marker == open && length >= minimum && tail.trim().is_empty()
            }) {
                fence = None;
            }
            lines.push(String::new());
        } else if fence.is_some() {
            lines.push(String::new());
        } else {
            lines.push(line.to_owned());
        }
    }
    let references = lines.iter().map(String::as_str).collect::<Vec<_>>();
    let start = (0..references.len())
        .rev()
        .find(|index| {
            lane_boundary(&references, *index)
                .is_some_and(|boundary| boundary.resets_authority_record())
        })
        .map_or(0, |index| index + 1);
    lines.into_iter().skip(start).collect()
}

pub(super) fn has_strict_work_signal(lines: &[&str]) -> bool {
    lines
        .iter()
        .filter_map(|line| {
            ["task kind", "lane type", "risk"]
                .iter()
                .find_map(|field| field_value(line, field))
        })
        .any(value_has_strict_signal)
}

pub(super) fn field_value<'a>(line: &'a str, expected: &str) -> Option<&'a str> {
    line.split_once(':')
        .and_then(|(key, value)| (metadata_key(key) == expected).then_some(value.trim()))
}

fn fence_marker(line: &str) -> Option<(u8, usize, &str)> {
    let trimmed = line.trim_start();
    let marker = *trimmed.as_bytes().first()?;
    (marker == b'`' || marker == b'~').then_some(())?;
    let length = trimmed.bytes().take_while(|byte| *byte == marker).count();
    (length >= 3).then_some((marker, length, &trimmed[length..]))
}

fn value_has_strict_signal(value: &str) -> bool {
    category_clauses(value).iter().any(|tokens| {
        tokens.iter().enumerate().any(|(index, token)| {
            !negated(tokens, index)
                && (STRICT_SIGNALS.contains(&token.as_str()) || compound_signal(tokens, index))
        })
    })
}

fn category_clauses(value: &str) -> Vec<Vec<String>> {
    let mut clauses = vec![Vec::new()];
    let mut token = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            token.push(character);
            continue;
        }
        finish_token(&mut clauses, &mut token);
        if clause_delimiter(character) {
            start_clause(&mut clauses);
        }
    }
    finish_token(&mut clauses, &mut token);
    clauses
}

fn clause_delimiter(character: char) -> bool {
    matches!(
        character,
        ',' | ';' | ':' | '.' | '!' | '?' | '‒' | '–' | '—' | '―' | '−'
    )
}

fn finish_token(clauses: &mut Vec<Vec<String>>, token: &mut String) {
    if token.is_empty() {
        return;
    }
    if matches!(token.as_str(), "but" | "yet" | "however") {
        start_clause(clauses);
    } else {
        clauses
            .last_mut()
            .expect("one clause exists")
            .push(std::mem::take(token));
        return;
    }
    token.clear();
}

fn start_clause(clauses: &mut Vec<Vec<String>>) {
    if clauses.last().is_some_and(|clause| !clause.is_empty()) {
        clauses.push(Vec::new());
    }
}

fn compound_signal(tokens: &[String], index: usize) -> bool {
    let token = tokens[index].as_str();
    let next = tokens.get(index + 1).map(String::as_str);
    matches!(
        (token, next),
        ("high", Some("risk")) | ("multi", Some("lane")) | ("merge", Some("sensitive"))
    ) || matches!(
        (
            token,
            next,
            tokens.get(index + 2).map(String::as_str),
            tokens.get(index + 3).map(String::as_str),
        ),
        ("high", Some("consequence"), Some("external"), Some("state"))
    )
}

fn negated(tokens: &[String], index: usize) -> bool {
    tokens[index].starts_with("non-")
        || tokens[..index]
            .iter()
            .rev()
            .take(3)
            .any(|token| matches!(token.as_str(), "no" | "not" | "without" | "non"))
}

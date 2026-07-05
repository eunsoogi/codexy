const SENTINEL_MARKERS: &str = "sentinel|codexy-sentinel";
const GENERIC_REVIEWER_GATE_MARKERS: &str = "reviewer gate|reviewer-gate";
const PASS_MARKERS: &str = "sentinel: pass|sentinel pass|sentinel returned pass|sentinel status: pass|sentinel verdict: pass|sentinel result: pass|sentinel gate returned pass|sentinel reviewer gate returned pass|sentinel reviewer gate result: pass|sentinel reviewer gate verdict: pass|sentinel reviewer-gate returned pass|sentinel reviewer-gate result: pass|sentinel reviewer-gate verdict: pass";
const BLOCK_MARKERS: &str = "sentinel: block|sentinel block|sentinel returned block|sentinel status: block|sentinel verdict: block|sentinel result: block|sentinel gate returned block";
const UNOBSERVABLE_MARKERS: &str = "sentinel: unobservable|sentinel unobservable|sentinel status: unobservable|sentinel verdict: unobservable|sentinel result: unobservable|sentinel gate returned unobservable|sentinel pending|has not returned|hasn't returned|not returned|did not return pass or block|no pass or block|no pass/block|no verdict|stuck waiting|waiting for verdict|pending verdict|pending after bounded wait|delayed after bounded wait|timed out after bounded wait|produced no verdict|still running";
const READINESS_MARKERS: &str = "merge-ready|merge ready|merge readiness: yes|merge readiness yes|merge readiness: true|merge readiness true|ready to merge|ready for merge|ready for merge gates|ready for parent handoff|pr-ready|pr ready|pr readiness: yes|pr readiness yes|pr readiness: true|pr readiness true|pull-request-ready|pull request ready|parent can open pr next|parent can create pr next|parent can open the pr next|push-ready|push ready|ready to push|ready for push|push readiness: yes|push readiness yes|push readiness: true|push readiness true|pushed: yes|pushed yes|pushed: true|pushed true|remote/pr head match: yes|remote/pr head match yes|remote and pr head match";

pub(super) fn check(handoff: &str) -> Vec<String> {
    let text = handoff.to_ascii_lowercase();
    if !claims_readiness(&text) {
        return Vec::new();
    }
    if has_any(&text, GENERIC_REVIEWER_GATE_MARKERS) && !has_any(&text, SENTINEL_MARKERS) {
        return vec![
            "Generic reviewer-gate evidence cannot satisfy packaged Sentinel readiness proof"
                .into(),
        ];
    }
    if !has_any(&text, SENTINEL_MARKERS) {
        return Vec::new();
    }
    match current_sentinel_status(&text) {
        Some(SentinelStatus::Block) => {
            vec!["Sentinel BLOCK verdict cannot satisfy PR readiness or push readiness".into()]
        }
        Some(SentinelStatus::Unobservable) => {
            vec![
                "Sentinel UNOBSERVABLE or pending verdict cannot satisfy PR readiness or push readiness".into(),
            ]
        }
        Some(SentinelStatus::Pass) => Vec::new(),
        None => {
            vec![
                "Sentinel readiness evidence must state PASS, BLOCK, or UNOBSERVABLE explicitly"
                    .into(),
            ]
        }
    }
}

fn claims_readiness(text: &str) -> bool {
    READINESS_MARKERS
        .split('|')
        .any(|phrase| has_affirmed_phrase(text, phrase))
}

fn has_any(text: &str, phrases: &str) -> bool {
    phrases
        .split('|')
        .any(|phrase| has_affirmed_phrase(text, phrase))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SentinelStatus {
    Pass,
    Block,
    Unobservable,
}

fn current_sentinel_status(text: &str) -> Option<SentinelStatus> {
    PASS_MARKERS
        .split('|')
        .map(|phrase| (SentinelStatus::Pass, phrase))
        .chain(
            BLOCK_MARKERS
                .split('|')
                .map(|phrase| (SentinelStatus::Block, phrase)),
        )
        .chain(
            UNOBSERVABLE_MARKERS
                .split('|')
                .map(|phrase| (SentinelStatus::Unobservable, phrase)),
        )
        .filter_map(|(status, phrase)| {
            last_affirmed_phrase_start(text, phrase).map(|start| (start, status))
        })
        .max_by_key(|(start, _)| *start)
        .map(|(_, status)| status)
}

fn last_affirmed_phrase_start(text: &str, phrase: &str) -> Option<usize> {
    let mut rest = text;
    let mut offset = 0;
    let mut last = None;
    while let Some(index) = rest.find(phrase) {
        let start = offset + index;
        let end = start + phrase.len();
        if phrase_has_boundaries(text, start, end)
            && !is_locally_negated(&text[..start])
            && !is_locally_negated_after(&text[end..])
            && !has_negative_label_value(&text[end..])
        {
            last = Some(start);
        }
        offset = end;
        rest = &text[offset..];
    }
    last
}

fn has_affirmed_phrase(text: &str, phrase: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let start = offset + index;
        let end = start + phrase.len();
        if phrase_has_boundaries(text, start, end)
            && !is_locally_negated(&text[..start])
            && !has_negative_label_value(&text[end..])
        {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

fn has_negative_label_value(suffix: &str) -> bool {
    let Some(value) = label_value(suffix) else {
        return false;
    };
    if is_standalone_negative_no(value) {
        return true;
    }
    [
        "false",
        "not ready",
        "not yet ready",
        "not currently ready",
        "isn't ready",
        "isn't currently ready",
        "not applicable",
        "n/a",
    ]
    .iter()
    .any(|phrase| value.strip_prefix(phrase).is_some_and(starts_with_boundary))
}

fn is_standalone_negative_no(value: &str) -> bool {
    let rest = value.strip_prefix("no");
    rest.is_some_and(|rest| {
        let rest = rest.trim_start_matches([' ', '\t']);
        rest.is_empty() || rest.starts_with(['.', ',', ';', '!', '?', '\n', '\r'])
    })
}

fn label_value(suffix: &str) -> Option<&str> {
    let suffix = suffix.trim_start_matches([' ', '\t']);
    let value = suffix
        .strip_prefix(':')
        .or_else(|| suffix.strip_prefix('?'))?;
    Some(value.trim_start_matches([' ', '\t', '\n', '\r', '-', '*']))
}

fn is_locally_negated(prefix: &str) -> bool {
    let clause = &prefix[last_clause_boundary(prefix).unwrap_or(0)..];
    clause
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '\'')
        .filter(|word| !word.is_empty())
        .rev()
        .take(4)
        .any(|word| {
            matches!(
                word,
                "no" | "not"
                    | "without"
                    | "isn't"
                    | "wasn't"
                    | "hasn't"
                    | "missing"
                    | "absent"
                    | "lacking"
            )
        })
}

fn is_locally_negated_after(suffix: &str) -> bool {
    let clause = &suffix[..suffix
        .find(|character: char| matches!(character, '.' | '!' | '?' | ';' | ':' | ',' | '\n'))
        .unwrap_or(suffix.len())];
    let words: Vec<&str> = clause
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '\'')
        .filter(|word| !word.is_empty())
        .take(5)
        .collect();
    words
        .iter()
        .any(|word| matches!(*word, "missing" | "absent" | "lacking"))
        || words
            .windows(2)
            .any(|pair| matches!(pair, ["not", "provided"]))
}

fn last_clause_boundary(text: &str) -> Option<usize> {
    let mut boundary = None;
    for (index, character) in text.char_indices() {
        let end = index + character.len_utf8();
        if matches!(character, '.' | '!' | '?' | ';' | ':' | ',' | '\n') {
            boundary = Some(end);
        }
    }
    boundary
}

fn phrase_has_boundaries(text: &str, start: usize, end: usize) -> bool {
    is_boundary(text[..start].chars().next_back()) && is_boundary(text[end..].chars().next())
}

fn starts_with_boundary(rest: &str) -> bool {
    is_boundary(rest.chars().next())
}

fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| !character.is_ascii_alphanumeric())
}

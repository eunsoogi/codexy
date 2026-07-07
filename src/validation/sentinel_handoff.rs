const SENTINEL_MARKERS: &str = "sentinel|codexy-sentinel|packaged reviewer gate|reviewer gate";
const PASS_MARKERS: &str = "sentinel: pass|sentinel pass|sentinel returned pass|sentinel status: pass|sentinel verdict: pass|sentinel result: pass|sentinel gate returned pass|reviewer gate returned pass";
const BLOCK_MARKERS: &str = "sentinel: block|sentinel block|sentinel returned block|sentinel status: block|sentinel verdict: block|sentinel result: block|sentinel gate returned block|reviewer gate returned block";
const UNOBSERVABLE_MARKERS: &str = "sentinel: unobservable|sentinel unobservable|sentinel status: unobservable|sentinel verdict: unobservable|sentinel result: unobservable|sentinel gate returned unobservable|sentinel pending|has not returned|hasn't returned|not returned|did not return pass or block|no pass or block|no pass/block|no verdict|stuck waiting|waiting for verdict|pending verdict|pending after bounded wait|delayed after bounded wait|timed out after bounded wait|produced no verdict|still running";
const READINESS_MARKERS: &str = "merge-ready|merge ready|merge-readiness|merge readiness|merge readiness: yes|merge readiness yes|merge readiness: true|merge readiness true|ready to merge|ready for merge|ready for merge gates|ready for parent handoff|ready for handoff|pr-ready|pr ready|pr-readiness|pr readiness|pr readiness: yes|pr readiness yes|pr readiness: true|pr readiness true|pull-request-ready|pull request ready|parent can open pr next|parent can create pr next|parent can open the pr next|push-ready|push ready|push-readiness|ready to push|ready for push|push readiness|push readiness: yes|push readiness yes|push readiness: true|push readiness true|pushed: yes|pushed yes|pushed: true|pushed true|remote/pr head match: yes|remote/pr head match yes|remote and pr head match";
const MAINTAINER_FALLBACK_APPROVAL_MARKERS: &str = "maintainer explicitly approved fallback|maintainer explicitly approved a fallback|maintainer explicitly approved the fallback|maintainer approval: fallback approved|maintainer approval fallback approved";
const FUTURE_STATUS_CONTEXT_MARKERS: &str = "before push|before readiness|before handoff|before merge|before parent handoff|before pr readiness|before merge readiness|before push readiness|required before|needed before|must pass before|needs to pass before|should pass before";
const FUTURE_STATUS_PREFIX_MARKERS: &str = "waiting for|wait for|awaiting|will rerun|will re-run|needs rerun|needs re-run|need rerun|need re-run|rerun required|re-run required";
const STATUS_NOISE_WORDS: &str = "pass|passed|passes|block|blocked|returned|return|test|tests|focused|but|before|after|waiting|wait|rerun|retry";
pub(super) fn check(handoff: &str, head_ref_oid: Option<&str>) -> Vec<String> {
    let text = handoff.to_ascii_lowercase();
    if !has_any(&text, READINESS_MARKERS) {
        return Vec::new();
    }
    if !has_any(&text, SENTINEL_MARKERS) {
        return vec!["Sentinel readiness evidence must be present".into()];
    }
    let status = status_marker_starts(&text)
        .into_iter()
        .max_by_key(|(start, _)| *start);
    match status {
        Some((_, SentinelStatus::Block)) | Some((_, SentinelStatus::Unobservable))
            if has_any(&text, MAINTAINER_FALLBACK_APPROVAL_MARKERS) =>
        {
            Vec::new()
        }
        Some((_, SentinelStatus::Block)) => {
            vec!["Sentinel BLOCK verdict cannot satisfy PR readiness or push readiness".into()]
        }
        Some((_, SentinelStatus::Unobservable)) => vec![
            "Sentinel UNOBSERVABLE or pending verdict cannot satisfy PR readiness or push readiness"
                .into(),
        ],
        Some((start, SentinelStatus::Pass)) if names_head(&text, start, head_ref_oid) => Vec::new(),
        Some((_, SentinelStatus::Pass)) => {
            vec!["Sentinel PASS readiness evidence must name the current PR head SHA".into()]
        }
        None => vec![
            "Sentinel readiness evidence must state PASS, BLOCK, or UNOBSERVABLE explicitly".into(),
        ],
    }
}
fn names_head(text: &str, start: usize, head_ref_oid: Option<&str>) -> bool {
    let Some(head) = head_ref_oid.map(str::trim).filter(|head| !head.is_empty()) else {
        return false;
    };
    let bounds = clause_bounds(text, start);
    text[bounds.0..bounds.1].contains(&head.to_ascii_lowercase())
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

fn status_marker_starts(text: &str) -> Vec<(usize, SentinelStatus)> {
    let explicit_statuses = PASS_MARKERS
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
        .flat_map(|(status, phrase)| {
            affirmed_phrase_starts(text, phrase).map(move |start| (start, status, phrase))
        });

    let named_reviewer_statuses = [
        (SentinelStatus::Pass, "pass"),
        (SentinelStatus::Block, "block"),
        (SentinelStatus::Unobservable, "unobservable"),
    ]
    .into_iter()
    .flat_map(|(status, phrase)| {
        affirmed_phrase_starts(text, phrase).map(move |start| (start, status, phrase))
    });

    explicit_statuses
        .chain(named_reviewer_statuses)
        .filter(|(start, _, phrase)| is_sentinel_status_context(text, *start, phrase))
        .filter(|(start, _, phrase)| !has_future_status_context(text, *start, phrase))
        .map(|(start, status, _)| (start, status))
        .collect()
}

fn is_sentinel_status_context(text: &str, start: usize, phrase: &str) -> bool {
    if phrase.contains("sentinel") || phrase.contains("reviewer gate") {
        return true;
    }
    let context_start = last_status_context_boundary(&text[..start]).unwrap_or(0);
    let prefix = &text[context_start..start];
    let Some(marker_end) = last_sentinel_marker_end(prefix) else {
        return false;
    };
    reviewer_name_context(&prefix[marker_end..])
}

fn has_future_status_context(text: &str, start: usize, phrase: &str) -> bool {
    let end = start + phrase.len();
    let (clause_start, clause_end) = clause_bounds(text, start);
    let prefix = &text[clause_start..start];
    let suffix = &text[end..clause_end];
    has_any(prefix, FUTURE_STATUS_PREFIX_MARKERS) || has_any(suffix, FUTURE_STATUS_CONTEXT_MARKERS)
}

fn affirmed_phrase_starts<'a>(text: &'a str, phrase: &'a str) -> impl Iterator<Item = usize> + 'a {
    let mut rest = text;
    let mut offset = 0;
    std::iter::from_fn(move || {
        while let Some(index) = rest.find(phrase) {
            let start = offset + index;
            let end = start + phrase.len();
            offset = end;
            rest = &text[offset..];
            if phrase_has_boundaries(text, start, end)
                && !is_locally_negated(&text[..start])
                && !has_negative_label_value(&text[end..])
            {
                return Some(start);
            }
        }
        None
    })
}

fn has_affirmed_phrase(text: &str, phrase: &str) -> bool {
    affirmed_phrase_starts(text, phrase).next().is_some()
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
                "no" | "not" | "without" | "isn't" | "wasn't" | "hasn't"
            )
        })
}

fn last_clause_boundary(text: &str) -> Option<usize> {
    text.rfind(['.', '!', '?', ';', ':', ',', '\n'])
        .map(|index| index + 1)
}

fn next_clause_boundary(text: &str) -> Option<usize> {
    text.find(['.', '!', '?', ';', '\n'])
}

fn last_status_context_boundary(text: &str) -> Option<usize> {
    text.rfind(['.', '!', '?', ';', '\n'])
        .map(|index| index + 1)
}

fn last_sentinel_marker_end(text: &str) -> Option<usize> {
    SENTINEL_MARKERS
        .split('|')
        .filter_map(|phrase| {
            affirmed_phrase_starts(text, phrase)
                .last()
                .map(|start| start + phrase.len())
        })
        .max()
}

fn reviewer_name_context(text: &str) -> bool {
    let words: Vec<_> = text
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    words.is_empty() || words.len() <= 4 && !words.iter().any(|word| status_noise_word(word))
}

fn status_noise_word(word: &str) -> bool {
    STATUS_NOISE_WORDS.split('|').any(|noise| word == noise)
}

fn clause_bounds(text: &str, start: usize) -> (usize, usize) {
    let clause_start = last_clause_boundary(&text[..start]).unwrap_or(0);
    let suffix = &text[start..];
    let clause_end = next_clause_boundary(suffix)
        .map(|offset| start + offset)
        .unwrap_or(text.len());
    (clause_start, clause_end)
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

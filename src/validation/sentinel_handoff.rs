pub(super) const SENTINEL_MARKERS: &str = "sentinel|codexy-sentinel";
const GENERIC_REVIEWER_GATE_MARKERS: &str = "reviewer gate|reviewer-gate";
const PASS_MARKERS: &str = "sentinel: pass|sentinel pass|sentinel returned pass|sentinel status: pass|sentinel verdict: pass|sentinel result: pass|sentinel gate returned pass";
const BLOCK_MARKERS: &str = "sentinel: block|sentinel block|sentinel returned block|sentinel status: block|sentinel verdict: block|sentinel result: block|sentinel gate returned block|reviewer gate: block|reviewer gate returned block|reviewer gate block|reviewer gate verdict: block|reviewer gate result: block|reviewer-gate: block|reviewer-gate returned block|reviewer-gate block|reviewer-gate verdict: block|reviewer-gate result: block";
const UNOBSERVABLE_MARKERS: &str = "sentinel: unobservable|sentinel unobservable|sentinel status: unobservable|sentinel verdict: unobservable|sentinel result: unobservable|sentinel gate returned unobservable|sentinel pending|has not returned|hasn't returned|not returned|did not return pass or block|no pass or block|no pass/block|no verdict|stuck waiting|waiting for verdict|pending verdict|pending after bounded wait|delayed after bounded wait|timed out after bounded wait|produced no verdict|still running";
const READINESS_MARKERS: &str = "merge-ready|merge ready|merge-readiness|merge readiness|merge readiness: yes|merge readiness yes|merge readiness: true|merge readiness true|ready to merge|ready for merge|ready for merge gates|ready for parent handoff|ready for handoff|parent-handoff-ready|parent handoff ready|pr-ready|pr ready|pr is ready|pr-readiness|pr readiness|pr readiness: yes|pr readiness yes|pr readiness: true|pr readiness true|pull-request-ready|pull request ready|pull request is ready|parent can merge|parent can open pr next|parent can create pr next|parent can open the pr next|push-ready|push ready|push-readiness|ready to push|ready for push|push readiness|push readiness: yes|push readiness yes|push readiness: true|push readiness true|pushed: yes|pushed yes|pushed: true|pushed true|remote/pr head match: yes|remote/pr head match yes|remote and pr head match";
const HISTORICAL_STATUS_PREFIX_MARKERS: &str = "previous sentinel|prior sentinel|old sentinel|earlier sentinel|superseded sentinel|initial sentinel|previous codexy-sentinel|prior codexy-sentinel|old codexy-sentinel|earlier codexy-sentinel|superseded codexy-sentinel|initial codexy-sentinel|previous reviewer gate|prior reviewer gate|old reviewer gate|earlier reviewer gate|superseded reviewer gate|initial reviewer gate|previous reviewer-gate|prior reviewer-gate|old reviewer-gate|earlier reviewer-gate|superseded reviewer-gate|initial reviewer-gate";
const FUTURE_STATUS_CONTEXT_MARKERS: &str = "before push|before readiness|before handoff|before merge|before parent handoff|before pr readiness|before merge readiness|before push readiness|required before|needed before|must pass before|needs to pass before|should pass before|planned after|after planned|planned rerun|planned review|planned pass|to be run|will be run";
const FUTURE_STATUS_PREFIX_MARKERS: &str = "waiting for|wait for|waiting on|wait on|awaiting|pending|will rerun|will re-run|will return|will report|will be|expected to|is expected to|should return|should report|should be|needs rerun|needs re-run|need rerun|need re-run|rerun required|re-run required";
const STATUS_NOISE_WORDS: &str =
    "pass|passed|passes|block|blocked|test|tests|focused|but|before|after|waiting|wait|rerun|retry";
const LOCAL_NEGATION_WORDS: &str = "no|not|without|never|isn't|aren't|wasn't|hasn't|haven't|didn't|doesn't|don't|can't|cannot|won't";
pub(super) fn check(handoff: &str, head_ref_oid: Option<&str>) -> Vec<String> {
    let text = handoff.to_ascii_lowercase();
    let claims_readiness = claims_readiness(&text);
    let claims_completion = super::completion_handoff::claims_completion(handoff);
    let has_sentinel = has_any(&text, SENTINEL_MARKERS);
    if !claims_readiness && !claims_completion {
        return Vec::new();
    }
    if has_any(&text, GENERIC_REVIEWER_GATE_MARKERS) && !has_any(&text, SENTINEL_MARKERS) {
        return vec![
            "Generic reviewer-gate evidence cannot satisfy packaged Sentinel readiness proof"
                .into(),
        ];
    }
    if !has_sentinel {
        return vec!["Sentinel readiness evidence must be present".into()];
    }
    let status = status_marker_starts(&text)
        .into_iter()
        .max_by_key(|(start, _)| *start);
    match status {
        Some((start, SentinelStatus::Unobservable))
            if super::sentinel_handoff_evidence::fallback_after(&text, start)
                && super::sentinel_handoff_evidence::names_head(&text, start, head_ref_oid) =>
        {
            Vec::new()
        }
        Some((start, SentinelStatus::Block))
            if super::sentinel_handoff_evidence::fallback_after(&text, start)
                && super::sentinel_handoff_evidence::names_head(&text, start, head_ref_oid) =>
        {
            Vec::new()
        }
        Some((start, SentinelStatus::Unobservable))
            if super::sentinel_handoff_evidence::fallback_after(&text, start) =>
        {
            vec!["Sentinel fallback readiness evidence must name the current PR head SHA".into()]
        }
        Some((start, SentinelStatus::Block))
            if super::sentinel_handoff_evidence::fallback_after(&text, start) =>
        {
            vec!["Sentinel fallback readiness evidence must name the current PR head SHA".into()]
        }
        Some((_, SentinelStatus::Block)) => {
            vec!["Sentinel BLOCK verdict cannot satisfy PR readiness or push readiness".into()]
        }
        Some((_, SentinelStatus::Unobservable)) => vec![
            "Sentinel UNOBSERVABLE or pending verdict cannot satisfy PR readiness or push readiness"
                .into(),
        ],
        Some((start, SentinelStatus::Pass))
            if !super::sentinel_handoff_evidence::names_head(&text, start, head_ref_oid) =>
        {
            vec!["Sentinel PASS readiness evidence must name the current PR head SHA".into()]
        }
        Some((start, SentinelStatus::Pass))
            if !super::sentinel_handoff_reviewer::pass_names_reviewer(&text, start) =>
        {
            vec!["Sentinel PASS readiness evidence must name the packaged Sentinel reviewer".into()]
        }
        Some((_, SentinelStatus::Pass)) => Vec::new(),
        None => vec![
            "Sentinel readiness evidence must state PASS, BLOCK, or UNOBSERVABLE explicitly".into(),
        ],
    }
}
fn claims_readiness(text: &str) -> bool {
    has_any(text, READINESS_MARKERS)
        || child_handoff_claims_current_pr_readiness(text)
        || super::codex_review_handoff_readiness::claims_completion(text)
}
fn child_handoff_claims_current_pr_readiness(text: &str) -> bool {
    let claims_child = super::child_handoff_readiness_claims::child_readiness(text);
    claims_child
        && (super::child_handoff_readiness_claims::pr_ready(text)
            || super::child_handoff_readiness_claims::synced(text)
            || super::child_handoff_readiness_claims::pushed(text))
}
pub(super) fn has_any(text: &str, phrases: &str) -> bool {
    phrases
        .split('|')
        .any(|phrase| affirmed_phrase_starts(text, phrase).next().is_some())
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
    if phrase.contains("sentinel")
        || phrase.contains("reviewer gate")
        || phrase.contains("reviewer-gate")
    {
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
    let context_start = last_status_context_boundary(&text[..start]).unwrap_or(clause_start);
    let prefix = &text[context_start..start];
    let status_context = &text[context_start..end];
    let suffix = &text[end..clause_end];
    has_any(prefix, FUTURE_STATUS_PREFIX_MARKERS)
        || has_any(status_context, HISTORICAL_STATUS_PREFIX_MARKERS)
        || has_any(suffix, FUTURE_STATUS_CONTEXT_MARKERS)
        || prefix.trim_end().ends_with(" will")
}
pub(super) fn affirmed_phrase_starts<'a>(
    text: &'a str,
    phrase: &'a str,
) -> impl Iterator<Item = usize> + 'a {
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
                && !super::sentinel_handoff_evidence::has_non_claim_phrase_context(
                    &text[..start],
                    &text[end..],
                )
            {
                return Some(start);
            }
        }
        None
    })
}
fn is_locally_negated(prefix: &str) -> bool {
    let clause = &prefix[last_clause_boundary(prefix).unwrap_or(0)..];
    clause
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '\'')
        .filter(|word| !word.is_empty())
        .rev()
        .take(4)
        .any(|word| {
            LOCAL_NEGATION_WORDS
                .split('|')
                .any(|negation| word == negation)
        })
}
fn last_clause_boundary(text: &str) -> Option<usize> {
    text.rfind(['.', '!', '?', ';', ':', ',', '\n'])
        .map(|index| index + 1)
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
    words.is_empty()
        || words.len() <= 4
            && !words
                .iter()
                .any(|word| STATUS_NOISE_WORDS.split('|').any(|noise| *word == noise))
}
pub(super) fn clause_bounds(text: &str, start: usize) -> (usize, usize) {
    let clause_start = last_clause_boundary(&text[..start]).unwrap_or(0);
    let suffix = &text[start..];
    let clause_end = suffix
        .find(['.', '!', '?', ';', '\n'])
        .map(|offset| start + offset)
        .unwrap_or(text.len());
    (clause_start, clause_end)
}
fn phrase_has_boundaries(text: &str, start: usize, end: usize) -> bool {
    is_boundary(text[..start].chars().next_back()) && is_boundary(text[end..].chars().next())
}
pub(super) fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| !character.is_ascii_alphanumeric())
}

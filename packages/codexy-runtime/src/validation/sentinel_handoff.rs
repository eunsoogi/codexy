pub(super) const SENTINEL_MARKERS: &str = "sentinel|codexy-sentinel";
#[path = "sentinel_handoff_result.rs"]
mod result;
#[path = "sentinel_handoff_result_context.rs"]
mod result_context;
#[cfg(test)]
#[path = "sentinel_handoff_result_tests.rs"]
mod result_tests;

pub(super) fn packaged_terminal_result(text: &str) -> bool {
    result_context::packaged_terminal_result(text)
}

pub(super) fn active_result_line(text: &str, start: usize) -> bool {
    result::active(text, start)
}

const GENERIC_REVIEWER_GATE_MARKERS: &str = "reviewer gate|reviewer-gate";
const READINESS_MARKERS: &str = "merge-ready|merge ready|merge-readiness|merge readiness|merge readiness: yes|merge readiness yes|merge readiness: true|merge readiness true|ready to merge|ready for merge|ready for merge gates|ready for parent handoff|ready for handoff|parent-handoff-ready|parent handoff ready|pr-ready|pr ready|pr is ready|pr-readiness|pr readiness|pr readiness: yes|pr readiness yes|pr readiness: true|pr readiness true|pull-request-ready|pull request ready|pull request is ready|parent can merge|parent can open pr next|parent can create pr next|parent can open the pr next|push-ready|push ready|push-readiness|ready to push|ready for push|push readiness|push readiness: yes|push readiness yes|push readiness: true|push readiness true|pushed: yes|pushed yes|pushed: true|pushed true|remote/pr head match: yes|remote/pr head match yes|remote and pr head match";
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
    let status = match result::select(&text) {
        result::Selection::Modeled(status) => status,
        result::Selection::ReviewerChanged => {
            return vec![
                "Sentinel reviewer changed or duplicated during a non-terminal wait; retain the same reviewer for its natural terminal result".into(),
            ];
        }
    };
    match status {
        Some((
            start,
            super::sentinel_handoff_status::SentinelState::Terminal(
                super::sentinel_handoff_status::TerminalStatus::Unobservable,
            ),
        ))
            if super::sentinel_handoff_evidence::fallback_after(&text, start)
                && super::sentinel_handoff_evidence::names_head(&text, start, head_ref_oid) =>
        {
            Vec::new()
        }
        Some((
            start,
            super::sentinel_handoff_status::SentinelState::Terminal(
                super::sentinel_handoff_status::TerminalStatus::Block,
            ),
        ))
            if super::sentinel_handoff_evidence::fallback_after(&text, start)
                && super::sentinel_handoff_evidence::names_head(&text, start, head_ref_oid) =>
        {
            Vec::new()
        }
        Some((
            start,
            super::sentinel_handoff_status::SentinelState::Terminal(
                super::sentinel_handoff_status::TerminalStatus::Unobservable,
            ),
        ))
            if super::sentinel_handoff_evidence::fallback_after(&text, start) =>
        {
            vec!["Sentinel fallback readiness evidence must name the current PR head SHA".into()]
        }
        Some((
            start,
            super::sentinel_handoff_status::SentinelState::Terminal(
                super::sentinel_handoff_status::TerminalStatus::Block,
            ),
        ))
            if super::sentinel_handoff_evidence::fallback_after(&text, start) =>
        {
            vec!["Sentinel fallback readiness evidence must name the current PR head SHA".into()]
        }
        Some((
            _,
            super::sentinel_handoff_status::SentinelState::Terminal(
                super::sentinel_handoff_status::TerminalStatus::Block,
            ),
        )) => {
            vec!["Sentinel BLOCK verdict cannot satisfy PR readiness or push readiness".into()]
        }
        Some((_, super::sentinel_handoff_status::SentinelState::Running)) => vec![
            "Sentinel reviewer is still running; continue event-driven observation without messaging, interrupting, replacing, or duplicating it".into(),
        ],
        Some((_, super::sentinel_handoff_status::SentinelState::Pending)) => vec![
            "Sentinel reviewer remains pending after a bounded wait; retain the same reviewer for its natural terminal result".into(),
        ],
        Some((
            _,
            super::sentinel_handoff_status::SentinelState::Terminal(
                super::sentinel_handoff_status::TerminalStatus::Unobservable,
            ),
        )) => vec![
            "Sentinel UNOBSERVABLE verdict cannot satisfy PR readiness or push readiness".into(),
        ],
        Some((
            start,
            super::sentinel_handoff_status::SentinelState::Terminal(
                super::sentinel_handoff_status::TerminalStatus::Pass,
            ),
        ))
            if !super::sentinel_handoff_evidence::names_head(&text, start, head_ref_oid) =>
        {
            vec!["Sentinel PASS readiness evidence must name the current PR head SHA".into()]
        }
        Some((
            start,
            super::sentinel_handoff_status::SentinelState::Terminal(
                super::sentinel_handoff_status::TerminalStatus::Pass,
            ),
        ))
            if !super::sentinel_handoff_reviewer::pass_names_reviewer(&text, start) =>
        {
            vec!["Sentinel PASS readiness evidence must name the packaged Sentinel reviewer".into()]
        }
        Some((
            _,
            super::sentinel_handoff_status::SentinelState::Terminal(
                super::sentinel_handoff_status::TerminalStatus::Pass,
            ),
        )) => Vec::new(),
        None => vec![
            "Sentinel readiness evidence must state PASS, BLOCK, or UNOBSERVABLE explicitly".into(),
        ],
    }
}
fn claims_readiness(text: &str) -> bool {
    let current = super::readiness_context::current_text(text);
    current.split(['\n', '.']).any(|fragment| {
        !super::completion_handoff_waiting::readiness_status::is_neutral_heading(fragment)
            && has_any(fragment, READINESS_MARKERS)
    }) || child_handoff_claims_current_pr_readiness(&current)
        || super::completion_handoff::claims_completion(&current)
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

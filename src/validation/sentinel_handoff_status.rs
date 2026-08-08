use super::sentinel_handoff::{SENTINEL_MARKERS, affirmed_phrase_starts, clause_bounds, has_any};

const PASS: &str = "sentinel: pass|sentinel pass|sentinel returned pass|sentinel status: pass|sentinel verdict: pass|sentinel result: pass|sentinel gate returned pass";
const BLOCK: &str = "sentinel: block|sentinel block|sentinel returned block|sentinel status: block|sentinel verdict: block|sentinel result: block|sentinel gate returned block|reviewer gate: block|reviewer gate returned block|reviewer gate block|reviewer gate verdict: block|reviewer gate result: block|reviewer-gate: block|reviewer-gate returned block|reviewer-gate block|reviewer-gate verdict: block|reviewer-gate result: block";
const UNOBSERVABLE: &str = "sentinel: unobservable|sentinel unobservable|sentinel status: unobservable|sentinel verdict: unobservable|sentinel result: unobservable|sentinel gate returned unobservable";
const PENDING: &str = "sentinel pending|has not returned|hasn't returned|not returned|did not return pass or block|no pass or block|no pass/block|no verdict|stuck waiting|waiting for verdict|pending verdict|pending after bounded wait|delayed after bounded wait|timed out after bounded wait|produced no verdict";
const RUNNING: &str = "still running|is running";
const HISTORICAL: &str = "previous sentinel|prior sentinel|old sentinel|earlier sentinel|superseded sentinel|initial sentinel|previous codexy-sentinel|prior codexy-sentinel|old codexy-sentinel|earlier codexy-sentinel|superseded codexy-sentinel|initial codexy-sentinel|previous reviewer gate|prior reviewer gate|old reviewer gate|earlier reviewer gate|superseded reviewer gate|initial reviewer gate|previous reviewer-gate|prior reviewer-gate|old reviewer-gate|earlier reviewer-gate|superseded reviewer-gate|initial reviewer-gate";
const FUTURE: &str = "before push|before readiness|before handoff|before merge|before parent handoff|before pr readiness|before merge readiness|before push readiness|required before|needed before|must pass before|needs to pass before|should pass before|planned after|after planned|planned rerun|planned review|planned pass|to be run|will be run";
const FUTURE_PREFIX: &str = "waiting for|wait for|waiting on|wait on|awaiting|pending|will rerun|will re-run|will return|will report|will be|expected to|is expected to|should return|should report|should be|needs rerun|needs re-run|need rerun|need re-run|rerun required|re-run required";
const NOISE: &str = "pass|passed|passes|block|blocked|test|tests|focused|but|before|after|waiting|wait|rerun|retry|evidence|proof";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SentinelState {
    Pending,
    Running,
    Terminal(TerminalStatus),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TerminalStatus {
    Pass,
    Block,
    Unobservable,
}

pub(super) fn marker_starts(text: &str) -> Vec<(usize, SentinelState)> {
    let statuses = [
        (SentinelState::Terminal(TerminalStatus::Pass), PASS),
        (SentinelState::Terminal(TerminalStatus::Block), BLOCK),
        (SentinelState::Pending, PENDING),
        (SentinelState::Running, RUNNING),
        (
            SentinelState::Terminal(TerminalStatus::Unobservable),
            UNOBSERVABLE,
        ),
    ]
    .into_iter()
    .flat_map(|(status, phrases)| {
        phrases.split('|').flat_map(move |phrase| {
            affirmed_phrase_starts(text, phrase).map(move |start| (start, status, phrase))
        })
    });
    let named = [
        (SentinelState::Terminal(TerminalStatus::Pass), "pass"),
        (SentinelState::Terminal(TerminalStatus::Block), "block"),
        (
            SentinelState::Terminal(TerminalStatus::Unobservable),
            "unobservable",
        ),
    ]
    .into_iter()
    .flat_map(|(status, phrase)| {
        affirmed_phrase_starts(text, phrase).map(move |start| (start, status, phrase))
    });
    statuses
        .chain(named)
        .filter(|(start, _, phrase)| sentinel_context(text, *start, phrase))
        .filter(|(start, _, phrase)| !future_context(text, *start, phrase))
        .map(|(start, status, _)| (start, status))
        .collect()
}

fn sentinel_context(text: &str, start: usize, phrase: &str) -> bool {
    if phrase.contains("sentinel")
        || phrase.contains("reviewer gate")
        || phrase.contains("reviewer-gate")
    {
        return true;
    }
    let boundary = text[..start]
        .rfind(['.', '!', '?', ';', '\n'])
        .map(|i| i + 1)
        .unwrap_or(0);
    let prefix = &text[boundary..start];
    let marker_end = SENTINEL_MARKERS
        .split('|')
        .filter_map(|marker| {
            affirmed_phrase_starts(prefix, marker)
                .last()
                .map(|i| i + marker.len())
        })
        .max();
    marker_end.is_some_and(|end| reviewer_name_context(&prefix[end..]))
        || has_any(
            &text[result_bounds(text, start).0..result_bounds(text, start).1],
            "current run",
        )
}

fn future_context(text: &str, start: usize, phrase: &str) -> bool {
    let end = start + phrase.len();
    let (clause_start, clause_end) = clause_bounds(text, start);
    let boundary = text[..start]
        .rfind(['.', '!', '?', ';', '\n'])
        .map(|i| i + 1)
        .unwrap_or(clause_start);
    let prefix = &text[boundary..start];
    let status = &text[boundary..end];
    let suffix = &text[end..clause_end];
    has_any(prefix, FUTURE_PREFIX)
        || has_any(status, HISTORICAL)
        || has_any(suffix, FUTURE)
        || prefix.trim_end().ends_with(" will")
}

fn reviewer_name_context(text: &str) -> bool {
    let words: Vec<_> = text
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    words.is_empty()
        || words.len() <= 4
            && !words
                .iter()
                .any(|word| NOISE.split('|').any(|noise| *word == noise))
}

fn result_bounds(text: &str, start: usize) -> (usize, usize) {
    let beginning = text[..start]
        .rfind(['.', '!', '?', ';', '\n'])
        .map(|i| i + 1)
        .unwrap_or(0);
    let ending = text[start..]
        .find(['.', '!', '?', ';', '\n'])
        .map(|i| start + i)
        .unwrap_or(text.len());
    (beginning, ending)
}

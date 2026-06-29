use super::duplicate_state_targets;
use serde_json::Value;

#[rustfmt::skip]
const DUPLICATE_STATE_PHRASES: &[&str] = &["duplicate/no-active-work", "no-active-work", "no active work", "duplicate pr", "duplicate issue", "duplicate lane"];
#[rustfmt::skip]
const DUPLICATE_STATE_TARGETS: &[&str] = &["pr #", "pull request #", "issue #", "github state", "current issue", "current pr", "current pull request"];
#[rustfmt::skip]
const DUPLICATE_STATE_CHECKS: &[&str] = &["re-check", "rechecked", "re-checked", "checked", "confirmed", "current github state", "after current"];
#[rustfmt::skip]
const CODEXY_CONTRACT_PHRASES: &[&str] = &["@codexy", "$codex-orchestration", "active codexy workflow", "active codexy plugin workflow", "preserve codexy workflow", "preserved codexy workflow", "routes through $codex-orchestration", "route through $codex-orchestration"];
#[rustfmt::skip]
const OWNERSHIP_BOUNDARY_PHRASES: &[&str] = &["child-owned", "child owned", "parent orchestrator", "parent monitors", "parent monitor", "who may edit", "who may only orchestrate", "only orchestrate", "receive edits"];
#[rustfmt::skip]
const NEGATED_CONTRACT_PHRASES: &[&str] = &["not captured", "not active", "not available", "not preserved", "was not preserved", "no active @codexy", "no active codexy workflow", "missing", "omitted", "without @codexy", "without codexy"];
#[rustfmt::skip]
const PLANNED_CODEXY_CONTRACT_PHRASES: &[&str] = &["should be restored", "should be preserved", "to be restored", "to be preserved", "will be restored", "will be preserved", "needs to be restored", "needs to be preserved"];
#[rustfmt::skip]
const NEGATED_DUPLICATE_STATE_PHRASES: &[&str] = &["not captured", "not checked", "not re-checked", "not preserved", "was not preserved", "did not check", "missing", "omitted", "without checking", "no duplicate/no-active-work state was captured", "no duplicate state was captured", "no no-active-work state was captured"];
#[rustfmt::skip]
const PLANNED_DUPLICATE_STATE_PHRASES: &[&str] = &["should be checked", "should be re-checked", "to be checked", "to be re-checked", "will be checked", "will be re-checked", "needs to be checked", "needs to be re-checked"];
#[rustfmt::skip]
const NEGATED_OWNERSHIP_BOUNDARY_PHRASES: &[&str] = &["not captured", "not available", "not preserved", "was not preserved", "missing", "omitted", "without boundary", "without ownership", "no parent/child ownership boundary was captured", "no parent-child ownership boundary was captured", "no ownership boundary was captured"];
#[rustfmt::skip]
const PLANNED_OWNERSHIP_BOUNDARY_PHRASES: &[&str] = &["should be captured", "should be preserved", "to be captured", "to be preserved", "will be captured", "will be preserved", "needs to be captured", "needs to be preserved"];
#[rustfmt::skip]
const PLANNED_STOP_CONDITION_PHRASES: &[&str] = &["should stop", "should be checked", "should be captured", "should be preserved", "to be checked", "to be captured", "to be preserved", "will be checked", "will be captured", "will be preserved"];

pub(super) fn has_codexy_orchestration_contract(text: &str) -> bool {
    text.lines().any(|line| {
        codexy_contract_value(line.trim()).is_some_and(|contract| {
            has_real_value(contract)
                && has_codexy_contract_phrase(contract)
                && !has_planned_codexy_contract_evidence(contract)
                && !has_negated_contract_evidence(contract)
        })
    })
}

pub(super) fn has_duplicate_or_no_active_work_state(text: &str, pr_state: &Value) -> bool {
    text.lines().any(|line| {
        duplicate_state_value(line.trim()).is_some_and(|state| {
            has_real_value(state)
                && has_duplicate_state_phrase(state)
                && has_concrete_duplicate_state_evidence(state)
                && duplicate_state_targets::matches_current_duplicate_state_target(state, pr_state)
                && !has_planned_duplicate_state_evidence(state)
                && !has_negated_duplicate_state_evidence(state)
        })
    })
}

pub(super) fn has_parent_child_ownership_boundary(text: &str) -> bool {
    text.lines().any(|line| {
        ownership_boundary_value(line.trim()).is_some_and(|boundary| {
            has_real_value(boundary)
                && has_ownership_boundary_phrase(boundary)
                && !has_planned_ownership_boundary_evidence(boundary)
                && !has_negated_ownership_boundary_evidence(boundary)
        })
    })
}

pub(super) fn has_authoritative_stop_condition(text: &str) -> bool {
    text.lines().any(|line| {
        ["stop condition", "authoritative stop condition"]
            .iter()
            .any(|label| {
                field_value(line.trim(), label).is_some_and(|condition| {
                    has_real_value(condition)
                        && !has_planned_stop_condition_evidence(condition)
                        && !has_negated_stop_condition_evidence(condition)
                })
            })
    })
}

fn codexy_contract_value<'a>(line: &'a str) -> Option<&'a str> {
    [
        "codexy orchestration contract",
        "codexy plugin workflow",
        "codexy workflow",
    ]
    .iter()
    .find_map(|label| field_value(line, label))
}

fn duplicate_state_value<'a>(line: &'a str) -> Option<&'a str> {
    [
        "duplicate/no-active-work state",
        "duplicate state",
        "no-active-work state",
        "no active work state",
    ]
    .iter()
    .find_map(|label| field_value(line, label))
}

fn ownership_boundary_value<'a>(line: &'a str) -> Option<&'a str> {
    [
        "parent/child ownership boundary",
        "parent-child ownership boundary",
        "ownership boundary",
    ]
    .iter()
    .find_map(|label| field_value(line, label))
}

fn field_value<'a>(line: &'a str, label: &str) -> Option<&'a str> {
    metadata_line(line)
        .strip_prefix(label)
        .and_then(|rest| {
            rest.strip_prefix(':')
                .or_else(|| rest.strip_prefix(" -"))
                .or_else(|| rest.strip_prefix(" is "))
        })
        .map(str::trim)
}

#[rustfmt::skip]
fn metadata_line(line: &str) -> &str { let line = line.trim().trim_start_matches(['-', '*']).trim_start(); let line = line.strip_prefix("[x]").or_else(|| line.strip_prefix("[X]")).unwrap_or(line).trim_start(); line.trim_start_matches('#').trim_start() }

fn has_codexy_contract_phrase(text: &str) -> bool {
    has_any(text, CODEXY_CONTRACT_PHRASES)
}

fn has_duplicate_state_phrase(text: &str) -> bool {
    has_any(text, DUPLICATE_STATE_PHRASES)
}
fn has_concrete_duplicate_state_evidence(text: &str) -> bool {
    has_any(text, DUPLICATE_STATE_TARGETS) && has_any(text, DUPLICATE_STATE_CHECKS)
}

fn has_ownership_boundary_phrase(text: &str) -> bool {
    has_any(text, OWNERSHIP_BOUNDARY_PHRASES)
}

fn has_negated_contract_evidence(text: &str) -> bool {
    has_any(text, NEGATED_CONTRACT_PHRASES)
}

fn has_planned_codexy_contract_evidence(text: &str) -> bool {
    has_any(text, PLANNED_CODEXY_CONTRACT_PHRASES)
}

fn has_negated_duplicate_state_evidence(text: &str) -> bool {
    has_any(text, NEGATED_DUPLICATE_STATE_PHRASES)
}

fn has_planned_duplicate_state_evidence(text: &str) -> bool {
    has_any(text, PLANNED_DUPLICATE_STATE_PHRASES)
}

fn has_negated_ownership_boundary_evidence(text: &str) -> bool {
    has_any(text, NEGATED_OWNERSHIP_BOUNDARY_PHRASES)
}

fn has_planned_ownership_boundary_evidence(text: &str) -> bool {
    has_any(text, PLANNED_OWNERSHIP_BOUNDARY_PHRASES)
}

fn has_negated_stop_condition_evidence(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    has_any(
        &text,
        &[
            "current stop condition was not captured",
            "current stop condition was not preserved",
            "authoritative stop condition was not captured",
            "authoritative stop condition was not preserved",
            "no stop condition",
            "no current stop condition",
            "no authoritative stop condition",
            "stop condition was not captured",
            "stop condition was not preserved",
            "stop condition is missing",
            "stop condition was missing",
            "stop condition missing",
            "evidence is missing",
            "evidence was missing",
        ],
    )
}

fn has_planned_stop_condition_evidence(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    has_any(&text, PLANNED_STOP_CONDITION_PHRASES)
}

fn has_real_value(value: &str) -> bool {
    if value.is_empty() || is_bare_no_value(value) {
        return false;
    }

    ![
        "none",
        "false",
        "not captured",
        "not available",
        "not applicable",
        "not-applicable",
        "not preserved",
        "not requested",
        "not checked",
        "missing",
        "was not captured",
        "was not checked",
        "was not preserved",
        "n/a",
        "na",
    ]
    .iter()
    .any(|phrase| value.strip_prefix(phrase).is_some_and(starts_with_boundary))
}

fn is_bare_no_value(value: &str) -> bool {
    value.trim_matches(|character: char| {
        character.is_ascii_punctuation() || character.is_whitespace()
    }) == "no"
}

#[rustfmt::skip]
fn starts_with_boundary(rest: &str) -> bool { rest.chars().next().is_none_or(|character| !character.is_ascii_alphanumeric()) }

fn has_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}

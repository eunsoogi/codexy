#[rustfmt::skip]
const DUPLICATE_STATE_PHRASES: &[&str] = &["duplicate/no-active-work", "no-active-work", "no active work", "duplicate pr", "duplicate issue", "duplicate lane"];
#[rustfmt::skip]
const DUPLICATE_STATE_TARGETS: &[&str] = &["pr #", "pull request #", "issue #", "github state", "current issue", "current pr", "current pull request"];
#[rustfmt::skip]
const DUPLICATE_STATE_CHECKS: &[&str] = &["re-check", "rechecked", "re-checked", "checked", "confirmed", "current github state", "after current"];

pub(super) fn has_codexy_orchestration_contract(text: &str) -> bool {
    text.lines().any(|line| {
        codexy_contract_value(line.trim()).is_some_and(|contract| {
            has_real_value(contract)
                && has_codexy_contract_phrase(contract)
                && !has_negated_contract_evidence(contract)
        })
    })
}

pub(super) fn has_duplicate_or_no_active_work_state(text: &str) -> bool {
    text.lines().any(|line| {
        duplicate_state_value(line.trim()).is_some_and(|state| {
            has_real_value(state)
                && has_duplicate_state_phrase(state)
                && has_concrete_duplicate_state_evidence(state)
                && !has_negated_duplicate_state_evidence(state)
        })
    })
}

pub(super) fn has_parent_child_ownership_boundary(text: &str) -> bool {
    text.lines().any(|line| {
        ownership_boundary_value(line.trim()).is_some_and(|boundary| {
            has_real_value(boundary)
                && has_ownership_boundary_phrase(boundary)
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
                    has_real_value(condition) && !has_negated_stop_condition_evidence(condition)
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

fn metadata_line(line: &str) -> &str {
    let line = line.trim().trim_start_matches(['-', '*']).trim_start();
    line.strip_prefix("[x]")
        .or_else(|| line.strip_prefix("[X]"))
        .unwrap_or(line)
        .trim_start()
}

fn has_codexy_contract_phrase(text: &str) -> bool {
    has_any(
        text,
        &[
            "@codexy",
            "$codex-orchestration",
            "active codexy workflow",
            "codexy plugin workflow",
            "codexy workflow",
            "orchestration workflow",
        ],
    )
}

fn has_duplicate_state_phrase(text: &str) -> bool {
    has_any(text, DUPLICATE_STATE_PHRASES)
}

fn has_concrete_duplicate_state_evidence(text: &str) -> bool {
    has_any(text, DUPLICATE_STATE_TARGETS) && has_any(text, DUPLICATE_STATE_CHECKS)
}

fn has_ownership_boundary_phrase(text: &str) -> bool {
    has_any(
        text,
        &[
            "child-owned",
            "child owned",
            "parent orchestrator",
            "parent monitors",
            "parent monitor",
            "owner boundary",
            "ownership boundary",
        ],
    )
}

fn has_negated_contract_evidence(text: &str) -> bool {
    has_any(
        text,
        &[
            "not captured",
            "not active",
            "not available",
            "not preserved",
            "was not preserved",
            "missing",
            "omitted",
            "without @codexy",
            "without codexy",
        ],
    )
}

fn has_negated_duplicate_state_evidence(text: &str) -> bool {
    has_any(
        text,
        &[
            "not captured",
            "not checked",
            "not re-checked",
            "not preserved",
            "was not preserved",
            "did not check",
            "missing",
            "omitted",
            "without checking",
        ],
    )
}

fn has_negated_ownership_boundary_evidence(text: &str) -> bool {
    has_any(
        text,
        &[
            "not captured",
            "not available",
            "not preserved",
            "was not preserved",
            "missing",
            "omitted",
            "without boundary",
            "without ownership",
        ],
    )
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

fn starts_with_boundary(rest: &str) -> bool {
    rest.chars()
        .next()
        .is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn has_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}

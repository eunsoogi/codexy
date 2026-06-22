pub(super) fn check(handoff: &str) -> Vec<String> {
    let text = handoff.to_ascii_lowercase();
    if !claims_compacted_continuation_readiness(&text) {
        return Vec::new();
    }

    let mut errors = Vec::new();
    if !has_codexy_orchestration_contract(&text) {
        errors.push("compacted continuation evidence missing Codexy orchestration contract: include active @Codexy or $codex-orchestration workflow instructions before continuing".into());
    }
    if !has_duplicate_or_no_active_work_state(&text) {
        errors.push("compacted continuation evidence missing duplicate/no-active-work state: re-check current issue and PR status before editing".into());
    }
    if !has_parent_child_ownership_boundary(&text) {
        errors.push("compacted continuation evidence missing parent/child ownership boundary: preserve who may edit and who may only orchestrate".into());
    }
    if !has_authoritative_stop_condition(&text) {
        errors.push("compacted continuation evidence missing authoritative stop condition: include the current stop condition before continuing".into());
    }
    if !has_git_graph_log_preflight(&text) {
        errors.push("compacted continuation evidence missing git graph/log preflight: include pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph before editing".into());
    }
    errors
}

fn claims_compacted_continuation_readiness(text: &str) -> bool {
    has_any(
        text,
        &[
            "compacted continuation",
            "after compaction",
            "post-compaction",
            "post compaction",
            "context compaction",
            "goal continuation",
        ],
    ) && has_any(
        text,
        &[
            "ready to continue",
            "continuation readiness",
            "resume",
            "continue",
            "next action",
            "before editing",
        ],
    )
}

fn has_codexy_orchestration_contract(text: &str) -> bool {
    has_any(
        text,
        &[
            "@codexy",
            "$codex-orchestration",
            "codexy orchestration contract",
            "codexy plugin workflow",
            "active codexy workflow",
        ],
    )
}

fn has_duplicate_or_no_active_work_state(text: &str) -> bool {
    has_any(
        text,
        &[
            "duplicate/no-active-work",
            "no-active-work",
            "no active work",
            "duplicate pr",
            "duplicate issue",
            "duplicate lane",
        ],
    )
}

fn has_parent_child_ownership_boundary(text: &str) -> bool {
    has_any(
        text,
        &[
            "parent/child ownership",
            "parent-child ownership",
            "child-owned",
            "parent orchestrator",
            "ownership boundary",
        ],
    )
}

fn has_authoritative_stop_condition(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim();
        ["stop condition", "authoritative stop condition"]
            .iter()
            .any(|label| stop_condition_value(line, label).is_some_and(has_real_value))
    })
}

fn stop_condition_value<'a>(line: &'a str, label: &str) -> Option<&'a str> {
    line.strip_prefix(label)
        .and_then(|rest| {
            rest.strip_prefix(':')
                .or_else(|| rest.strip_prefix(" -"))
                .or_else(|| rest.strip_prefix(" is "))
        })
        .map(str::trim)
}

fn has_real_value(value: &str) -> bool {
    !value.is_empty()
        && ![
            "none",
            "false",
            "no",
            "not captured",
            "not available",
            "not applicable",
            "not-applicable",
            "n/a",
            "na",
        ]
        .iter()
        .any(|phrase| value.strip_prefix(phrase).is_some_and(starts_with_boundary))
}

fn starts_with_boundary(rest: &str) -> bool {
    rest.chars()
        .next()
        .is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn has_git_graph_log_preflight(text: &str) -> bool {
    [
        "pwd",
        "git status --short --branch",
        "git rev-parse head",
        "git rev-parse origin/main",
    ]
    .iter()
    .all(|phrase| text.contains(phrase))
        && text.contains("git log --graph")
}

fn has_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}

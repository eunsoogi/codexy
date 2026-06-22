pub(super) fn check(evidence: &str) -> Vec<String> {
    let mut errors = Vec::new();
    if has_false_thread_tool_blocker(evidence) {
        errors.push("thread-tool discovery evidence reports a blocker from tool_search absence even though thread/start and turn/start events prove a real thread surface exists".to_owned());
    }
    if has_forbidden_codex_cli_thread_fallback(evidence) {
        errors.push("thread-tool discovery evidence uses a forbidden Codex CLI or app-server fallback claim as a substitute for true thread/worktree tools".to_owned());
    }
    errors
}

pub(super) fn has_false_thread_tool_blocker(evidence: &str) -> bool {
    has_real_thread_event_evidence(evidence)
        && has_tool_search_miss(evidence)
        && evidence
            .lines()
            .any(has_affirmative_thread_tool_absence_blocker)
}

pub(super) fn has_forbidden_codex_cli_thread_fallback(evidence: &str) -> bool {
    has_thread_requirement_context(evidence)
        && evidence.lines().any(|line| {
            has_forbidden_codex_cli_surface(line)
                && has_fallback_or_substitution_claim(line)
                && (!has_negated_fallback_claim(line)
                    || has_affirmative_forbidden_satisfied_by_claim(line))
        })
}

fn has_real_thread_event_evidence(evidence: &str) -> bool {
    evidence.contains("thread/start") && evidence.contains("turn/start")
}

fn has_tool_search_miss(evidence: &str) -> bool {
    (evidence.contains("tool_search") || evidence.contains("tool search"))
        && [
            "miss",
            "missed",
            "no true thread tools",
            "no callable true thread tools",
            "no codex_app namespace",
            "not exposed",
            "absence",
            "absent",
        ]
        .into_iter()
        .any(|marker| evidence.contains(marker))
}

fn has_affirmative_thread_tool_absence_blocker(line: &str) -> bool {
    line.contains("blocker")
        && (line.contains("thread tool") || line.contains("thread/worktree tool"))
        && [
            "absent",
            "absence",
            "unavailable",
            "missing",
            "not available",
            "not exposed",
        ]
        .into_iter()
        .any(|marker| line.contains(marker))
        && !has_negated_blocker_claim(line)
}

fn has_thread_requirement_context(evidence: &str) -> bool {
    evidence.contains("thread requirement")
        || evidence.contains("thread tool")
        || evidence.contains("thread/worktree tool")
        || evidence.contains("child routing required")
        || evidence.contains("thread/worktree tool discovery")
}

fn has_forbidden_codex_cli_surface(evidence: &str) -> bool {
    [
        "codex exec",
        "codex fork",
        "codex app-server",
        "codex debug app-server",
        "app-server fallback",
    ]
    .into_iter()
    .any(|marker| evidence.contains(marker))
}

fn has_fallback_or_substitution_claim(evidence: &str) -> bool {
    [
        "fallback",
        "substitute",
        "substitution",
        "satisfied by",
        "satisfies",
        "as a replacement",
        "instead of thread",
    ]
    .into_iter()
    .any(|marker| evidence.contains(marker))
}

fn has_affirmative_forbidden_satisfied_by_claim(line: &str) -> bool {
    [
        "codex exec",
        "codex fork",
        "codex app-server",
        "codex debug app-server",
        "app-server fallback",
    ]
    .into_iter()
    .any(|surface| {
        [
            format!("satisfied by {surface}"),
            format!("satisfies {surface}"),
            format!("{surface} satisfies"),
        ]
        .into_iter()
        .any(|marker| line.contains(&marker))
    })
}

fn has_negated_fallback_claim(line: &str) -> bool {
    [
        "did not use",
        "didn't use",
        "do not use",
        "not used",
        "not use",
        "not fallback substitute",
        "not fallback substitutes",
        "not a fallback substitute",
        "not fallback or substitute",
        "no fallback",
        "no cli fallback",
        "without fallback",
        "without using",
    ]
    .into_iter()
    .any(|marker| line.contains(marker))
}

fn has_negated_blocker_claim(line: &str) -> bool {
    [
        "no blocker",
        "not a blocker",
        "not blocked",
        "not absent",
        "not unavailable",
        "not missing",
        "do not report",
        "must not report",
        "should not report",
    ]
    .into_iter()
    .any(|marker| line.contains(marker))
}

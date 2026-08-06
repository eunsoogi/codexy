pub(super) fn check(evidence: &str, original_evidence: &str) -> Vec<String> {
    let mut errors = Vec::new();
    errors.extend(super::child_lane_active_threads::check(evidence));
    if super::child_lane_ownership_subagents::has_subagent_as_thread_owner(evidence) {
        errors.push(
            "child-owned lane claims a subagent as the Codex subthread/worktree owner".to_owned(),
        );
    }
    if has_false_thread_tool_blocker(evidence) {
        errors.push("thread-tool discovery evidence reports a blocker from tool_search absence even though thread/start and turn/start events prove a real thread surface exists".to_owned());
    }
    if super::child_lane_thread_tool_handlers::has_uncaptured_defect(evidence, original_evidence) {
        errors.push("thread-tool handler evidence includes `No handler registered for tool` for an expected or discovered Codex app thread tool; report both the discovered tool surface and the missing handler as a dogfooding/tool-exposure defect instead of using ordinary unavailable-tool fallback".to_owned());
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
        && evidence
            .lines()
            .any(has_affirmative_forbidden_fallback_claim)
}

fn has_real_thread_event_evidence(evidence: &str) -> bool {
    let has_thread_start = evidence
        .lines()
        .any(|line| line.contains("thread/start") && !has_negated_thread_event_claim(line));
    let has_turn_start = evidence
        .lines()
        .any(|line| line.contains("turn/start") && !has_negated_thread_event_claim(line));

    has_thread_start && has_turn_start
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
    forbidden_surfaces().into_iter().any(|surface| {
        [
            format!("satisfied by {surface}"),
            format!("satisfies {surface}"),
            format!("{surface} satisfies"),
        ]
        .into_iter()
        .any(|marker| {
            line.match_indices(&marker)
                .any(|(start, _)| !has_negated_marker_prefix(line, start))
        })
    })
}

fn has_affirmative_forbidden_fallback_claim(line: &str) -> bool {
    if has_affirmative_forbidden_satisfied_by_claim(line) {
        return true;
    }
    forbidden_surfaces().into_iter().any(|surface| {
        surface_claims(line, surface)
            .into_iter()
            .any(|claim| has_fallback_or_substitution_claim(claim.text) && !claim.is_negated)
    })
}

fn forbidden_surfaces() -> [&'static str; 5] {
    [
        "codex exec",
        "codex fork",
        "codex app-server",
        "codex debug app-server",
        "app-server fallback",
    ]
}

struct SurfaceClaim<'a> {
    text: &'a str,
    is_negated: bool,
}

fn surface_claims<'a>(line: &'a str, surface: &str) -> Vec<SurfaceClaim<'a>> {
    line.match_indices(surface)
        .map(|(start, _)| {
            let end = line[start..]
                .find([';', '.'])
                .map_or(line.len(), |offset| start + offset);
            SurfaceClaim {
                text: &line[start..end],
                is_negated: has_negated_surface_fallback_claim(line, start, end),
            }
        })
        .collect()
}

fn has_negated_surface_fallback_claim(line: &str, start: usize, end: usize) -> bool {
    let prefix_start = line[..start]
        .rfind([';', '.'])
        .map_or(0, |offset| offset + 1);
    let prefix = &line[prefix_start..start];
    let text = &line[start..end];
    [
        "did not use",
        "didn't use",
        "do not use",
        "not use",
        "without using",
    ]
    .into_iter()
    .any(|marker| prefix.contains(marker))
        || [
            "not used",
            "not fallback substitute",
            "not fallback substitutes",
            "not a fallback substitute",
            "not fallback or substitute",
            "no fallback",
            "no cli fallback",
            "without fallback",
        ]
        .into_iter()
        .any(|marker| text.contains(marker))
}

fn has_negated_marker_prefix(line: &str, start: usize) -> bool {
    let prefix_start = line[..start]
        .rfind([';', '.'])
        .map_or(0, |offset| offset + 1);
    let prefix = line[prefix_start..start].trim_end();
    ["not", "no", "never"]
        .into_iter()
        .any(|marker| prefix.ends_with(marker))
}

fn has_negated_thread_event_claim(line: &str) -> bool {
    [
        "no thread/start",
        "not thread/start",
        "without thread/start",
        "thread/start not",
        "thread/start or turn/start events were not",
        "thread/start and turn/start events were not",
        "thread/start or turn/start events were absent",
        "thread/start and turn/start events were absent",
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

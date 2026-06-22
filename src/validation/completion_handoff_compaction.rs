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
    if !has_git_graph_log_preflight(&text) {
        errors.push("compacted continuation evidence missing git graph/log preflight: include pwd, status, head/base refs, and recent graph before editing".into());
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

fn has_git_graph_log_preflight(text: &str) -> bool {
    has_any(
        text,
        &["git log --graph", "git graph/log", "graph/log preflight"],
    ) && has_any(
        text,
        &["git status", "git rev-parse", "head/base", "head and base"],
    )
}

fn has_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}

mod duplicate_state_targets;
mod evidence_fields;
mod git_preflight;
mod git_preflight_lines;

use serde_json::Value;

pub(super) fn check(handoff: &str, pr_state: &Value) -> Vec<String> {
    let text = handoff.to_ascii_lowercase();
    if !claims_compacted_continuation_readiness(&text) {
        return Vec::new();
    }

    let mut errors = Vec::new();
    if !evidence_fields::has_codexy_orchestration_contract(&text) {
        errors.push("compacted continuation evidence missing Codexy orchestration contract: include active @Codexy or $codex-orchestration workflow instructions before continuing".into());
    }
    if !evidence_fields::has_duplicate_or_no_active_work_state(&text, pr_state) {
        errors.push("compacted continuation evidence missing duplicate/no-active-work state: re-check current issue and PR status before editing".into());
    }
    if !evidence_fields::has_parent_child_ownership_boundary(&text) {
        errors.push("compacted continuation evidence missing parent/child ownership boundary: preserve who may edit and who may only orchestrate".into());
    }
    if !evidence_fields::has_authoritative_stop_condition(&text) {
        errors.push("compacted continuation evidence missing authoritative stop condition: include the current stop condition before continuing".into());
    }
    if !git_preflight::has_git_graph_log_preflight(&text) {
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
            "compaction continuation",
            "compaction handoff",
            "compaction resume",
            "conversation compaction",
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
            "resuming",
            "continue",
            "continuing",
            "next action",
            "before editing",
        ],
    )
}

fn has_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}

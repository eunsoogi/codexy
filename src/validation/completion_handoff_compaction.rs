mod duplicate_state_targets;
mod evidence_fields;
mod git_preflight;
mod git_preflight_commands;
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
    if !git_preflight::has_git_graph_log_preflight(handoff) {
        errors.push("compacted continuation evidence missing git graph/log preflight: include pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph before editing".into());
    }
    errors
}

fn claims_compacted_continuation_readiness(text: &str) -> bool {
    let lines: Vec<_> = text.lines().map(str::trim).collect();
    lines.iter().enumerate().any(|(index, line)| {
        has_compaction_context(line)
            && (has_continuation_context(line)
                || has_pending_edit_plan(line)
                || (is_compaction_context_heading(line)
                    && following_lines(&lines, index)
                        .any(|line| has_continuation_context(line) || has_pending_edit_plan(line))))
    })
}

fn has_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}

fn has_compaction_context(line: &str) -> bool {
    has_any(
        line,
        &[
            "compacted continuation",
            "after compaction",
            "compaction continuation",
            "compaction handoff",
            "compaction resume",
            "compaction summary",
            "conversation compaction",
            "post-compaction",
            "post compaction",
            "context compaction",
            "goal continuation",
        ],
    )
}

fn has_continuation_context(line: &str) -> bool {
    has_any(
        line,
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

fn has_pending_edit_plan(line: &str) -> bool {
    has_any(
        line,
        &[
            "will edit",
            "will start editing",
            "going to edit",
            "start editing",
            "edit the pr now",
            "edit the pr branch",
        ],
    )
}

fn is_compaction_context_heading(line: &str) -> bool {
    let line = handoff_line_metadata(line);
    [
        "compacted continuation",
        "compaction continuation",
        "compaction handoff",
        "compaction resume",
        "compaction summary",
        "conversation compaction",
        "post-compaction",
        "post compaction",
        "context compaction",
        "goal continuation",
    ]
    .iter()
    .any(|phrase| {
        line.starts_with(phrase)
            && line[phrase.len()..]
                .trim_start()
                .chars()
                .next()
                .is_none_or(|character| matches!(character, ':' | '-'))
    })
}

fn handoff_line_metadata(line: &str) -> &str {
    let line = line.trim().trim_start_matches(['-', '*']).trim_start();
    let line = line
        .strip_prefix("[x]")
        .or_else(|| line.strip_prefix("[X]"))
        .or_else(|| line.strip_prefix("[ ]"))
        .unwrap_or(line)
        .trim_start();
    line.trim_start_matches('#').trim_start()
}

fn following_lines<'a>(lines: &'a [&str], index: usize) -> impl Iterator<Item = &'a str> {
    lines
        .iter()
        .skip(index + 1)
        .filter(|line| !line.is_empty())
        .copied()
}

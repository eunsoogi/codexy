mod duplicate_state_targets;
mod evidence_fields;
mod git_preflight;
mod git_preflight_commands;
mod git_preflight_lines;
mod review_request_context;

use serde_json::Value;

use super::codex_review_handoff_events::{
    has_codex_review_output, has_latest_eyes_request_without_later_codex_output,
};
use review_request_context::{has_codex_review_request_context, has_review_request_context};

const COMPACTION_CONTEXT_PHRASES: &[&str] = &[
    "compacted continuation",
    "after compaction",
    "after-compaction",
    "compaction continuation",
    "compaction handoff",
    "compaction resume",
    "compaction summary",
    "from compaction",
    "summary after compaction",
    "conversation compaction",
    "post-compaction",
    "post compaction",
    "context compaction",
    "goal continuation",
];

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
    if has_codex_review_request_context(&text) {
        if !has_pr_comments_and_reviews_evidence(pr_state) {
            errors.push("duplicate current-head Codex review request evidence missing: include freshly captured PR comments and reviews before planning @codex review".into());
        } else if has_codex_review_output(pr_state)
            || has_latest_eyes_request_without_later_codex_output(pr_state)
        {
            errors.push("duplicate current-head Codex review request blocked: re-read latest PR comments/reviews immediately before posting and do not post @codex review when a request or current-head output already exists".into());
        }
    }
    errors
}

fn has_pr_comments_and_reviews_evidence(pr_state: &Value) -> bool {
    has_array_field(pr_state, "comments")
        && (has_array_field(pr_state, "reviews") || has_array_field(pr_state, "latestReviews"))
}

fn has_array_field(value: &Value, field: &str) -> bool {
    value.get(field).is_some_and(Value::is_array)
}

fn claims_compacted_continuation_readiness(text: &str) -> bool {
    let lines: Vec<_> = text.lines().map(str::trim).collect();
    lines.iter().enumerate().any(|(index, line)| {
        has_compaction_context(line)
            && (has_continuation_context(line)
                || has_pending_edit_plan(line)
                || has_review_request_context(line)
                || (is_compaction_context_heading(line)
                    && following_lines(&lines, index).any(|line| {
                        has_continuation_context(line)
                            || has_pending_edit_plan(line)
                            || has_review_request_context(line)
                    })))
    })
}

fn has_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}

fn has_compaction_context(line: &str) -> bool {
    has_any(line, COMPACTION_CONTEXT_PHRASES)
}

fn has_continuation_context(line: &str) -> bool {
    !has_negated_continuation_or_edit_context(line)
        && (has_any(
            line,
            &[
                "ready to continue",
                "continuation readiness",
                "resume",
                "resuming",
                "continue",
                "continuing",
                "before editing",
            ],
        ) || has_next_action_continuation_context(line))
}

fn has_next_action_continuation_context(line: &str) -> bool {
    let Some((_, action)) = line.split_once("next action") else {
        return false;
    };
    let action = action.trim_start_matches(|c: char| c == ':' || c == '-' || c.is_whitespace());
    has_any(
        action,
        &["continue", "continuing", "resume", "resuming", "edit"],
    ) || has_review_request_context(action)
}

fn has_pending_edit_plan(line: &str) -> bool {
    !has_negated_continuation_or_edit_context(line)
        && has_any(
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

fn has_negated_continuation_or_edit_context(line: &str) -> bool {
    has_any(
        line,
        &[
            "cannot continue",
            "do not continue",
            "don't continue",
            "not continue",
            "not continuing",
            "will not continue",
            "won't continue",
            "cannot edit",
            "do not edit",
            "don't edit",
            "not edit",
            "not editing",
            "will not edit",
            "won't edit",
        ],
    )
}

fn is_compaction_context_heading(line: &str) -> bool {
    let line = handoff_line_metadata(line);
    COMPACTION_CONTEXT_PHRASES.iter().any(|phrase| {
        line.starts_with(phrase) && starts_heading_suffix_or_boundary(&line[phrase.len()..])
    })
}

fn starts_heading_suffix_or_boundary(remainder: &str) -> bool {
    let remainder = remainder.trim_start();
    if starts_heading_boundary(remainder) {
        return true;
    }
    ["summary", "readiness"].iter().any(|suffix| {
        remainder.starts_with(suffix) && starts_heading_boundary(&remainder[suffix.len()..])
    })
}

fn starts_heading_boundary(remainder: &str) -> bool {
    remainder
        .trim_start()
        .chars()
        .next()
        .is_none_or(|character| matches!(character, ':' | '-'))
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

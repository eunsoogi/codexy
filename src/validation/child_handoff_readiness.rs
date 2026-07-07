use serde_json::Value;

use super::child_handoff_readiness_claims as claims;
use super::child_handoff_readiness_status::{
    branch_status_not_pr_branch, branch_status_not_pushed, dirty_status, pr_branch_statuses,
    status_fields,
};
use super::child_handoff_readiness_text::has_non_claim_phrase_label;

pub(super) fn check(handoff: &str, pr_state: &Value) -> Vec<String> {
    let text = handoff.to_ascii_lowercase();
    let claims_pr_ready = claims::pr_ready(&text);
    let claims_child_readiness = claims::child_readiness(&text);
    let claims_clean = claims::clean(&text);
    let claims_synced = claims::synced(&text);
    let claims_pushed = claims::pushed(&text);
    let claims_bare_pr_ready = claims_pr_ready && has_standalone_ready_line(&text);
    let claims_current_pr_readiness = claims_bare_pr_ready
        || (claims_child_readiness && (claims_pr_ready || claims_synced || claims_pushed));
    let mut errors = Vec::new();
    errors.extend(
        negative_proof_labels(
            &text,
            claims_pr_ready,
            claims_clean,
            claims_synced,
            claims_pushed,
        )
        .map(|label| {
            format!("child handoff claims readiness but {label} proof is negative or non-claim")
        }),
    );
    if !claims_child_readiness && !claims_current_pr_readiness {
        return errors;
    }
    if claims_clean || claims_current_pr_readiness {
        let lines: Vec<_> = status_fields(pr_state).collect();
        if lines.is_empty() {
            errors.push(
                "child handoff claims clean/PR-ready worktree but current local git status evidence is missing".into(),
            );
        } else if let Some(status) = dirty_status(&lines) {
            errors.push(format!(
                "child handoff claims clean/PR-ready worktree but current status is dirty: {status}"
            ));
        } else if claims_current_pr_readiness {
            if let Some(status) = branch_status_not_pr_branch(&lines, pr_state) {
                errors.push(format!(
                    "child handoff claims PR readiness but current branch status does not match PR branch: {status}"
                ));
            } else if let Some(status) = branch_status_not_pushed(&lines) {
                errors.push(format!(
                    "child handoff claims PR readiness but current branch status is not pushed: {status}"
                ));
            }
        }
    }
    if claims_synced || claims_pushed {
        let statuses = pr_branch_statuses(pr_state);
        if statuses.is_empty() {
            errors.push(
                "child handoff claims pushed/synced branch but matching current branch status evidence is missing"
                    .into(),
            );
        } else if let Some(status) = branch_status_not_pushed(&statuses) {
            errors.push(format!(
                "child handoff claims pushed/synced branch but current branch status is not pushed: {status}"
            ));
        }
        if let Some(error) = pushed_head_mismatch(handoff, pr_state) {
            errors.push(error);
        }
    }
    if claims_current_pr_readiness {
        if let Some(error) = super::child_handoff_readiness_heads::captured_head_mismatch(pr_state)
        {
            errors.push(error);
        }
        if let Some(state) = string_field(pr_state, "mergeStateStatus") {
            if !state.eq_ignore_ascii_case("CLEAN") {
                errors.push(format!(
                    "child handoff claims PR readiness but mergeStateStatus is {state}"
                ));
            }
        }
        if pr_state.get("isDraft").and_then(Value::as_bool) == Some(true) {
            errors.push("child handoff claims PR readiness but PR is still draft".into());
        }
        let Some(threads) = pr_state.get("reviewThreads") else {
            errors.push(
                "child handoff claims PR readiness but reviewThreads.nodes PR state evidence is missing"
                    .into(),
            );
            return errors;
        };
        if threads.get("nodes").and_then(Value::as_array).is_none() {
            errors.push(
                "child handoff claims PR readiness but reviewThreads.nodes PR state evidence is missing"
                    .into(),
            );
            return errors;
        }
        if let Some(error) = super::review_thread_evidence::check(threads) {
            errors.push(error);
        } else if let Some(thread) = super::review_thread_readiness::check(pr_state) {
            errors.push(format!("child handoff claims PR readiness but {thread}"));
        }
    }
    errors
}

fn negative_proof_labels(
    text: &str,
    claims_pr_ready: bool,
    claims_clean: bool,
    claims_synced: bool,
    claims_pushed: bool,
) -> impl Iterator<Item = &'static str> + '_ {
    [
        (
            claims_pr_ready || claims_clean,
            "clean",
            &["clean", "branch clean", "worktree clean"][..],
        ),
        (claims_pr_ready || claims_synced, "synced", &["synced"][..]),
        (
            claims_pr_ready || claims_synced || claims_pushed,
            "pushed",
            &["pushed", "pushed branch", "remote/pr head match"][..],
        ),
        (
            claims_pr_ready,
            "PR-ready",
            &[
                "pr ready",
                "pr-ready",
                "pr is ready",
                "pull-request-ready",
                "pull request ready",
                "pull request is ready",
                "pr readiness",
                "pr-readiness",
                "ready for parent handoff",
                "ready for handoff",
                "parent-handoff-ready",
                "parent handoff ready",
                "parent can open pr next",
            ][..],
        ),
        (
            claims_pr_ready,
            "merge-ready",
            &[
                "merge-ready",
                "merge ready",
                "ready to merge",
                "ready for merge",
                "merge readiness",
                "merge-readiness",
                "parent can merge",
            ][..],
        ),
    ]
    .into_iter()
    .filter_map(|(required, label, phrases)| {
        (required
            && phrases
                .iter()
                .any(|phrase| has_non_claim_phrase_label(text, phrase)))
        .then_some(label)
    })
}

fn pushed_head_mismatch(handoff: &str, pr_state: &Value) -> Option<String> {
    let Some(pr_head) = string_field(pr_state, "headRefOid").filter(|head| !head.is_empty()) else {
        return Some(
            "child handoff claims pushed/synced head but PR state is missing headRefOid".into(),
        );
    };
    let claimed = super::child_handoff_readiness_heads::claimed_pushed_heads(handoff);
    if claimed.is_empty() {
        return Some(format!(
            "child handoff claims pushed/synced head but PR headRefOid is {pr_head}, not any comparable handoff head"
        ));
    }
    if claimed
        .iter()
        .all(|oid| pr_head.to_ascii_lowercase().starts_with(oid))
    {
        return None;
    }
    let mismatched = claimed
        .iter()
        .find(|oid| !pr_head.to_ascii_lowercase().starts_with(*oid));
    Some(format!(
        "child handoff claims pushed/synced head but PR headRefOid is {pr_head}, not {}",
        mismatched.map_or("any comparable handoff head", String::as_str)
    ))
}

fn has_standalone_ready_line(text: &str) -> bool {
    claims::standalone_ready_line(text)
        || text.lines().any(|line| {
            let line = line.trim().trim_start_matches(['-', '*']).trim();
            let line = line
                .strip_prefix("[x]")
                .or_else(|| line.strip_prefix("[X]"))
                .map(str::trim_start)
                .unwrap_or(line)
                .trim_end_matches('.');
            has_affirmative_ready_label(line)
        })
}

fn has_affirmative_ready_label(line: &str) -> bool {
    claims::ready_label_phrases().iter().any(|label| {
        line.strip_prefix(label)
            .and_then(|rest| rest.trim_start().strip_prefix(':'))
            .is_some_and(has_affirmative_yes_value)
    })
}

fn has_affirmative_yes_value(value: &str) -> bool {
    let value = value.trim();
    value == "yes"
        || value.strip_prefix("yes").is_some_and(|rest| {
            rest.as_bytes()
                .first()
                .is_some_and(|ch| !ch.is_ascii_alphanumeric())
        })
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

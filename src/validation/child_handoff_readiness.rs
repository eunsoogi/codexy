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
    if !claims_child_readiness {
        return errors;
    }
    if claims_clean || claims_pr_ready {
        let lines: Vec<_> = status_fields(pr_state).collect();
        if lines.is_empty() {
            errors.push(
                "child handoff claims clean/PR-ready worktree but current local git status evidence is missing".into(),
            );
        } else if let Some(status) = dirty_status(&lines) {
            errors.push(format!(
                "child handoff claims clean/PR-ready worktree but current status is dirty: {status}"
            ));
        } else if claims_pr_ready {
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
    if claims_pr_ready {
        if let Some(error) = captured_head_mismatch(pr_state) {
            errors.push(error);
        }
        if let Some(state) = string_field(pr_state, "mergeStateStatus") {
            if !state.eq_ignore_ascii_case("CLEAN") {
                errors.push(format!(
                    "child handoff claims PR readiness but mergeStateStatus is {state}"
                ));
            }
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
        } else if let Some(thread) = unresolved_thread(pr_state) {
            errors.push(format!(
                "child handoff claims PR readiness but unresolved review thread remains: {thread}"
            ));
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
            claims_pr_ready || claims_pushed,
            "pushed",
            &["pushed", "remote/pr head match"][..],
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

fn captured_head_mismatch(pr_state: &Value) -> Option<String> {
    let pr_head = string_field(pr_state, "headRefOid")?;
    [
        ("localHeadOid", "current local HEAD"),
        ("localHead", "current local HEAD"),
        ("gitHeadOid", "current local HEAD"),
        ("remoteHeadOid", "remote-tracking HEAD"),
        ("remoteHead", "remote-tracking HEAD"),
    ]
    .into_iter()
    .filter_map(|(field, label)| {
        string_field(pr_state, field)
            .filter(|head| !head.is_empty())
            .map(|head| (label, head))
    })
    .find_map(|(label, head)| {
        (!heads_match(pr_head, head)).then(|| {
            format!(
                "child handoff claims PR readiness but {label} is {head}, not PR headRefOid {pr_head}"
            )
        })
    })
}

fn heads_match(pr_head: &str, captured_head: &str) -> bool {
    let pr_head = pr_head.to_ascii_lowercase();
    let captured_head = captured_head.to_ascii_lowercase();
    pr_head.starts_with(&captured_head) || captured_head.starts_with(&pr_head)
}

fn unresolved_thread(pr_state: &Value) -> Option<String> {
    let nodes = pr_state.get("reviewThreads")?.get("nodes")?.as_array()?;
    nodes.iter().find_map(|thread| {
        (thread.get("isResolved").and_then(Value::as_bool) == Some(false)).then(|| {
            format!(
                "{} at {}",
                string_field(thread, "id").unwrap_or("unknown thread"),
                string_field(thread, "path").unwrap_or("unknown path")
            )
        })
    })
}
fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

use serde_json::Value;

use super::child_handoff_readiness_status::{branch_diverged, dirty_status, pr_branch_statuses};
use super::child_handoff_readiness_text::has_non_claim_phrase_label;

pub(super) fn check(handoff: &str, pr_state: &Value) -> Vec<String> {
    let text = handoff.to_ascii_lowercase();
    let claims_pr_ready = claims_pr_ready(&text);
    let mut errors = Vec::new();
    if claims_pr_ready {
        errors.extend(negative_proof_labels(&text).map(|label| {
            format!("child handoff claims readiness but {label} proof is negative or non-claim")
        }));
    }
    if !claims_child_readiness(&text) {
        return errors;
    }
    if claims_clean(&text) {
        if let Some(status) = dirty_status(pr_state) {
            errors.push(format!(
                "child handoff claims clean worktree but current status is dirty: {status}"
            ));
        }
    }
    if claims_synced_or_pushed(&text) {
        let statuses = pr_branch_statuses(pr_state);
        if statuses.is_empty() {
            errors.push(
                "child handoff claims pushed/synced branch but matching current branch status evidence is missing"
                    .into(),
            );
        } else {
            if let Some(status) = statuses.iter().find(|status| branch_diverged(status)) {
                errors.push(format!(
                    "child handoff claims pushed/synced branch but current branch status is not pushed: {status}"
                ));
            }
        }
        if let Some(error) = pushed_head_mismatch(handoff, pr_state) {
            errors.push(error);
        }
    }
    if claims_pr_ready {
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

fn claims_child_readiness(text: &str) -> bool {
    [
        "child handoff",
        "parent handoff",
        "pr ready for parent handoff",
        "parent can open pr next: yes",
        "parent can merge",
        "remote/pr head match: yes",
        "pushed: yes",
        "branch clean",
        "clean, synced",
        "synced, and pushed",
    ]
    .iter()
    .any(|phrase| super::child_handoff_readiness_text::has_affirmed_phrase(text, phrase))
}

fn claims_clean(text: &str) -> bool {
    [
        "branch clean",
        "worktree clean",
        "dirty state: clean",
        " clean,",
    ]
    .iter()
    .any(|phrase| super::child_handoff_readiness_text::has_affirmed_phrase(text, phrase))
}

fn claims_synced_or_pushed(text: &str) -> bool {
    ["synced", "pushed", "remote/pr head match: yes"]
        .iter()
        .any(|phrase| super::child_handoff_readiness_text::has_affirmed_phrase(text, phrase))
}

fn claims_pr_ready(text: &str) -> bool {
    [
        "pr ready",
        "pr-ready",
        "ready for parent handoff",
        "ready to merge",
        "merge-ready",
        "merge ready",
        "parent can open pr next: yes",
        "parent can merge",
    ]
    .iter()
    .any(|phrase| super::child_handoff_readiness_text::has_affirmed_phrase(text, phrase))
}

fn negative_proof_labels(text: &str) -> impl Iterator<Item = &'static str> + '_ {
    [
        ("clean", &["clean", "branch clean", "worktree clean"][..]),
        ("synced", &["synced"][..]),
        ("pushed", &["pushed", "remote/pr head match"][..]),
        (
            "PR-ready",
            &[
                "pr ready",
                "pr-ready",
                "ready for parent handoff",
                "parent can open pr next",
            ][..],
        ),
        (
            "merge-ready",
            &[
                "merge-ready",
                "merge ready",
                "ready to merge",
                "parent can merge",
            ][..],
        ),
    ]
    .into_iter()
    .filter_map(|(label, phrases)| {
        phrases
            .iter()
            .any(|phrase| has_non_claim_phrase_label(text, phrase))
            .then_some(label)
    })
}

fn pushed_head_mismatch(handoff: &str, pr_state: &Value) -> Option<String> {
    let Some(pr_head) = string_field(pr_state, "headRefOid").filter(|head| !head.is_empty()) else {
        return Some(
            "child handoff claims pushed/synced head but PR state is missing headRefOid".into(),
        );
    };
    let claimed = hex_refs(handoff);
    if claimed
        .iter()
        .any(|oid| pr_head.to_ascii_lowercase().starts_with(oid))
    {
        return None;
    }
    Some(format!(
        "child handoff claims pushed/synced head but PR headRefOid is {pr_head}, not {}",
        claimed
            .first()
            .map_or("any comparable handoff head", String::as_str)
    ))
}

fn unresolved_thread(pr_state: &Value) -> Option<String> {
    let nodes = pr_state.get("reviewThreads")?.get("nodes")?.as_array()?;
    nodes.iter().find_map(|thread| {
        (thread.get("isResolved").and_then(Value::as_bool) == Some(false))
            .then(|| thread_label(thread))
    })
}

fn thread_label(thread: &Value) -> String {
    let id = string_field(thread, "id").unwrap_or("unknown thread");
    let path = string_field(thread, "path").unwrap_or("unknown path");
    format!("{id} at {path}")
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn hex_refs(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_ascii_hexdigit())
        .filter(|part| (7..=40).contains(&part.len()))
        .map(str::to_ascii_lowercase)
        .collect()
}

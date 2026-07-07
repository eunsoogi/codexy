use serde_json::Value;

use super::codex_review_handoff_actionable::has_actionable_codex_review_output;
use super::codex_review_handoff_events::{
    has_codex_review_activity, has_codex_review_output,
    has_latest_eyes_request_without_later_codex_output, has_unresolved_codex_review_thread,
};

pub(super) fn check(handoff: &str, pr_state: &Value) -> Vec<String> {
    let claims_ready = super::codex_review_handoff_readiness::claims_ready(handoff);
    let claims_completion = super::codex_review_handoff_readiness::claims_completion(handoff);
    let has_override = super::codex_review_handoff_readiness::states_override(handoff);
    if let Some(error) = super::codex_review_fresh_request::check(handoff, pr_state) {
        return vec![error];
    }
    if claims_ready
        && !has_head_ref_oid(pr_state)
        && (claims_completion || has_codex_review_activity(pr_state))
    {
        return vec![
            "completion handoff PR state missing required field: headRefOid before Codex review readiness claims"
                .into(),
        ];
    }
    if claims_ready && has_unresolved_codex_review_thread(pr_state) {
        return vec![format!(
            "unresolved Codex review thread blocks merge/readiness claims: PR #{}",
            pr_number(pr_state)
        )];
    }
    if claims_ready
        && has_actionable_codex_review_output(pr_state)
        && !super::review_thread_resolution::claims_review_response(handoff)
    {
        return vec![format!(
            "actionable Codex review output blocks merge/readiness claims until the handoff documents addressed feedback or an accepted no-change rationale: PR #{}",
            pr_number(pr_state)
        )];
    }
    if claims_ready && has_latest_eyes_request_without_later_codex_output(pr_state) && !has_override
    {
        return vec![format!(
            "eyes-only Codex review request or unacknowledged Codex review request is not review completion: PR #{} needs actual Codex review output, an explicit completion signal, or a maintainer override before merge/readiness claims",
            pr_number(pr_state)
        )];
    }
    if claims_ready && !has_codex_review_output(pr_state) && !has_override {
        return vec![format!(
            "current-head Codex review output is required before Codex review completion claims: PR #{} needs matching Codex review output or a maintainer override before merge/readiness claims",
            pr_number(pr_state)
        )];
    }
    if claims_ready && (has_codex_review_output(pr_state) || has_override) {
        if let Some(error) = review_thread_evidence_error(pr_state) {
            return vec![format!(
                "{error} before merge/readiness claims: PR #{}",
                pr_number(pr_state)
            )];
        }
        if let Some(error) = super::review_thread_readiness::check(pr_state) {
            return vec![format!("{error}: PR #{}", pr_number(pr_state))];
        }
    }
    Vec::new()
}
fn has_head_ref_oid(pr_state: &Value) -> bool {
    pr_state
        .get("headRefOid")
        .and_then(Value::as_str)
        .is_some_and(|head| !head.trim().is_empty())
}
fn review_thread_evidence_error(pr_state: &Value) -> Option<String> {
    let Some(threads) = pr_state.get("reviewThreads") else {
        return Some(
            "incomplete reviewThreads.nodes PR state evidence: missing reviewThreads".into(),
        );
    };
    if threads.get("nodes").and_then(Value::as_array).is_none() {
        return Some("incomplete reviewThreads.nodes PR state evidence: missing nodes".into());
    }
    super::review_thread_evidence::check(threads)
}
fn pr_number(pr_state: &Value) -> String {
    pr_state
        .get("number")
        .and_then(Value::as_u64)
        .map_or_else(|| "<unknown>".to_owned(), |number| number.to_string())
}

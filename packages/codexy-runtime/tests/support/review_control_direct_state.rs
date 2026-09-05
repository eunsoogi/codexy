use serde_json::{Value, json};

pub(crate) const SYNTHETIC_BASE: &str = "synthetic-base";
pub(crate) const SYNTHETIC_UPDATED_BASE: &str = "synthetic-updated-base";
pub(crate) const SYNTHETIC_FULL_HEAD: &str = "synthetic-full-head";
pub(crate) const SYNTHETIC_DELTA_HEAD: &str = "synthetic-delta-head";
pub(crate) const SYNTHETIC_CURRENT_HEAD: &str = "synthetic-current-head";
pub(crate) const SYNTHETIC_INTEGRATION_EVIDENCE: &str = "synthetic-integration-evidence";
pub(crate) const SYNTHETIC_REPAIR_EVIDENCE: &str = "synthetic-repair-evidence";

pub(crate) fn strict_control(issue_number: u64, head: &str) -> Value {
    json!({
        "schema": "codexy.review-control-state.v1",
        "issue_number": issue_number,
        "profile": "strict",
        "reviewer": {
            "name": "codexy-sentinel",
            "model": "gpt-6-astra",
            "reasoning_effort": "xhigh"
        },
        "reviewed_head": head,
        "terminal_result": "PASS",
        "unresolved_findings": [],
        "full_review_count": 1,
        "delta_review_count": 0,
        "terminal_review_count": 1,
        "terminal_review_limit": 3,
        "terminal_review_history": [{
            "id": "strict-full-1",
            "kind": "full",
            "reviewer": {
                "name": "codexy-sentinel",
                "model": "gpt-6-astra",
                "reasoning_effort": "xhigh"
            },
            "reviewed_head": head,
            "terminal_result": "PASS",
            "unresolved_findings": []
        }]
    })
}

pub(crate) fn strict_genesis(issue_number: u64) -> Value {
    json!({
        "schema": "codexy.review-control-state.v1",
        "issue_number": issue_number,
        "profile": "strict",
        "reviewer": {
            "name": "codexy-sentinel",
            "model": "gpt-6-astra",
            "reasoning_effort": "xhigh"
        },
        "unresolved_findings": [],
        "full_review_count": 0,
        "delta_review_count": 0,
        "terminal_review_count": 0,
        "terminal_review_limit": 3,
        "terminal_review_history": []
    })
}

pub(crate) fn reviewer() -> Value {
    json!({
        "name": "codexy-sentinel",
        "model": "gpt-6-astra",
        "reasoning_effort": "xhigh"
    })
}

pub(crate) fn pr_snapshot(
    pr_number: u64,
    base_oid: &str,
    head_oid: &str,
    control: Option<Value>,
) -> Value {
    let mut snapshot = json!({
        "repository": "eunsoogi/codexy",
        "number": pr_number,
        "baseRefName": "main",
        "baseRefOid": base_oid,
        "headRefOid": head_oid,
        "url": format!("https://github.com/eunsoogi/codexy/pull/{pr_number}"),
        "capture": {
            "provider": "github",
            "method": "graphql",
            "authenticated": true
        }
    });
    if let Some(control) = control {
        snapshot["reviewControl"] = control;
    }
    snapshot
}

pub(crate) fn review_event(id: &str, kind: &str, head: &str, result: &str) -> Value {
    json!({
        "id": id,
        "kind": kind,
        "reviewer": reviewer(),
        "reviewed_head": head,
        "terminal_result": result,
        "unresolved_findings": []
    })
}

pub(crate) fn post_cap_control(
    issue_number: u64,
    full_head: &str,
    delta_head: &str,
    current_head: &str,
) -> Value {
    post_cap_control_with_evidence(
        issue_number,
        full_head,
        delta_head,
        current_head,
        "mandatory_base_integration",
        SYNTHETIC_INTEGRATION_EVIDENCE,
    )
}

pub(crate) fn post_cap_control_with_evidence(
    issue_number: u64,
    full_head: &str,
    delta_head: &str,
    current_head: &str,
    reason: &str,
    evidence_commit: &str,
) -> Value {
    if reason == "in_scope_contract_root_repair" {
        return post_cap_control_with_findings(
            issue_number,
            full_head,
            delta_head,
            current_head,
            reason,
            evidence_commit,
            "BLOCK",
            json!([{"id": "goal-objective-delimiter", "path": "packages/codexy-runtime/src/validation/child_goal_reporting/receipt/parse.rs"}]),
            json!(["goal-objective-delimiter"]),
        );
    }
    post_cap_control_with_findings(
        issue_number,
        full_head,
        delta_head,
        current_head,
        reason,
        evidence_commit,
        "PASS",
        json!([]),
        json!([]),
    )
}

pub(crate) fn post_cap_control_with_findings(
    issue_number: u64,
    full_head: &str,
    delta_head: &str,
    current_head: &str,
    reason: &str,
    evidence_commit: &str,
    delta_result: &str,
    delta_findings: Value,
    finding_ids: Value,
) -> Value {
    json!({
        "schema": "codexy.review-control-state.v1",
        "issue_number": issue_number,
        "profile": "strict",
        "reviewer": reviewer(),
        "reviewed_head": current_head,
        "terminal_result": "PASS",
        "unresolved_findings": [],
        "full_review_count": 1,
        "delta_review_count": 1,
        "terminal_review_count": 3,
        "terminal_review_limit": 3,
        "terminal_review_history": [
            review_event("strict-full-1", "full", full_head, "PASS"),
            {
                "id": "strict-delta-1",
                "kind": "delta",
                "reviewer": reviewer(),
                "reviewed_head": delta_head,
                "terminal_result": delta_result,
                "unresolved_findings": delta_findings
            },
            review_event("strict-required-head-1", "required_current_head", current_head, "PASS")
        ],
        "post_cap_re_review": {
            "reason": reason,
            "prior_reviewed_head": delta_head,
            "qualifying_change": {
                "from_head": delta_head,
                "to_head": current_head,
                "evidence_commit": evidence_commit,
                "finding_ids": finding_ids
            }
        }
    })
}

pub(crate) fn post_cap_prior(control: &Value) -> Value {
    let mut prior = control.clone();
    prior["reviewed_head"] = prior["terminal_review_history"][1]["reviewed_head"].clone();
    prior["terminal_result"] = prior["terminal_review_history"][1]["terminal_result"].clone();
    prior["unresolved_findings"] = prior["terminal_review_history"][1]["unresolved_findings"].clone();
    prior["terminal_review_count"] = json!(2);
    prior["terminal_review_history"]
        .as_array_mut()
        .expect("post-cap history array")
        .truncate(2);
    prior
        .as_object_mut()
        .expect("post-cap control object")
        .remove("post_cap_re_review");
    prior
}

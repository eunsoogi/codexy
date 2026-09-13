#![allow(dead_code, unused_imports)]

use serde_json::{Value, json};

#[path = "review_control_direct_state/connector.rs"]
mod connector;

pub(crate) use connector::{connector_pr_snapshot, connector_pr_snapshot_without_derived};

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
        "unresolved_findings": []
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
        "unresolved_findings": []
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
            "authenticated": true,
            "owningIssue": {
                "repository": "eunsoogi/codexy",
                "number": pr_number,
                "url": format!("https://github.com/eunsoogi/codexy/issues/{pr_number}"),
                "association": "owner-assignment"
            }
        }
    });
    if let Some(control) = control {
        snapshot["reviewControl"] = control;
    }
    snapshot
}

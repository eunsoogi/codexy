use serde_json::{Value, json};

use super::pr_snapshot;

pub(crate) fn connector_pr_snapshot(
    pr_number: u64,
    issue_number: u64,
    base_oid: &str,
    head_oid: &str,
    control: Option<Value>,
) -> Value {
    let mut snapshot = pr_snapshot(pr_number, base_oid, head_oid, control);
    snapshot["capture"] = connector_capture(pr_number, issue_number, base_oid, head_oid);
    snapshot
}

pub(crate) fn connector_pr_snapshot_without_derived(
    pr_number: u64,
    issue_number: u64,
    base_oid: &str,
    head_oid: &str,
    control: Option<Value>,
) -> Value {
    let mut snapshot = json!({
        "capture": connector_capture(pr_number, issue_number, base_oid, head_oid)
    });
    if let Some(control) = control {
        snapshot["reviewControl"] = control;
    }
    snapshot
}

fn connector_capture(pr_number: u64, issue_number: u64, base_oid: &str, head_oid: &str) -> Value {
    json!({
        "provider": "github",
        "method": "connector",
        "authenticated": true,
        "source": {
            "tool": "mcp__codex_apps__github_get_pr_info",
            "arguments": {
                "repository_full_name": "eunsoogi/codexy",
                "pr_number": pr_number
            },
            "result": {
                "url": format!("https://github.com/eunsoogi/codexy/pull/{pr_number}"),
                "number": pr_number,
                "base": "main",
                "base_sha": base_oid,
                "head_sha": head_oid,
                "title": "connector fixture"
            }
        },
        "owningIssue": {
            "repository": "eunsoogi/codexy",
            "number": issue_number,
            "url": format!("https://github.com/eunsoogi/codexy/issues/{issue_number}"),
            "association": "owner-assignment"
        }
    })
}

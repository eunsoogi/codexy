use serde_json::{Value, json};

pub(crate) fn ci_response(pull_request: u64, base: &str, head: &str) -> Value {
    json!({
        "number": pull_request,
        "baseRefName": "main",
        "baseRefOid": base,
        "headRefName": "feature",
        "headRefOid": head,
        "statusCheckRollup": [{
            "__typename": "CheckRun",
            "completedAt": "2026-09-06T00:00:00Z",
            "conclusion": "SUCCESS",
            "detailsUrl": "https://github.com/eunsoogi/codexy/actions/runs/1",
            "name": "Rust",
            "startedAt": "2026-09-06T00:00:00Z",
            "status": "COMPLETED",
            "workflowName": "Rust tests"
        }]
    })
}

pub(crate) fn maintainer_response(
    pull_request: u64,
    issue: u64,
    base: &str,
    head: &str,
) -> Value {
    let repository = "eunsoogi/codexy";
    let pull_url = format!("https://github.com/{repository}/pull/{pull_request}");
    let issue_url = format!("https://github.com/{repository}/issues/{issue}");
    let comment_id = 5_554_573_060u64;
    let comment_url = format!("{pull_url}#issuecomment-{comment_id}");
    let body = format!(
        "## Maintainer disposition recorded by the release orchestrator\n\nThis records the maintainer's existing instruction in the release conversation: differences between the actually used specialist models and the planned 1.7.0 specialist routing are accepted for this milestone. The orchestrator is recording that instruction, not obtaining or inventing a new approval.\n\nScope of this disposition:\n- Repository: {repository}\n- Owning issue: #{issue}\n- Pull request: #{pull_request}\n- Base: {base}\n- Head: {head}\n- Finding: selected-reviewer-policy-mismatch\n- Finding path: plugins/codexy/agents/codexy-sentinel.toml\n- Accepted difference: the retained Sentinel's actual gpt-5.6-sol/xhigh execution may stand despite the planned newer model routing. Preserve the actual native reviewer identity, runtime model, verdicts and review count; do not relabel execution or repeat review solely for the model difference.\n\nThis disposition accepts only that model-policy difference for the bound review history. It does not accept code defects, waive CI or review findings, authorize merge, reset review counters, or authorize a fourth review. Future source validation must reread this comment and verify its identity, repository authority and exact scope."
    );
    json!({
        "data": {
            "repository": {
                "pullRequest": {
                    "number": pull_request,
                    "url": pull_url,
                    "baseRefOid": base,
                    "headRefOid": head,
                    "repository": {"nameWithOwner": repository},
                    "comments": {
                        "nodes": [{
                            "id": "IC_kwDOS6i-_88AAAABSxQPBA",
                            "databaseId": comment_id,
                            "url": comment_url,
                            "body": body,
                            "createdAt": "2026-09-05T20:28:23Z",
                            "updatedAt": "2026-09-05T20:28:23Z",
                            "author": {"login": "eunsoogi"},
                            "authorAssociation": "OWNER",
                            "isMinimized": false
                        }],
                        "pageInfo": {"hasNextPage": false}
                    }
                },
                "issue": {
                    "number": issue,
                    "url": issue_url,
                    "repository": {"nameWithOwner": repository}
                }
            }
        }
    })
}

use std::process::Command;

#[test]
fn readiness_guard_accepts_full_pr_state_with_lossy_review_thread_body()
-> Result<(), Box<dyn std::error::Error>> {
    let script = readiness_guard();
    let temp = tempfile::tempdir()?;
    let pr_state = temp.path().join("lossy-review-thread-body.json");
    let mut json = br#"{
        "number": 260,
        "state": "OPEN",
        "repository": "eunsoogi/codexy",
        "url": "https://github.com/eunsoogi/codexy/pull/260",
        "labels": [{"name": "type/fix"}, {"name": "area/workflow"}],
        "repositoryLabels": [{"name": "type/fix"}, {"name": "area/workflow"}],
        "closingIssuesReferences": [{"number": 260, "labels": [{"name": "type/fix"}]}],
        "reviewThreads": {
            "nodes": [
                {
                    "id": "PRRT_kwDO260",
                    "isResolved": false,
                    "isOutdated": false,
                    "path": "plugins/codexy/hooks/codexy-pr-label-check.sh",
                    "comments": {
                        "nodes": [
                            {
                                "author": {"login": "chatgpt-codex-connector[bot]"},
                                "body": "unrelated review text with lossy byte "#
        .to_vec();
    json.push(0x80);
    json.extend_from_slice(
        br#" and emoji marker",
                                "url": "https://github.com/eunsoogi/codexy/pull/260#discussion_r1",
                                "createdAt": "2026-07-05T00:00:00Z",
                                "commit": {"oid": "7b44fb58f93dc80a451f2f417db1d3db1233c43c"}
                            }
                        ]
                    }
                }
            ]
        }
    }"#,
    );
    std::fs::write(&pr_state, json)?;

    let jq_output = Command::new("jq")
        .arg("-e")
        .arg(".")
        .arg(&pr_state)
        .output()?;
    assert!(
        jq_output.status.success(),
        "fixture should match the issue evidence by passing jq validation: {}",
        output_text(&jq_output)
    );

    let output = Command::new(&script)
        .args([
            "--check-pr-labels",
            "--pr-state-file",
            pr_state.to_str().ok_or("pr state path")?,
        ])
        .output()?;
    assert!(
        output.status.success(),
        "guard should accept valid label evidence even when unrelated review-thread text contains lossy bytes: {}",
        output_text(&output)
    );

    Ok(())
}

fn output_text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn readiness_guard() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/codexy/hooks/codexy-readiness-guard.sh")
}

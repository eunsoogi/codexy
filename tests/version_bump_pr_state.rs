use serde_json::{json, Value};
use std::{fs, path::Path, process::Command};
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn production_pr_state_enriches_issue_labels_and_passes_handoff() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = tempdir()?;
    let paths = write_inputs(temp.path(), false, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")?;
    let output = temp.path().join("pr-state.json");
    let result = run_builder(root, &paths, &output, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")?;
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    let state: Value = serde_json::from_slice(&fs::read(&output)?)?;
    let issue: Value = serde_json::from_slice(&fs::read(&paths.issue)?)?;
    assert_eq!(state["repository"], "eunsoogi/codexy");
    assert_eq!(state["closingIssuesReferences"][0]["labels"], issue["labels"]);
    let handoff = temp.path().join("handoff.md");
    fs::write(
        &handoff,
        "Verification completed. PR #999 is open for CI and review; this automation does not claim completion.\n",
    )?;
    let validation = Command::new(root.join("scripts/validate-plugin-config"))
        .args(["--check-completion-handoff", "--handoff-file"])
        .arg(&handoff)
        .arg("--pr-state-file")
        .arg(&output)
        .output()?;
    assert!(validation.status.success(), "{}", String::from_utf8_lossy(&validation.stderr));
    Ok(())
}

#[test]
fn production_pr_state_rejects_cross_repository_or_mismatched_oid_without_mutation() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (cross_repository, actual_oid) in [
        (true, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        (false, "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
    ] {
        let temp = tempdir()?;
        let paths = write_inputs(temp.path(), cross_repository, actual_oid)?;
        let output = temp.path().join("pr-state.json");
        fs::write(&output, b"sentinel\n")?;
        let result = run_builder(
            root,
            &paths,
            &output,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )?;
        assert!(!result.status.success());
        assert_eq!(fs::read(&output)?, b"sentinel\n");
    }
    Ok(())
}

struct Inputs {
    pr: std::path::PathBuf,
    issue: std::path::PathBuf,
    labels: std::path::PathBuf,
    threads: std::path::PathBuf,
}

fn write_inputs(
    directory: &Path,
    cross_repository: bool,
    oid: &str,
) -> Result<Inputs, Box<dyn std::error::Error>> {
    let issue_labels = json!([
        {"name": "priority/medium"}, {"name": "status/ready"},
        {"name": "type/ci"}, {"name": "area/release"}
    ]);
    let pr_labels = json!([
        {"name": "priority/medium"}, {"name": "status/review"},
        {"name": "type/ci"}, {"name": "area/release"}
    ]);
    let repository_labels = json!([
        {"name": "priority/medium"}, {"name": "status/ready"}, {"name": "status/review"},
        {"name": "type/ci"}, {"name": "area/release"}, {"name": "area/qa"}
    ]);
    let values = [
        ("pr.json", json!({
            "number": 999, "state": "OPEN", "isDraft": false, "mergeStateStatus": "CLEAN",
            "reviewDecision": "", "headRefName": "codexy/version-1.3.1", "headRefOid": oid,
            "headRepository": {"name": "codexy"}, "headRepositoryOwner": {"login": "eunsoogi"},
            "isCrossRepository": cross_repository, "labels": pr_labels,
            "closingIssuesReferences": [{"number": 301}]
        })),
        ("issue.json", json!({"number": 301, "labels": issue_labels})),
        ("labels.json", repository_labels),
        ("threads.json", json!({"pageInfo": {"hasNextPage": false}, "nodes": []})),
    ];
    for (name, value) in values {
        fs::write(directory.join(name), serde_json::to_vec(&value)?)?;
    }
    Ok(Inputs {
        pr: directory.join("pr.json"),
        issue: directory.join("issue.json"),
        labels: directory.join("labels.json"),
        threads: directory.join("threads.json"),
    })
}

fn run_builder(root: &Path, paths: &Inputs, output: &Path, oid: &str) -> std::io::Result<std::process::Output> {
    Command::new(root.join("scripts/build-version-pr-state"))
        .arg("--pr-json").arg(&paths.pr)
        .arg("--issue-json").arg(&paths.issue)
        .arg("--repository-labels-json").arg(&paths.labels)
        .arg("--review-threads-json").arg(&paths.threads)
        .args(["--repository", "eunsoogi/codexy", "--expected-head-ref", "codexy/version-1.3.1"])
        .arg("--expected-head-oid").arg(oid)
        .arg("--output").arg(output)
        .output()
}

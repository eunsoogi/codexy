use serde_json::json;
use std::{fs, path::Path, process::Command};

use super::version_bump_pr_test_support::markdown_section_lines;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn refreshed_snapshot_replaces_stale_release_metadata_before_publication() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = tempfile::tempdir()?;
    let issue = temp.path().join("issue.json");
    let taxonomy = temp.path().join("repository-labels.json");
    let changes = temp.path().join("changed-files.txt");
    let output = temp.path().join("metadata");
    fs::write(&changes, "Cargo.toml\n")?;

    write_snapshot(
        &issue,
        &taxonomy,
        "https://github.com/openai-codex/codexy.release_1/issues/301",
        "priority/medium",
        "area/workflow",
    )?;
    render(root, &issue, &taxonomy, &changes, &output)?;
    let stale_body = fs::read_to_string(output.join("body.md"))?;
    let stale_labels = fs::read(output.join("labels.json"))?;
    render(root, &issue, &taxonomy, &changes, &output)?;
    assert_eq!(stale_body, fs::read_to_string(output.join("body.md"))?);
    assert_eq!(stale_labels, fs::read(output.join("labels.json"))?);

    write_snapshot(
        &issue,
        &taxonomy,
        "https://github.com/eunsoogi/codexy/issues/301",
        "priority/high",
        "area/qa",
    )?;
    render(root, &issue, &taxonomy, &changes, &output)?;
    let refreshed_body = fs::read_to_string(output.join("body.md"))?;

    assert_ne!(stale_body, refreshed_body);
    assert_eq!(
        markdown_section_lines(&refreshed_body, "## Evidence"),
        [
            "- Governing release issue: https://github.com/eunsoogi/codexy/issues/301",
            "- Full release-candidate validation ran before branch or pull-request mutation.",
            "- Post-creation readiness gates are pending.",
        ]
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(output.join("labels.json"))?)?,
        json!({"labels": ["area/qa", "priority/high", "status/review", "type/ci"]})
    );
    Ok(())
}

fn write_snapshot(
    issue: &Path,
    taxonomy: &Path,
    url: &str,
    priority: &str,
    area: &str,
) -> TestResult {
    fs::write(
        issue,
        serde_json::to_vec(&json!({
            "number": 301, "state": "OPEN", "url": url,
            "labels": [
                {"name": priority}, {"name": "status/ready"},
                {"name": "type/ci"}, {"name": area}
            ],
            "milestone": {"title": "1.3.1"}, "assignees": [{"login": "eunsoogi"}]
        }))?,
    )?;
    fs::write(
        taxonomy,
        serde_json::to_vec(&json!([
            {"name": priority}, {"name": "status/ready"},
            {"name": "status/review"}, {"name": "type/ci"}, {"name": area}
        ]))?,
    )?;
    Ok(())
}

fn render(
    root: &Path,
    issue: &Path,
    taxonomy: &Path,
    changes: &Path,
    output: &Path,
) -> std::io::Result<std::process::Output> {
    Command::new(root.join("scripts/render-version-pr-metadata"))
        .args([
            "--version", "1.3.1", "--issue-json", issue.to_str().unwrap(),
            "--repository-labels-json", taxonomy.to_str().unwrap(),
            "--changed-files-file", changes.to_str().unwrap(),
            "--output-dir", output.to_str().unwrap(),
            "--publication-phase", "provisional",
        ])
        .output()
}

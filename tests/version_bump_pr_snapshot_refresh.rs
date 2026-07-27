use serde_json::json;
use std::{fs, path::Path, process::Command};

use super::version_bump_pr_test_support::markdown_section_lines;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

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
    let stale = render(root, &issue, &taxonomy, &changes, &output)?;
    let repeated = render(root, &issue, &taxonomy, &changes, &output)?;
    assert_eq!(&stale.body, &repeated.body);
    assert_eq!(&stale.labels, &repeated.labels);

    write_snapshot(
        &issue,
        &taxonomy,
        "https://github.com/eunsoogi/codexy/issues/301",
        "priority/high",
        "area/qa",
    )?;
    let refreshed = render(root, &issue, &taxonomy, &changes, &output)?;

    assert_ne!(stale.body, refreshed.body);
    assert_eq!(
        markdown_section_lines(&refreshed.body, "## Evidence"),
        [
            "- Governing release issue: https://github.com/eunsoogi/codexy/issues/301",
            "- Full release-candidate validation ran before branch or pull-request mutation.",
            "- Post-creation readiness gates are pending.",
        ]
    );
    assert_eq!(
        refreshed.labels,
        json!({"labels": ["area/qa", "priority/high", "status/review", "type/ci"]})
    );
    Ok(())
}

#[test]
fn renderer_process_result_equivalence_matrix() -> TestResult {
    let temp = tempfile::tempdir()?;
    let cases = [
        ("success", true, "# Ready\n", r#"{"labels":["type/ci"]}"#, true),
        ("nonzero with plausible output", false, "# Ready\n", r#"{"labels":["type/ci"]}"#, false),
        ("empty output", true, "", r#"{"labels":["type/ci"]}"#, false),
        ("malformed output", true, "# Ready\n", "not-json", false),
    ];
    let mut mismatches = Vec::new();
    for (index, (name, succeeded, body, labels, expected)) in cases.into_iter().enumerate() {
        let output = temp.path().join(index.to_string());
        fs::create_dir(&output)?;
        fs::write(output.join("body.md"), body)?;
        fs::write(output.join("labels.json"), labels)?;
        let result = interpret_renderer_result(
            RendererProcessResult { succeeded, stderr: "renderer failed".into() },
            &output,
        );
        if result.is_ok() != expected {
            mismatches.push(name);
        }
    }
    assert!(mismatches.is_empty(), "process-result mismatches: {mismatches:?}");
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
) -> TestResult<RenderedMetadata> {
    let output_result = Command::new(root.join("scripts/render-version-pr-metadata"))
        .args([
            "--version", "1.3.1", "--issue-json", issue.to_str().unwrap(),
            "--repository-labels-json", taxonomy.to_str().unwrap(),
            "--changed-files-file", changes.to_str().unwrap(),
            "--output-dir", output.to_str().unwrap(),
            "--publication-phase", "provisional",
        ])
        .output()?;
    interpret_renderer_result(RendererProcessResult::from(output_result), output)
}

#[derive(Debug)]
struct RendererProcessResult {
    succeeded: bool,
    stderr: String,
}

impl From<std::process::Output> for RendererProcessResult {
    fn from(output: std::process::Output) -> Self {
        Self {
            succeeded: output.status.success(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

#[derive(Debug)]
struct RenderedMetadata {
    body: String,
    labels: serde_json::Value,
}

fn interpret_renderer_result(
    result: RendererProcessResult,
    output: &Path,
) -> TestResult<RenderedMetadata> {
    if !result.succeeded {
        return Err(std::io::Error::other(format!(
            "metadata renderer failed: {}",
            result.stderr.trim()
        ))
        .into());
    }
    let body = fs::read_to_string(output.join("body.md"))?;
    if body.trim().is_empty() {
        return Err(std::io::Error::other("metadata renderer produced an empty body.md").into());
    }
    Ok(RenderedMetadata {
        body,
        labels: serde_json::from_slice(&fs::read(output.join("labels.json"))?)?,
    })
}

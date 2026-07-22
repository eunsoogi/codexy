use serde_json::json;
use std::{fs, path::Path, process::Command};

use super::version_bump_pr_test_support::{
    markdown_headings, markdown_section_lines,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn renderer_emits_hook_valid_metadata_from_authoritative_issue() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = tempfile::tempdir()?;
    let issue_path = temp.path().join("issue.json");
    let labels_path = temp.path().join("repository-labels.json");
    let changes_path = temp.path().join("changed-files.txt");
    let output_dir = temp.path().join("metadata");
    fs::write(
        &issue_path,
        serde_json::to_vec(&json!({
            "number": 301,
            "state": "OPEN",
            "url": "https://github.com/eunsoogi/codexy/issues/301",
            "labels": [
                {"name": "priority/medium"},
                {"name": "status/ready"},
                {"name": "type/ci"},
                {"name": "area/workflow"},
                {"name": "area/release"}
            ],
            "milestone": {"title": "1.3.1"},
            "assignees": [{"login": "eunsoogi"}]
        }))?,
    )?;
    fs::write(
        &labels_path,
        serde_json::to_vec(&json!([
            {"name": "priority/medium"},
            {"name": "status/ready"},
            {"name": "status/review"},
            {"name": "type/ci"},
            {"name": "area/workflow"},
            {"name": "area/release"}
        ]))?,
    )?;
    fs::write(
        &changes_path,
        "Cargo.lock\nCargo.toml\n.agents/plugins/marketplace.json\n\
         plugins/codexy/.codex-plugin/plugin.json\n",
    )?;
    let output = Command::new(root.join("scripts/render-version-pr-metadata"))
        .args([
            "--version",
            "1.3.1",
            "--issue-json",
            path_text(&issue_path)?,
            "--repository-labels-json",
            path_text(&labels_path)?,
            "--changed-files-file",
            path_text(&changes_path)?,
            "--output-dir",
            path_text(&output_dir)?,
        ])
        .output()?;
    assert!(
        output.status.success(),
        "renderer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let title = fs::read_to_string(output_dir.join("title.txt"))?;
    let body = fs::read_to_string(output_dir.join("body.md"))?;
    let labels: serde_json::Value =
        serde_json::from_slice(&fs::read(output_dir.join("labels.json"))?)?;
    let first_render = (
        title.clone(),
        body.clone(),
        fs::read(output_dir.join("labels.json"))?,
    );
    assert_eq!(title, "chore(plugin): bump version to 1.3.1\n");
    assert_eq!(
        markdown_headings(&body),
        [
            "## Summary",
            "## Rationale",
            "## Changed Areas",
            "## Verification",
            "## Evidence",
            "## Not Run",
            "## Follow-ups",
        ]
    );
    assert_eq!(
        markdown_section_lines(&body, "## Changed Areas"),
        [
            "- `.agents/plugins/marketplace.json`",
            "- `Cargo.lock`",
            "- `Cargo.toml`",
            "- `plugins/codexy/.codex-plugin/plugin.json`",
        ]
    );
    assert_eq!(
        markdown_section_lines(&body, "## Verification"),
        [
            "- `scripts/sync-plugin-version --check`",
            "- `scripts/validate-plugin-config --check`",
            "- `cargo test --locked`",
            "- `git diff --check`",
            "- `plugins/codexy/hooks/codexy-pr-title-check.sh --pr-title <title>`",
            "- `plugins/codexy/hooks/codexy-pr-label-check.sh --pr-state-file <pr-state>`",
            "- `scripts/validate-plugin-config --check-completion-handoff --handoff-file <handoff> --pr-state-file <pr-state>`",
            "- `plugins/codexy/hooks/codexy-merge-message-check.sh --expected-pr <pr-number> --expected-issue <issue-number> --merge-message-file <merge-message>`",
        ]
    );
    assert!(body.ends_with("Fixes #301\n"));
    assert_eq!(
        labels,
        json!({"labels": [
            "area/release", "area/workflow", "priority/medium", "status/review", "type/ci"
        ]})
    );
    let hook = Command::new(root.join("plugins/codexy/hooks/codexy-pr-title-check.sh"))
        .args(["--pr-title", title.trim_end()])
        .output()?;
    assert!(
        hook.status.success(),
        "title hook failed: {}",
        String::from_utf8_lossy(&hook.stderr)
    );
    let rerender = Command::new(root.join("scripts/render-version-pr-metadata"))
        .args([
            "--version", "1.3.1", "--issue-json", path_text(&issue_path)?,
            "--repository-labels-json", path_text(&labels_path)?,
            "--changed-files-file", path_text(&changes_path)?,
            "--output-dir", path_text(&output_dir)?,
        ])
        .output()?;
    assert!(rerender.status.success());
    assert_eq!(first_render.0, fs::read_to_string(output_dir.join("title.txt"))?);
    assert_eq!(first_render.1, fs::read_to_string(output_dir.join("body.md"))?);
    assert_eq!(first_render.2, fs::read(output_dir.join("labels.json"))?);

    let pr_state = temp.path().join("pr-state.json");
    let pr_labels = labels["labels"].as_array().ok_or("rendered labels")?;
    fs::write(
        &pr_state,
        serde_json::to_vec(&json!({
            "number": 999, "state": "OPEN", "isDraft": false,
            "mergeStateStatus": "CLEAN", "reviewDecision": "APPROVED",
            "headRefName": "codexy/version-1.3.1", "repository": "eunsoogi/codexy",
            "labels": pr_labels, "repositoryLabels": serde_json::from_slice::<serde_json::Value>(&fs::read(&labels_path)?)?,
            "closingIssuesReferences": [{"number": 301, "labels": [{"name": "type/ci"}]}],
            "reviewThreads": {"pageInfo": {"hasNextPage": false}, "nodes": []}
        }))?,
    )?;
    command_passes(
        Command::new(root.join("plugins/codexy/hooks/codexy-pr-label-check.sh"))
            .args(["--pr-state-file", path_text(&pr_state)?]),
        "label hook",
    )?;
    let handoff = temp.path().join("handoff.md");
    fs::write(&handoff, "Verification completed. This PR is open for CI and review; this automation does not claim completion.\n")?;
    command_passes(
        Command::new(env!("CARGO_BIN_EXE_codexy-validate")).args([
            "--check-completion-handoff", "--handoff-file", path_text(&handoff)?,
            "--pr-state-file", path_text(&pr_state)?,
        ]),
        "completion handoff",
    )?;
    let merge_message = temp.path().join("merge-message.txt");
    fs::write(&merge_message, format!("{} (#999)\n\n{}", title.trim_end(), body))?;
    command_passes(
        Command::new(root.join("plugins/codexy/hooks/codexy-merge-message-check.sh"))
            .args(["--expected-pr", "999", "--expected-issue", "301", "--merge-message-file", path_text(&merge_message)?]),
        "merge-message hook",
    )?;
    Ok(())
}

fn path_text(path: &Path) -> Result<&str, &'static str> {
    path.to_str().ok_or("non-UTF-8 test path")
}

fn command_passes(command: &mut Command, context: &str) -> TestResult {
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!("{context} failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(())
}

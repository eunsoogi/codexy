use serde_json::json;
use serde_yaml::Value;
use std::{fs, path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn workflow_requires_issue_scope_and_reconciles_one_pr() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let text = fs::read_to_string(root.join(".github/workflows/plugin-version-bump.yml"))?;
    let document: Value = serde_yaml::from_str(&text)?;
    let workflow = document.as_mapping().ok_or("workflow root")?;
    let dispatch = workflow
        .iter()
        .find(|(key, _)| key.as_str() == Some("on") || **key == Value::Bool(true))
        .and_then(|(_, value)| value.get("workflow_dispatch"))
        .ok_or("workflow_dispatch")?;
    let issue = dispatch
        .get("inputs")
        .and_then(|inputs| inputs.get("issue"))
        .ok_or("governing issue input")?;
    assert_eq!(issue.get("required").and_then(Value::as_bool), Some(true));

    let permissions = workflow
        .get(Value::String("permissions".into()))
        .ok_or("workflow permissions")?;
    assert_eq!(permissions.get("issues").and_then(Value::as_str), Some("write"));
    let steps = workflow
        .get(Value::String("jobs".into()))
        .and_then(|jobs| jobs.get("open-version-pr"))
        .and_then(|job| job.get("steps"))
        .and_then(Value::as_sequence)
        .ok_or("version bump steps")?;
    let validate_issue = named_step_run(steps, "Validate governing release issue")?;
    let synchronize = named_step_run(steps, "Synchronize plugin version")?;
    let validate_release = named_step_run(steps, "Validate release candidate")?;
    let reconcile = named_step_run(steps, "Open version bump pull request")?;
    assert!(validate_issue.contains("gh issue view"));
    assert!(validate_issue.contains("scripts/render-version-pr-metadata"));
    assert_eq!(synchronize, "scripts/sync-plugin-version --version \"$VERSION\"");
    for command in [
        "scripts/sync-plugin-version --check",
        "scripts/validate-plugin-config --check",
        "cargo test --locked",
        "git diff --check",
    ] {
        assert!(validate_release.contains(command), "missing validation: {command}");
    }
    for command in [
        "gh pr list",
        "git ls-remote",
        "gh api --method PUT",
        "codexy-pr-title-check.sh",
        "codexy-pr-label-check.sh",
        "--check-completion-handoff",
    ] {
        assert!(reconcile.contains(command), "missing reconciliation: {command}");
    }
    assert!(!reconcile.contains("--force"));
    assert!(reconcile.find("gh pr list") < reconcile.find("git push"));
    assert!(reconcile.contains("[ \"$pr_count\" -eq 0 ] && [ \"$remote_exists\" = false ]"));
    assert!(reconcile.contains("[ \"$pr_count\" -eq 1 ] && [ \"$remote_exists\" = true ]"));
    assert!(reconcile.contains("Branch and pull-request state disagree"));
    Ok(())
}

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
    for section in [
        "## Summary",
        "## Rationale",
        "## Changed Areas",
        "## Verification",
        "## Evidence",
        "## Not Run",
        "## Follow-ups",
    ] {
        assert!(body.contains(section), "missing body section: {section}");
    }
    for changed in ["Cargo.lock", "Cargo.toml", ".agents/plugins/marketplace.json"] {
        assert!(body.contains(&format!("- `{changed}`")));
    }
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

fn named_step_run<'a>(steps: &'a [Value], name: &str) -> Result<&'a str, &'static str> {
    steps
        .iter()
        .find(|step| step.get("name").and_then(Value::as_str) == Some(name))
        .and_then(|step| step.get("run"))
        .and_then(Value::as_str)
        .ok_or("named workflow step or run command missing")
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

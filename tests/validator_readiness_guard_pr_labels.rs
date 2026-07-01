use std::process::Command;

#[test]
fn readiness_guard_checks_pr_labels_against_repository_taxonomy()
-> Result<(), Box<dyn std::error::Error>> {
    let script = readiness_guard();
    let temp = tempfile::tempdir()?;

    let unlabeled = write_pr_state(
        temp.path(),
        "unlabeled.json",
        r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","labels":[],"repositoryLabels":[{"name":"type/fix"},{"name":"status/review"},{"name":"area/workflow"}]}"#,
    )?;
    let bad = Command::new(&script)
        .args([
            "--check-pr-labels",
            "--pr-state-file",
            unlabeled.to_str().ok_or("unlabeled state path")?,
        ])
        .output()?;
    assert!(
        !bad.status.success(),
        "guard should reject unlabeled PRs when repository labels are captured"
    );
    assert!(
        output_text(&bad).contains("PR labels missing label application evidence"),
        "unexpected output: {}",
        output_text(&bad)
    );

    let string_unlabeled = write_pr_state(
        temp.path(),
        "string-unlabeled.json",
        r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","labels":[],"repositoryLabels":["type/fix","status/review"]}"#,
    )?;
    let string_bad = Command::new(&script)
        .args([
            "--check-pr-labels",
            "--pr-state-file",
            string_unlabeled
                .to_str()
                .ok_or("string unlabeled state path")?,
        ])
        .output()?;
    assert!(
        !string_bad.status.success(),
        "guard should reject unlabeled PRs when repository labels are captured as strings"
    );
    assert!(
        output_text(&string_bad).contains("PR labels missing label application evidence"),
        "unexpected output: {}",
        output_text(&string_bad)
    );

    let labeled = write_pr_state(
        temp.path(),
        "labeled.json",
        r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","labels":[{"name":"type/fix"},{"name":"area/workflow"}],"repositoryLabels":[{"name":"type/fix"},{"name":"status/review"},{"name":"area/workflow"}]}"#,
    )?;
    let good = Command::new(&script)
        .args([
            "--check-pr-labels",
            "--pr-state-file",
            labeled.to_str().ok_or("labeled state path")?,
        ])
        .output()?;
    assert!(
        good.status.success(),
        "guard should accept labeled PRs without hard-coding taxonomy labels\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&good.stdout),
        String::from_utf8_lossy(&good.stderr)
    );

    let string_labeled = write_pr_state(
        temp.path(),
        "string-labeled.json",
        r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","labels":["type/fix"],"repositoryLabels":["type/fix","status/review"]}"#,
    )?;
    let string_good = Command::new(&script)
        .args([
            "--check-pr-labels",
            "--pr-state-file",
            string_labeled.to_str().ok_or("string labeled state path")?,
        ])
        .output()?;
    assert!(
        string_good.status.success(),
        "guard should accept PR labels captured as strings\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&string_good.stdout),
        String::from_utf8_lossy(&string_good.stderr)
    );
    Ok(())
}

#[test]
fn readiness_guard_ignores_nested_labels_before_top_level_pr_labels()
-> Result<(), Box<dyn std::error::Error>> {
    let script = readiness_guard();
    let temp = tempfile::tempdir()?;

    for (name, json) in [
        (
            "repository-before-pr-labels.json",
            r#"{"number":209,"state":"OPEN","repository":{"labels":{"nodes":[{"name":"type/fix"}]}},"labels":[],"repositoryLabels":[{"name":"type/fix"}]}"#,
        ),
        (
            "closing-issue-before-pr-labels.json",
            r#"{"number":209,"state":"OPEN","closingIssuesReferences":[{"number":216,"labels":[{"name":"type/fix"}]}],"labels":[],"repositoryLabels":[{"name":"type/fix"}]}"#,
        ),
    ] {
        let pr_state = write_pr_state(temp.path(), name, json)?;
        let output = Command::new(&script)
            .args([
                "--check-pr-labels",
                "--pr-state-file",
                pr_state.to_str().ok_or("pr state path")?,
            ])
            .output()?;
        assert!(
            !output.status.success(),
            "guard should reject unlabeled PR even when nested labels appear first for {name}"
        );
        assert!(
            output_text(&output).contains("PR labels missing label application evidence"),
            "unexpected output: {}",
            output_text(&output)
        );
    }
    Ok(())
}

#[test]
fn readiness_guard_allows_missing_or_empty_repository_label_taxonomy()
-> Result<(), Box<dyn std::error::Error>> {
    let script = readiness_guard();
    let temp = tempfile::tempdir()?;

    for (name, json) in [
        (
            "missing-taxonomy.json",
            r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","labels":[]}"#,
        ),
        (
            "empty-taxonomy.json",
            r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","labels":[],"repositoryLabels":[]}"#,
        ),
        (
            "empty-graphql-taxonomy.json",
            r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","labels":[],"repositoryLabels":{"nodes":[]},"repository":{"labels":{"nodes":[]}}}"#,
        ),
        (
            "closing-issue-graphql-labels-only.json",
            r#"{"number":209,"state":"OPEN","labels":[],"closingIssuesReferences":{"nodes":[{"number":216,"labels":{"nodes":[{"name":"type/fix"}]}}]}}"#,
        ),
        (
            "nested-repository-labels-only.json",
            r#"{"number":209,"state":"OPEN","repository":{"metadata":{"labels":{"nodes":[{"name":"type/fix"}]}}},"labels":[]}"#,
        ),
    ] {
        let pr_state = write_pr_state(temp.path(), name, json)?;
        let output = Command::new(&script)
            .args([
                "--check-pr-labels",
                "--pr-state-file",
                pr_state.to_str().ok_or("pr state path")?,
            ])
            .output()?;
        assert!(
            output.status.success(),
            "guard should allow no-repository-label-taxonomy state for {name}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
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

fn write_pr_state(
    dir: &std::path::Path,
    name: &str,
    json: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let path = dir.join(name);
    std::fs::write(&path, json)?;
    Ok(path)
}

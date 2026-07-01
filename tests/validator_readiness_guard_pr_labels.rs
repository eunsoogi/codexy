use std::process::Command;

#[test]
fn readiness_guard_checks_pr_labels_against_repository_taxonomy()
-> Result<(), Box<dyn std::error::Error>> {
    let script = readiness_guard();
    let temp = tempfile::tempdir()?;

    for (name, json, reason) in [
        (
            "unlabeled.json",
            r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","labels":[],"repositoryLabels":[{"name":"type/fix"},{"name":"status/review"},{"name":"area/workflow"}]}"#,
            "repository labels are captured",
        ),
        (
            "string-unlabeled.json",
            r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","labels":[],"repositoryLabels":["type/fix","status/review"]}"#,
            "repository labels are captured as strings",
        ),
        (
            "graphql-string-unlabeled.json",
            r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","labels":[],"repositoryLabels":{"nodes":["type/fix","status/review"]}}"#,
            "repository label nodes are strings",
        ),
        (
            "fallback-unlabeled.json",
            r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","repositoryLabels":[],"labels":[],"repository":{"labels":{"nodes":[{"name":"type/fix"}]}}}"#,
            "fallback repository labels exist",
        ),
    ] {
        assert_rejects(&script, temp.path(), name, json, reason)?;
    }

    for (name, json, reason) in [
        (
            "labeled.json",
            r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","labels":[{"name":"type/fix"},{"name":"area/workflow"}],"repositoryLabels":[{"name":"type/fix"},{"name":"status/review"},{"name":"area/workflow"}]}"#,
            "labeled PRs without hard-coding taxonomy labels",
        ),
        (
            "string-labeled.json",
            r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","labels":["type/fix"],"repositoryLabels":["type/fix","status/review"]}"#,
            "PR labels captured as strings",
        ),
        (
            "graphql-string-labeled.json",
            r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","labels":{"nodes":["type/fix"]},"repositoryLabels":{"nodes":["type/fix","status/review"]}}"#,
            "PR label nodes captured as strings",
        ),
        (
            "fallback-labeled.json",
            r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","repositoryLabels":null,"labels":{"nodes":[{"name":"type/fix"}]},"repository":{"labels":{"nodes":[{"name":"type/fix"}]}}}"#,
            "labeled PRs when fallback repository labels exist",
        ),
    ] {
        assert_accepts(&script, temp.path(), name, json, reason)?;
    }
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

fn assert_rejects(
    script: &std::path::Path,
    dir: &std::path::Path,
    name: &str,
    json: &str,
    reason: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = guard_output(script, dir, name, json)?;
    assert!(
        !output.status.success(),
        "guard should reject unlabeled PRs when {reason}"
    );
    assert!(
        output_text(&output).contains("PR labels missing label application evidence"),
        "unexpected output: {}",
        output_text(&output)
    );
    Ok(())
}

fn assert_accepts(
    script: &std::path::Path,
    dir: &std::path::Path,
    name: &str,
    json: &str,
    reason: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = guard_output(script, dir, name, json)?;
    assert!(
        output.status.success(),
        "guard should accept {reason}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn guard_output(
    script: &std::path::Path,
    dir: &std::path::Path,
    name: &str,
    json: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let pr_state = write_pr_state(dir, name, json)?;
    Ok(Command::new(script)
        .args([
            "--check-pr-labels",
            "--pr-state-file",
            pr_state.to_str().ok_or("pr state path")?,
        ])
        .output()?)
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

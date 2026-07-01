use std::process::Command;

#[test]
fn readiness_guard_rejects_incomplete_pr_label_state() -> Result<(), Box<dyn std::error::Error>> {
    let script = readiness_guard();
    let temp = tempfile::tempdir()?;

    for (name, json, expected) in [
        (
            "missing-taxonomy.json",
            r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","labels":[]}"#,
            "repositoryLabels taxonomy",
        ),
        ("empty-state.json", "", "malformed JSON evidence"),
        (
            "truncated-object-labels.json",
            r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","labels":[{"name":"type/fix"}],"repositoryLabels":[{"name":"type/fix"}]"#,
            "malformed JSON evidence",
        ),
        (
            "truncated-string-labels.json",
            r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","labels":["type/fix"],"repositoryLabels":["type/fix"]"#,
            "malformed JSON evidence",
        ),
        (
            "missing-repository-identity.json",
            r#"{"number":209,"state":"OPEN","labels":[],"repositoryLabels":["type/fix"]}"#,
            "repository identity evidence",
        ),
        (
            "closing-issue-labels-only.json",
            r#"{"number":209,"state":"OPEN","repository":"eunsoogi/codexy","labels":[],"closingIssuesReferences":{"nodes":[{"number":216,"labels":{"nodes":[{"name":"type/fix"}]}}]}}"#,
            "repositoryLabels taxonomy",
        ),
        (
            "nested-repository-labels-only.json",
            r#"{"number":209,"state":"OPEN","url":"https://github.com/eunsoogi/codexy/pull/209","repository":{"metadata":{"labels":{"nodes":[{"name":"type/fix"}]}}},"labels":[]}"#,
            "repositoryLabels taxonomy",
        ),
    ] {
        let pr_state = temp.path().join(name);
        std::fs::write(&pr_state, json)?;
        let output = Command::new(&script)
            .args([
                "--check-pr-labels",
                "--pr-state-file",
                pr_state.to_str().ok_or("pr state path")?,
            ])
            .output()?;
        assert!(
            !output.status.success(),
            "guard should reject incomplete PR state for {name}"
        );
        assert!(
            output_text(&output).contains(expected),
            "expected {expected} in output: {}",
            output_text(&output)
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

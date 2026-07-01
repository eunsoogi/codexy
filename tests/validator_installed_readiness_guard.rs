use std::process::Command;

#[allow(unused)]
mod support;

#[test]
fn installed_readiness_guard_validates_merge_bodies() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    assert!(!plugin_root.join("scripts/validate-plugin-config").exists());
    let script = plugin_root.join("hooks/codexy-readiness-guard.sh");

    let bad = Command::new(&script)
        .args([
            "--check-merge-message",
            "--expected-pr",
            "204",
            "--merge-message",
            "fix(workflow): x (#204)\n\nCloses #999\n",
        ])
        .output()?;
    assert!(!bad.status.success());
    assert!(
        output_text(&bad).contains("merge commit message must not contain closing references"),
        "unexpected output: {}",
        output_text(&bad)
    );

    let good = Command::new(&script)
        .args([
            "--check-merge-message",
            "--expected-pr",
            "204",
            "--expected-issue",
            "206",
            "--merge-message",
            "fix(workflow): x (#204)\n\nFixes #206\n",
        ])
        .output()?;
    assert!(
        good.status.success(),
        "installed guard should accept valid merge messages\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&good.stdout),
        String::from_utf8_lossy(&good.stderr)
    );
    Ok(())
}

#[test]
fn installed_readiness_guard_validates_pr_labels() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    assert!(!plugin_root.join("scripts/validate-plugin-config").exists());
    let script = plugin_root.join("hooks/codexy-readiness-guard.sh");

    let labeled = write_pr_state(
        temp.path(),
        "labeled.json",
        r#"{"number":216,"state":"OPEN","repository":"eunsoogi/codexy","labels":{"nodes":[{"name":"type/fix"},{"name":"area/workflow"}]},"repositoryLabels":{"nodes":[{"name":"type/fix"},{"name":"area/workflow"},{"name":"status/review"}]}}"#,
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
        "installed guard should accept labeled PRs\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&good.stdout),
        String::from_utf8_lossy(&good.stderr)
    );

    let unlabeled = write_pr_state(
        temp.path(),
        "unlabeled.json",
        r#"{"number":216,"state":"OPEN","repository":"eunsoogi/codexy","labels":[],"repositoryLabels":{"nodes":[{"name":"type/fix"},{"name":"status/review"}]}}"#,
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
        "installed guard should reject unlabeled PRs when repository labels exist"
    );
    assert!(
        output_text(&bad).contains("PR labels missing label application evidence"),
        "unexpected output: {}",
        output_text(&bad)
    );

    let no_taxonomy = write_pr_state(
        temp.path(),
        "no-taxonomy.json",
        r#"{"number":216,"state":"OPEN","repository":"eunsoogi/codexy","labels":[]}"#,
    )?;
    let no_taxonomy_output = Command::new(&script)
        .args([
            "--check-pr-labels",
            "--pr-state-file",
            no_taxonomy.to_str().ok_or("no taxonomy state path")?,
        ])
        .output()?;
    assert!(
        !no_taxonomy_output.status.success(),
        "installed guard should reject missing repository label taxonomy"
    );
    assert!(
        output_text(&no_taxonomy_output).contains("repositoryLabels taxonomy"),
        "unexpected output: {}",
        output_text(&no_taxonomy_output)
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

fn write_pr_state(
    dir: &std::path::Path,
    name: &str,
    json: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let path = dir.join(name);
    std::fs::write(&path, json)?;
    Ok(path)
}

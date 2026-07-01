use std::process::Command;

#[test]
fn readiness_guard_checks_pr_titles() -> Result<(), Box<dyn std::error::Error>> {
    let script = readiness_guard();

    let bad = Command::new(&script)
        .args([
            "--check-pr-title",
            "--pr-title",
            "Require descriptive child thread titles",
        ])
        .output()?;
    assert!(!bad.status.success(), "guard should reject plain PR titles");
    assert!(
        String::from_utf8_lossy(&bad.stdout)
            .contains("PR title must use Conventional Commit style"),
        "unexpected stdout: {}",
        String::from_utf8_lossy(&bad.stdout)
    );

    let good = Command::new(&script)
        .args([
            "--check-pr-title",
            "--pr-title",
            "fix(workflow): enforce PR title gate",
        ])
        .output()?;
    assert!(
        good.status.success(),
        "guard should accept Conventional Commit PR titles\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&good.stdout),
        String::from_utf8_lossy(&good.stderr)
    );
    Ok(())
}

#[test]
fn readiness_guard_checks_squash_subject_suffix_spacing() -> Result<(), Box<dyn std::error::Error>>
{
    let script = readiness_guard();

    let bad = Command::new(&script)
        .args([
            "--check-merge-message",
            "--expected-pr",
            "204",
            "--merge-message",
            "fix(workflow): enforce gate(#204)\n\nFixes #206\n",
        ])
        .output()?;
    assert!(
        !bad.status.success(),
        "guard should reject unseparated PR suffixes"
    );
    assert!(
        String::from_utf8_lossy(&bad.stdout)
            .contains("merge commit subject must end with the expected PR suffix"),
        "unexpected stdout: {}",
        String::from_utf8_lossy(&bad.stdout)
    );

    let good = Command::new(&script)
        .args([
            "--check-merge-message",
            "--expected-pr",
            "204",
            "--merge-message",
            "fix(workflow): enforce gate (#204)\n\nFixes #206\n",
        ])
        .output()?;
    assert!(
        good.status.success(),
        "guard should accept separated PR suffixes\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&good.stdout),
        String::from_utf8_lossy(&good.stderr)
    );
    Ok(())
}

#[test]
fn readiness_guard_rejects_whitespace_only_summaries() -> Result<(), Box<dyn std::error::Error>> {
    let script = readiness_guard();

    let bad_title = Command::new(&script)
        .args(["--check-pr-title", "--pr-title", "fix:   "])
        .output()?;
    assert!(
        !bad_title.status.success(),
        "guard should reject whitespace-only PR title summaries"
    );
    assert!(
        String::from_utf8_lossy(&bad_title.stdout)
            .contains("PR title must use Conventional Commit style"),
        "unexpected stdout: {}",
        String::from_utf8_lossy(&bad_title.stdout)
    );

    let bad_merge_message = Command::new(&script)
        .args([
            "--check-merge-message",
            "--expected-pr",
            "204",
            "--merge-message",
            "fix:    (#204)\n\nFixes #206\n",
        ])
        .output()?;
    assert!(
        !bad_merge_message.status.success(),
        "guard should reject whitespace-only merge subject summaries"
    );
    assert!(
        String::from_utf8_lossy(&bad_merge_message.stdout)
            .contains("merge commit subject must use Conventional Commit style"),
        "unexpected stdout: {}",
        String::from_utf8_lossy(&bad_merge_message.stdout)
    );
    Ok(())
}

fn readiness_guard() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/codexy/hooks/codexy-readiness-guard.sh")
}

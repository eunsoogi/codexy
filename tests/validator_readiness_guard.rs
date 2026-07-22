use std::process::Command;

#[test]
fn readiness_guard_rejects_lifecycle_event_invocation() -> Result<(), Box<dyn std::error::Error>> {
    let script = readiness_guard();

    let output = Command::new(&script).arg("UserPromptSubmit").output()?;
    assert!(
        !output.status.success(),
        "guard must not retain a UserPromptSubmit diagnostic mode\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("hookSpecificOutput"),
        "lifecycle invocation emitted model context"
    );
    assert!(
        std::fs::read_to_string(&script)?
            .find("UserPromptSubmit")
            .is_none(),
        "static readiness guard retained lifecycle-specific diagnostics"
    );
    Ok(())
}

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
        output_text(&bad)
            .find("merge commit subject must end with the expected PR suffix")
            .is_some(),
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
fn readiness_guard_delegates_merge_body_validation() -> Result<(), Box<dyn std::error::Error>> {
    let script = readiness_guard();

    let bad = Command::new(&script)
        .args([
            "--check-merge-message",
            "--expected-pr",
            "204",
            "--merge-message",
            "fix(workflow): x (#204)\n\nCloses #999\n",
        ])
        .output()?;
    assert!(
        !bad.status.success(),
        "guard should reject closing references when no issue is expected"
    );
    assert!(
        output_text(&bad)
            .find("merge commit message must not contain closing references")
            .is_some(),
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
        "guard should accept validator-valid merge bodies\nstdout:\n{}\nstderr:\n{}",
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

    let empty_title = Command::new(&script)
        .args(["--check-pr-title", "--pr-title", "fix: "])
        .output()?;
    assert!(
        !empty_title.status.success(),
        "guard should reject empty PR title summaries"
    );
    assert!(
        String::from_utf8_lossy(&empty_title.stdout)
            .contains("PR title must use Conventional Commit style"),
        "unexpected stdout: {}",
        String::from_utf8_lossy(&empty_title.stdout)
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
        output_text(&bad_merge_message)
            .find("merge commit subject must use Conventional Commit style")
            .is_some(),
        "unexpected output: {}",
        output_text(&bad_merge_message)
    );

    let empty_merge_message = Command::new(&script)
        .args([
            "--check-merge-message",
            "--expected-pr",
            "204",
            "--merge-message",
            "fix:  (#204)\n\nFixes #206\n",
        ])
        .output()?;
    assert!(
        !empty_merge_message.status.success(),
        "guard should reject empty merge subject summaries"
    );
    assert!(
        output_text(&empty_merge_message)
            .find("merge commit subject must use Conventional Commit style")
            .is_some(),
        "unexpected output: {}",
        output_text(&empty_merge_message)
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

fn readiness_guard() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/codexy/hooks/codexy-readiness-guard.sh")
}

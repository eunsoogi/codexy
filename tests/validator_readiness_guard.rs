use std::process::Command;

#[test]
fn readiness_guard_context_includes_expected_pr_flag() -> Result<(), Box<dyn std::error::Error>> {
    let script = readiness_guard();

    let output = Command::new(&script).arg("UserPromptSubmit").output()?;
    assert!(
        output.status.success(),
        "guard context should emit successfully\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let hook_json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let context = hook_json["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .ok_or("guard context should include additionalContext")?;
    assert!(
        context.contains("--check-merge-message --expected-pr PR_NUMBER"),
        "merge guidance must include the required expected PR flag: {context}"
    );
    assert!(
        context.contains("--expected-issue ISSUE_NUMBER"),
        "issue-backed merge guidance must include the conditional expected issue flag: {context}"
    );
    assert!(
        context.contains("issue-backed"),
        "merge guidance must tell agents when the expected issue flag applies: {context}"
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
        output_text(&bad).contains("merge commit subject must end with the expected PR suffix"),
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
            .contains("merge commit subject must use Conventional Commit style"),
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
            .contains("merge commit subject must use Conventional Commit style"),
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

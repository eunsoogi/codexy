use std::process::Command;

#[test]
fn validator_cli_accepts_merge_message_with_final_expected_closing_reference()
-> Result<(), Box<dyn std::error::Error>> {
    let message = "fix(workflow): tighten merge evidence (#122)\n\nFixes #121\n";
    let output = validate_message(message)?;
    assert!(
        output.status.success(),
        "validator should accept merge messages with exactly one final expected closing reference\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_merge_message_missing_expected_issue_reference()
-> Result<(), Box<dyn std::error::Error>> {
    reject_message(
        "fix(workflow): tighten merge evidence (#122)\n\nReviewed and verified.\n",
        "final closing line must be exactly",
    )
}

#[test]
fn validator_cli_checks_known_pr_suffix() -> Result<(), Box<dyn std::error::Error>> {
    reject_message_for_pr(
        "fix(workflow): require issue references in merge messages\n\nFixes #121\n",
        123,
        "subject must end with the expected PR suffix",
    )?;
    let message =
        "fix(workflow): require issue references in merge messages (#123)\n\nFixes #121\n";
    let output = validate_message_for_pr(message, 123)?;
    assert!(
        output.status.success(),
        "validator should accept a squash subject ending with the expected PR suffix\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_checks_merge_message_file_input() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let message_path = temp.path().join("merge-message.txt");
    std::fs::write(
        &message_path,
        "fix(workflow): tighten merge evidence (#122)\n\nFixes #121\n",
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--check-merge-message",
            "--expected-issue",
            "121",
            "--merge-message-file",
            message_path.to_str().ok_or("message path")?,
        ])
        .output()?;
    assert!(
        output.status.success(),
        "validator should accept file-provided merge messages with the expected issue reference\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn validate_message(message: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--check-merge-message",
            "--expected-issue",
            "121",
            "--merge-message",
            message,
        ])
        .output()?)
}

fn validate_message_for_pr(
    message: &str,
    expected_pr: u64,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--check-merge-message",
            "--expected-issue",
            "121",
            "--expected-pr",
            &expected_pr.to_string(),
            "--merge-message",
            message,
        ])
        .output()?)
}

fn reject_message(message: &str, expected: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = validate_message(message)?;
    assert!(
        !output.status.success(),
        "validator should reject {message:?}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn reject_message_for_pr(
    message: &str,
    expected_pr: u64,
    expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = validate_message_for_pr(message, expected_pr)?;
    assert!(
        !output.status.success(),
        "validator should reject {message:?}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

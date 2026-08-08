use std::process::Command;

#[test]
fn validator_cli_rejects_plain_squash_subject() -> Result<(), Box<dyn std::error::Error>> {
    reject_message_for_pr(
        "Require descriptive child thread titles\n\nFixes #121\n",
        123,
        "merge commit subject must use Conventional Commit style",
    )
}

#[test]
fn validator_cli_rejects_plain_squash_subject_with_pr_suffix()
-> Result<(), Box<dyn std::error::Error>> {
    reject_message_for_pr(
        "Refactor oversized Codexy skill instructions (#203)\n\nFixes #121\n",
        203,
        "merge commit subject must use Conventional Commit style",
    )
}

#[test]
fn validator_cli_accepts_conventional_subject_with_pr_suffix()
-> Result<(), Box<dyn std::error::Error>> {
    let output =
        validate_message_for_pr("fix(workflow): enforce gate (#204)\n\nFixes #121\n", 204)?;
    assert!(
        output.status.success(),
        "validator should accept a Conventional Commit squash subject with expected PR suffix\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_unseparated_pr_suffix() -> Result<(), Box<dyn std::error::Error>> {
    reject_message_for_pr(
        "fix(workflow): enforce gate(#204)\n\nFixes #121\n",
        204,
        "merge commit subject must end with the expected PR suffix",
    )
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

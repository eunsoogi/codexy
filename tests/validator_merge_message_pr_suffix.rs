use std::process::Command;

#[test]
fn validator_cli_checks_pr_suffix_without_expected_issue() -> Result<(), Box<dyn std::error::Error>>
{
    reject_pr_only_message(
        "fix(workflow): require PR suffix\n\nReviewed and verified.\n",
        "subject must end with the expected PR suffix",
    )?;

    let output = validate_pr_only_message(
        "fix(workflow): require PR suffix (#188)\n\nReviewed and verified.\n",
    )?;
    assert!(
        output.status.success(),
        "validator should accept PR-only merge messages with the expected subject suffix\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn validate_pr_only_message(
    message: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--check-merge-message",
            "--expected-pr",
            "188",
            "--merge-message",
            message,
        ])
        .output()?)
}

fn reject_pr_only_message(message: &str, expected: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = validate_pr_only_message(message)?;
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

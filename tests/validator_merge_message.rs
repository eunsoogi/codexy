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
fn validator_cli_rejects_incidental_only_issue_reference() -> Result<(), Box<dyn std::error::Error>>
{
    reject_message(
        "fix(workflow): tighten merge evidence (#122)\n\nRationale: see #121.\n",
        "final closing line must be exactly",
    )
}

#[test]
fn validator_cli_rejects_duplicate_expected_closing_references()
-> Result<(), Box<dyn std::error::Error>> {
    reject_message(
        "fix(workflow): tighten merge evidence (#122)\n\nFixes #121\n\nFollow-up.\n\nFixes #121\n",
        "exactly one closing reference",
    )
}

#[test]
fn validator_cli_rejects_ambiguous_closing_references() -> Result<(), Box<dyn std::error::Error>> {
    reject_message(
        "fix(workflow): tighten merge evidence (#122)\n\nFixes #120\nFixes #121\n",
        "exactly one closing reference",
    )
}

#[test]
fn validator_cli_rejects_extra_closes_keyword_reference() -> Result<(), Box<dyn std::error::Error>>
{
    reject_message(
        "fix(workflow): tighten merge evidence (#122)\n\nCloses #120\n\nFixes #121\n",
        "exactly one closing reference",
    )
}

#[test]
fn validator_cli_rejects_extra_resolves_keyword_reference() -> Result<(), Box<dyn std::error::Error>>
{
    reject_message(
        "fix(workflow): tighten merge evidence (#122)\n\nResolves: #120\n\nFixes #121\n",
        "exactly one closing reference",
    )
}

#[test]
fn validator_cli_rejects_uppercase_extra_closing_keyword() -> Result<(), Box<dyn std::error::Error>>
{
    reject_message(
        "fix(workflow): tighten merge evidence (#122)\n\nCLOSES #120\n\nFixes #121\n",
        "exactly one closing reference",
    )
}

#[test]
fn validator_cli_rejects_uppercase_final_fixes_line() -> Result<(), Box<dyn std::error::Error>> {
    reject_message(
        "fix(workflow): tighten merge evidence (#122)\n\nFIXES #121\n",
        "final closing line must be exactly",
    )
}

#[test]
fn validator_cli_rejects_sentence_form_closing_reference() -> Result<(), Box<dyn std::error::Error>>
{
    reject_message(
        "fix(workflow): tighten merge evidence (#122)\n\nThis also fixes #120.\n\nFixes #121\n",
        "exactly one closing reference",
    )
}

#[test]
fn validator_cli_rejects_cross_repo_closing_reference() -> Result<(), Box<dyn std::error::Error>> {
    reject_message(
        "fix(workflow): tighten merge evidence (#122)\n\nFixes owner/repo#120\n\nFixes #121\n",
        "exactly one closing reference",
    )
}

#[test]
fn validator_cli_rejects_multiple_closing_references_on_one_line()
-> Result<(), Box<dyn std::error::Error>> {
    reject_message(
        "fix(workflow): tighten merge evidence (#122)\n\nResolves #10, resolves #123\n\nFixes #121\n",
        "exactly one closing reference",
    )
}

#[test]
fn validator_cli_rejects_padded_final_closing_reference() -> Result<(), Box<dyn std::error::Error>>
{
    reject_message(
        "fix(workflow): tighten merge evidence (#122)\n\n  Fixes #121  \n",
        "final closing line must be exactly",
    )
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

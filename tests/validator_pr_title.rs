use std::process::Command;

#[test]
fn validator_cli_rejects_plain_pr_title() -> Result<(), Box<dyn std::error::Error>> {
    reject_title(
        "Require descriptive child thread titles",
        "PR title must use Conventional Commit style",
    )
}

#[test]
fn validator_cli_accepts_conventional_pr_title() -> Result<(), Box<dyn std::error::Error>> {
    let output = validate_title("fix(workflow): enforce PR title gate")?;
    assert!(
        output.status.success(),
        "validator should accept a Conventional Commit PR title\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn validate_title(title: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-pr-title", "--pr-title", title])
        .output()?)
}

fn reject_title(title: &str, expected: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = validate_title(title)?;
    assert!(
        !output.status.success(),
        "validator should reject {title:?}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

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

#[test]
fn validator_cli_accepts_bang_conventional_pr_title() -> Result<(), Box<dyn std::error::Error>> {
    let output = validate_title("fix!: enforce breaking workflow gate")?;
    assert!(
        output.status.success(),
        "validator should accept a Conventional Commit PR title with bang marker\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_conventional_issue_title() -> Result<(), Box<dyn std::error::Error>> {
    let output = validate_issue_title("fix(agents): reject negated sentinel evidence")?;
    assert!(
        !output.status.success(),
        "validator should reject Conventional Commit issue titles"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("issue title must not use Conventional Commit style"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_capitalized_conventional_issue_title()
-> Result<(), Box<dyn std::error::Error>> {
    reject_issue_title(
        "Fix(agents): reject negated sentinel evidence",
        "issue title must not use Conventional Commit style",
    )
}

#[test]
fn validator_cli_rejects_prefix_only_conventional_issue_title()
-> Result<(), Box<dyn std::error::Error>> {
    reject_issue_title(
        "Fix(agents) reject negated sentinel evidence",
        "issue title must not use Conventional Commit style",
    )
}

#[test]
fn validator_cli_rejects_bare_colon_conventional_issue_title()
-> Result<(), Box<dyn std::error::Error>> {
    reject_issue_title(
        "Fix(agents):",
        "issue title must not use Conventional Commit style",
    )?;
    reject_issue_title(
        "Fix!: ",
        "issue title must not use Conventional Commit style",
    )
}

#[test]
fn validator_cli_rejects_repeated_colon_conventional_issue_title()
-> Result<(), Box<dyn std::error::Error>> {
    reject_issue_title(
        "Fix:: break",
        "issue title must not use Conventional Commit style",
    )?;
    reject_issue_title(
        "Fix(agents)::",
        "issue title must not use Conventional Commit style",
    )?;
    reject_issue_title(
        "Fix(agents):: break",
        "issue title must not use Conventional Commit style",
    )?;
    reject_issue_title(
        "Fix!::",
        "issue title must not use Conventional Commit style",
    )?;
    reject_issue_title(
        "Fix!:: break",
        "issue title must not use Conventional Commit style",
    )?;
    reject_issue_title(
        "Fix(agents)!:: break",
        "issue title must not use Conventional Commit style",
    )
}

#[test]
fn validator_cli_rejects_adjacent_colon_conventional_issue_title()
-> Result<(), Box<dyn std::error::Error>> {
    reject_issue_title(
        "Fix(agents):reject",
        "issue title must not use Conventional Commit style",
    )?;
    reject_issue_title(
        "Fix!:break",
        "issue title must not use Conventional Commit style",
    )?;
    reject_issue_title(
        "Fix:break",
        "issue title must not use Conventional Commit style",
    )
}

#[test]
fn validator_cli_rejects_issue_title_with_leading_whitespace()
-> Result<(), Box<dyn std::error::Error>> {
    reject_issue_title(
        " Reject negated sentinel reasoning evidence",
        "issue title must start with an uppercase descriptive title",
    )
}

#[test]
fn validator_cli_accepts_descriptive_issue_title() -> Result<(), Box<dyn std::error::Error>> {
    let output = validate_issue_title("Reject negated sentinel reasoning evidence")?;
    assert!(
        output.status.success(),
        "validator should accept descriptive issue titles\nstdout:\n{}\nstderr:\n{}",
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

fn validate_issue_title(title: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-issue-title", "--issue-title", title])
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

fn reject_issue_title(title: &str, expected: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = validate_issue_title(title)?;
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

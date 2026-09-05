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
    let output = validate_title("fix(workflow)!: enforce breaking workflow gate")?;
    assert!(
        output.status.success(),
        "validator should accept a Conventional Commit PR title with bang marker\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_enforces_scoped_pr_titles_and_reference_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    for title in [
        "feat(task): desc",
        "feat(task)!: desc",
        "test(ci): measure Rust 1.95 costs",
        "feat(task): support HTTP/2 on port 8080",
    ] {
        assert!(
            validate_title(title)?.status.success(),
            "validator should accept {title:?}"
        );
    }
    for title in [
        "feat: desc",
        "feat(): desc",
        "feat(task): desc (#900)",
        "feat(task): desc #900",
        "feat(task): desc (PR #926)",
        "feat(task): desc PR #926",
        "feat(task): desc issue #926",
    ] {
        assert!(
            !validate_title(title)?.status.success(),
            "validator should reject {title:?}"
        );
    }
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

#[test]
fn validator_cli_enforces_descriptive_issue_title_matrix()
-> Result<(), Box<dyn std::error::Error>> {
    for title in [
        "Reduce CI build time",
        "CI fails when cache restore times out",
        "Fix cache restoration after a runner restart",
        "Support HTTP/2 requests on port 8080",
        "Explain cache failures: retain the original error",
        "Cache-aware scheduling reduces redundant work",
    ] {
        assert!(
            validate_issue_title(title)?.status.success(),
            "validator should accept {title:?}"
        );
    }
    for title in [
        "CI: reduce build time",
        "ci: reduce build time",
        "Fix(task): reject invalid titles",
        "CI : reduce build time",
        "Fix (task) : reject invalid titles",
        "CI： reduce build time",
        "CI - reduce build time",
        "CI – reduce build time",
        "CI — reduce build time",
        "[CI] Reduce build time",
        "CI",
        "Fix",
        "lowercase prose",
        "\u{200b}Invisible prefix",
        "Multiline\ntitle",
    ] {
        assert!(
            !validate_issue_title(title)?.status.success(),
            "validator should reject {title:?}"
        );
    }
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

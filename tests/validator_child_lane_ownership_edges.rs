use std::process::{Command, Output};

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-child-lane-ownership", "--evidence-file"])
        .arg(&evidence_path)
        .output()?)
}

#[test]
fn validator_allows_absent_parent_authored_review_response()
-> Result<(), Box<dyn std::error::Error>> {
    for review_response in [
        "Review response: no parent-authored implementation commits; child fixed feedback",
        "Review response: no orchestrator-authored implementation commits; child fixed feedback",
        "Review response: no parent commit; child-authored commit def456 fixed feedback",
        "Review response: not parent-authored; child-authored commit def456 fixed feedback",
        "Review response: without parent-authored commits; child-authored commit def456 fixed feedback",
        "Review response: not orchestrator-authored; child-authored commit def456 fixed feedback",
        "Review response: without orchestrator-authored commits; child-authored commit def456 fixed feedback",
    ] {
        let output = run_ownership_validator(&format!(
            "Lane ownership: child-owned\n{review_response}\nMaintainer reassignment: none\n"
        ))?;

        assert!(
            output.status.success(),
            "validator should allow review-response evidence that explicitly denies parent/orchestrator-authored fixes `{review_response}`\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_contradictory_parent_authored_review_response()
-> Result<(), Box<dyn std::error::Error>> {
    for review_response in [
        "Review response: no parent-authored implementation commits; parent-authored review-response commit abc123 fixed feedback",
        "Review response: no parent-authored implementation commits; parent review-response commit abc123 fixed feedback",
        "Review response: no parent-authored implementation commits; parent commit abc123 fixed feedback",
        "Review response: no parent commit; parent commit abc123 fixed feedback",
        "Review response: no parent commit; parent review-response commit abc123 fixed feedback",
        "Review response: no parent commit; parent patched the child-owned branch with commit abc123",
        "Review response: orchestrator fixed the child-owned PR in commit abc123",
        "Review response: orchestrator review-response commit abc123 fixed feedback",
        "Review response: not parent-authored; parent-authored review-response commit abc123 fixed feedback",
        "Review response: without parent-authored commits; parent-authored review-response commit abc123 fixed feedback",
    ] {
        let output = run_ownership_validator(&format!(
            "Lane ownership: child-owned\n{review_response}\nMaintainer reassignment: none\n"
        ))?;

        assert!(
            !output.status.success(),
            "validator should reject contradictory review-response evidence `{review_response}`"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_comma_separated_parent_authored_review_response()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Review response: no parent-authored implementation commits, but parent-authored review-response commit abc123 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject parent-authored evidence after a same-line denial"
    );
    Ok(())
}

#[test]
fn validator_allows_empty_parent_authored_field_with_next_line_absence()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent-authored implementation commits:
none
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow empty parent-authored fields with absence values on the next line\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_parent_authored_fix_with_unrelated_no_value()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Review response: parent-authored implementation commit abc123; no tests run
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject parent-authored fixes even when the same line mentions unrelated no-values"
    );
    Ok(())
}

#[test]
fn validator_rejects_parent_authored_field_with_unrelated_no_value()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent-authored implementation commits: parent-authored implementation commit abc123; no tests run
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject parent-authored field values even when they mention unrelated no-values"
    );
    Ok(())
}

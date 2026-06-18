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
        "Review response: no parent pushed commit; child-authored commit def456 fixed feedback",
        "Parent implementation commits: no",
        "Parent commit: no",
        "Orchestrator implementation commits: no",
        "Orchestrator commit: no",
        "Review response: child-authored commit def456 fixed feedback; verified by parent",
        "Review response: child-authored commit def456 fixed feedback; verified by orchestrator",
        "Review response: child-authored commit def456 fixed feedback, verified by parent",
        "Review response: child-authored commit def456 fixed feedback and verified by orchestrator",
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
        "Review response: orchestrator implemented the fix in commit abc123",
        "Review response: orchestrator implementation commit abc123 fixed feedback",
        "Review response: orchestrator commit abc123 fixed feedback",
        "Review response: orchestrator fixed the child-owned PR in commit abc123",
        "Review response: orchestrator review-response commit abc123 fixed feedback",
        "Review response: review-response commit abc123 by parent",
        "Review response: review-response commit abc123 by the parent",
        "Review response: review-response commit abc123 by the parent fixed feedback",
        "Review response: implementation commit abc123 by orchestrator",
        "Review response: implementation commit abc123 by the orchestrator",
        "Review response: implementation commit abc123 by the orchestrator fixed feedback",
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
    for evidence in [
        r#"Lane ownership: child-owned
Parent-authored implementation commits:
none
Maintainer reassignment: none
"#,
        r#"Lane ownership: child-owned
Parent-authored implementation commits: none; child-authored commit def456
Maintainer reassignment: none
"#,
    ] {
        let output = run_ownership_validator(evidence)?;

        assert!(
            output.status.success(),
            "validator should allow parent-authored fields with absence evidence\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_ignores_non_affirmative_child_owned_mentions() -> Result<(), Box<dyn std::error::Error>>
{
    for evidence in [
        r#"Child-owned lane: no
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none
"#,
        r#"Child-owned lane: not child-owned
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none
"#,
        r#"Lane ownership: parent-owned (not child-owned)
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none
"#,
    ] {
        let output = run_ownership_validator(evidence)?;

        assert!(
            output.status.success(),
            "validator should require affirmative child-owned ownership\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_keeps_child_owned_label_that_negates_parent_owned()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned (not parent-owned)
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should keep affirmative child-owned ownership labels that negate parent ownership"
    );
    Ok(())
}

#[test]
fn validator_treats_child_owned_pr_as_child_owned_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    for evidence in [
        r#"Child-owned PR: #128
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none
"#,
        r#"Owner: child-owned
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none
"#,
        r#"Lane owner: child-owned
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none
"#,
    ] {
        let output = run_ownership_validator(evidence)?;

        assert!(
            !output.status.success(),
            "validator should treat child-owned PR/owner evidence as child-owned lane evidence"
        );
    }
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

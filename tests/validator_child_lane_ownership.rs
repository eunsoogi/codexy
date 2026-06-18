use std::process::Command;

#[test]
fn validator_rejects_parent_authored_child_lane_fix_without_reassignment()
-> Result<(), Box<dyn std::error::Error>> {
    for review_response in [
        "Review response: parent-authored implementation commit abc123 fixed feedback.",
        "Review response: parent authored implementation commit abc123 fixed feedback.",
        "Review response: orchestrator-authored implementation commit abc123 fixed feedback.",
        "Review response: orchestrator-authored review-response commit abc123 fixed feedback.",
        "Review response: parent patched the child-owned branch with commit abc123",
        "Review response: orchestrator patched the child-owned branch with commit abc123",
    ] {
        let temp = tempfile::tempdir()?;
        let evidence_path = temp.path().join("handoff.md");
        std::fs::write(
            &evidence_path,
            format!(
                "Lane ownership: child-owned\n\
                 PR: #128\n\
                 {review_response}\n\
                 Maintainer reassignment: none\n"
            ),
        )?;

        let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
            .args(["--check-child-lane-ownership", "--evidence-file"])
            .arg(&evidence_path)
            .output()?;

        assert!(
            !output.status.success(),
            "validator should reject `{review_response}` without explicit reassignment"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(
                "child-owned lane contains parent-authored implementation or review-response evidence"
            ),
            "stderr should explain the ownership violation, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_parent_authored_child_lane_fix_with_reassignment()
-> Result<(), Box<dyn std::error::Error>> {
    for phrase in [
        "Maintainer reassignment: explicit maintainer reassignment to parent in thread.",
        "Maintainer reassignment: explicit maintainer reassignment to the parent",
        "Maintainer reassignment: explicit maintainer reassignment to orchestrator",
        "Maintainer reassignment: explicit maintainer reassignment to the orchestrator",
        "Maintainer reassignment:\n- explicit maintainer reassignment to parent",
        "Maintainer reassignment: reassigns implementation ownership to the parent",
        "Maintainer reassignment: reassigns implementation ownership to the orchestrator",
    ] {
        let temp = tempfile::tempdir()?;
        let evidence_path = temp.path().join("handoff.md");
        std::fs::write(
            &evidence_path,
            format!(
                "Lane ownership: child-owned\n\
                 PR: #128\n\
                 Review response: parent-authored implementation commit abc123 fixed feedback.\n\
                 {phrase}\n"
            ),
        )?;

        let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
            .args(["--check-child-lane-ownership", "--evidence-file"])
            .arg(&evidence_path)
            .output()?;

        assert!(
            output.status.success(),
            "validator should allow explicit maintainer reassignment phrase `{phrase}`\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_negative_reassignment_phrasing() -> Result<(), Box<dyn std::error::Error>> {
    for phrase in [
        "Explicit maintainer reassignment: no",
        "Explicit maintainer reassignment: not provided",
        "Maintainer reassignment: missing",
        "Maintainer reassignment: absent",
        "Maintainer reassignment needed: explicit maintainer reassignment to parent",
        "Maintainer reassignment requested: explicit maintainer reassignment to parent",
        "Maintainer reassignment pending: explicit maintainer reassignment to parent",
        "Maintainer reassignment: pending explicit maintainer reassignment to parent",
        "Maintainer reassignment: pending explicit maintainer reassignment to orchestrator",
        "Maintainer reassignment: requested explicit maintainer reassignment to parent",
        "Maintainer reassignment: requested explicit maintainer reassignment to orchestrator",
        "Maintainer reassignment: needed explicit maintainer reassignment to parent",
        "Maintainer reassignment: needed explicit maintainer reassignment to orchestrator",
        "Maintainer reassignment: no explicit maintainer reassignment",
        "Maintainer reassignment: no explicit maintainer reassignment to parent",
        "Maintainer reassignment: there is no explicit maintainer reassignment to parent",
        "Maintainer reassignment: there is no explicit maintainer reassignment to orchestrator",
        "Maintainer reassignment: there was no explicit maintainer reassignment to parent",
        "Maintainer reassignment: there was no explicit maintainer reassignment to orchestrator",
        "Maintainer reassignment: missing explicit maintainer reassignment",
        "Maintainer reassignment: we need explicit maintainer reassignment to parent",
        "Maintainer reassignment: waiting for explicit maintainer reassignment to parent",
        "Maintainer reassignment: not reassigned to parent",
        "Maintainer reassignment: without explicit maintainer reassignment to parent",
        "Maintainer reassignment: explicit maintainer reassignment to parent not provided",
        "Maintainer reassignment: explicit maintainer reassignment to parent not provided.",
        "Maintainer reassignment: explicit maintainer reassignment to parent is missing",
        "Maintainer reassignment: explicit maintainer reassignment to parent not granted",
        "Maintainer reassignment: explicit maintainer reassignment to parent is not granted",
        "Maintainer reassignment: explicit maintainer reassignment to parent was not granted",
        "Maintainer reassignment: explicit maintainer reassignment to parent has not been granted",
        "Maintainer reassignment: explicit maintainer reassignment to parent was denied",
        "Maintainer reassignment: explicit maintainer reassignment to parent was rejected",
        "Maintainer reassignment: explicit maintainer reassignment to parent requested",
        "Maintainer reassignment: explicit maintainer reassignment to the parent is missing",
        "Maintainer reassignment: reassigns implementation ownership to the parent was not granted",
    ] {
        let temp = tempfile::tempdir()?;
        let evidence_path = temp.path().join("handoff.md");
        std::fs::write(
            &evidence_path,
            format!(
                "Lane ownership: child-owned\n\
                 Review response: parent-authored implementation commit abc123 fixed feedback.\n\
                 {phrase}\n"
            ),
        )?;

        let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
            .args(["--check-child-lane-ownership", "--evidence-file"])
            .arg(&evidence_path)
            .output()?;

        assert!(
            !output.status.success(),
            "validator should reject negative reassignment phrase `{phrase}`"
        );
    }
    Ok(())
}

#[test]
fn validator_allows_absent_parent_authored_fix_evidence() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(
        &evidence_path,
        r#"Lane ownership: child-owned
Parent-authored implementation commits: no
Maintainer reassignment: none
"#,
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-child-lane-ownership", "--evidence-file"])
        .arg(&evidence_path)
        .output()?;

    assert!(
        output.status.success(),
        "validator should not flag explicitly absent parent-authored fixes\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_child_owned_lane_without_parent_authored_fixes()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(
        &evidence_path,
        r#"Lane ownership: child-owned
PR: #129
Review response: child-authored commit def456 fixed feedback.
Maintainer reassignment: none
"#,
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-child-lane-ownership", "--evidence-file"])
        .arg(&evidence_path)
        .output()?;

    assert!(
        output.status.success(),
        "validator should allow child-authored child lane evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

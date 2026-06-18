use std::process::Command;

#[test]
fn validator_rejects_parent_authored_child_lane_fix_without_reassignment()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(
        &evidence_path,
        r#"Lane ownership: child-owned
PR: #128
Review response: parent-authored implementation commit abc123 fixed feedback.
Maintainer reassignment: none
"#,
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-child-lane-ownership", "--evidence-file"])
        .arg(&evidence_path)
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject parent-authored child lane fixes without explicit reassignment"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(
            "child-owned lane contains parent-authored implementation or review-response evidence"
        ),
        "stderr should explain the ownership violation, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_parent_authored_child_lane_fix_with_reassignment()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(
        &evidence_path,
        r#"Lane ownership: child-owned
PR: #128
Review response: parent-authored implementation commit abc123 fixed feedback.
Maintainer reassignment: explicit maintainer reassignment to parent in thread.
"#,
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-child-lane-ownership", "--evidence-file"])
        .arg(&evidence_path)
        .output()?;

    assert!(
        output.status.success(),
        "validator should allow explicit maintainer reassignment\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_negative_reassignment_phrasing() -> Result<(), Box<dyn std::error::Error>> {
    for phrase in [
        "Explicit maintainer reassignment: no",
        "Explicit maintainer reassignment: not provided",
        "Maintainer reassignment: missing",
        "Maintainer reassignment: absent",
        "Maintainer reassignment: no explicit maintainer reassignment",
        "Maintainer reassignment: missing explicit maintainer reassignment",
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
fn validator_rejects_parent_authored_fix_with_unrelated_no_value()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(
        &evidence_path,
        r#"Lane ownership: child-owned
Review response: parent-authored implementation commit abc123; no tests run
Maintainer reassignment: none
"#,
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-child-lane-ownership", "--evidence-file"])
        .arg(&evidence_path)
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject parent-authored fixes even when the same line mentions unrelated no-values"
    );
    Ok(())
}

#[test]
fn validator_rejects_parent_authored_field_with_unrelated_no_value()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(
        &evidence_path,
        r#"Lane ownership: child-owned
Parent-authored implementation commits: parent-authored implementation commit abc123; no tests run
Maintainer reassignment: none
"#,
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-child-lane-ownership", "--evidence-file"])
        .arg(&evidence_path)
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject parent-authored field values even when they mention unrelated no-values"
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

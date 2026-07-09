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
fn validator_rejects_no_disposition_proof_for_old_owner() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-old found for issue #269.
Old owner disposition: no stopped, unusable, or superseded evidence exists for existing owner thread thread-old issue #269.
Thread creation: created replacement child thread thread-new for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject no-proof old-owner disposition wording"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("old owner"),
        "stderr should name missing old-owner disposition evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_none_disposition_proof_for_old_owner() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-old found for issue #269.
Old owner disposition: none of the stopped, unusable, or superseded evidence exists for existing owner thread thread-old issue #269.
Thread creation: created replacement child thread thread-new for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject none-proof old-owner disposition wording"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("old owner"),
        "stderr should name missing old-owner disposition evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_no_longer_disposition_claims() -> Result<(), Box<dyn std::error::Error>> {
    for disposition in [
        "existing owner thread thread-old was no longer stopped for issue #269",
        "existing owner thread thread-old was no longer unusable for issue #269",
        "existing owner thread thread-old was no longer superseded for issue #269",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-old found for issue #269.
Old owner disposition: {disposition}.
Thread creation: created replacement child thread thread-new for issue #269.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should reject no-longer disposition claim `{disposition}`"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("old owner"),
            "stderr should name missing old-owner disposition evidence, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_accepts_affirmative_old_owner_disposition() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-old found for issue #269.
Old owner disposition: existing owner thread thread-old was stopped as unusable and superseded for issue #269.
Thread creation: created replacement child thread thread-new for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should keep accepting affirmative old-owner disposition\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_accepts_no_longer_active_old_owner_disposition()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-old found for issue #269.
Old owner disposition: existing owner thread thread-old no longer active, was stopped as unusable and superseded for issue #269.
Thread creation: created replacement child thread thread-new for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept no-longer-active affirmative disposition\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_accepts_not_blocked_because_old_owner_was_stopped()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-old found for issue #269.
Old owner disposition: existing owner thread thread-old not blocked because thread-old was stopped for issue #269.
Thread creation: created replacement child thread thread-new for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not let unrelated not-blocked wording negate stopped evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_accepts_affirmative_disposition_after_but_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    for disposition in [
        "no owner disposition existed earlier but existing owner thread thread-old was stopped as unusable for issue #269",
        "none of the first checks superseded the owner but existing owner thread thread-old was unusable for issue #269",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-old found for issue #269.
Old owner disposition: {disposition}.
Thread creation: created replacement child thread thread-new for issue #269.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            output.status.success(),
            "validator should accept affirmative disposition after but boundary for `{disposition}`\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

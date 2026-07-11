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
fn validator_rejects_contracted_negated_old_owner_dispositions()
-> Result<(), Box<dyn std::error::Error>> {
    for disposition in [
        "existing owner thread thread-old can't be stopped for issue #269",
        "existing owner thread thread-old hasn't stopped for issue #269",
        "existing owner thread thread-old doesn't seem stopped for issue #269",
        "existing owner thread thread-old couldn't be stopped for issue #269",
        "existing owner thread thread-old hasnt stopped for issue #269",
        "existing owner thread thread-old doesnt seem stopped for issue #269",
        "existing owner thread thread-old couldnt be stopped for issue #269",
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
            "validator should reject contracted negated disposition `{disposition}`"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("old owner was stopped, unusable, or explicitly superseded"),
            "stderr should name missing old-owner disposition evidence, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

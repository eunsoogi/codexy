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
fn validator_rejects_unchecked_reassignment_checklist() -> Result<(), Box<dyn std::error::Error>> {
    let phrase = "- [ ] Maintainer reassignment: explicit maintainer reassignment to parent";
    let output = run_ownership_validator(&format!(
        "Lane ownership: child-owned\n\
         Review response: parent-authored implementation commit abc123 fixed feedback\n\
         {phrase}\n"
    ))?;

    assert!(
        !output.status.success(),
        "validator should reject unchecked reassignment checklist evidence"
    );
    Ok(())
}

#[test]
fn validator_rejects_denied_reassignment_value_prefixes() -> Result<(), Box<dyn std::error::Error>>
{
    for phrase in [
        "Maintainer reassignment: denied explicit maintainer reassignment to parent",
        "Maintainer reassignment: rejected explicit maintainer reassignment to parent",
    ] {
        let output = run_ownership_validator(&format!(
            "Lane ownership: child-owned\n\
             Review response: parent-authored implementation commit abc123 fixed feedback\n\
             {phrase}\n"
        ))?;

        assert!(
            !output.status.success(),
            "validator should reject denied reassignment value prefix `{phrase}`"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_contextual_missing_reassignment_text() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        "Lane ownership: child-owned\n\
         Review response: parent-authored implementation commit abc123 fixed feedback\n\
         Maintainer reassignment: explicit maintainer reassignment to parent is missing from the handoff\n",
    )?;

    assert!(
        !output.status.success(),
        "validator should reject contextual missing reassignment evidence"
    );
    Ok(())
}

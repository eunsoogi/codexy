use std::process::{Command, Output};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn completion_handoff_rejects_negated_no_remediation_claims() -> TestResult {
    for handoff in [
        "Not all touched files were already below 250 LOC and no LOC remediation was needed. --check-touched-loc passed.",
        "All touched files were not already below 250 LOC and no LOC remediation was needed. --check-touched-loc passed.",
        "LOC remediation: not applicable because no touched file was below 250 LOC. --check-touched-loc passed.",
        "LOC remediation: not applicable; no touched file stayed within the LOC limit. --check-touched-loc passed.",
        "LOC remediation: not applicable because it is false that no touched file exceeded 250 LOC. --check-touched-loc passed.",
        "Fallback lane had all touched files below 250 LOC and no LOC remediation was needed. --check-touched-loc passed.",
        "Previous lane had all touched files below 250 LOC and no LOC remediation was needed. --check-touched-loc passed.",
    ] {
        let output = validate(handoff)?;
        assert!(!output.status.success(), "unexpectedly accepted: {handoff}");
        assert!(stderr(&output).contains("LOC remediation evidence must name"));
    }
    Ok(())
}

#[test]
fn completion_handoff_accepts_affirmative_no_remediation_claim() -> TestResult {
    let output = validate(
        "LOC remediation: not applicable because no touched file exceeded 250 LOC. --check-touched-loc passed.",
    )?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn completion_handoff_accepts_closed_s_ending_quote_before_marker() -> TestResult {
    let output = validate(
        "LOC remediation: 'approved policies' before helper extraction moved rules into src/parser_rules.rs. --check-touched-loc passed.",
    )?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

fn validate(handoff: &str) -> TestResult<Output> {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(
        &state_path,
        r#"{"number":360,"state":"CLOSED","mergeStateStatus":"CLEAN","isDraft":false,"headRefOid":"0123456789012345678901234567890123456789"}"#,
    )?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--check-completion-handoff",
            "--handoff-file",
            handoff_path.to_str().ok_or("handoff path")?,
            "--pr-state-file",
            state_path.to_str().ok_or("state path")?,
        ])
        .output()?)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

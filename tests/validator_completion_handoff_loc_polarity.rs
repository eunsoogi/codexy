use std::process::{Command, Output};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn completion_handoff_models_without_attachment_locally() -> TestResult {
    for handoff in [
        "LOC remediation: without altering behavior helper extraction moved parser rules into src/parser_rules.rs. --check-touched-loc passed.",
        "LOC remediation: without any behavior changes module splitting moved parser rules into src/parser_rules.rs. --check-touched-loc passed.",
        "LOC remediation: helper extraction did not change behavior and moved parser rules into src/parser_rules.rs. --check-touched-loc passed.",
    ] {
        assert!(
            validate(handoff)?.status.success(),
            "unexpectedly rejected: {handoff}"
        );
    }
    for handoff in [
        "LOC remediation: rules passed without helper extraction in src/parser_rules.rs. --check-touched-loc passed.",
        "LOC remediation: helper extraction without altering behavior not performed in src/parser_rules.rs. --check-touched-loc passed.",
        "LOC remediation: helper extraction did not occur in src/parser_rules.rs. --check-touched-loc passed.",
    ] {
        assert!(
            !validate(handoff)?.status.success(),
            "unexpectedly accepted: {handoff}"
        );
    }
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

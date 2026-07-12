use std::process::{Command, Output};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn completion_handoff_rejects_formatting_only_loc_evidence() -> TestResult {
    let output =
        validate("LOC remediation: blank-line deletion only. --check-touched-loc passed.")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("formatting-only LOC remediation"));
    Ok(())
}

#[test]
fn completion_handoff_rejects_quoted_or_negated_cosmetic_claims() -> TestResult {
    for handoff in [
        "LOC remediation: \"blank-line deletion\" is not acceptable. --check-touched-loc passed.",
        "LOC remediation: not formatting-only. --check-touched-loc passed.",
    ] {
        let output = validate(handoff)?;

        assert!(!output.status.success());
        assert!(stderr(&output).contains("formatting-only LOC remediation"));
    }
    Ok(())
}

#[test]
fn completion_handoff_accepts_structural_loc_evidence() -> TestResult {
    let output = validate(
        "LOC remediation: helper extraction moved parser rules into src/parser_rules.rs. --check-touched-loc passed.",
    )?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn completion_handoff_accepts_touched_loc_structural_evidence() -> TestResult {
    let output = validate(
        "Touched LOC: helper extraction moved parser rules into src/parser_rules.rs. --check-touched-loc passed.",
    )?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn completion_handoff_rejects_quoted_or_negated_structural_markers() -> TestResult {
    for handoff in [
        "LOC remediation: quoted evidence \"helper extraction\". --check-touched-loc passed.",
        "LOC remediation: not helper extraction. --check-touched-loc passed.",
    ] {
        let output = validate(handoff)?;

        assert!(!output.status.success());
        assert!(stderr(&output).contains("LOC remediation evidence must name"));
    }
    Ok(())
}

#[test]
fn completion_handoff_requires_structural_class_and_file_boundary_in_one_clause() -> TestResult {
    for handoff in [
        "LOC remediation: helper extraction performed. Touched file: src/parser_rules.rs. --check-touched-loc passed.",
        "Previous LOC remediation: helper extraction moved rules into src/old_rules.rs. --check-touched-loc passed.",
    ] {
        let output = validate(handoff)?;

        assert!(!output.status.success());
    }
    Ok(())
}

#[test]
fn completion_handoff_rejects_terminal_reviewer_false_positives() -> TestResult {
    for handoff in [
        "LOC remediation: we did not perform helper extraction in src/parser_rules.rs. --check-touched-loc passed.",
        "LOC remediation: responsibility separation occurred. For example, helper extraction moved parser rules into src/example_rules.rs. --check-touched-loc passed.",
        "LOC remediation: module splitting was considered. Later unrelated file: src/unrelated.rs. --check-touched-loc passed.",
    ] {
        let output = validate(handoff)?;

        assert!(!output.status.success(), "unexpectedly accepted: {handoff}");
        assert!(stderr(&output).contains("LOC remediation evidence must name"));
    }
    Ok(())
}

#[test]
fn completion_handoff_rejects_all_terminal_parser_boundaries() -> TestResult {
    for handoff in [
        "LOC remediation: we did not actually plan or intend to perform helper extraction in src/parser_rules.rs. --check-touched-loc passed.",
        "LOC remediation: reviewer text said \"the team used helper extraction in src/example_rules.rs\". --check-touched-loc passed.",
        "LOC remediation: for example, helper extraction moved parser rules into src/example_rules.rs. --check-touched-loc passed.",
        "LOC remediation: module splitting was considered for src/parser_rules.rs. --check-touched-loc passed.",
    ] {
        let output = validate(handoff)?;

        assert!(!output.status.success(), "unexpectedly accepted: {handoff}");
        assert!(stderr(&output).contains("LOC remediation evidence must name"));
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

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

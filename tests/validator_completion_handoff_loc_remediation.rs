use std::process::Output;

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
        "LOC remediation: reviewer text said “the team used helper extraction in src/example_rules.rs”. --check-touched-loc passed.",
        "LOC remediation: reviewer text said 'the team used helper extraction in src/example_rules.rs'. --check-touched-loc passed.",
        "LOC remediation: reviewer text said `the team used helper extraction in src/example_rules.rs`. --check-touched-loc passed.",
        "LOC remediation: reviewer text said 'reviewers' helper extraction moved rules into src/example_rules.rs'. --check-touched-loc passed.",
        "LOC remediation: reviewer text said 'reviewers' helper extraction moved rules into src/example_rules.rs. --check-touched-loc passed.",
        "LOC remediation: reviewer text said 'reviewers' reported helper extraction moved rules into src/example_rules.rs. --check-touched-loc passed.",
        "LOC remediation: for example, helper extraction moved parser rules into src/example_rules.rs. --check-touched-loc passed.",
        "LOC remediation: module splitting was considered for src/parser_rules.rs. --check-touched-loc passed.",
    ] {
        let output = validate(handoff)?;

        assert!(!output.status.success(), "unexpectedly accepted: {handoff}");
        assert!(stderr(&output).contains("LOC remediation evidence must name"));
    }
    Ok(())
}

#[test]
fn completion_handoff_accepts_closed_typographic_quote_before_evidence() -> TestResult {
    let output = validate(
        "LOC remediation: reviewer cited “helper extraction” before helper extraction moved rules into src/parser_rules.rs. --check-touched-loc passed.",
    )?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn completion_handoff_accepts_markdown_bullet_evidence() -> TestResult {
    for prefix in ["- ", "+ ", "*   "] {
        let output = validate(&format!(
            "{prefix}LOC remediation: helper extraction moved parser rules into src/parser_rules.rs. --check-touched-loc passed."
        ))?;
        assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    }
    Ok(())
}

#[test]
fn completion_handoff_accepts_plural_possessive_evidence() -> TestResult {
    let output = validate(
        "LOC remediation: reviewers' helper extraction moved rules into src/parser_rules.rs. --check-touched-loc passed.",
    )?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn completion_handoff_accepts_truthful_no_remediation_needed() -> TestResult {
    let output = validate(
        "All touched files were already below 250 LOC and no LOC remediation was needed. --check-touched-loc passed.",
    )?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

fn completion_handoff_rejects(handoff: &str) -> TestResult {
    let output = validate(handoff)?;

    assert!(!output.status.success(), "unexpectedly accepted: {handoff}");
    assert!(stderr(&output).contains("LOC remediation evidence must name"));
    Ok(())
}

#[test]
fn completion_handoff_rejects_postposed_negation() -> TestResult {
    completion_handoff_rejects(
        "LOC remediation: helper extraction was not performed in src/parser_rules.rs. --check-touched-loc passed.",
    )
}

#[test]
fn completion_handoff_accepts_postposed_negation_beyond_six_words() -> TestResult {
    let output = validate(
        "LOC remediation: helper extraction moved parser rules into src/parser_rules.rs while preserving every tested behavior and not changing behavior. --check-touched-loc passed.",
    )?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn completion_handoff_accepts_without_changing_behavior() -> TestResult {
    let output = validate(
        "LOC remediation: helper extraction moved parser rules into src/parser_rules.rs without changing behavior. --check-touched-loc passed.",
    )?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn completion_handoff_rejects_negation_after_without_changing_behavior() -> TestResult {
    for outcome in ["not performed", "no extraction occurred"] {
        completion_handoff_rejects(&format!(
            "LOC remediation: helper extraction without changing behavior {outcome} in src/parser_rules.rs. --check-touched-loc passed.",
        ))?;
    }
    Ok(())
}

#[test]
fn completion_handoff_rejects_local_without_marker() -> TestResult {
    completion_handoff_rejects(
        "LOC remediation: rules passed without helper extraction in src/parser_rules.rs. --check-touched-loc passed.",
    )
}

#[test]
fn completion_handoff_rejects_postposed_example_only() -> TestResult {
    completion_handoff_rejects(
        "LOC remediation: helper extraction moved rules into src/parser_rules.rs as an example only. --check-touched-loc passed.",
    )
}

#[test]
fn completion_handoff_rejects_foreign_lane_structural_claim() -> TestResult {
    completion_handoff_rejects(
        "LOC remediation: fallback lane used helper extraction in src/fallback_rules.rs. --check-touched-loc passed.",
    )
}

#[test]
fn completion_handoff_rejects_foreign_fallback_lane_variants() -> TestResult {
    for lane in ["fallback-lane", "fallback child lane"] {
        completion_handoff_rejects(&format!(
            "LOC remediation: {lane} used helper extraction in src/fallback_rules.rs. --check-touched-loc passed.",
        ))?;
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
    crate::support::validator_completion_handoff_files(&handoff_path, &state_path)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

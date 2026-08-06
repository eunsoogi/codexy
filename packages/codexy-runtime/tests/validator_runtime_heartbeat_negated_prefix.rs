use std::{fs, path::Path};

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const DISCOVERY_REQUIREMENT: &str =
    "MUST search the callable tool surface for `automation_update`";

fn validate_discovery_clause(replacement: &str) -> TestResult<std::process::Output> {
    let fixture = support::instruction_policy_fixture(Path::new(
        "skills/codex-orchestration/references/runtime-heartbeats.md",
    ))?;
    let path = fixture.path();
    let original = fs::read_to_string(&path)?;
    let updated = original.replace(DISCOVERY_REQUIREMENT, replacement);
    assert_ne!(
        updated, original,
        "fixture discovery clause was not replaced"
    );
    fs::write(path, updated)?;
    support::validator_instruction_policy_file(path)
}

#[test]
fn validator_rejects_must_not_prefix_for_discovery_clause() -> TestResult {
    let output = validate_discovery_clause(
        "MUST NOT search the callable tool surface for `automation_update`",
    )?;
    assert!(
        !output.status.success(),
        "validator accepted a MUST NOT discovery clause as required policy"
    );
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn validator_rejects_soft_modal_prefix_for_discovery_clause() -> TestResult {
    for replacement in [
        "MUST decide whether the owner MAY search the callable tool surface for `automation_update`",
        "MUST decide whether the owner MAY choose to search the callable tool surface for `automation_update`",
    ] {
        let output = validate_discovery_clause(replacement)?;
        assert!(
            !output.status.success(),
            "validator accepted optional discovery action {replacement:?} as required policy"
        );
        assert!(
            support::stderr(&output).contains("runtime heartbeat contract"),
            "validator rejected optional discovery action {replacement:?} for an unexpected reason: {}",
            support::stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_accepts_mandatory_modal_prefix_for_discovery_clause() -> TestResult {
    let replacement = "MUST\nsearch the callable tool surface for `automation_update`";
    let output = validate_discovery_clause(replacement)?;
    assert!(
        output.status.success(),
        "validator rejected mandatory discovery action {replacement:?}: {}",
        support::stderr(&output)
    );
    Ok(())
}

#[test]
fn validator_accepts_unnegated_discovery_clause() -> TestResult {
    let fixture = support::instruction_policy_fixture(Path::new(
        "skills/codex-orchestration/references/runtime-heartbeats.md",
    ))?;
    let output = support::validator_instruction_policy_file(fixture.path())?;
    assert!(
        output.status.success(),
        "validator rejected the unnegated discovery clause: {}",
        support::stderr(&output)
    );
    Ok(())
}

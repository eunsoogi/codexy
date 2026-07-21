use std::fs;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const DISCOVERY_REQUIREMENT: &str =
    "MUST\nsearch the callable tool surface for `automation_update`";

fn validate_discovery_clause(replacement: &str) -> TestResult<std::process::Output> {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = plugin_root.join("skills/codex-orchestration/references/runtime-heartbeats.md");
    let original = fs::read_to_string(&path)?;
    let updated = original.replace(DISCOVERY_REQUIREMENT, replacement);
    assert_ne!(
        updated, original,
        "fixture discovery clause was not replaced"
    );
    fs::write(path, updated)?;
    support::validator_instruction_policy(&plugin_root)
}

#[test]
fn validator_rejects_must_not_prefix_for_discovery_clause() -> TestResult {
    let output = validate_discovery_clause(
        "MUST NOT\nsearch the callable tool surface for `automation_update`",
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
    for replacement in [
        "the owner MUST search the callable tool surface for `automation_update`",
        "the owner MUST choose to search the callable tool surface for `automation_update`",
    ] {
        let output = validate_discovery_clause(replacement)?;
        assert!(
            output.status.success(),
            "validator rejected mandatory discovery action {replacement:?}: {}",
            support::stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_accepts_unnegated_discovery_clause() -> TestResult {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let output = support::validator_instruction_policy(&plugin_root)?;
    assert!(
        output.status.success(),
        "validator rejected the unnegated discovery clause: {}",
        support::stderr(&output)
    );
    Ok(())
}

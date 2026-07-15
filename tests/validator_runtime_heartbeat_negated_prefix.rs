use std::fs;

mod support;

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
    support::validator(&plugin_root, "--check")
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
fn validator_accepts_unnegated_discovery_clause() -> TestResult {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let output = support::validator(&plugin_root, "--check")?;
    assert!(
        output.status.success(),
        "validator rejected the unnegated discovery clause: {}",
        support::stderr(&output)
    );
    Ok(())
}

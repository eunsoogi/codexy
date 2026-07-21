use std::fs;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const AUTOMATION_UPDATE: &str = "automation_update";

fn validate_discovery_name(replacement: &str) -> TestResult<std::process::Output> {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = plugin_root.join("skills/codex-orchestration/references/runtime-heartbeats.md");
    let original = fs::read_to_string(&path)?;
    let updated = original.replace(AUTOMATION_UPDATE, replacement);
    assert_ne!(updated, original, "fixture discovery name was not replaced");
    fs::write(path, updated)?;
    support::validator_instruction_policy(&plugin_root)
}

#[test]
fn validator_rejects_discovery_name_without_identifier_underscore() -> TestResult {
    let output = validate_discovery_name("automationupdate")?;
    assert!(
        !output.status.success(),
        "validator accepted automationupdate as automation_update"
    );
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn validator_accepts_emphasized_discovery_name_with_identifier_underscore() -> TestResult {
    let output = validate_discovery_name("automation_*update*")?;
    assert!(
        output.status.success(),
        "validator rejected emphasized automation_update: {}",
        support::stderr(&output)
    );
    Ok(())
}

#[test]
fn validator_accepts_underscore_emphasized_identifier() -> TestResult {
    let output = validate_discovery_name("_automation_update_")?;
    assert!(
        output.status.success(),
        "validator rejected underscore-emphasized automation_update: {}",
        support::stderr(&output)
    );
    Ok(())
}

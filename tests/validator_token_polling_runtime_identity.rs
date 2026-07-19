use std::fs;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const HEARTBEAT_IDENTITY: &str = "heartbeat route MUST bind";
const PROCESS_IDENTITY: &str = "separate process-backed monitor MUST bind";

fn validate_polling_policy(removed_identity: Option<&str>) -> TestResult<std::process::Output> {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = plugin_root.join("skills/token-efficient-orchestration/SKILL.md");
    let original = fs::read_to_string(&path)?;
    let updated = removed_identity.map_or_else(
        || original.clone(),
        |identity| original.replace(identity, "removed runtime monitor identity"),
    );
    if removed_identity.is_some() {
        assert_ne!(
            updated, original,
            "fixture polling identity was not replaced"
        );
    }
    fs::write(path, updated)?;
    support::validator_instruction_policy(&plugin_root)
}

#[test]
fn validator_rejects_heartbeat_identity_without_process_monitor_proof() -> TestResult {
    let output = validate_polling_policy(Some(PROCESS_IDENTITY))?;
    assert!(
        !output.status.success(),
        "validator accepted heartbeat identity without process-monitor proof"
    );
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn validator_rejects_process_identity_without_heartbeat_alternative() -> TestResult {
    let output = validate_polling_policy(Some(HEARTBEAT_IDENTITY))?;
    assert!(
        !output.status.success(),
        "validator accepted process identity without heartbeat alternative"
    );
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn validator_accepts_complete_polling_runtime_identities() -> TestResult {
    let output = validate_polling_policy(None)?;
    assert!(
        output.status.success(),
        "validator rejected complete polling identities: {}",
        support::stderr(&output)
    );
    Ok(())
}

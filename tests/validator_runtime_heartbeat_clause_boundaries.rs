use std::fs;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const HEARTBEAT_KIND: &str = "kind=heartbeat";

fn validate_heartbeat_kind(replacement: &str) -> TestResult<std::process::Output> {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = plugin_root.join("skills/codex-orchestration/references/runtime-heartbeats.md");
    let original = fs::read_to_string(&path)?;
    let updated = original.replace(HEARTBEAT_KIND, replacement);
    assert_ne!(updated, original, "fixture heartbeat kind was not replaced");
    fs::write(path, updated)?;
    support::validator_instruction_policy(&plugin_root)
}

#[test]
fn validator_rejects_longer_heartbeat_kind_token() -> TestResult {
    let output = validate_heartbeat_kind("kind=heartbeat_optional")?;
    assert!(
        !output.status.success(),
        "validator accepted kind=heartbeat_optional as kind=heartbeat"
    );
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn validator_rejects_hyphen_extended_heartbeat_kind_token() -> TestResult {
    let output = validate_heartbeat_kind("kind=heartbeat-optional")?;
    assert!(
        !output.status.success(),
        "validator accepted kind=heartbeat-optional as kind=heartbeat"
    );
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn validator_accepts_punctuation_delimited_heartbeat_kind() -> TestResult {
    let output = validate_heartbeat_kind("kind=heartbeat;")?;
    assert!(
        output.status.success(),
        "validator rejected punctuation-delimited kind=heartbeat: {}",
        support::stderr(&output)
    );
    Ok(())
}

#[test]
fn validator_accepts_markdown_emphasized_heartbeat_kind() -> TestResult {
    let output = validate_heartbeat_kind("kind=*heartbeat*")?;
    assert!(
        output.status.success(),
        "validator rejected Markdown-emphasized kind=heartbeat: {}",
        support::stderr(&output)
    );
    Ok(())
}

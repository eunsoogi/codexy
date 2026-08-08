use std::{fs, path::Path};

use crate::support;
#[path = "validator_runtime_heartbeat/lifecycle.rs"]
mod lifecycle;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn validate_section(section: &str) -> TestResult<std::process::Output> {
    let fixture = support::instruction_policy_fixture(Path::new(
        "skills/codex-orchestration/references/runtime-heartbeats.md",
    ))?;
    let path = fixture.path();
    let original = fs::read_to_string(&path)?;
    fs::write(
        &path,
        lifecycle::replace_sentence(&original, &format!("\n\n{section}")),
    )?;
    support::validator_instruction_policy_file(path)
}

#[test]
fn conditional_parent_heading_does_not_supply_nested_policy() -> TestResult {
    let sentence = lifecycle::SENTENCE;
    for headings in [
        "## If available\n### Goal lifecycle",
        "If available\n------------\n### Goal lifecycle",
    ] {
        let output = validate_section(&format!("{headings}\n{sentence}"))?;
        assert!(!output.status.success(), "accepted headings {headings:?}");
        assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    }
    Ok(())
}

#[test]
fn nonconditional_heading_state_and_sibling_reset_remain_valid() -> TestResult {
    let sentence = lifecycle::SENTENCE;
    for section in [
        format!("## Current policy\n### Goal lifecycle\n{sentence}"),
        format!(
            "## If available\n### Optional route\nNo contract here.\n\n## Current policy\n### Goal lifecycle\n{sentence}"
        ),
    ] {
        let output = validate_section(&section)?;
        assert!(output.status.success(), "{}", support::stderr(&output));
    }
    Ok(())
}

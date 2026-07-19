use std::fs;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CLAUSE: &str = "MUST NOT retain or recreate an execution goal solely to preserve a successfully registered heartbeat";

fn validate_section(section: &str) -> TestResult<std::process::Output> {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = plugin_root.join("skills/codex-orchestration/references/runtime-heartbeats.md");
    let original = fs::read_to_string(&path)?;
    let sentence = format!(
        "The owner {CLAUSE}; it MAY keep a goal only while an implementation obligation remains."
    );
    fs::write(
        &path,
        original.replace(&sentence, &format!("\n\n{section}")),
    )?;
    support::validator_instruction_policy(&plugin_root)
}

#[test]
fn conditional_parent_heading_does_not_supply_nested_policy() -> TestResult {
    let sentence = format!(
        "The owner {CLAUSE}; it MAY keep a goal only while an implementation obligation remains."
    );
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
    let sentence = format!(
        "The owner {CLAUSE}; it MAY keep a goal only while an implementation obligation remains."
    );
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

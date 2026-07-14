use std::fs;

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CLAUSE: &str = "MUST NOT retain or recreate an execution goal solely to preserve a successfully registered heartbeat";

fn validate_replacement(replacement: &str) -> TestResult<std::process::Output> {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = plugin_root.join("skills/codex-orchestration/references/runtime-heartbeats.md");
    let original = fs::read_to_string(&path)?;
    fs::write(&path, original.replace(CLAUSE, replacement))?;
    support::validator(&plugin_root, "--check")
}

#[test]
fn numbered_historical_heading_does_not_supply_policy() -> TestResult {
    let output = validate_replacement(&format!(
        "removed heartbeat policy\n\n## 1. Historical Example\nThis policy was retired. {CLAUSE}."
    ))?;
    assert!(!output.status.success());
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn fenced_pseudo_heading_does_not_reset_historical_policy() -> TestResult {
    let output = validate_replacement(&format!(
        "removed heartbeat policy\n\n## Historical Example\n```markdown\n## Current Policy\n```\nThis policy was retired. {CLAUSE}."
    ))?;
    assert!(!output.status.success());
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn fenced_current_clause_does_not_supply_policy() -> TestResult {
    let output = validate_replacement(&format!(
        "removed heartbeat policy\n\n## Current Policy\n```text\n{CLAUSE}.\n```"
    ))?;
    assert!(!output.status.success());
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn setext_current_heading_resets_historical_policy() -> TestResult {
    let output = validate_replacement(&format!(
        "removed heartbeat policy\n\n## Historical Example\nThis policy was retired.\n\nCurrent Policy\n--------------\n{CLAUSE}."
    ))?;
    assert!(
        output.status.success(),
        "validator ignored active policy after a Setext heading: {}",
        support::stderr(&output)
    );
    Ok(())
}

#[test]
fn indented_atx_heading_does_not_reset_historical_policy() -> TestResult {
    let output = validate_replacement(&format!(
        "removed heartbeat policy\n\n## Historical Example\n    ## Current Policy\nThis policy was retired. {CLAUSE}."
    ))?;
    assert!(!output.status.success());
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn indented_setext_underline_does_not_reset_historical_policy() -> TestResult {
    let output = validate_replacement(&format!(
        "removed heartbeat policy\n\n## Historical Example\nCurrent Policy\n    --------------\nThis policy was retired. {CLAUSE}."
    ))?;
    assert!(!output.status.success());
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn indented_fence_markers_do_not_hide_live_policy() -> TestResult {
    let output = validate_replacement(&format!(
        "removed heartbeat policy\n\n## Current Policy\n    ```text\n{CLAUSE}.\n    ```\n"
    ))?;
    assert!(
        output.status.success(),
        "validator hid live policy behind indented fence markers: {}",
        support::stderr(&output)
    );
    Ok(())
}

#[test]
fn tab_indented_heading_does_not_reset_historical_policy() -> TestResult {
    let output = validate_replacement(&format!(
        "removed heartbeat policy\n\n## Historical Example\n\t## Current Policy\nThis policy was retired. {CLAUSE}."
    ))?;
    assert!(!output.status.success());
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn punctuation_before_weakening_suffix_does_not_supply_policy() -> TestResult {
    for suffix in [
        ", except during maintenance",
        "; unless explicitly approved",
    ] {
        let output = validate_replacement(&format!("{CLAUSE}{suffix}."))?;
        assert!(
            !output.status.success(),
            "validator accepted weakened clause ending in {suffix:?}"
        );
        assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    }
    Ok(())
}

#[test]
fn safe_punctuation_after_required_clause_remains_valid() -> TestResult {
    for suffix in [", and record the result", "; the result remains auditable"] {
        let output = validate_replacement(&format!("{CLAUSE}{suffix}."))?;
        assert!(
            output.status.success(),
            "validator rejected safe punctuation ending in {suffix:?}: {}",
            support::stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn nested_heading_inherits_historical_scope() -> TestResult {
    let output = validate_replacement(&format!(
        "removed heartbeat policy\n\n## Historical Example\n### Retired route\n{CLAUSE}."
    ))?;
    assert!(!output.status.success());
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn sibling_or_parent_heading_resets_historical_scope() -> TestResult {
    for heading in ["## Current Policy", "# Current Policy"] {
        let output = validate_replacement(&format!(
            "removed heartbeat policy\n\n## Historical Example\n### Retired route\nOld policy.\n\n{heading}\n{CLAUSE}."
        ))?;
        assert!(
            output.status.success(),
            "validator failed to reset historical scope at {heading:?}: {}",
            support::stderr(&output)
        );
    }
    Ok(())
}

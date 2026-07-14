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
fn period_before_weakening_suffix_does_not_supply_policy() -> TestResult {
    let output = validate_replacement(&format!(
        "{CLAUSE}. Unless explicitly approved, the heartbeat may be skipped."
    ))?;
    assert!(
        !output.status.success(),
        "validator accepted a period-separated weakening suffix"
    );
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn safe_punctuation_after_required_clause_remains_valid() -> TestResult {
    for suffix in [
        ", and record the result",
        "; the result remains auditable",
        ". The result remains auditable in the handoff",
    ] {
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

#[test]
fn conditional_weakening_suffix_does_not_supply_policy() -> TestResult {
    for suffix in [" when possible", ", if available", "; as needed"] {
        let output = validate_replacement(&format!("{CLAUSE}{suffix}."))?;
        assert!(
            !output.status.success(),
            "validator accepted conditional suffix {suffix:?}"
        );
        assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    }
    Ok(())
}

#[test]
fn safe_conditional_words_after_clause_remain_valid() -> TestResult {
    for suffix in [
        " when the deadline arrives",
        ", if the first attempt fails, MUST retry",
        "; as documented in the receipt",
    ] {
        let output = validate_replacement(&format!("{CLAUSE}{suffix}."))?;
        assert!(
            output.status.success(),
            "validator rejected safe suffix {suffix:?}: {}",
            support::stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn excluded_markdown_blocks_do_not_stitch_clause_fragments() -> TestResult {
    let (prefix, suffix) = CLAUSE.split_once(" solely").ok_or("clause split")?;
    for block in ["```text\nignored\n```", "    ignored"] {
        let output = validate_replacement(&format!("{prefix}\n{block}\nsolely{suffix}."))?;
        assert!(
            !output.status.success(),
            "validator stitched a clause across {block:?}"
        );
        assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    }
    Ok(())
}

#[test]
fn blank_markdown_paragraphs_do_not_stitch_clause_fragments() -> TestResult {
    let (prefix, suffix) = CLAUSE.split_once(" solely").ok_or("clause split")?;
    for boundary in ["\n\n", "\n   \n"] {
        let output = validate_replacement(&format!("{prefix}{boundary}solely{suffix}."))?;
        assert!(
            !output.status.success(),
            "validator stitched a clause across paragraph boundary {boundary:?}"
        );
        assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    }
    Ok(())
}

#[test]
fn soft_line_wrap_may_complete_required_clause() -> TestResult {
    let (prefix, suffix) = CLAUSE.split_once(" solely").ok_or("clause split")?;
    let output = validate_replacement(&format!("{prefix}\nsolely{suffix}."))?;
    assert!(
        output.status.success(),
        "validator rejected a soft-wrapped clause: {}",
        support::stderr(&output)
    );
    Ok(())
}

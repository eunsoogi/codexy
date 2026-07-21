use std::fs;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CLAUSE: &str = "MUST NOT retain or recreate an execution goal solely to preserve a successfully registered heartbeat";

fn validate_replacement(replacement: &str) -> TestResult<std::process::Output> {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = plugin_root.join("skills/codex-orchestration/references/runtime-heartbeats.md");
    let original = fs::read_to_string(&path)?;
    fs::write(&path, original.replace(CLAUSE, replacement))?;
    support::validator_instruction_policy(&plugin_root)
}

#[test]
fn punctuation_before_weakening_suffix_does_not_supply_policy() -> TestResult {
    for suffix in [
        ", except during maintenance",
        "; unless explicitly approved",
        ". Unless explicitly approved, the heartbeat may be skipped",
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
fn punctuation_wrapped_weakening_suffixes_do_not_supply_policy() -> TestResult {
    let suffixes = [
        " (unless explicitly approved, the heartbeat may be skipped)",
        " [unless explicitly approved, the heartbeat may be skipped]",
        " {unless explicitly approved, the heartbeat may be skipped}",
        " ) unless explicitly approved, the heartbeat may be skipped",
        " [({unless explicitly approved, the heartbeat may be skipped})]",
    ];
    let mut accepted = Vec::new();
    for suffix in suffixes {
        let output = validate_replacement(&format!("{CLAUSE}{suffix}."))?;
        if output.status.success() {
            accepted.push(suffix);
        } else {
            assert!(support::stderr(&output).contains("runtime heartbeat contract"));
        }
    }
    assert!(
        accepted.is_empty(),
        "validator accepted punctuation-wrapped weakening suffixes: {accepted:?}"
    );
    Ok(())
}

#[test]
fn safe_punctuation_wrapped_suffixes_remain_valid() -> TestResult {
    for suffix in [
        " (and record the result in the handoff)",
        " [and record the result in the handoff]",
        " {and record the result in the handoff}",
        " [({and record the result in the handoff})]",
    ] {
        let output = validate_replacement(&format!("{CLAUSE}{suffix}."))?;
        assert!(
            output.status.success(),
            "validator rejected safe punctuation-wrapped suffix {suffix:?}: {}",
            support::stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn emphasized_weakening_suffix_does_not_supply_policy() -> TestResult {
    let output = validate_replacement(&format!(
        "{CLAUSE}. **Unless** explicitly approved, the heartbeat may be skipped."
    ))?;
    assert!(
        !output.status.success(),
        "validator accepted an emphasized weakening suffix"
    );
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn conditional_prefix_does_not_supply_policy() -> TestResult {
    for prefix in ["When possible", "If available", "As needed"] {
        let output = validate_replacement(&format!("{prefix}, the owner {CLAUSE}."))?;
        assert!(
            !output.status.success(),
            "validator accepted conditional prefix {prefix:?}"
        );
        assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    }
    Ok(())
}

#[test]
fn safe_prefix_remains_valid() -> TestResult {
    let output = validate_replacement(&format!(
        "After successful registration, the owner {CLAUSE}."
    ))?;
    assert!(
        output.status.success(),
        "validator rejected safe prefix: {}",
        support::stderr(&output)
    );
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
fn markdown_boundary_before_weakening_suffix_does_not_supply_policy() -> TestResult {
    let output = validate_replacement(&format!(
        "{CLAUSE}\n\nUnless explicitly approved, the heartbeat may be skipped."
    ))?;
    assert!(
        !output.status.success(),
        "validator accepted a required clause before a separate weakening block"
    );
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn consecutive_markdown_boundaries_before_weakening_suffix_do_not_supply_policy() -> TestResult {
    let output = validate_replacement(&format!(
        "{CLAUSE}\n\n\n\nUnless explicitly approved, the heartbeat may be skipped."
    ))?;
    assert!(
        !output.status.success(),
        "validator accepted a required clause before consecutive weakening blocks"
    );
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn conditional_markdown_heading_does_not_supply_policy() -> TestResult {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = plugin_root.join("skills/codex-orchestration/references/runtime-heartbeats.md");
    let original = fs::read_to_string(&path)?;
    let sentence = format!(
        "The owner {CLAUSE}; it MAY keep a goal only while an implementation obligation remains."
    );
    fs::write(
        &path,
        original.replace(&sentence, &format!("\n\n## If available\n{sentence}")),
    )?;
    let output = support::validator_instruction_policy(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator accepted a required clause beneath a conditional heading"
    );
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

#[test]
fn adversative_weakening_suffix_does_not_supply_policy() -> TestResult {
    let mut accepted = Vec::new();
    for suffix in [
        ", but MAY skip the heartbeat",
        "; however, MAY skip the heartbeat",
        ", but the owner MAY skip the heartbeat",
    ] {
        let output = validate_replacement(&format!("{CLAUSE}{suffix}."))?;
        if output.status.success() {
            accepted.push(suffix);
        } else {
            assert!(support::stderr(&output).contains("runtime heartbeat contract"));
        }
    }
    assert!(accepted.is_empty(), "validator accepted {accepted:?}");
    Ok(())
}

#[test]
fn safe_adversative_suffixes_remain_valid() -> TestResult {
    for suffix in [
        ", but MUST record the result",
        "; however, MUST preserve the evidence",
        ", but the owner MUST record the result",
    ] {
        let output = validate_replacement(&format!("{CLAUSE}{suffix}."))?;
        assert!(
            output.status.success(),
            "validator rejected safe adversative suffix {suffix:?}: {}",
            support::stderr(&output)
        );
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

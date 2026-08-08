use std::{fs, path::Path};

use crate::support;
#[path = "validator_runtime_heartbeat/lifecycle.rs"]
mod lifecycle;
use lifecycle::CLAUSE;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn validate_replacement(replacement: &str) -> TestResult<std::process::Output> {
    let fixture = support::instruction_policy_fixture(Path::new(
        "skills/codex-orchestration/references/runtime-heartbeats.md",
    ))?;
    let path = fixture.path();
    let original = fs::read_to_string(&path)?;
    fs::write(
        &path,
        lifecycle::replace_sentence(
            &original,
            &replacement.replacen(CLAUSE, lifecycle::SENTENCE, 1),
        ),
    )?;
    support::validator_instruction_policy_file(path)
}

#[test]
fn conditional_heading_after_clause_is_a_weakening_suffix() -> TestResult {
    for heading in [
        "Unless explicitly approved",
        "If available",
        "Only if approved",
    ] {
        let output = validate_replacement(&format!(
            "{CLAUSE}\n\n## {heading}\nThe heartbeat MAY be skipped.\n\n## Required lifecycle\nThe lifecycle remains mandatory"
        ))?;
        assert!(
            !output.status.success(),
            "validator accepted conditional heading {heading:?} after the clause"
        );
        assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    }
    Ok(())
}

#[test]
fn safe_heading_after_clause_remains_valid() -> TestResult {
    for heading in [
        "Audit evidence",
        "Required follow-up",
        "If available for audit evidence",
    ] {
        let output = validate_replacement(&format!(
            "{CLAUSE}\n\n## {heading}\nThe owner MUST record the result.\n\n## Required lifecycle\nThe lifecycle remains mandatory"
        ))?;
        assert!(
            output.status.success(),
            "validator rejected safe heading {heading:?}: {}",
            support::stderr(&output)
        );
    }
    Ok(())
}

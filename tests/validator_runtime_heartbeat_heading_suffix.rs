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

use std::path::Path;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn assert_rejected_clauses(
    relative: &str,
    clauses: &[&str],
    replacement: &str,
    expected_error: &str,
) -> TestResult {
    let relative = Path::new(relative);
    let fixture = support::plugin_fixture_with_mutable_files(&[relative])?;
    let path = fixture.root().join(relative);
    for clause in clauses {
        fixture.reset_file(relative)?;
        let original = std::fs::read_to_string(&path)?;
        let mutated = original.replace(clause, replacement);
        assert_ne!(original, mutated, "fixture is missing required clause {clause:?}");
        std::fs::write(&path, mutated)?;
        let output = support::validator_instruction_policy(fixture.root())?;
        assert!(!output.status.success(), "validator accepted {clause:?}");
        assert!(support::stderr(&output).contains(expected_error));
    }
    Ok(())
}

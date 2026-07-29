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
    let fixture = support::instruction_policy_fixture(relative)?;
    let path = fixture.path();
    for clause in clauses {
        fixture.reset()?;
        let original = std::fs::read_to_string(&path)?;
        let mutated = original.replace(clause, replacement);
        assert_ne!(original, mutated, "fixture is missing required clause {clause:?}");
        std::fs::write(&path, mutated)?;
        let output = support::validator_instruction_policy_file(path)?;
        assert!(!output.status.success(), "validator accepted {clause:?}");
        assert!(support::stderr(&output).contains(expected_error));
    }
    Ok(())
}

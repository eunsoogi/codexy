use super::{TestResult, blocked_evidence, run_validator, valid_gate, valid_pre_mutation};

#[test]
fn validator_rejects_numeric_padding_in_every_substantive_field() -> TestResult {
    for gate in [
        valid_gate().replace(
            "Should the irreversible migration preserve legacy identifiers or replace them?",
            "2025 2026 2027 2028 choose migration?",
        ),
        valid_gate().replace(
            "preserve identifiers and retain compatibility|replace identifiers and require migration",
            "2025 2026 2027 2028 choose migration|replace identifiers and require migration",
        ),
        valid_gate().replace(
            "material impact=the choice changes persisted identifiers and migration behavior",
            "material impact=2025 2026 2027 2028 choose migration",
        ),
    ] {
        let output = run_validator(&blocked_evidence(gate, valid_pre_mutation()))?;
        assert!(
            !output.status.success(),
            "numeric padding passed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_preserves_distinct_numbered_branch_metadata() -> TestResult {
    let gate = valid_gate().replace(
        "preserve identifiers and retain compatibility|replace identifiers and require migration",
        "adopt version 2025 migration|adopt version 2026 migration",
    );
    let output = run_validator(&blocked_evidence(gate, valid_pre_mutation()))?;
    assert!(
        output.status.success(),
        "materially distinct numbered branches were collapsed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_bare_ordinal_branch_labels() -> TestResult {
    let gate = valid_gate().replace(
        "preserve identifiers and retain compatibility|replace identifiers and require migration",
        "1 preserve existing identifiers|2 preserve existing identifiers",
    );
    let output = run_validator(&blocked_evidence(gate, valid_pre_mutation()))?;
    assert!(
        !output.status.success(),
        "bare ordinal labels fabricated distinct decision branches: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

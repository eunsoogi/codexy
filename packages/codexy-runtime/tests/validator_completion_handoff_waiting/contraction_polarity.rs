use super::{TestResult, stderr, validate};

#[test]
fn validator_preserves_contracted_review_polarity() -> TestResult {
    for (handoff, actionable) in [
        (
            "Blocked: review feedback isn't resolved while work is incomplete.",
            true,
        ),
        (
            "Blocked: review feedback isn't unresolved while work is incomplete.",
            false,
        ),
    ] {
        let output = validate(handoff)?;
        assert_eq!(
            output.status.success(),
            actionable,
            "incorrect contracted review disposition for {handoff}: {}",
            stderr(&output)
        );
    }
    Ok(())
}

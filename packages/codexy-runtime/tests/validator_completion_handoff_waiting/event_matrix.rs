use super::{TestResult, stderr, validate};

const SEPARATORS: &[&str] = &[". ", " ", " and ", " or ", " but ", " while ", " because "];
const ASYNC_SUBJECTS: &[&str] = &["CI", "Sentinel", "reviewer", "resource", "tool"];
const LIFECYCLE_STATES: &[&str] = &["queued", "running", "idle", "unavailable"];
const POLARITIES: &[&str] = &["", "no "];

#[test]
fn validator_keeps_pending_review_separate_from_later_operational_open() -> TestResult {
    assert_disposition(
        "Blocked: review feedback is pending and implementation remains open.",
        false,
    )
}

#[test]
fn validator_keeps_ci_negation_out_of_later_review_event() -> TestResult {
    assert_disposition(
        "Blocked: no CI result and review feedback remains unresolved.",
        true,
    )
}

#[test]
fn validator_keeps_predicate_free_ci_negation_out_of_later_review_event() -> TestResult {
    assert_disposition(
        "Blocked: no CI and review feedback remains unresolved.",
        true,
    )
}

#[test]
fn validator_extracts_subject_predicate_events_independent_of_separator() -> TestResult {
    for separator in SEPARATORS {
        assert_disposition(
            &format!(
                "Blocked: review feedback remains unresolved{separator}implementation remains open."
            ),
            true,
        )?;
        assert_disposition(
            &format!(
                "Blocked: no review feedback remains unresolved{separator}implementation remains open."
            ),
            false,
        )?;
        assert_disposition(
            &format!(
                "Blocked: review feedback is pending{separator}implementation remains open."
            ),
            false,
        )?;
        assert_disposition(
            &format!(
                "Blocked: no CI result{separator}review feedback remains unresolved."
            ),
            true,
        )?;
        for polarity in POLARITIES {
            for subject in ASYNC_SUBJECTS {
                for state in LIFECYCLE_STATES {
                    assert_disposition(
                        &format!(
                            "Blocked: {polarity}{subject} {state}{separator}implementation remains open."
                        ),
                        false,
                    )?;
                }
            }
            assert_disposition(
                &format!(
                    "Blocked: {polarity}implementation remains open{separator}review feedback is pending."
                ),
                false,
            )?;
        }
    }
    Ok(())
}

fn assert_disposition(handoff: &str, actionable: bool) -> TestResult {
    let output = validate(handoff)?;
    assert_eq!(
        output.status.success(),
        actionable,
        "incorrect event disposition for {handoff}: {}",
        stderr(&output)
    );
    Ok(())
}

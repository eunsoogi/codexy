use super::{TestResult, stderr, validate};

const SEPARATORS: &[&str] = &[". ", " ", " and ", " or ", " but ", " while ", " because "];
const ASYNC_SUBJECTS: &[&str] = &["CI", "Sentinel", "reviewer", "resource", "tool"];
const LIFECYCLE_STATES: &[&str] = &["queued", "running", "idle", "unavailable"];
const POLARITIES: &[&str] = &["", "no "];
const OTHER_SUBJECTS: &[(&str, &str)] = &[
    ("CI", "has returned"),
    ("CI", "result"),
    ("implementation", "remains incomplete"),
    ("implementation", "remains"),
];
const REVIEW_PREDICATE: &str = "remains unresolved";
const COORDINATION_FORMS: &[(&str, &str)] = &[
    ("neither ", "nor"),
    ("no ", "and"),
    ("", "and"),
    ("", "or"),
    ("", "but"),
];

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
fn validator_keeps_nested_review_subject_polarity_local() -> TestResult {
    assert_disposition(
        "Blocked: reviewer reports no review feedback remains unresolved.",
        false,
    )?;
    assert_disposition(
        "Blocked: reviewer reports review feedback remains unresolved.",
        true,
    )
}

#[test]
fn validator_keeps_coordinated_external_predicate_local() -> TestResult {
    assert_disposition(
        "Blocked: no CI or Sentinel result has returned.",
        false,
    )?;
    assert_disposition(
        "Blocked: no CI result and review feedback remains unresolved.",
        true,
    )
}

#[test]
fn validator_bounds_coordinated_negation_to_its_completed_external_event() -> TestResult {
    assert_disposition(
        "Blocked: neither CI nor Sentinel has returned and review feedback remains unresolved.",
        true,
    )?;
    assert_disposition("Blocked: neither CI nor Sentinel has returned.", false)?;
    assert_disposition(
        "Blocked: neither CI has returned nor review feedback remains unresolved.",
        false,
    )?;
    assert_disposition(
        "Blocked: no CI has returned and review feedback remains unresolved.",
        true,
    )
}

#[test]
fn validator_models_coordination_grammar_across_subject_predicates() -> TestResult {
    for (other, other_predicate) in OTHER_SUBJECTS {
        for review_first in [false, true] {
            for (polarity, coordinator) in COORDINATION_FORMS {
                if review_first && *polarity == "no " {
                    continue;
                }
                let (left, left_predicate, right, right_predicate) = if review_first {
                    ("review feedback", REVIEW_PREDICATE, *other, *other_predicate)
                } else {
                    (*other, *other_predicate, "review feedback", REVIEW_PREDICATE)
                };
                let actionable = *polarity != "neither ";
                let body = format!(
                    "{polarity}{left} {left_predicate} {coordinator} {right} {right_predicate}"
                );
                for handoff in [
                    format!("Blocked: {body}."),
                    format!("BLOCKED: {}", body.to_ascii_uppercase()),
                    format!("Blocked: {}", body.replace(' ', ", ")),
                    format!("Blocked: {}", body.replace(' ', "\t")),
                ] {
                    assert_disposition(&handoff, actionable)?;
                }
            }
        }
    }
    Ok(())
}

#[test]
fn validator_ends_available_reviewer_event_before_later_feedback() -> TestResult {
    assert_disposition(
        "Blocked: no reviewer available; review feedback remains unresolved.",
        true,
    )
}

#[test]
fn validator_keeps_negated_later_review_feedback_nonterminal() -> TestResult {
    assert_disposition(
        "Blocked: review feedback is pending and no review comments remain unresolved.",
        false,
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

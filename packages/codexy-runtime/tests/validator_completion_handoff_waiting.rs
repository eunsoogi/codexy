type TestResult = Result<(), Box<dyn std::error::Error>>;

const CLEAN_PR_STATE: &str = r#"{
    "number": 128,
    "state": "OPEN",
    "isDraft": false,
    "mergeStateStatus": "CLEAN",
    "reviewDecision": "APPROVED",
    "reviewProfile": "light",
    "headRefOid": "32b03a210b3defb2d29dd352283ea2488e60d893",
    "latestReviews": [],
    "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[]}
}"#;

#[test]
fn validator_preserves_waiting_predicate_state_boundaries() -> TestResult {
    for handoff in [
        "Blocked while queued worktree setup is pending.\n",
        "Blocked while child work is pending.\n",
        "Blocked while CI is pending.\n",
    ] {
        assert_waiting_error(handoff)?;
    }

    assert_valid("No blocked state while CI is pending.\n")?;
    assert_valid("Blocked while CI is pending; required checks are failing.\n")?;
    assert_valid("Blocked while CI is pending; required checks are failing: \"no\".\n")?;
    Ok(())
}

#[test]
fn validator_preserves_false_check_label_separators_and_values() -> TestResult {
    for label in [
        "required checks are failing: no",
        "required checks are failing? false",
        "required checks are failing = none",
        "required checks are failing - no",
    ] {
        assert_waiting_error(&format!("Blocked while CI is pending; {label}.\n"))?;
    }
    Ok(())
}

fn assert_waiting_error(handoff: &str) -> TestResult {
    let output = crate::support::validator_completion_handoff(handoff, CLEAN_PR_STATE)?;
    assert!(
        !output.status.success(),
        "validator should reject false blocked waiting evidence\nhandoff:\n{handoff}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("waiting state evidence"),
        "unexpected stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn assert_valid(handoff: &str) -> TestResult {
    let output = crate::support::validator_completion_handoff(handoff, CLEAN_PR_STATE)?;
    assert!(
        output.status.success(),
        "validator should preserve non-waiting evidence\nhandoff:\n{handoff}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

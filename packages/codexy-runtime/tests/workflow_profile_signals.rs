use super::workflow_profile_contract::assert_profile_result;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn strict_signals_are_complete_and_negation_aware() -> TestResult {
    for task_kind in [
        "destructive mutation",
        "security and secrets",
        "permission change",
        "secret rotation",
        "release work",
        "publication workflow",
        "high-consequence external-state mutation",
        "high-risk mutation",
        "high-risk guardrail",
        "multi-lane coordination",
        "merge-sensitive change",
    ] {
        assert_profile_result(
            "strict task kind requires formal proof",
            &format!("Task kind: {task_kind}"),
            false,
        )?;
    }
    for task_kind in [
        "no security concerns",
        "non-security documentation",
        "not a release",
        "without secret material",
    ] {
        assert_profile_result(
            "negated strict signal remains lightweight",
            &format!("Task kind: {task_kind}"),
            true,
        )?;
    }
    Ok(())
}

#[test]
fn strict_signal_negation_is_category_local() -> TestResult {
    for task_kind in [
        "no security but release work",
        "without permission; publication workflow",
        "not a release, but security review",
    ] {
        assert_profile_result(
            "an affirmative category after a delimiter requires strict proof",
            &format!("Task kind: {task_kind}"),
            false,
        )?;
    }
    assert_profile_result(
        "one negation may cover coordinated categories in its clause",
        "Task kind: no security or release work",
        true,
    )
}

#[test]
fn strict_signals_require_exact_category_tokens() -> TestResult {
    for task_kind in ["secretary notes", "secretarial work"] {
        assert_profile_result(
            "a category prefix inside another token remains lightweight",
            &format!("Task kind: {task_kind}"),
            true,
        )?;
    }
    Ok(())
}

#[test]
fn strict_signals_before_ownership_metadata_remain_in_the_current_lane() -> TestResult {
    let ownership = "Ownership metadata source: current-thread-classified\nLane ownership: current-thread-owned";
    assert_profile_result(
        "a strict profile before ownership metadata still requires formal proof",
        &format!("Workflow profile: strict\n{ownership}"),
        false,
    )?;
    assert_profile_result(
        "security risk before ownership metadata cannot be downgraded by light",
        &format!("Workflow profile: light\nTask kind: security review\n{ownership}"),
        false,
    )?;
    assert_profile_result(
        "strict risk before a review-response boundary stays historical",
        &format!(
            "Task kind: security review\nReview response: current lane\nWorkflow profile: light\nTask kind: documentation\n{ownership}"
        ),
        true,
    )
}

#[test]
fn strict_category_delimiters_and_compounds_are_structural() -> TestResult {
    for task_kind in [
        "not a release: security review",
        "no security — release work",
        "without permission – publication workflow",
        "no security ‒ release work",
        "security-sensitive review",
    ] {
        assert_profile_result(
            "an affirmative exact category after a delimiter requires strict proof",
            &format!("Task kind: {task_kind}"),
            false,
        )?;
    }
    for task_kind in [
        "non-security-sensitive documentation",
        "secretary-sensitive notes",
        "no security-sensitive concerns",
    ] {
        assert_profile_result(
            "negated or non-category compounds remain lightweight",
            &format!("Task kind: {task_kind}"),
            true,
        )?;
    }
    Ok(())
}

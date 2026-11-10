// Signal tests separate exact strict-risk categories from harmless prefixes, local negation, and collaboration shape.
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
        "merge-sensitive change",
        "cross-lane merge-sensitive integration",
    ] {
        assert_profile_result(
            "strict task kind requires formal proof",
            &format!("Task kind: {task_kind}"),
            false,
        )?;
    }
    for task_kind in [
        "multi-lane coordination",
        "independent multi-lane work",
        "durable delegated work",
    ] {
        assert_profile_result(
            "collaboration shape alone does not require strict proof",
            &format!("Task kind: {task_kind}"),
            true,
        )?;
    }
    assert_profile_result(
        "materially shared executable contracts require strict evidence",
        "Workflow profile: light\nTask kind: materially shared executable-contract change",
        false,
    )?;
    assert_profile_result(
        "a negated shared-contract risk does not require strict evidence",
        "Workflow profile: standard\nTask kind: no shared executable-contract change",
        true,
    )?;
    for task_kind in [
        "no security concerns",
        "non-security documentation",
        "not a release",
        "without secret material",
        "no shared executable-contract changes",
        "no materially shared executable-contract changes",
        "not a materially shared executable-contract change",
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
fn plural_shared_contract_risk_respects_negation_and_quotation() -> TestResult {
    for risk in [
        "materially shared executable contracts",
        "shared executable contracts",
    ] {
        assert_profile_result(
            "plural shared-contract risk cannot be downgraded to light",
            &format!("Workflow profile: light\nTask kind: {risk}"),
            false,
        )?;
    }
    for negated_risk in [
        "no materially shared executable contracts",
        "no shared executable contracts",
    ] {
        assert_profile_result(
            "negated plural shared-contract risk remains standard",
            &format!("Workflow profile: standard\nTask kind: {negated_risk}"),
            true,
        )?;
    }
    assert_profile_result(
        "quoted plural shared-contract wording does not assert risk",
        "Workflow profile: standard\nTask kind: ordinary mutation\nQuoted example: `materially shared executable contracts`",
        true,
    )
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

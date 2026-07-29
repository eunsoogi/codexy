use super::workflow_profile_contract::assert_profile_result;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn spaced_hyphens_are_clause_boundaries() -> TestResult {
    for task_kind in [
        "no security - release work",
        "no security ‐ release work",
        "no security ‑ release work",
    ] {
        assert_profile_result(
            "a spaced hyphen ends the negated category clause",
            &format!("Task kind: {task_kind}"),
            false,
        )?;
    }
    Ok(())
}

#[test]
fn compound_hyphens_preserve_category_polarity() -> TestResult {
    for task_kind in [
        "permission-free documentation",
        "permission‐free documentation",
        "permission‑free documentation",
        "security-free notes",
    ] {
        assert_profile_result(
            "a joined free suffix negates its exact category",
            &format!("Task kind: {task_kind}"),
            true,
        )?;
    }
    for task_kind in [
        "permission-sensitive change",
        "security-sensitive review",
        "permission - free documentation",
    ] {
        assert_profile_result(
            "an affirmative category or spaced separator remains strict",
            &format!("Task kind: {task_kind}"),
            false,
        )?;
    }
    Ok(())
}

#[test]
fn coordinated_prefix_negation_has_bounded_scope() -> TestResult {
    for task_kind in [
        "no security or release or publication work",
        "without permission or security review",
        "not a release or publication workflow",
    ] {
        assert_profile_result(
            "prefix negation propagates across one coordinated category chain",
            &format!("Task kind: {task_kind}"),
            true,
        )?;
    }
    for task_kind in [
        "no security or release but publication work",
        "no security or release; publication work",
        "no security review followed by release work",
    ] {
        assert_profile_result(
            "adversative, delimiter, or ordinary prose ends coordinated negation",
            &format!("Task kind: {task_kind}"),
            false,
        )?;
    }
    Ok(())
}

#[test]
fn affirmative_not_only_and_postfix_negation_are_distinct() -> TestResult {
    for task_kind in [
        "not only security review",
        "security is involved",
        "security - not involved",
    ] {
        assert_profile_result(
            "affirmative involvement remains strict",
            &format!("Task kind: {task_kind}"),
            false,
        )?;
    }
    for task_kind in [
        "security is not involved",
        "permission is not involved",
        "not only secretary notes",
    ] {
        assert_profile_result(
            "postfix negation and exact-token controls remain lightweight",
            &format!("Task kind: {task_kind}"),
            true,
        )?;
    }
    Ok(())
}

#[test]
fn only_direct_prefix_negation_propagates_through_coordination() -> TestResult {
    for task_kind in [
        "permission-free and release work",
        "permission‐free or publication work",
        "security‑free and release work",
        "security is not involved and release work continues",
        "permission is not involved or publication proceeds",
        "non-security and release work",
    ] {
        assert_profile_result(
            "category-local suffix and postfix polarity cannot suppress a later category",
            &format!("Task kind: {task_kind}"),
            false,
        )?;
    }
    for task_kind in [
        "no security and release work",
        "without permission or publication workflow",
        "not a release and publication workflow",
        "permission-free and secretary notes",
        "security is not involved or secretary notes",
        "non-security and secretary notes",
    ] {
        assert_profile_result(
            "only direct prefix negation propagates through a category chain",
            &format!("Task kind: {task_kind}"),
            true,
        )?;
    }
    Ok(())
}

#[test]
fn coordinated_prefix_negation_stops_before_predicate_prose() -> TestResult {
    for task_kind in [
        "no security is involved and release work continues",
        "no security concerns remain and release work continues",
    ] {
        assert_profile_result(
            "predicate prose ends coordinated prefix negation before an affirmative category",
            &format!("Task kind: {task_kind}"),
            false,
        )?;
    }
    for task_kind in [
        "no security and release work",
        "no security or release or publication work",
    ] {
        assert_profile_result(
            "adjacent coordinated categories retain direct prefix negation",
            &format!("Task kind: {task_kind}"),
            true,
        )?;
    }
    Ok(())
}

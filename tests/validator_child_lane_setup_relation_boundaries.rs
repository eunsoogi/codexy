use super::validator_child_lane_setup_action_binding::{
    assert_with_classification, child_owned_classification,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_limits_timing_negation_to_the_before_classification_phrase() -> TestResult {
    for (form, setup) in [
        ("explicit active", "The child created branch codexy/463, not a worktree, before classification."),
        ("explicit passive", "Branch codexy/463 was created by the child, not a worktree, before classification."),
        ("unqualified active", "Created branch codexy/463, not a worktree, before classification."),
        ("unqualified passive", "Branch codexy/463 was created, not a worktree, before classification."),
    ] {
        assert_with_classification(
            &format!("object contrast does not negate {form} timing"),
            child_owned_classification(),
            setup,
            false,
        )?;
    }
    Ok(())
}

#[test]
fn validator_treats_semicolons_as_relation_boundaries() -> TestResult {
    for (form, setup) in [
        ("explicit active", "The child created branch codexy/463 after classification."),
        ("explicit passive", "Branch codexy/463 was created by the child after classification."),
        ("unqualified active", "Created branch codexy/463 after classification."),
        ("unqualified passive", "Branch codexy/463 was created after classification."),
    ] {
        assert_with_classification(
            &format!("semicolon bounds the {form} relation"),
            child_owned_classification(),
            &format!("The child reviewed requirements before classification; {setup}"),
            true,
        )?;
    }
    Ok(())
}

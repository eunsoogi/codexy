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
fn validator_preserves_object_and_timing_polarity_across_punctuation() -> TestResult {
    for (open, close) in [("(", ")"), ("[", "]"), ("—", "—")] {
        assert_with_classification(
            "punctuated object contrast remains affirmative setup",
            child_owned_classification(),
            &format!("The child created branch codexy/463 {open}not a worktree{close} before classification."),
            false,
        )?;
        assert_with_classification(
            "punctuated timing negation remains negative setup",
            child_owned_classification(),
            &format!("The child created branch codexy/463 {open}not at any point in time before classification{close} but after classification."),
            true,
        )?;
    }
    Ok(())
}

#[test]
fn validator_distinguishes_lexical_hyphens_from_dash_delimiters() -> TestResult {
    for (form, setup) in [
        ("explicit active", "The child created branch codexy/463"),
        ("explicit passive", "Branch codexy/463 was created by the child"),
        ("unqualified active", "Created branch codexy/463"),
        ("unqualified passive", "Branch codexy/463 was created"),
    ] {
        for (kind, suffix, expected) in [
            (
                "lexical hyphen timing negation",
                "not at any point-in-time before classification but after classification.",
                true,
            ),
            (
                "standalone dash timing negation",
                "- not at any point in time before classification - but after classification.",
                true,
            ),
            (
                "standalone dash object contrast",
                "- not a worktree - before classification.",
                false,
            ),
        ] {
            assert_with_classification(
                &format!("{kind} preserves {form} polarity"),
                child_owned_classification(),
                &format!("{setup} {suffix}"),
                expected,
            )?;
        }
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

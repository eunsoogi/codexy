use super::validator_child_lane_setup_actor_grammar::assert_result;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_binds_setup_actions_to_the_nearest_predicate_subject() -> TestResult {
    for (label, setup, expected) in [
        (
            "reported parent setup is not attributed to the child reporter",
            "The child reports the parent created branch codexy/463 after classification.",
            true,
        ),
        (
            "reported child setup remains attributed to the child",
            "The parent reports the child created branch codexy/463 after classification.",
            false,
        ),
        (
            "nested parent setup is not attributed to the child modifier",
            "The child who reviewed it says the parent created branch codexy/463 after classification.",
            true,
        ),
        (
            "reported passive parent setup remains parent attributed",
            "The child reports branch codexy/463 was created by the parent after classification.",
            true,
        ),
        (
            "reported passive child setup remains child attributed",
            "The parent reports branch codexy/463 was created by the child after classification.",
            false,
        ),
        (
            "child subject is inherited across and then setup predicates",
            "The child did not create a worktree after classification and then switched to branch codexy/463 before classification.",
            false,
        ),
        (
            "parent subject is inherited across and then setup predicates",
            "The parent did not create a worktree after classification and then switched to branch codexy/463 before classification.",
            true,
        ),
        (
            "contrastive parent does not replace the child predicate subject",
            "The child not the parent created branch codexy/463 after classification.",
            false,
        ),
        (
            "contrastive child does not replace the parent predicate subject",
            "The parent not the child created branch codexy/463 after classification.",
            true,
        ),
        (
            "direct child setup remains child attributed",
            "The child created branch codexy/463 after classification.",
            false,
        ),
        (
            "direct parent setup remains parent attributed",
            "The parent created branch codexy/463 after classification.",
            true,
        ),
        (
            "coordinated child and parent setup remains fail closed",
            "The child and the parent created branch codexy/463 after classification.",
            false,
        ),
    ] {
        assert_result(label, setup, expected)?;
    }
    Ok(())
}

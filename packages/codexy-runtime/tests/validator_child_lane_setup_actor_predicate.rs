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

#[test]
fn validator_binds_setup_predicates_to_structural_clause_subjects() -> TestResult {
    for (label, setup, expected) in [
        (
            "relative clause child object does not replace the parent subject",
            "The parent who reviewed the child created branch codexy/463 after classification.",
            true,
        ),
        (
            "relative clause parent object does not replace the child subject",
            "The child who reviewed the parent created branch codexy/463 after classification.",
            false,
        ),
        (
            "object of an earlier predicate does not replace the inherited child subject",
            "The child reviewed the parent and then created branch codexy/463 after classification.",
            false,
        ),
        (
            "object of an earlier predicate does not replace the inherited parent subject",
            "The parent reviewed the child and then created branch codexy/463 after classification.",
            true,
        ),
        (
            "together with mixed subjects fail closed",
            "The child together with the parent created branch codexy/463 after classification.",
            false,
        ),
        (
            "reciprocal together with mixed subjects fail closed",
            "The parent together with the child created branch codexy/463 after classification.",
            false,
        ),
    ] {
        assert_result(label, setup, expected)?;
    }
    Ok(())
}

#[test]
fn validator_tracks_governing_subjects_across_predicate_and_relative_boundaries(
) -> TestResult {
    for (label, setup, expected) in [
        (
            "plain and retains the parent subject after a child object",
            "The parent reviewed the child and created branch codexy/463 after classification.",
            true,
        ),
        (
            "plain and retains the child subject after a parent object",
            "The child reviewed the parent and created branch codexy/463 after classification.",
            false,
        ),
        (
            "relative report child object does not replace the parent subject",
            "The parent who told the child about review created branch codexy/463 after classification.",
            true,
        ),
        (
            "relative report parent object does not replace the child subject",
            "The child who told the parent about review created branch codexy/463 after classification.",
            false,
        ),
        (
            "possessive relative child does not replace the parent subject",
            "The parent whose child reviewed it created branch codexy/463 after classification.",
            true,
        ),
        (
            "possessive relative parent does not replace the child subject",
            "The child whose parent reviewed it created branch codexy/463 after classification.",
            false,
        ),
        (
            "reported child subject overrides the parent reporter",
            "The parent reports the child created branch codexy/463 after classification.",
            false,
        ),
        (
            "reported parent subject overrides the child reporter",
            "The child reports the parent created branch codexy/463 after classification.",
            true,
        ),
        (
            "and then retains the parent subject after a child object",
            "The parent reviewed the child and then created branch codexy/463 after classification.",
            true,
        ),
        (
            "and then retains the child subject after a parent object",
            "The child reviewed the parent and then created branch codexy/463 after classification.",
            false,
        ),
    ] {
        assert_result(label, setup, expected)?;
    }
    Ok(())
}

#[test]
fn validator_binds_direct_auxiliary_setup_to_the_clause_subject() -> TestResult {
    for (label, setup, expected) in [
        (
            "reported parent did create remains parent attributed",
            "The child reports the parent did create branch codexy/463 after classification.",
            true,
        ),
        (
            "reported child did create remains child attributed",
            "The parent reports the child did create branch codexy/463 after classification.",
            false,
        ),
        (
            "reported parent does create remains parent attributed",
            "The child reports the parent does create branch codexy/463 after classification.",
            true,
        ),
        (
            "reported child does create remains child attributed",
            "The parent reports the child does create branch codexy/463 after classification.",
            false,
        ),
        (
            "parent plural direct do setup remains parent attributed",
            "The parent and the orchestrator do create branch codexy/463 after classification.",
            true,
        ),
        (
            "child plural direct do setup remains fail closed",
            "The child and the parent do create branch codexy/463 after classification.",
            false,
        ),
        (
            "adverb before the direct auxiliary does not replace the parent subject",
            "The child reports the parent deliberately did create branch codexy/463 after classification.",
            true,
        ),
        (
            "relative report child object with an adverb remains parent attributed",
            "The parent who told the child yesterday created branch codexy/463 after classification.",
            true,
        ),
        (
            "relative report parent object with an adverb remains child attributed",
            "The child who told the parent yesterday created branch codexy/463 after classification.",
            false,
        ),
    ] {
        assert_result(label, setup, expected)?;
    }
    Ok(())
}

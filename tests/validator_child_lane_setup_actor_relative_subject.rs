use super::validator_child_lane_setup_actor_grammar::assert_result;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_keeps_whose_relative_report_roles_out_of_the_main_subject() -> TestResult {
    for (label, setup, expected) in [
        (
            "whose relative report child object does not replace the parent subject",
            "The parent whose orchestrator told the child did create branch codexy/463 after classification.",
            true,
        ),
        (
            "whose relative report parent object does not replace the child subject",
            "The child whose orchestrator told the parent did create branch codexy/463 after classification.",
            false,
        ),
        (
            "whose present relative report child object does not replace the parent subject",
            "The parent whose orchestrator tells the child does create branch codexy/463 after classification.",
            true,
        ),
        (
            "whose present relative report parent object does not replace the child subject",
            "The child whose orchestrator tells the parent does create branch codexy/463 after classification.",
            false,
        ),
        (
            "a completed whose relative clause permits a new child main subject",
            "The parent whose orchestrator reviewed it says the child did create branch codexy/463 after classification.",
            false,
        ),
        (
            "a completed whose relative clause permits a new parent main subject",
            "The child whose orchestrator reviewed it says the parent did create branch codexy/463 after classification.",
            true,
        ),
        (
            "compound whose subject does not replace the parent main subject",
            "The parent whose orchestrator and parent told the child did create branch codexy/463 after classification.",
            true,
        ),
        (
            "compound whose subject does not replace the child main subject",
            "The child whose parent and orchestrator told the parent did create branch codexy/463 after classification.",
            false,
        ),
        (
            "modified whose subject does not replace the parent main subject",
            "The parent whose current orchestrator told the child did create branch codexy/463 after classification.",
            true,
        ),
        (
            "modified whose subject does not replace the child main subject",
            "The child whose current orchestrator told the parent did create branch codexy/463 after classification.",
            false,
        ),
        (
            "relative never does not negate the parent main setup",
            "The parent whose current orchestrator never told the child did create branch codexy/463 after classification.",
            true,
        ),
        (
            "relative never does not license the child main setup",
            "The child whose current orchestrator never told the parent did create branch codexy/463 after classification.",
            false,
        ),
        (
            "relative not does not negate the parent main setup",
            "The parent whose current orchestrator did not tell the child did create branch codexy/463 after classification.",
            true,
        ),
        (
            "relative not does not license the child main setup",
            "The child whose current orchestrator did not tell the parent did create branch codexy/463 after classification.",
            false,
        ),
        (
            "main clause never continues to negate child setup",
            "The child whose current orchestrator told the parent never did create branch codexy/463 before classification.",
            true,
        ),
        (
            "main clause not continues to negate child setup",
            "The child whose current orchestrator told the parent did not create branch codexy/463 before classification.",
            true,
        ),
    ] {
        assert_result(label, setup, expected)?;
    }
    Ok(())
}

fn assignment(extra: &str) -> String {
    super::validator_child_goal_reporting::clear_child_assignment(extra)
}

#[test]
fn denial_grammar_covers_bypasses_without_treating_history_as_policy()
-> Result<(), Box<dyn std::error::Error>> {
    for prohibition in [
        "Goal tools are not authorized; this is not an example and remains mandatory.",
        "Never use goal tools during this implementation.",
        "Goal tools are not permitted during this implementation.",
    ] {
        let output = crate::support::validator_child_lane_ownership(&assignment(prohibition))?;
        assert!(!output.status.success(), "must reject: {prohibition}");
    }
    let output = crate::support::validator_child_lane_ownership(&assignment(
        "The incident wording “Do not use goal tools” is historical text, not current policy.",
    ))?;
    assert!(
        output.status.success(),
        "historical wording must stay inert: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn clear_goal_sequence_requires_call_and_active_readback()
-> Result<(), Box<dyn std::error::Error>> {
    let get = "Parent goal post-result: operation=get_goal; exact tool result={\"goal\":null}; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:get";
    let pre = "Parent goal pre-delivery: operation=create_goal; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:create";
    let call = "Goal tool call: create_goal(objective=implement issue #873)";
    for (events, expected) in [
        (format!("{get}\n{pre}"), "requires an actual create_goal tool call"),
        (
            format!("{get}\n{pre}\n{call}"),
            "requires an active readback after create_goal",
        ),
    ] {
        let output = crate::support::validator_child_lane_ownership(&assignment(&events))?;
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
    }
    Ok(())
}

#[test]
fn goal_sequence_binds_assignment_call_and_active_objective()
-> Result<(), Box<dyn std::error::Error>> {
    let get = "Parent goal post-result: operation=get_goal; exact tool result={\"goal\":null}; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:get";
    let active = "Parent goal post-result: operation=get_goal; exact tool result={\"goal\":{\"objective\":\"different issue #873\",\"status\":\"active\"}}; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:active";
    for (events, expected) in [
        (
            format!("{get}\nGoal tool call: create_goal(objective=issue #999)"),
            "create_goal objective must exactly match the authorized assignment objective",
        ),
        (
            format!("{get}\nGoal tool call: create_goal(objective=implement issue #873)\n{active}"),
            "active goal readback objective must match create_goal",
        ),
    ] {
        let output = crate::support::validator_child_lane_ownership(&assignment(&events))?;
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
    }
    Ok(())
}

#[test]
fn successful_sequence_rejects_duplicate_create_and_contradictory_envelope()
-> Result<(), Box<dyn std::error::Error>> {
    let duplicate = assignment(
        "Parent goal post-result: operation=get_goal; exact tool result={\"goal\":null}; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:get\n\
         Goal tool call: create_goal(objective=implement issue #873)\n\
         Parent goal post-result: operation=get_goal; exact tool result={\"goal\":{\"objective\":\"implement issue #873\",\"status\":\"active\"}}; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:active\n\
         Goal tool call: create_goal(objective=issue #999)",
    );
    let output = crate::support::validator_child_lane_ownership(&duplicate)?;
    assert!(!output.status.success());

    let contradictory = assignment(
        "Parent goal post-result: operation=get_goal; exact tool result={\"goal\":null,\"status\":\"active\"}; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:get\n\
         Goal tool call: create_goal(objective=implement issue #873)",
    );
    let output = crate::support::validator_child_lane_ownership(&contradictory)?;
    assert!(!output.status.success());
    Ok(())
}

#[test]
fn denial_grammar_handles_contractions_without_reversing_positive_rules()
-> Result<(), Box<dyn std::error::Error>> {
    for prohibition in [
        "Goal tools aren’t authorized for this task.",
        "The child can't use goal tools.",
        "No goal tools during implementation.",
        "The child mustn’t use goal tools.",
    ] {
        let output = crate::support::validator_child_lane_ownership(&assignment(prohibition))?;
        assert!(!output.status.success(), "must reject: {prohibition}");
    }
    for requirement in [
        "The parent must not prohibit goal tools.",
        "Goal tools are never disabled.",
        "Do not disable goal tools.",
        "Goal tools remain required; unrelated checks must not be skipped.",
    ] {
        let output = crate::support::validator_child_lane_ownership(&assignment(requirement))?;
        assert!(
            output.status.success(),
            "positive requirement must pass: {requirement}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn objective_binding_rejects_reference_collisions_and_negated_scope()
-> Result<(), Box<dyn std::error::Error>> {
    for objective in [
        "implement issue #8730",
        "implement issue #873 and issue #999",
        "ignore issue #873",
        "do not implement issue #873",
    ] {
        let evidence = assignment(&format!(
            "Parent goal post-result: operation=get_goal; exact tool result={{\"goal\":null}}; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:get\nGoal tool call: create_goal(objective={objective})"
        ));
        let output = crate::support::validator_child_lane_ownership(&evidence)?;
        assert!(!output.status.success(), "must reject: {objective}");
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("create_goal objective must exactly match the authorized assignment objective")
        );
    }
    Ok(())
}

#[test]
fn objective_binding_rejects_non_issue_scope_broadening()
-> Result<(), Box<dyn std::error::Error>> {
    let evidence = assignment(
        "Parent goal post-result: operation=get_goal; exact tool result={\"goal\":null}; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:get\n\
         Goal tool call: create_goal(objective=implement issue #873 and publish release artifacts)",
    );
    let output = crate::support::validator_child_lane_ownership(&evidence)?;
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(
            "create_goal objective must exactly match the authorized assignment objective"
        )
    );
    Ok(())
}

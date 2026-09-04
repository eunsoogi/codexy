#[test]
fn prose_controls_do_not_block_a_child_goal_report() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let skill = std::fs::read_to_string(root.join("plugins/codexy/skills/goal-lifecycle/SKILL.md"))?;
    let reporting = std::fs::read_to_string(
        root.join("plugins/codexy/skills/orchestration/references/goal-transition-reporting.md"),
    )?;
    for required in [
        "concrete delegated task assignment is explicit authorization",
        "MUST NOT require a second instruction containing the word",
        "MUST NOT broaden scope, invent",
    ] {
        assert!(skill.contains(required), "missing lifecycle rule: {required}");
    }
    for required in [
        "parent-supplied assignment that names the objective and success criteria",
        "Authorized goal objective:",
        "same source task id and transition key",
    ] {
        assert!(reporting.contains(required), "missing reporting rule: {required}");
    }
    Ok(())
}

pub(super) fn clear_child_assignment(extra: &str) -> String {
    format!(
        "Source thread id: parent-873\n\
         Goal control state: source_thread_id=parent-873\n\
         Assignment objective: implement issue #873\n\
         Success criteria: focused regression coverage and stop before merge\n\
         Authorized goal objective: implement issue #873\n\
         {extra}\n\
         Ownership metadata source: parent-supplied\n\
         Lane ownership: child-owned\n\
         Task classification:\n\
         | Field | Value |\n\
         | --- | --- |\n\
         | Lane type | implementation |\n\
         | Secondary surfaces | validators |\n\
         | Owner decision | affirmative child-owned because the delegated child owns the work |\n\
         | Atomic scope | issue-sized |\n\
         | Required skills | orchestration, goal-lifecycle |\n\
         | Required tools/evidence | goal readback and focused validation |\n\
         | First allowed action | call get_goal before implementation |\n\
         | Stop/blocker | unrelated active goal |\n"
    )
}

pub(super) fn get_transaction(key: &str, result: &str, parent: &str) -> String {
    format!(
        "Goal tool call: get_goal; parent task={parent}; transition key={key}\n\
         Parent goal post-result: operation=get_goal; exact tool result={result}; parent task={parent}; delivery=confirmed; task surface=codex task/thread; transition key={key}"
    )
}

pub(super) fn create_transaction(objective: &str, key: &str, parent: &str) -> String {
    format!(
        "Parent goal pre-delivery: operation=create_goal; pending objective={objective}; parent task={parent}; delivery=confirmed; task surface=codex task/thread; issue/pr=#873; plan step=implementation; branch=eunsoogi/873-finite-goals; worktree=/worktree; head=abc873; clean/index=clean; evidence=clear assignment; next action=create finite goal; transition key={key}\n\
         Goal tool call: create_goal(objective={objective}); parent task={parent}; transition key={key}\n\
         Parent goal post-result: operation=create_goal; exact tool result={{\"goal\":{{\"objective\":\"{objective}\",\"status\":\"active\"}}}}; parent task={parent}; delivery=confirmed; task surface=codex task/thread; transition key={key}"
    )
}

pub(super) fn successful_sequence() -> String {
    format!(
        "{}\n{}\n{}",
        get_transaction("873:get-null", "{\"goal\":null}", "parent-873"),
        create_transaction("implement issue #873", "873:create", "parent-873"),
        get_transaction(
            "873:get-active",
            "{\"goal\":{\"objective\":\"implement issue #873\",\"status\":\"active\"}}",
            "parent-873",
        )
    )
}

pub(super) const AUTHENTIC_OBJECTIVE: &str = "Implement GitHub issue #873, create focused regression coverage, verify all required checks, and open a PR linked to #873 from a child-owned branch based on current origin/main; stop before merge.";

pub(super) fn authentic_assignment() -> String {
    let sequence = format!(
        "{}\n{}\n{}",
        get_transaction("873:get-null", "{\"goal\":null}", "parent-873"),
        create_transaction(AUTHENTIC_OBJECTIVE, "873:create", "parent-873"),
        get_transaction(
            "873:get-active",
            &format!(
                "{{\"goal\":{{\"objective\":\"{AUTHENTIC_OBJECTIVE}\",\"status\":\"active\"}}}}"
            ),
            "parent-873",
        )
    );
    clear_child_assignment(&sequence).replace("implement issue #873", AUTHENTIC_OBJECTIVE)
}

#[test]
fn rejects_goal_tool_prohibition_on_clear_child_implementation()
-> Result<(), Box<dyn std::error::Error>> {
    let output = crate::support::validator_child_lane_ownership(&clear_child_assignment(
        "Goal tools are not authorized unless the user explicitly requests a goal.",
    ))?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("clear delegated implementation must not prohibit available goal tools"));
    Ok(())
}

#[test]
fn clear_assignment_needs_no_second_goal_opt_in() -> Result<(), Box<dyn std::error::Error>> {
    let output = crate::support::validator_child_lane_ownership(&clear_child_assignment(
        &successful_sequence(),
    ))?;
    assert!(
        output.status.success(),
        "a concrete assignment is authorization: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn ambiguous_discussion_does_not_create_a_goal_contract()
-> Result<(), Box<dyn std::error::Error>> {
    let output = crate::support::validator_child_lane_ownership(
        "Discussion note: perhaps issue #873 could be explored later.\n\
         Quoted anti-pattern: goal tools are not authorized unless a user requests a goal.\n",
    )?;
    assert!(output.status.success());
    Ok(())
}

#[test]
fn unrelated_active_goal_is_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let active = get_transaction(
        "873:get",
        "{\"goal\":{\"objective\":\"issue #999\",\"status\":\"active\"}}",
        "parent-873",
    );
    let output = crate::support::validator_child_lane_ownership(&clear_child_assignment(&active))?;
    assert!(output.status.success(), "readback alone preserves the active goal");
    let output = crate::support::validator_child_lane_ownership(&clear_child_assignment(&format!(
        "{active}\n{}",
        create_transaction("implement issue #873", "873:create", "parent-873")
    )))?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("active goal must be preserved and must not be replaced by create_goal"));
    Ok(())
}

#[test]
fn rejects_equivalent_goal_tool_prohibitions() -> Result<(), Box<dyn std::error::Error>> {
    for prohibition in [
        "You are not authorized to use goal tools.",
        "Goal tools are **not authorized** for this task.",
        "The child must not use goal tools.",
        "Goal tools are disabled while implementation continues.",
    ] {
        let output = crate::support::validator_child_lane_ownership(&clear_child_assignment(
            prohibition,
        ))?;
        assert!(!output.status.success(), "must reject: {prohibition}");
    }
    Ok(())
}

#[test]
fn quoted_and_inert_prohibition_examples_are_ignored() -> Result<(), Box<dyn std::error::Error>> {
    for example in [
        "Quoted anti-pattern: Do not use goal tools.",
        "- \"Do not use goal tools\" is wording from the incident.",
        "> Do not use goal tools.",
        "```text\nDo not use goal tools.\n```",
    ] {
        let evidence = clear_child_assignment(&format!("{example}\n{}", successful_sequence()));
        let output = crate::support::validator_child_lane_ownership(&evidence)?;
        assert!(output.status.success(), "must ignore inert example: {example}");
    }
    Ok(())
}

#[test]
fn latest_get_goal_result_controls_creation() -> Result<(), Box<dyn std::error::Error>> {
    let events = format!(
        "{}\n{}\n{}\n{}",
        get_transaction(
            "873:get-old",
            "{\"goal\":{\"objective\":\"issue #999\",\"status\":\"active\"}}",
            "parent-873",
        ),
        get_transaction("873:get-null", "{\"goal\":null}", "parent-873"),
        create_transaction("implement issue #873", "873:create", "parent-873"),
        get_transaction(
            "873:get-new",
            "{\"goal\":{\"objective\":\"implement issue #873\",\"status\":\"active\"}}",
            "parent-873",
        )
    );
    let output = crate::support::validator_child_lane_ownership(&clear_child_assignment(&events))?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    Ok(())
}

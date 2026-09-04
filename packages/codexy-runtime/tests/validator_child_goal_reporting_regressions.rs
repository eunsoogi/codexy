use super::validator_child_goal_reporting::{
    AUTHENTIC_OBJECTIVE, authentic_assignment, clear_child_assignment, create_transaction,
    get_transaction, successful_sequence,
};

fn validate(evidence: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(crate::support::validator_child_lane_ownership(evidence)?)
}

#[test]
fn semicolon_bearing_authentic_objective_is_bound_end_to_end()
-> Result<(), Box<dyn std::error::Error>> {
    let valid = authentic_assignment();
    let output = validate(&valid)?;
    assert!(
        output.status.success(),
        "authentic assignment must validate: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let broadened = format!("{AUTHENTIC_OBJECTIVE} publish release artifacts");
    let result_marker = format!(
        "exact tool result={{\"goal\":{{\"objective\":\"{AUTHENTIC_OBJECTIVE}\",\"status\":\"active\"}}}}; parent task=parent-873"
    );
    let broadened_result = valid.replacen(
        &result_marker,
        &format!(
            "exact tool result={{\"goal\":{{\"objective\":\"{broadened}\",\"status\":\"active\"}}}}; parent task=parent-873"
        ),
        1,
    );
    assert!(!validate(&broadened_result)?.status.success());

    let objective_marker = format!("\"objective\":\"{AUTHENTIC_OBJECTIVE}\"");
    let broadened_readback = valid
        .rsplit_once(&objective_marker)
        .map(|(prefix, suffix)| {
            format!("{prefix}\"objective\":\"{broadened}\"{suffix}")
        })
        .expect("active readback objective fixture");
    assert!(!validate(&broadened_readback)?.status.success());
    Ok(())
}

#[test]
fn authorization_state_drives_the_whole_sequence() -> Result<(), Box<dyn std::error::Error>> {
    let output = validate(&clear_child_assignment(""))?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("clear delegated assignment requires an actual get_goal tool call"));

    let ambiguous = format!(
        "Discussion note: perhaps issue #873 could be explored later.\n\
         Authorized goal objective: implement issue #873\n{}",
        successful_sequence()
    );
    let output = validate(&ambiguous)?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("create_goal requires a clear delegated assignment authorization"));

    let read_only = get_transaction("873:get", "{\"goal\":null}", "parent-873");
    let output = validate(&format!("Discussion note: maybe later.\n{read_only}"))?;
    assert!(output.status.success(), "a read-only get must not authorize creation");
    Ok(())
}

#[test]
fn authorized_record_cannot_broaden_the_assignment() -> Result<(), Box<dyn std::error::Error>> {
    let evidence = clear_child_assignment(&successful_sequence()).replace(
        "Authorized goal objective: implement issue #873",
        "Authorized goal objective: implement issue #873 and publish release artifacts",
    );
    let output = validate(&evidence)?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("authorized goal objective must exactly match the assignment objective"));
    Ok(())
}

#[test]
fn clear_assignment_requires_success_criteria_and_bound_control_state()
-> Result<(), Box<dyn std::error::Error>> {
    for fragment in [
        "Success criteria: focused regression coverage and stop before merge\n",
        "Goal control state: source_thread_id=parent-873\n",
    ] {
        let output = validate(&clear_child_assignment(&successful_sequence()).replace(fragment, ""))?;
        assert!(!output.status.success(), "missing contract must fail: {fragment}");
    }
    Ok(())
}

#[test]
fn create_transaction_is_source_bound() -> Result<(), Box<dyn std::error::Error>> {
    for evidence in [
        clear_child_assignment(&successful_sequence()).replace(
            "Source thread id: parent-873\n",
            "",
        ),
        clear_child_assignment(&successful_sequence()).replace(
            "Goal tool call: create_goal(objective=implement issue #873); parent task=parent-873",
            "Goal tool call: create_goal(objective=implement issue #873); parent task=wrong-parent",
        ),
    ] {
        let output = validate(&evidence)?;
        assert!(!output.status.success());
    }
    Ok(())
}

#[test]
fn create_transaction_requires_pre_key_and_post_receipts()
-> Result<(), Box<dyn std::error::Error>> {
    let valid = clear_child_assignment(&successful_sequence());
    let cases = [
        valid
            .lines()
            .filter(|line| !line.starts_with("Parent goal pre-delivery: operation=create_goal"))
            .collect::<Vec<_>>()
            .join("\n"),
        valid.replace(
            "next action=create finite goal; transition key=873:create",
            "next action=create finite goal; transition key=873:wrong",
        ),
        valid
            .lines()
            .filter(|line| !line.starts_with("Parent goal post-result: operation=create_goal"))
            .collect::<Vec<_>>()
            .join("\n"),
    ];
    for evidence in cases {
        let output = validate(&evidence)?;
        assert!(!output.status.success());
    }
    Ok(())
}

#[test]
fn create_pre_receipt_requires_all_bound_fields() -> Result<(), Box<dyn std::error::Error>> {
    for fragment in [
        "issue/pr=#873; ",
        "pending objective=implement issue #873; ",
        "plan step=implementation; ",
        "branch=eunsoogi/873-finite-goals; ",
        "worktree=/worktree; ",
        "head=abc873; ",
        "clean/index=clean; ",
        "evidence=clear assignment; ",
        "next action=create finite goal; ",
    ] {
        let output = validate(&clear_child_assignment(
            &successful_sequence().replace(fragment, ""),
        ))?;
        assert!(!output.status.success(), "missing field must fail: {fragment}");
    }
    Ok(())
}

#[test]
fn denial_grammar_handles_cannot_and_scoped_safety_rules()
-> Result<(), Box<dyn std::error::Error>> {
    let output = validate(&clear_child_assignment(
        "Goal tools cannot be used for this implementation.",
    ))?;
    assert!(!output.status.success());

    for rule in [
        "Goal tools must not be used to overwrite an unrelated active goal.",
        "Goal tools must not be used to broaden scope.",
        "Goal tools must not be used to invent work.",
        "Goal tools must not be used to replace external proof.",
    ] {
        let evidence = clear_child_assignment(&format!("{rule}\n{}", successful_sequence()));
        let output = validate(&evidence)?;
        assert!(output.status.success(), "must accept safety rule: {rule}");
    }
    Ok(())
}

#[test]
fn null_state_requires_create_and_active_readback() -> Result<(), Box<dyn std::error::Error>> {
    let get = get_transaction("873:get", "{\"goal\":null}", "parent-873");
    let output = validate(&clear_child_assignment(&get))?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("requires an actual create_goal tool call"));

    let create = create_transaction("implement issue #873", "873:create", "parent-873");
    let output = validate(&clear_child_assignment(&format!("{get}\n{create}")))?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("active get_goal readback"));
    Ok(())
}

#[test]
fn objective_binding_covers_call_post_and_readback() -> Result<(), Box<dyn std::error::Error>> {
    let valid = clear_child_assignment(&successful_sequence());
    for evidence in [
        valid.replace(
            "create_goal(objective=implement issue #873)",
            "create_goal(objective=implement issue #873 and publish release artifacts)",
        ),
        valid.replacen(
            "\"objective\":\"implement issue #873\",\"status\":\"active\"",
            "\"objective\":\"issue #999\",\"status\":\"active\"",
            1,
        ),
        valid.rsplit_once("\"objective\":\"implement issue #873\"")
            .map(|(prefix, suffix)| format!("{prefix}\"objective\":\"issue #999\"{suffix}"))
            .expect("readback objective fixture"),
    ] {
        let output = validate(&evidence)?;
        assert!(!output.status.success());
    }
    Ok(())
}

#[test]
fn duplicate_create_and_contradictory_envelope_fail()
-> Result<(), Box<dyn std::error::Error>> {
    let duplicate = format!(
        "{}\n{}",
        successful_sequence(),
        create_transaction("implement issue #873", "873:create-two", "parent-873")
    );
    let output = validate(&clear_child_assignment(&duplicate))?;
    assert!(!output.status.success());

    let contradictory = get_transaction(
        "873:get",
        "{\"goal\":null,\"status\":\"active\"}",
        "parent-873",
    );
    let output = validate(&clear_child_assignment(&contradictory))?;
    assert!(!output.status.success());
    Ok(())
}

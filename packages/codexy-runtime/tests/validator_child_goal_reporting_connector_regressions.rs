use super::validator_child_goal_reporting::{clear_child_assignment, get_transaction, successful_sequence};

fn validate(evidence: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(crate::support::validator_child_lane_ownership(evidence)?)
}

#[test]
fn malformed_create_call_is_not_silently_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let active = get_transaction(
        "873:get",
        "{\"goal\":{\"objective\":\"implement issue #873\",\"status\":\"active\"}}",
        "parent-873",
    );
    let malformed = "Goal tool call: create_goal; parent task=parent-873; transition key=873:create";
    let output = validate(&clear_child_assignment(&format!("{active}\n{malformed}")))?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("malformed"));
    Ok(())
}

#[test]
fn null_goal_with_non_string_status_is_invalid() -> Result<(), Box<dyn std::error::Error>> {
    let evidence = clear_child_assignment(&successful_sequence()).replace(
        "exact tool result={\"goal\":null}; parent task=parent-873",
        "exact tool result={\"goal\":null,\"status\":123}; parent task=parent-873",
    );
    let output = validate(&evidence)?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("authoritative"));
    Ok(())
}

#[test]
fn objective_binding_preserves_case_for_exact_matches() -> Result<(), Box<dyn std::error::Error>> {
    let cased = clear_child_assignment(&successful_sequence())
        .replace("implement issue #873", "Implement issue #873");
    assert!(validate(&cased)?.status.success());

    let mismatched = cased.replacen(
        "Authorized goal objective: Implement issue #873",
        "Authorized goal objective: implement issue #873",
        1,
    );
    let output = validate(&mismatched)?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("exactly match the assignment objective"));
    Ok(())
}

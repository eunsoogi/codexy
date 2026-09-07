use crate::support::TestResult;
use serde_json::{Value, json};

use super::{BASE_OID, HEAD_OID, ISSUE_NUMBER, PR_NUMBER, direct_state, validate};

#[test]
fn completion_handoff_accepts_authenticated_connector_snapshot() -> TestResult {
    let output = validate(connector_pr_state())?;
    assert!(
        output.status.success(),
        "authenticated connector snapshot must pass completion handoff: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn completion_handoff_rejects_connector_provenance_bypass() -> TestResult {
    let mut missing_capture = connector_pr_state();
    missing_capture
        .as_object_mut()
        .expect("connector PR state object")
        .remove("capture");
    let mut missing_source = connector_pr_state();
    missing_source["capture"]
        .as_object_mut()
        .expect("connector capture")
        .remove("source");
    let mut changed_arguments = connector_pr_state();
    changed_arguments["capture"]["source"]["arguments"]["repository_full_name"] =
        json!("other/codexy");
    let mut changed_result = connector_pr_state();
    changed_result["capture"]["source"]["result"]["head_sha"] = json!(BASE_OID);

    for (label, state) in [
        ("missing capture", missing_capture),
        ("missing connector source", missing_source),
        ("changed connector arguments", changed_arguments),
        ("changed connector result", changed_result),
    ] {
        let output = validate(state)?;
        assert!(
            !output.status.success(),
            "{label} must block completion handoff; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn connector_pr_state() -> Value {
    let mut state = direct_state::connector_pr_snapshot(
        PR_NUMBER,
        ISSUE_NUMBER,
        BASE_OID,
        HEAD_OID,
        Some(direct_state::strict_control(ISSUE_NUMBER, HEAD_OID)),
    );
    state["state"] = json!("OPEN");
    state["isDraft"] = json!(true);
    state["mergeStateStatus"] = json!("CLEAN");
    state["reviewProfile"] = json!("strict");
    state
}

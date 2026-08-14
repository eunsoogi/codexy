use super::*;

#[test]
fn completion_handoff_requires_typed_selected_profile_evidence() -> TestResult {
    assert!(!validate(None)?.status.success());
    assert!(validate(Some("light"))?.status.success());
    assert!(!validate(Some("standard"))?.status.success());
    assert!(!validate(Some("strict"))?.status.success());
    for evidence in [
        r#"{"schema":"codexy.review-readiness.v1","head_oid":"stale","profile":"standard","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"state":"passed"}"#,
        r#"{"schema":"codexy.review-readiness.v1","head_oid":"h","profile":"unknown","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"state":"passed"}"#,
        r#"{"schema":"codexy.review-readiness.v1","head_oid":"h","profile":"standard","reviewer":null,"state":"passed"}"#,
        r#"{"schema":"codexy.review-readiness.v1","head_oid":"h","profile":"standard","reviewer":{"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"},"state":"passed"}"#,
    ] {
        assert!(!validate_evidence(evidence)?.status.success());
    }
    Ok(())
}
#[test]
fn completion_handoff_binds_the_terminal_event_of_its_review_ledger() -> TestResult {
    assert!(validate_bound(|_| {})?.status.success());
    for mutate in [
        |state: &mut Value| state["reviewEvidence"]["event_id"] = json!("other"),
        |state: &mut Value| state["reviewLedger"]["events"][0]["head_oid"] = json!("stale"),
        |state: &mut Value| state["reviewLedger"]["events"][0]["state"] = json!("delta"),
        |state: &mut Value| state["reviewLedger"]["events"][1]["base_oid"] = json!("other"),
        |state: &mut Value| state["reviewLedger"]["events"][1]["boundaries"] = json!(["other"]),
        |state: &mut Value| {
            state["reviewLedger"]["events"]
                .as_array_mut()
                .expect("events")
                .remove(0);
        },
    ] {
        assert!(!validate_bound(mutate)?.status.success());
    }
    Ok(())
}

#[test]
fn completion_handoff_rejects_missing_or_empty_typed_review_identity() -> TestResult {
    assert!(
        !validate_bound(|state| {
            state["headRefOid"] = json!("");
            state["reviewEvidence"]["head_oid"] = json!("");
            for event in state["reviewLedger"]["events"]
                .as_array_mut()
                .expect("events")
            {
                event["head_oid"] = json!("");
            }
        })?
        .status
        .success()
    );
    assert!(
        !validate_bound(|state| {
            state.as_object_mut().expect("state").remove("reviewLedger");
        })?
        .status
        .success()
    );
    assert!(
        !validate_bound(|state| state["reviewProfile"] = json!("light"))?
            .status
            .success()
    );
    Ok(())
}

#[test]
fn completion_handoff_accepts_the_recordable_escalated_delta_cycle() -> TestResult {
    assert!(validate_escalated_delta(|_| {})?.status.success());
    Ok(())
}

#[test]
fn completion_handoff_accepts_the_escalated_parent_decision_cycle() -> TestResult {
    assert!(validate_escalated_parent_decision(|_| {})?.status.success());
    for mutate in [
        |state: &mut Value| {
            state["reviewLedger"]["events"][3]["predecessor_event_id"] = json!("e-strict")
        },
        |state: &mut Value| state["reviewLedger"]["events"][3]["head_oid"] = json!("stale"),
        |state: &mut Value| state["reviewLedger"]["events"][3]["base_oid"] = json!("other"),
        |state: &mut Value| state["reviewLedger"]["events"][3]["boundaries"] = json!(["other"]),
    ] {
        assert!(!validate_escalated_parent_decision(mutate)?.status.success());
    }
    Ok(())
}

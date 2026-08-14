use super::*;

#[test]
fn completion_handoff_rejects_invalid_cycle_event_ids() -> TestResult {
    for mutate in [
        |state: &mut Value| {
            state["reviewLedger"]["events"][0]["id"] = json!("");
            state["reviewLedger"]["events"][1]["predecessor_event_id"] = json!("");
        },
        |state: &mut Value| {
            state["reviewLedger"]["events"][0]["id"] = json!("not valid");
            state["reviewLedger"]["events"][1]["predecessor_event_id"] = json!("not valid");
        },
        |state: &mut Value| {
            state["reviewLedger"]["events"][0]["id"] = json!("e-passed");
            state["reviewLedger"]["events"][1]["predecessor_event_id"] = json!("e-passed");
        },
    ] {
        assert!(!validate_bound(mutate)?.status.success());
    }
    Ok(())
}
#[test]
fn completion_handoff_rejects_a_nonlight_zero_review_cycle() -> TestResult {
    assert!(
        !validate_bound(|state| {
            state["reviewLedger"]["events"]
                .as_array_mut()
                .expect("events")
                .remove(0);
            state["reviewLedger"]["events"][0]["predecessor_event_id"] = Value::Null;
            state["reviewLedger"]["events"][0]["full_used"] = json!(0);
            state["reviewLedger"]["events"][0]["delta_used"] = json!(0);
        })?
        .status
        .success()
    );
    Ok(())
}

#[test]
fn completion_handoff_binds_delta_to_the_preceding_full_base() -> TestResult {
    assert!(validate_delta_base(|_| {})?.status.success());
    for mutate in [
        |state: &mut Value| {
            state["reviewLedger"]["events"][1]
                .as_object_mut()
                .expect("delta")
                .remove("base_oid");
        },
        |state: &mut Value| state["reviewLedger"]["events"][1]["base_oid"] = json!("older"),
        |state: &mut Value| state["reviewLedger"]["events"][1]["base_oid"] = json!(""),
    ] {
        assert!(!validate_delta_base(mutate)?.status.success());
    }
    Ok(())
}

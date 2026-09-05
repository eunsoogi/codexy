use serde_json::json;

use crate::support::TestResult;

#[path = "support/review_control_import.rs"]
mod import_support;

use import_support::{
    envelope, event, git_sha, history_event, legacy_event, light_control, run_build, run_import,
    run_producer, snapshot, stderr,
};

#[test]
fn import_preserves_supported_reviewer_migration() -> TestResult {
    let current_head = git_sha("HEAD")?;
    let full_head = git_sha("HEAD^^")?;
    let delta_head = git_sha("HEAD^")?;
    let current = snapshot(&current_head, &delta_head, None);
    let envelope = envelope(vec![
        legacy_event("legacy-full", "full", &full_head, "legacy-turn", 10),
        event("current-delta", "delta", &delta_head, "current-turn", 20),
    ]);
    let (result, state) = run_import(&current, &envelope)?;
    assert!(
        result.status.success(),
        "migration import failed: {}",
        stderr(&result)
    );
    let state = state.expect("successful migration import must write state");
    assert_eq!(
        state["reviewControl"]["reviewer_migration"]["history_boundary"],
        1
    );
    assert_eq!(
        state["reviewControl"]["terminal_review_history"][0]["reviewer"]["model"],
        "gpt-5.6-terra"
    );
    assert_eq!(
        state["reviewControl"]["terminal_review_history"][1]["reviewer"]["model"],
        "gpt-5.6-sol"
    );
    Ok(())
}

#[test]
fn import_orders_ordinals_within_each_source_task() -> TestResult {
    let current_head = git_sha("HEAD")?;
    let full_head = git_sha("HEAD^^")?;
    let delta_head = git_sha("HEAD^")?;
    let current = snapshot(&current_head, &delta_head, None);
    let mut events = vec![
        event("full-source", "full", &full_head, "full-turn", 147),
        event("delta-source", "delta", &delta_head, "delta-turn", 2),
    ];
    events[0]["thread_id"] = json!("full-source-task");
    events[1]["thread_id"] = json!("replacement-source-task");
    let (result, state) = run_import(&current, &envelope(events))?;
    assert!(
        result.status.success(),
        "task-local ordering failed: {}",
        stderr(&result)
    );
    let state = state.expect("successful task-local import must write state");
    assert_eq!(
        state["reviewControl"]["pre_pr_import"]["events"][0]["ordinal"],
        147
    );
    assert_eq!(
        state["reviewControl"]["pre_pr_import"]["events"][1]["ordinal"],
        2
    );

    let invalid = envelope(vec![
        event("same-full", "full", &full_head, "same-task-full", 10),
        event("same-delta", "delta", &delta_head, "same-task-delta", 10),
    ]);
    let (result, _) = run_import(&current, &invalid)?;
    assert!(
        !result.status.success(),
        "same-task ordinal regression must be rejected"
    );
    Ok(())
}

#[test]
fn ordinary_transition_inherits_import_marker() -> TestResult {
    let current_head = git_sha("HEAD")?;
    let reviewed_head = git_sha("HEAD^")?;
    let imported = snapshot(&current_head, &reviewed_head, None);
    let (_, imported_state) = run_import(
        &imported,
        &envelope(vec![event(
            "msg-full",
            "full",
            &reviewed_head,
            "turn-full",
            1,
        )]),
    )?;
    let previous = imported_state.expect("import state");
    let current = snapshot(&current_head, &current_head, None);
    let mut control = previous["reviewControl"].clone();
    control["reviewed_head"] = json!(current_head);
    control["delta_review_count"] = json!(1);
    control["terminal_review_count"] = json!(2);
    control["terminal_review_history"] = json!([
        previous["reviewControl"]["terminal_review_history"][0].clone(),
        history_event("msg-delta", "delta", &current_head)
    ]);
    let (result, state) = run_build(&current, &control, &previous)?;
    assert!(
        result.status.success(),
        "transition failed: {}",
        stderr(&result)
    );
    assert_eq!(
        state.expect("transition state")["reviewControl"]["pre_pr_import"],
        previous["reviewControl"]["pre_pr_import"]
    );

    let mut removed = control.clone();
    removed
        .as_object_mut()
        .expect("control object")
        .remove("pre_pr_import");
    let (result, _) = run_build(&current, &removed, &previous)?;
    assert!(
        !result.status.success(),
        "removed import marker must be rejected"
    );
    let mut changed = control;
    changed["pre_pr_import"]["complete"] = json!(false);
    let (result, _) = run_build(&current, &changed, &previous)?;
    assert!(
        !result.status.success(),
        "changed import marker must be rejected"
    );
    Ok(())
}

#[test]
fn ordinary_routes_reject_light_history_reset() -> TestResult {
    let current_head = git_sha("HEAD")?;
    let reviewed_head = git_sha("HEAD^")?;
    let current = snapshot(&current_head, &reviewed_head, None);
    let (_, imported_state) = run_import(
        &current,
        &envelope(vec![event(
            "msg-full",
            "full",
            &reviewed_head,
            "turn-full",
            1,
        )]),
    )?;
    let previous = imported_state.expect("import state");
    let light = light_control();
    let (result, _) = run_build(&current, &light, &previous)?;
    assert!(
        !result.status.success(),
        "build must reject light history reset"
    );
    let (result, _) = run_producer(&current, &light, &previous)?;
    assert!(
        !result.status.success(),
        "producer must reject light history reset"
    );
    Ok(())
}

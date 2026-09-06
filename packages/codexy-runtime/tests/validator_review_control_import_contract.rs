use serde_json::json;

use crate::support::TestResult;

#[path = "support/review_control_import.rs"]
mod import_support;

use import_support::{
    envelope, event, git_sha, history_event, legacy_event, run_build, run_import, snapshot, stderr,
};

#[test]
fn import_preserves_current_pr_identity_and_host_receipt_refs() -> TestResult {
    let current_head = git_sha("HEAD")?;
    let reviewed_head = git_sha("HEAD^")?;
    let current = snapshot(&current_head, &reviewed_head, None);
    let envelope = envelope(vec![event(
        "msg-full",
        "full",
        &reviewed_head,
        "turn-full",
        147,
    )]);
    let (result, state) = run_import(&current, &envelope)?;
    assert!(
        result.status.success(),
        "import failed: {}",
        stderr(&result)
    );
    let state = state.expect("successful import must write state");
    assert_eq!(state["number"], json!(942));
    assert_eq!(state["url"], "https://github.com/eunsoogi/codexy/pull/942");
    assert_eq!(state["baseRefOid"], reviewed_head);
    assert_eq!(state["headRefOid"], current_head);
    assert_eq!(state["reviewControl"]["issue_number"], json!(946));
    assert_eq!(state["reviewControl"]["reviewed_head"], reviewed_head);
    assert_eq!(
        state["reviewControl"]["terminal_review_history"][0]["id"],
        "msg-full"
    );
    assert_eq!(
        state["reviewControl"]["pre_pr_import"]["events"][0]["turn_id"],
        "turn-full"
    );
    Ok(())
}

#[test]
fn import_accepts_one_all_legacy_full_receipt() -> TestResult {
    assert_legacy_prefix(1)
}

#[test]
fn import_accepts_two_all_legacy_receipts() -> TestResult {
    assert_legacy_prefix(2)
}

fn assert_legacy_prefix(boundary: usize) -> TestResult {
    let current_head = git_sha("HEAD")?;
    let full_head = git_sha("HEAD^^")?;
    let delta_head = git_sha("HEAD^")?;
    let current = snapshot(&current_head, &delta_head, None);
    let events = match boundary {
        1 => vec![legacy_event("legacy-full", "full", &full_head, "full-turn", 10)],
        2 => vec![
            legacy_event("legacy-full", "full", &full_head, "full-turn", 10),
            legacy_event("legacy-delta", "delta", &delta_head, "delta-turn", 20),
        ],
        _ => unreachable!("test fixture boundary"),
    };
    let (result, state) = run_import(&current, &envelope(events.clone()))?;
    assert!(
        result.status.success(),
        "contiguous legacy prefix failed: {}",
        stderr(&result)
    );
    let state = state.expect("successful legacy import must write state");
    assert_eq!(
        state["reviewControl"]["reviewer_migration"]["history_boundary"],
        json!(boundary)
    );
    for (index, event) in events.iter().enumerate() {
        assert_eq!(
            state["reviewControl"]["terminal_review_history"][index]["id"],
            event["id"]
        );
        assert_eq!(
            state["reviewControl"]["terminal_review_history"][index]["reviewer"],
            event["reviewer"]
        );
    }
    Ok(())
}

#[test]
fn ordinary_transition_consumes_an_all_legacy_prefix() -> TestResult {
    let current_head = git_sha("HEAD")?;
    let reviewed_head = git_sha("HEAD^")?;
    let current = snapshot(&current_head, &reviewed_head, None);
    let (_, imported) = run_import(
        &current,
        &envelope(vec![legacy_event(
            "legacy-full",
            "full",
            &reviewed_head,
            "legacy-turn",
            10,
        )]),
    )?;
    let previous = imported.expect("legacy import state");
    let mut control = previous["reviewControl"].clone();
    control["reviewed_head"] = json!(current_head);
    control["terminal_result"] = json!("PASS");
    control["full_review_count"] = json!(1);
    control["delta_review_count"] = json!(1);
    control["terminal_review_count"] = json!(2);
    control["terminal_review_history"] = json!([
        previous["reviewControl"]["terminal_review_history"][0].clone(),
        history_event("current-delta", "delta", &current_head)
    ]);
    let (result, state) = run_build(&current, &control, &previous)?;
    assert!(
        result.status.success(),
        "ordinary transition rejected all-legacy prefix: {}",
        stderr(&result)
    );
    assert_eq!(
        state.expect("ordinary transition state")["reviewControl"]["reviewer_migration"][
            "history_boundary"
        ],
        1
    );
    Ok(())
}

#[test]
fn import_rejects_a_legacy_event_after_current_history() -> TestResult {
    let current_head = git_sha("HEAD")?;
    let full_head = git_sha("HEAD^")?;
    let delta_head = git_sha("HEAD")?;
    let current = snapshot(&current_head, &full_head, None);
    let (result, _) = run_import(
        &current,
        &envelope(vec![
            event("current-full", "full", &full_head, "current-turn", 10),
            legacy_event("legacy-delta", "delta", &delta_head, "legacy-turn", 20),
        ]),
    )?;
    assert!(
        !result.status.success(),
        "legacy reviewer after the current reviewer must be rejected"
    );
    Ok(())
}

#[test]
fn import_rejects_incomplete_duplicate_and_reordered_receipts() -> TestResult {
    let current_head = git_sha("HEAD")?;
    let first = git_sha("HEAD^^")?;
    let second = git_sha("HEAD^")?;
    let current = snapshot(&current_head, &second, None);
    let full = event("msg-full", "full", &first, "turn-full", 10);
    let delta = event("msg-delta", "delta", &second, "turn-delta", 20);
    let valid = envelope(vec![full, delta]);
    let mut incomplete = valid.clone();
    incomplete["complete"] = json!(false);
    let mut duplicate = valid.clone();
    duplicate["events"][1]["id"] = json!("msg-full");
    let mut reordered = valid;
    reordered["events"][0]["kind"] = json!("delta");
    for (label, candidate) in [
        ("incomplete", incomplete),
        ("duplicate", duplicate),
        ("reordered", reordered),
    ] {
        let (result, _) = run_import(&current, &candidate)?;
        assert!(!result.status.success(), "{label} receipt must be rejected");
    }
    Ok(())
}

#[test]
fn import_rejects_wrong_issue_and_existing_history() -> TestResult {
    let current_head = git_sha("HEAD")?;
    let reviewed_head = git_sha("HEAD^")?;
    let current = snapshot(&current_head, &reviewed_head, None);
    let mut wrong_issue = envelope(vec![event(
        "msg-full",
        "full",
        &reviewed_head,
        "turn-full",
        1,
    )]);
    wrong_issue["issue"]["number"] = json!(945);
    wrong_issue["issue"]["url"] = json!("https://github.com/eunsoogi/codexy/issues/945");
    let (result, _) = run_import(&current, &wrong_issue)?;
    assert!(
        !result.status.success(),
        "wrong owning issue must be rejected"
    );

    let existing = snapshot(&current_head, &reviewed_head, Some(json!({"history": []})));
    let (result, _) = run_import(
        &existing,
        &envelope(vec![event(
            "msg-full",
            "full",
            &reviewed_head,
            "turn-full",
            1,
        )]),
    )?;
    assert!(
        !result.status.success(),
        "genesis import must not replace existing history"
    );
    Ok(())
}

use std::fs;

use crate::support::TestResult;
use serde_json::json;

#[path = "support/review_control_direct_state.rs"]
mod direct_state;
#[path = "support/final_disposition.rs"]
mod final_support;

const ISSUE: u64 = 993;

#[test]
fn repaired_head_final_disposition_uses_producer_and_preserves_three_events() -> TestResult {
    let control = final_support::repaired_head_control(ISSUE);
    let previous = final_support::third_block_predecessor(&control);
    let produced = final_support::produce(&control, &previous)?;

    assert_eq!(produced["terminal_result"], "BLOCK");
    assert_eq!(produced["terminal_review_count"], 3);
    assert_eq!(produced["terminal_review_history"].as_array().map(Vec::len), Some(3));
    assert!(produced.get("final_disposition").is_some());
    assert_eq!(
        produced["unresolved_findings"],
        produced["terminal_review_history"][2]["unresolved_findings"]
    );
    assert_eq!(
        produced["final_disposition"]["remaining_finding_ids"],
        json!([])
    );
    assert_eq!(produced["final_disposition"]["kind"], "third_block_repair");

    let handoff = final_support::validate_handoff(
        &final_support::repaired_head_control(ISSUE),
        ISSUE,
        direct_state::SYNTHETIC_CURRENT_HEAD,
    )?;
    assert!(
        handoff.status.success(),
        "repaired-head final disposition must pass direct handoff: {}",
        String::from_utf8_lossy(&handoff.stderr)
    );
    Ok(())
}

#[test]
fn repaired_head_final_disposition_uses_build_pr_state() -> TestResult {
    let control = final_support::repaired_head_control(ISSUE);
    let previous = final_support::third_block_predecessor(&control);
    let state = final_support::build_pr_state(&control, &previous)?;

    assert_eq!(
        state["headRefOid"],
        state["reviewControl"]["final_disposition"]["head_oid"]
    );
    assert_eq!(
        state["reviewControl"]["reviewed_head"],
        state["reviewControl"]["terminal_review_history"][2]["reviewed_head"]
    );
    assert_eq!(state["reviewControl"]["terminal_result"], "BLOCK");
    assert_eq!(state["reviewControl"]["terminal_review_count"], 3);
    assert_eq!(
        state["reviewControl"]["terminal_review_history"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );
    Ok(())
}

#[test]
fn same_head_evidence_refresh_is_accepted_without_a_fourth_event() -> TestResult {
    let control = final_support::same_head_control(ISSUE);
    let previous = final_support::third_block_predecessor(&control);
    let produced = final_support::produce(&control, &previous)?;

    assert_eq!(produced["terminal_result"], "BLOCK");
    assert_eq!(produced["terminal_review_count"], 3);
    assert_eq!(
        produced["terminal_review_history"].as_array().map(Vec::len),
        Some(3)
    );
    assert_eq!(
        produced["final_disposition"]["evidence_refresh"]["proof_head"],
        produced["reviewed_head"]
    );
    let handoff = final_support::validate_handoff(
        &produced,
        ISSUE,
        produced["final_disposition"]["head_oid"]
            .as_str()
            .ok_or("produced final head")?,
    )?;
    assert!(
        handoff.status.success(),
        "same-head evidence refresh must pass handoff: {}",
        String::from_utf8_lossy(&handoff.stderr)
    );
    Ok(())
}

#[test]
fn final_disposition_rejects_history_rewrite_or_incomplete_finding_partition() -> TestResult {
    let control = final_support::repaired_head_control(ISSUE);
    let previous = final_support::third_block_predecessor(&control);

    let mut rewritten = control.clone();
    rewritten["terminal_review_history"][2]["terminal_result"] = json!("PASS");
    assert!(final_support::produce(&rewritten, &previous).is_err());

    let mut incomplete = control;
    incomplete["final_disposition"]["source_repair"]["finding_ids"] = json!([]);
    assert!(final_support::produce(&incomplete, &previous).is_err());
    Ok(())
}

#[test]
fn final_disposition_rejects_fourth_count_and_wrong_base() -> TestResult {
    let control = final_support::repaired_head_control(ISSUE);
    let previous = final_support::third_block_predecessor(&control);

    let mut fourth = control.clone();
    fourth["terminal_review_count"] = json!(4);
    assert!(final_support::produce(&fourth, &previous).is_err());

    let produced = final_support::produce(&control, &previous)?;
    let mut wrong_base = produced;
    wrong_base["final_disposition"]["base_oid"] = json!(direct_state::SYNTHETIC_UPDATED_BASE);
    let handoff = final_support::validate_handoff(
        &wrong_base,
        ISSUE,
        wrong_base["final_disposition"]["head_oid"]
            .as_str()
            .ok_or("wrong final head")?,
    )?;
    assert!(!handoff.status.success());
    Ok(())
}

#[test]
fn final_disposition_requires_authenticated_authority_and_locator() -> TestResult {
    let control = final_support::repaired_head_control(ISSUE);
    let previous = final_support::third_block_predecessor(&control);

    let mut caller_authority = control.clone();
    caller_authority["final_disposition"]["authority"] = json!({"schema": "forged"});
    assert!(final_support::produce(&caller_authority, &previous).is_err());

    let missing_locator = final_support::produce_without_locator(&control, &previous)?;
    assert!(!missing_locator.status.success());
    Ok(())
}

#[test]
fn final_disposition_rejects_pending_ci_source() -> TestResult {
    let control = final_support::repaired_head_control(ISSUE);
    let previous = final_support::third_block_predecessor(&control);
    let output = final_support::produce_with_fixture_mutation(
        &control,
        &previous,
        ISSUE,
        |fixture| {
            let mut response: serde_json::Value =
                serde_json::from_slice(&fs::read(&fixture.ci)?)?;
            response["statusCheckRollup"][0]["status"] = json!("IN_PROGRESS");
            response["statusCheckRollup"][0]["conclusion"] = json!(null);
            fs::write(&fixture.ci, serde_json::to_vec(&response)?)?;
            Ok(())
        },
    )?;
    assert!(!output.status.success());
    Ok(())
}

#[test]
fn final_disposition_rejects_draft_missing_threads_and_unresolved_threads() -> TestResult {
    let control = final_support::repaired_head_control(ISSUE);
    let previous = final_support::third_block_predecessor(&control);
    let produced = final_support::produce(&control, &previous)?;
    let head = produced["final_disposition"]["head_oid"]
        .as_str()
        .ok_or("produced final head")?;

    let draft = final_support::validate_handoff_with_state(
        &produced,
        ISSUE,
        head,
        |state| {
            state["isDraft"] = json!(true);
            Ok(())
        },
    )?;
    assert!(!draft.status.success());

    let missing_threads = final_support::validate_handoff_with_state(
        &produced,
        ISSUE,
        head,
        |state| {
            state
                .as_object_mut()
                .ok_or("handoff state object")?
                .remove("reviewThreads");
            Ok(())
        },
    )?;
    assert!(!missing_threads.status.success());

    let unresolved_threads = final_support::validate_handoff_with_state(
        &produced,
        ISSUE,
        head,
        |state| {
            state["reviewThreads"] = json!({
                "pageInfo": {"hasNextPage": false},
                "nodes": [{
                    "id": "thread-open",
                    "isResolved": false,
                    "isOutdated": false,
                    "path": "src/lib.rs",
                    "comments": {"nodes": [{"url": "https://github.com/eunsoogi/codexy/pull/993#discussion_r1"}]}
                }]
            });
            Ok(())
        },
    )?;
    assert!(!unresolved_threads.status.success());
    Ok(())
}

#[test]
fn final_disposition_rejects_reverted_and_out_of_scope_source_repairs() -> TestResult {
    let base = final_support::repaired_head_control(ISSUE);
    let previous = final_support::third_block_predecessor(&base);

    for head in [
        direct_state::SYNTHETIC_REVERTED_CURRENT_HEAD,
        direct_state::SYNTHETIC_OUT_OF_SCOPE_CURRENT_HEAD,
    ] {
        let mut control = base.clone();
        control["final_disposition"]["head_oid"] = json!(head);
        assert!(final_support::produce(&control, &previous).is_err());
    }
    Ok(())
}

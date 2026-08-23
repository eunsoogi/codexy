use serde_json::json;

pub(crate) mod support {
    pub(crate) use super::super::session_audit_stage_budget::support::{
        TestResult, rejected, refresh_receipt, report, validate,
    };
    pub(crate) use super::super::stage_budget_test_support::{
        declare, fixture, oversized, previous_anchor, set,
    };
    use serde_json::{Value, json};

    pub(crate) fn continuation(previous: &Value) -> Value {
        let mut next = fixture();
        set(&mut next, "stageSequence", json!(2));
        set(
            &mut next,
            "previousReceiptIdentity",
            previous["receiptIdentity"].clone(),
        );
        set(&mut next, "continuity.previous", previous_anchor(previous));
        set(&mut next, "usage", previous["usage"].clone());
        set(&mut next, "identity.volatile", json!("event-2"));
        set(&mut next, "events.identities", json!(["event-2"]));
        next["owner"] = previous["owner"].clone();
        next["identity"]["stable"] = previous["identity"]["stable"].clone();
        next["safety"] = previous["safety"].clone();
        next["proof"] = previous["proof"].clone();
        next["limits"] = previous["limits"].clone();
        next["measures"]["toolOutputBytes"]["value"] = previous["usage"]["toolOutputBytes"].clone();
        declare(&mut next, "compact");
        next
    }

    pub(crate) fn stage_receipt(stage: &str) -> Value {
        let mut receipt = fixture();
        receipt["stage"] = json!(stage);
        let owner = match stage {
            "root-planning" | "parent-integration" => "root",
            "selected-review" | "wait" => "selected-reviewer",
            _ => "child",
        };
        receipt["owner"]["kind"] = json!(owner);
        if stage == "selected-review" || stage == "wait" {
            receipt["safety"]["selectedReviewerState"] = json!("running");
            receipt["safety"]["externalGate"] = json!("pending");
        }
        declare(&mut receipt, "continue");
        receipt
    }
}

use support as budget;
type TestResult = support::TestResult;

#[test]
fn continuation_rejects_prior_identity_changes_and_history_replay() -> TestResult {
    let mut previous = budget::fixture();
    budget::set(&mut previous, "usage.contextBytes", json!(900));
    budget::declare(&mut previous, "compact");
    budget::refresh_receipt(&mut previous);
    for (path, value) in [
        ("identity.stable", json!("stage-999")),
        ("identity.volatile", json!("event-1")),
        ("safety.issuePrIdentity.issue", json!("999")),
        ("safety.ownerWorktree.branch", json!("eunsoogi-999")),
        ("safety.ownerWorktree.ownerThreadId", json!("thread-999")),
        ("owner.id", json!("child-999")),
        ("proof.goal", json!("complete")),
    ] {
        let mut changed = budget::continuation(&previous);
        budget::set(&mut changed, path, value);
        budget::rejected(&mut changed)?;
    }

    let mut prior_with_history = previous.clone();
    prior_with_history["events"]["identities"] = json!(["event-1", "prior-event"]);
    budget::refresh_receipt(&mut prior_with_history);
    let mut replay = budget::continuation(&prior_with_history);
    budget::set(
        &mut replay,
        "continuity.previous",
        budget::previous_anchor(&prior_with_history),
    );
    budget::set(&mut replay, "events.identities", json!(["prior-event"]));
    budget::set(&mut replay, "identity.volatile", json!("event-2"));
    budget::rejected(&mut replay)?;

    let mut tampered = budget::continuation(&prior_with_history);
    let mut prior = budget::previous_anchor(&prior_with_history);
    prior["events"]["identities"] = json!(["event-1", "tampered"]);
    budget::set(&mut tampered, "continuity.previous", prior);
    budget::rejected(&mut tampered)?;
    Ok(())
}

#[test]
fn continuation_rejects_resets_and_oversized_prior_metadata_tamper() -> TestResult {
    let mut previous = budget::fixture();
    budget::set(&mut previous, "usage.contextBytes", json!(900));
    budget::declare(&mut previous, "compact");
    budget::refresh_receipt(&mut previous);
    let mut reset = budget::continuation(&previous);
    budget::set(&mut reset, "usage.contextBytes", json!(100));
    budget::rejected(&mut reset)?;

    let mut prior = previous.clone();
    budget::oversized(&mut prior, "tool-output", "tool-output-prior", 1001, "replay-blocked");
    budget::declare(&mut prior, "stop_and_handoff");
    budget::refresh_receipt(&mut prior);
    let mut valid = budget::continuation(&prior);
    budget::set(&mut valid, "continuity.previous", budget::previous_anchor(&prior));
    budget::declare(&mut valid, "stop_and_handoff");
    assert!(budget::validate(&mut valid)?);

    let mut tampered = budget::continuation(&prior);
    let mut embedded = budget::previous_anchor(&prior);
    budget::set(&mut embedded, "oversizedResult.bytes", json!(1002));
    budget::set(&mut tampered, "continuity.previous", embedded);
    budget::declare(&mut tampered, "stop_and_handoff");
    budget::rejected(&mut tampered)?;

    let mut bounded = budget::continuation(&previous);
    let mut embedded = budget::previous_anchor(&previous);
    budget::set(
        &mut embedded,
        "events.identities",
        json!((0..257).map(|i| format!("event-{i}")).collect::<Vec<_>>()),
    );
    budget::set(&mut bounded, "continuity.previous", embedded);
    budget::rejected(&mut bounded)
}

#[test]
fn terminal_review_passes_handoff_and_preserve_reviewer_wait_owner() -> TestResult {
    let mut low_budget = budget::stage_receipt("selected-review");
    budget::set(&mut low_budget, "usage.contextBytes", json!(999));
    budget::set(&mut low_budget, "safety.selectedReviewerState", json!("pass"));
    budget::set(&mut low_budget, "safety.externalGate", json!("pass"));
    budget::declare(&mut low_budget, "stop_and_handoff");
    budget::report(&mut low_budget)?;

    let mut selected = budget::stage_receipt("selected-review");
    budget::set(&mut selected, "safety.selectedReviewerState", json!("pass"));
    budget::set(&mut selected, "safety.externalGate", json!("pass"));
    budget::declare(&mut selected, "stop_and_handoff");
    budget::report(&mut selected)?;

    let mut waiting = budget::continuation(&selected);
    budget::set(&mut waiting, "stage", json!("wait"));
    budget::set(&mut waiting, "safety.selectedReviewerState", json!("pass"));
    budget::set(&mut waiting, "safety.externalGate", json!("pass"));
    budget::declare(&mut waiting, "stop_and_handoff");
    budget::report(&mut waiting)?;

    let mut changed_owner = waiting.clone();
    budget::set(&mut changed_owner, "owner.id", json!("other-reviewer"));
    assert!(!budget::validate(&mut changed_owner)?);
    let mut again = waiting.clone();
    budget::set(&mut again, "stageSequence", json!(3));
    budget::set(&mut again, "previousReceiptIdentity", waiting["receiptIdentity"].clone());
    budget::set(&mut again, "continuity.previous", budget::previous_anchor(&waiting));
    budget::set(&mut again, "identity.volatile", json!("event-3"));
    budget::set(&mut again, "events.identities", json!(["event-3"]));
    budget::declare(&mut again, "stop_and_handoff");
    budget::report(&mut again)?;
    assert_eq!(again["owner"]["id"], selected["owner"]["id"]);
    Ok(())
}

#[test]
fn reviewer_matrix_rejects_invalid_selected_review_and_root_owner() -> TestResult {
    let mut root = budget::stage_receipt("root-planning");
    budget::report(&mut root)?;
    let mut wrong_owner = root.clone();
    budget::set(&mut wrong_owner, "owner.kind", json!("child"));
    budget::rejected(&mut wrong_owner)?;
    for (path, value) in [
        ("safety.selectedReviewerState", json!("not-applicable")),
        ("safety.externalGate", json!("none")),
        ("nextAction", json!("continue-stage")),
    ] {
        let mut invalid = budget::stage_receipt("selected-review");
        budget::set(&mut invalid, path, value);
        budget::rejected(&mut invalid)?;
    }
    Ok(())
}

#[test]
fn oversized_body_and_closed_units_never_cross_metadata_boundary() -> TestResult {
    let mut oversized = budget::fixture();
    budget::oversized(
        &mut oversized,
        "tool-output",
        "tool-output-replayed",
        1001,
        "replay-blocked",
    );
    budget::set(&mut oversized, "oversizedResult.bodyReplayed", json!(true));
    budget::rejected(&mut oversized)?;

    for (path, unit) in [
        ("units.context", "utf8_bytes_before_serialization"),
        ("units.toolOutput", "utf8_bytes_received"),
    ] {
        let mut wrong = budget::fixture();
        budget::set(&mut wrong, path, json!(unit));
        budget::rejected(&mut wrong)?;
    }
    Ok(())
}

#[test]
fn live_reviewer_budget_exhaustion_preserves_ownership_and_proof() -> TestResult {
    let mut receipt = budget::stage_receipt("selected-review");
    budget::set(&mut receipt, "usage.contextBytes", json!(1000));
    budget::declare(&mut receipt, "stop_and_handoff");
    let result = budget::report(&mut receipt)?;
    assert_eq!(result["nextAction"], "handoff-parent");
    assert_eq!(result["safety"]["selectedReviewerState"], "running");
    assert_eq!(result["safety"]["externalGate"], "pending");
    assert_eq!(result["proof"]["goal"], "active");
    assert_eq!(result["proof"]["plan"], "active");
    Ok(())
}

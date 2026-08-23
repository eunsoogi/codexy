use serde_json::json;

pub(crate) mod support {
    use serde_json::{Value, json};
    use sha2::{Digest as _, Sha256};
    use std::collections::HashSet;
    use std::{fs, process::Command};

    pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    fn execute(value: &Value) -> TestResult<std::process::Output> {
        let temp = tempfile::tempdir()?;
        let input = temp.path().join("stage-budget.json");
        fs::write(&input, serde_json::to_vec(value)?)?;
        Ok(Command::new(env!("CARGO_BIN_EXE_codexy-session-audit"))
            .args(["--stage-budget", input.to_str().unwrap()])
            .output()?)
    }

    pub(crate) fn validate(value: &mut Value) -> TestResult<bool> {
        refresh_receipt(value);
        Ok(execute(value)?.status.success())
    }

    pub(crate) fn report(value: &mut Value) -> TestResult<Value> {
        refresh_receipt(value);
        let output = execute(value)?;
        assert!(output.status.success(), "stage-budget validation failed");
        Ok(serde_json::from_slice(&output.stdout)?)
    }

    pub(crate) fn rejected(value: &mut Value) -> TestResult<()> {
        assert!(!validate(value)?);
        Ok(())
    }

    pub(crate) fn refresh_receipt(value: &mut Value) {
        let ids = value["events"]["identities"].as_array().unwrap();
        let duplicate_events = ids.len() as u64
            - ids.iter().collect::<HashSet<_>>().len() as u64;
        let local = duplicate_events
            + ["unchangedWaits", "fullStateReplays", "oversizedPreviewReads"]
                .into_iter()
                .map(|name| value["events"][name].as_u64().unwrap_or(0))
                .sum::<u64>();
        let prior = value["continuity"]["previous"]["cumulativeReplayEvents"]
            .as_u64()
            .unwrap_or(0);
        let cumulative = if value["stageSequence"] == 1 { local } else { prior + local };
        value["continuity"]["cumulativeReplayEvents"] = json!(cumulative);
        let anchor = (
            &value["stage"], &value["stageSequence"], &value["previousReceiptIdentity"],
            &value["owner"], &value["identity"], &value["safety"], &value["proof"],
            &value["limits"], &value["usage"], &value["events"], &value["oversizedResult"],
            cumulative,
        );
        value["receiptIdentity"] = json!(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&anchor).unwrap())
        ));
    }

}

use super::session_audit_stage_budget_continuity::support as budget;
type TestResult = budget::TestResult;

#[test]
fn stage_budget_receipt_supports_every_stage_and_emits_metadata_only_decision() -> TestResult {
    for stage in [
        "root-planning",
        "child-implementation",
        "repair",
        "selected-review",
        "wait",
        "parent-integration",
    ] {
        let mut receipt = budget::stage_receipt(stage);
        let result = budget::report(&mut receipt)?;
        assert_eq!(result["stage"], stage);
        assert_eq!(result["decision"], receipt["decision"]);
        assert_eq!(result["nextAction"], receipt["nextAction"]);
        assert_eq!(result["metadataOnly"], true);
        assert!(result.get("body").is_none());
    }
    Ok(())
}

#[test]
fn pending_selected_review_waits_for_event() -> TestResult {
    let mut receipt = budget::stage_receipt("selected-review");
    budget::set(&mut receipt, "safety.selectedReviewerState", json!("pending"));
    budget::set(&mut receipt, "safety.externalGate", json!("pending"));
    budget::declare(&mut receipt, "continue");
    let result = budget::report(&mut receipt)?;
    assert_eq!(result["nextAction"], "wait-for-event");
    Ok(())
}

#[test]
fn ordinary_external_wait_remains_nonterminal() -> TestResult {
    let mut receipt = budget::stage_receipt("wait");
    budget::set(&mut receipt, "owner.kind", json!("child"));
    budget::set(&mut receipt, "safety.selectedReviewerState", json!("not-applicable"));
    budget::set(&mut receipt, "safety.externalGate", json!("pending"));
    budget::declare(&mut receipt, "continue");
    let result = budget::report(&mut receipt)?;
    assert_eq!(result["decision"], "continue");
    assert_eq!(result["nextAction"], "wait-for-event");
    Ok(())
}

#[test]
fn receipt_rejects_event_replay_after_one_receipt_gap() -> TestResult {
    let mut first = budget::fixture();
    budget::refresh_receipt(&mut first);
    let mut second = budget::continuation(&first);
    budget::refresh_receipt(&mut second);
    let mut replay = budget::continuation(&second);
    budget::set(&mut replay, "stageSequence", json!(3));
    budget::set(&mut replay, "previousReceiptIdentity", second["receiptIdentity"].clone());
    budget::set(&mut replay, "continuity.previous", budget::previous_anchor(&second));
    budget::set(&mut replay, "identity.volatile", json!("event-3"));
    budget::set(&mut replay, "events.identities", json!(["event-1"]));
    budget::declare(&mut replay, "continue");
    budget::rejected(&mut replay)
}

#[test]
fn duplicate_events_and_replays_consume_budget_without_renewal() -> TestResult {
    let mut receipt = budget::fixture();
    budget::set(
        &mut receipt,
        "events.identities",
        json!(["event-1", "event-1", "event-2"]),
    );
    budget::set(&mut receipt, "events.fullStateReplays", json!(1));
    budget::set(&mut receipt, "limits.replayEvents", json!(3));
    budget::declare(&mut receipt, "compact");
    let result = budget::report(&mut receipt)?;
    assert_eq!(result["decision"], "compact");
    assert_eq!(result["nextAction"], "compact-context");
    assert_eq!(result["accounting"]["duplicateEventCount"], 1);
    assert_eq!(result["accounting"]["replayEvents"], 2);

    let mut exhausted = budget::fixture();
    budget::set(
        &mut exhausted,
        "events.identities",
        json!(["event-1", "event-1"]),
    );
    budget::set(&mut exhausted, "limits.replayEvents", json!(1));
    budget::declare(&mut exhausted, "stop_and_handoff");
    let result = budget::report(&mut exhausted)?;
    assert_eq!(result["nextAction"], "handoff-parent");
    Ok(())
}

#[test]
fn unavailable_measures_preserve_explicit_fallbacks() -> TestResult {
    let result = budget::report(&mut budget::fixture())?;
    for name in ["inputTokens", "cacheInputTokens"] {
        assert_eq!(result["measureAvailability"][name]["state"], "unavailable");
    }
    for name in ["input-tokens", "cache-input-tokens"] {
        assert!(result["accounting"]["measureFallbacks"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["measure"] == name)));
    }

    let mut unavailable = budget::fixture();
    budget::set(&mut unavailable, "usage.toolOutputBytes", json!(0));
    budget::set(
        &mut unavailable,
        "measures.toolOutputBytes",
        json!({"availability":"unavailable","value":null,"reason":"runtime-not-exposed"}),
    );
    budget::set(
        &mut unavailable,
        "oversizedResult",
        json!({"kind":"tool-output","identity":"tool-output-unavailable","bytes":0,"state":"unavailable","bodyReplayed":false}),
    );
    let result = budget::report(&mut unavailable)?;
    assert_eq!(result["measureAvailability"]["toolOutputBytes"]["state"], "unavailable");
    Ok(())
}

#[test]
fn oversized_history_and_metadata_bounds_are_accounted() -> TestResult {
    let mut history = budget::fixture();
    budget::set(&mut history, "limits.contextBytes", json!(10000));
    budget::set(&mut history, "usage.contextBytes", json!(37302));
    budget::set(&mut history, "events.oversizedPreviewReads", json!(1));
    budget::oversized(&mut history, "history", "history-487", 37302, "replay-blocked");
    budget::declare(&mut history, "stop_and_handoff");
    budget::report(&mut history)?;

    let mut false_positive = budget::fixture();
    budget::set(&mut false_positive, "events.oversizedPreviewReads", json!(1));
    budget::oversized(
        &mut false_positive,
        "history",
        "history-small",
        1,
        "replay-blocked",
    );
    budget::rejected(&mut false_positive)?;

    let mut mismatch = budget::fixture();
    budget::set(&mut mismatch, "limits.toolOutputBytes", json!(1000));
    budget::oversized(&mut mismatch, "tool-output", "tool-small", 1001, "oversized");
    budget::set(&mut mismatch, "usage.toolOutputBytes", json!(1000));
    budget::set(&mut mismatch, "measures.toolOutputBytes.value", json!(1000));
    budget::declare(&mut mismatch, "stop_and_handoff");
    budget::rejected(&mut mismatch)?;

    let mut metadata = budget::fixture();
    budget::set(
        &mut metadata,
        "safety.verification",
        json!((0..65).map(|i| format!("check-{i}")).collect::<Vec<_>>()),
    );
    budget::rejected(&mut metadata)
}

#[test]
fn replay_and_percent_boundaries_are_exact() -> TestResult {
    for (bytes, decision) in [
        (790, "continue"),
        (800, "compact"),
        (1000, "stop_and_handoff"),
    ] {
        let mut receipt = budget::fixture();
        budget::set(&mut receipt, "usage.contextBytes", json!(bytes));
        budget::declare(&mut receipt, decision);
        budget::report(&mut receipt)?;
    }
    let mut wait = budget::fixture();
    budget::set(&mut wait, "events.unchangedWaits", json!(1));
    budget::declare(&mut wait, "compact");
    budget::report(&mut wait)?;
    let mut exhausted = budget::fixture();
    budget::set(&mut exhausted, "events.unchangedWaits", json!(10));
    budget::set(&mut exhausted, "limits.replayEvents", json!(10));
    budget::declare(&mut exhausted, "stop_and_handoff");
    budget::report(&mut exhausted)?;
    Ok(())
}

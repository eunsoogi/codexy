//! Requirement-linked delivery routing regressions.

use super::*;

// The parent pair is independent of both assigned Worker pairs.
const PARENT: (&str, &str) = ("gpt-6.2-sol", "low");

#[test]
fn child_reports_construct_the_assigned_parent_request() -> TestResult {
    for report in ["progress", "decision", "blocker", "final"] {
        let mut input = request("general", PARENT);
        input["schema"] = json!("codexy.child-routing-request.v3");
        input["codex_thread_operation"] = json!("send_message_to_thread");
        input["codex_thread_direction"] = json!("child_to_root");
        input["parent_thread_id"] = json!("assigned-parent");
        input["report_prompt"] = json!(report);
        let result = resolve(&input)?;
        assert_eq!(result["tool_input"], json!({"threadId":"assigned-parent", "prompt":report, "model":PARENT.0, "thinking":PARENT.1}));
    }
    let mut missing = request("general", PARENT);
    missing["schema"] = json!("codexy.child-routing-request.v3");
    missing["codex_thread_operation"] = json!("send_message_to_thread");
    missing["codex_thread_direction"] = json!("child_to_root");
    missing["report_prompt"] = json!("retained report");
    let result = resolve(&missing)?;
    assert_eq!(result["reason"], "Child report requires an assigned parent thread ID.");
    assert!(result.get("tool_input").is_none());
    missing["parent_thread_id"] = json!("assigned-parent");
    missing["parent_host_id"] = json!("local");
    missing["named_specialist"] = json!("codexy-architect");
    assert_eq!(resolve(&missing)?["tool_input"]["hostId"], "local");
    missing["codex_thread_capabilities"] = request("general", DEFAULT_WORKER)["codex_thread_capabilities"].clone();
    let unavailable = resolve(&missing)?;
    assert_eq!(unavailable["reason"], "Parent model and thinking are not supported by declared thread capabilities.");
    assert!(unavailable.get("tool_input").is_none());
    Ok(())
}

#[test]
fn resolver_allows_only_creation_pairs_for_worker_delivery() -> TestResult {
    for pair in [
        DEFAULT_WORKER,
        HARD_WORKER,
        ("gpt-6.1-sol", "medium"),
        SOL,
        PARENT,
        ("gpt-6.2-sol", "max"),
        ("gpt-6-astra", "medium"),
    ] {
        let mut followup = bounded_request();
        followup["schema"] = json!("codexy.child-routing-request.v2");
        followup["codex_thread_operation"] = json!("send_message_to_thread");
        followup["codex_thread_direction"] = json!("parent_to_generic");
        followup["named_specialist"] = json!("codexy-architect");
        followup["recipient_assignment"] = json!({"model":pair.0,"thinking":pair.1});
        followup["codex_thread_capabilities"] =
            request("general", pair)["codex_thread_capabilities"].clone();
        let expected = if [DEFAULT_WORKER, HARD_WORKER].contains(&pair) {
            selected("parent_to_generic", "send_message_to_thread", pair)
        } else {
            json!({"route":"root_or_named_specialist","reason":"Assigned recipient model and thinking do not form a supported Worker pair."})
        };
        check(followup, expected)?;
    }
    let mut crossed = bounded_request();
    crossed["schema"] = json!("codexy.child-routing-request.v2");
    crossed["codex_thread_operation"] = json!("send_message_to_thread");
    crossed["codex_thread_direction"] = json!("parent_to_generic");
    crossed["recipient_assignment"] = json!({"model":HARD_WORKER.0,"thinking":HARD_WORKER.1});
    check(
        crossed,
        json!({"route":"root_or_named_specialist","reason":"Assigned recipient model and thinking are not supported by declared thread capabilities."}),
    )?;
    let mut legacy = request("general", DEFAULT_WORKER);
    legacy["codex_thread_operation"] = json!("send_message_to_thread");
    legacy["codex_thread_direction"] = json!("parent_to_generic");
    check(
        legacy,
        json!({"route":"root_or_named_specialist","reason":"Version 1 cannot identify a follow-up recipient; use schema v2 with recipient_assignment."}),
    )?;
    for pair in [
        PARENT,
        ("gpt-6.1-sol", "low"),
        DEFAULT_WORKER,
        SOL,
        ("gpt-6-astra", "medium"),
        ("gpt-6.1-terra", "high"),
    ] {
        let mut to_parent = request("general", pair);
        to_parent["codex_thread_operation"] = json!("send_message_to_thread");
        to_parent["codex_thread_direction"] = json!("child_to_root");
        check(
            to_parent,
            if pair == PARENT {
                selected("child_to_root", "send_message_to_thread", PARENT)
            } else {
                fallback()
            },
        )?;
    }
    let mut sender = request("general", DEFAULT_WORKER);
    sender["model"] = json!(DEFAULT_WORKER.0);
    sender["thinking"] = json!(DEFAULT_WORKER.1);
    rejects(
        sender,
        "Worker routing request contains sender settings; use recipient_assignment",
    )?;
    let mut unsupported = bounded_request();
    unsupported["codex_thread_operation"] = json!("unsupported_operation");
    rejects(
        unsupported,
        "Worker routing request names an unsupported Codex thread operation",
    )?;
    Ok(())
}

#[test]
fn creation_selection_is_preserved_in_recipient_followup() -> TestResult {
    let mut assignments = vec![
        request("general", DEFAULT_WORKER),
        request("simple", DEFAULT_WORKER),
        bounded_request(),
    ];
    for facts in hard_work_cases()? {
        let mut hard = request("general", HARD_WORKER);
        hard["hard_work"] = facts;
        assignments.push(hard);
    }
    for creation in assignments {
        let pair = if creation.get("hard_work").is_some() {
            HARD_WORKER
        } else {
            DEFAULT_WORKER
        };
        check(creation.clone(), selected("generic", "create_thread", pair))?;
        let chosen = resolve(&creation)?;
        let mut followup = request("general", pair);
        followup["schema"] = json!("codexy.child-routing-request.v2");
        followup["codex_thread_operation"] = json!("send_message_to_thread");
        followup["codex_thread_direction"] = json!("parent_to_generic");
        followup["recipient_assignment"] =
            json!({"model":chosen["model"],"thinking":chosen["thinking"]});
        check(
            followup,
            selected("parent_to_generic", "send_message_to_thread", pair),
        )?;
    }
    Ok(())
}

//! Requirement-linked delivery routing regressions.

use super::*;

// The parent pair is independent of both assigned Worker pairs.
const PARENT: (&str, &str) = ("gpt-6.1-sol", "low");

#[test]
fn resolver_keeps_recipient_settings_for_all_worker_delivery_pairs() -> TestResult {
    for pair in [
        LUNA,
        TERRA,
        SOL,
        PARENT,
        ("gpt-6.1-sol", "max"),
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
        let expected = if [LUNA, TERRA, SOL].contains(&pair) {
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
    crossed["recipient_assignment"] = json!({"model":SOL.0,"thinking":SOL.1});
    check(
        crossed,
        json!({"route":"root_or_named_specialist","reason":"Assigned recipient model and thinking are not supported by declared thread capabilities."}),
    )?;
    let mut legacy = request("general", LUNA);
    legacy["codex_thread_operation"] = json!("send_message_to_thread");
    legacy["codex_thread_direction"] = json!("parent_to_generic");
    check(
        legacy,
        json!({"route":"root_or_named_specialist","reason":"Version 1 cannot identify a follow-up recipient; use schema v2 with recipient_assignment."}),
    )?;
    for pair in [
        PARENT,
        LUNA,
        SOL,
        ("gpt-6-astra", "medium"),
        ("gpt-6-luna", "medium"),
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
    let mut sender = request("general", LUNA);
    sender["model"] = json!(LUNA.0);
    sender["thinking"] = json!(LUNA.1);
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
fn issue_1442_creation_selection_is_preserved_in_recipient_followup() -> TestResult {
    let mut assignments = vec![
        request("general", LUNA),
        request("simple", LUNA),
        bounded_request(),
    ];
    for facts in hard_work_cases()? {
        let mut hard = request("general", TERRA);
        hard["hard_work"] = facts;
        assignments.push(hard);
    }
    for creation in assignments {
        let pair = if creation.get("hard_work").is_some() {
            TERRA
        } else {
            LUNA
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

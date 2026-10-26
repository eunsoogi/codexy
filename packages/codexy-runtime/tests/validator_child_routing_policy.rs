//! Requirement-linked Worker routing checks, including #1412 bounded evidence,
//! exact capability admission, specialist precedence and recipient-bound delivery.

use std::{path::PathBuf, process::Command};
use serde_json::{json, Value};
use crate::support::TestResult;

#[path = "validator_child_routing_policy/authorized_dispatch.rs"]
mod authorized_dispatch;

const LUNA: (&str, &str) = ("gpt-6-luna", "max");
const SOL: (&str, &str) = ("gpt-6.1-sol", "medium");

#[test]
fn resolver_preserves_simple_general_specialist_and_fail_closed_routes() -> TestResult {
    check(request("general", SOL), selected("generic", "create_thread", SOL))?;
    check(request("simple", LUNA), selected("generic", "create_thread", LUNA))?;
    let mut specialist = request("general", SOL);
    specialist["named_specialist"] = json!("codexy-architect");
    check(specialist, json!({"route":"named_specialist","agent_type":"codexy-architect"}))?;
    for classification in ["simple", "bounded_evidence", "ambiguous", "high_risk", "incomplete"] {
        let mut missing = request(classification, LUNA);
        missing.as_object_mut().ok_or("request object")?.remove("simple_predicates");
        check(missing, fallback())?;
    }
    for predicate in ["fixed_scope", "deterministic_oracle", "low_risk_reversible", "no_unresolved_decision"] {
        let mut incomplete = request("simple", LUNA);
        incomplete["simple_predicates"][predicate] = json!(false);
        check(incomplete, fallback())?;
    }
    check(request("general", LUNA), fallback())?;
    let mut unknown = request("general", SOL);
    unknown["named_specialist"] = json!("codexy-unknown");
    rejects(unknown, "Worker routing request names an unknown packaged specialist")?;
    Ok(())
}

#[test]
fn issue_1412_bounded_summaries_without_deterministic_oracle_select_luna() -> TestResult {
    // Human-judged wording fails the old simple gate, while these finite summaries qualify.
    for (inputs, characters, items) in [(1, 1, 1), (2, 8_000, 10), (3, 12_000, 20)] {
        let mut bounded = bounded_request();
        bounded["simple_predicates"]["bounded_evidence"]["input_count"] = json!(inputs);
        bounded["simple_predicates"]["bounded_evidence"]["input_char_count"] = json!(characters);
        bounded["simple_predicates"]["bounded_evidence"]["output_item_limit"] = json!(items);
        check(bounded, selected("generic", "create_thread", LUNA))?;
    }
    let mut old_gate = bounded_request();
    old_gate["classification"] = json!("simple");
    check(old_gate, fallback())?;
    // Adding bounded facts cannot downgrade general integrations or unsafe/ambiguous work.
    for classification in ["general", "ambiguous", "high_risk", "incomplete"] {
        let mut broader = bounded_request();
        broader["classification"] = json!(classification);
        broader["codex_thread_capabilities"]["models"].as_array_mut().ok_or("models")?
            .push(json!({"model":SOL.0,"thinking":[SOL.1]}));
        let expected = if classification == "general" { selected("generic", "create_thread", SOL) } else { fallback() };
        check(broader, expected)?;
    }
    let mut specialist = bounded_request();
    specialist["named_specialist"] = json!("codexy-advisor");
    check(specialist, json!({"route":"named_specialist","agent_type":"codexy-advisor"}))?;
    Ok(())
}

#[test]
fn issue_1412_bounds_safety_and_capabilities_cannot_relax() -> TestResult {
    for predicate in ["fixed_scope", "low_risk_reversible", "no_unresolved_decision"] {
        let mut unsafe_request = bounded_request();
        unsafe_request["simple_predicates"][predicate] = json!(false);
        check(unsafe_request, fallback())?;
    }
    for (field, value) in [
        ("read_only", json!(false)), ("supplied_inputs_only", json!(false)),
        ("input_count", json!(0)), ("input_count", json!(4)),
        ("input_char_count", json!(0)), ("input_char_count", json!(12_001)),
        ("output_item_limit", json!(0)), ("output_item_limit", json!(21)),
    ] {
        let mut outside = bounded_request();
        outside["simple_predicates"]["bounded_evidence"][field] = value;
        check(outside, fallback())?;
    }
    let mut missing = bounded_request();
    missing["simple_predicates"]["bounded_evidence"] = Value::Null;
    check(missing, fallback())?;
    for pair in [SOL, ("gpt-6-luna", "medium"), ("gpt-5.6-terra", "high")] {
        let mut unsupported = bounded_request();
        unsupported["codex_thread_capabilities"] = request("general", pair)["codex_thread_capabilities"].clone();
        check(unsupported, fallback())?;
    }
    let mut unsupported = bounded_request();
    unsupported["codex_thread_capabilities"] = Value::Null;
    check(unsupported, fallback())?;
    // Typed, closed bounds reject malformed or invented authority/capability claims.
    for value in [json!({}), json!({"read_only":true,"authority":"assumed"}), json!(true), json!([true,true,2,234,4])] {
        let mut malformed = bounded_request();
        malformed["simple_predicates"]["bounded_evidence"] = value;
        rejects(malformed, "")?;
    }
    for value in [json!(-1), json!(1.5), json!("2"), json!(256)] {
        let mut malformed = bounded_request();
        malformed["simple_predicates"]["bounded_evidence"]["input_count"] = value;
        rejects(malformed, "")?;
    }
    Ok(())
}

#[test]
fn resolver_keeps_recipient_settings_for_both_worker_pairs() -> TestResult {
    for pair in [LUNA, SOL, ("gpt-6.1-sol", "max"), ("gpt-6-astra", "medium")] {
        let mut followup = bounded_request();
        followup["schema"] = json!("codexy.child-routing-request.v2");
        followup["codex_thread_operation"] = json!("send_message_to_thread");
        followup["codex_thread_direction"] = json!("parent_to_generic");
        followup["named_specialist"] = json!("codexy-architect");
        followup["recipient_assignment"] = json!({"model":pair.0,"thinking":pair.1});
        followup["codex_thread_capabilities"] = request("general", pair)["codex_thread_capabilities"].clone();
        let expected = if [LUNA, SOL].contains(&pair) {
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
    check(crossed, json!({"route":"root_or_named_specialist","reason":"Assigned recipient model and thinking are not supported by declared thread capabilities."}))?;
    let mut legacy = request("general", LUNA);
    legacy["codex_thread_operation"] = json!("send_message_to_thread");
    legacy["codex_thread_direction"] = json!("parent_to_generic");
    check(legacy, json!({"route":"root_or_named_specialist","reason":"Version 1 cannot identify a follow-up recipient; use schema v2 with recipient_assignment."}))?;
    for pair in [LUNA, SOL, ("gpt-6-astra", "medium"), ("gpt-6-luna", "medium")] {
        let mut to_parent = request("general", pair);
        to_parent["codex_thread_operation"] = json!("send_message_to_thread");
        to_parent["codex_thread_direction"] = json!("child_to_root");
        check(to_parent, if pair == LUNA { selected("child_to_root", "send_message_to_thread", LUNA) } else { fallback() })?;
    }
    let mut sender = request("general", LUNA);
    sender["model"] = json!(LUNA.0);
    sender["thinking"] = json!(LUNA.1);
    rejects(sender, "Worker routing request contains sender settings; use recipient_assignment")?;
    let mut unsupported = bounded_request();
    unsupported["codex_thread_operation"] = json!("unsupported_operation");
    rejects(unsupported, "Worker routing request names an unsupported Codex thread operation")?;
    Ok(())
}

fn request(classification: &str, pair: (&str, &str)) -> Value {
    json!({
        "schema":"codexy.child-routing-request.v1", "classification":classification,
        "simple_predicates":{"fixed_scope":true,"deterministic_oracle":true,
            "low_risk_reversible":true,"no_unresolved_decision":true},
        "codex_thread_operation":"create_thread",
        "codex_thread_capabilities":{"models":[{"model":pair.0,"thinking":[pair.1]}]}
    })
}

fn bounded_request() -> Value {
    let mut bounded = request("bounded_evidence", LUNA);
    bounded["simple_predicates"]["deterministic_oracle"] = json!(false);
    bounded["simple_predicates"]["bounded_evidence"] = json!({
        "read_only":true,"supplied_inputs_only":true,"input_count":2,
        "input_char_count":8_000,"output_item_limit":10
    });
    bounded
}

fn fallback() -> Value { json!({"route":"root_or_named_specialist"}) }

fn selected(route: &str, operation: &str, pair: (&str, &str)) -> Value {
    json!({"route":route,"codex_thread_operation":operation,"model":pair.0,"thinking":pair.1})
}

fn root() -> PathBuf {
    codexy_runtime::paths::repository_root().join("plugins/codexy")
}

fn check(request: Value, expected: Value) -> TestResult {
    assert_eq!(resolve(&request)?, expected, "request: {request}");
    let output = cli(&request)?;
    assert!(output.status.success(), "resolver failed: {}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty(), "successful resolver wrote stderr");
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, expected);
    Ok(())
}

fn rejects(request: Value, message: &str) -> TestResult {
    let error = resolve(&request).expect_err("invalid routing request passed");
    assert!(error.to_string().contains(message), "unexpected error: {error}");
    let output = cli(&request)?;
    assert!(!output.status.success(), "invalid routing request passed CLI");
    assert!(output.stdout.is_empty(), "invalid resolver wrote stdout");
    assert!(String::from_utf8_lossy(&output.stderr).contains(message));
    Ok(())
}

fn resolve(request: &Value) -> TestResult<Value> {
    Ok(codexy_runtime::validation::resolve_child_routing(&root(), &serde_json::to_string(request)?)?)
}

fn cli(request: &Value) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let request_path = temp.path().join("request.json");
    std::fs::write(&request_path, serde_json::to_vec(request)?)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .arg("--plugin-root").arg(root())
        .args(["--resolve-child-routing", "--routing-request-file"])
        .arg(request_path).output()?)
}

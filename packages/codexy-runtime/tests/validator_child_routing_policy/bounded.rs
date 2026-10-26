//! Requirement-linked bounded routing regressions.

use super::*;

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
    // Adding bounded facts cannot override the general default or admit unsafe/ambiguous work.
    for classification in ["general", "ambiguous", "high_risk", "incomplete"] {
        let mut broader = bounded_request();
        broader["classification"] = json!(classification);
        broader["codex_thread_capabilities"]["models"]
            .as_array_mut()
            .ok_or("models")?
            .push(json!({"model":SOL.0,"thinking":[SOL.1]}));
        let expected = if classification == "general" {
            selected("generic", "create_thread", LUNA)
        } else {
            fallback()
        };
        check(broader, expected)?;
    }
    let mut specialist = bounded_request();
    specialist["named_specialist"] = json!("codexy-advisor");
    check(
        specialist,
        json!({"route":"named_specialist","agent_type":"codexy-advisor"}),
    )?;
    Ok(())
}

#[test]
fn issue_1412_bounds_safety_and_capabilities_cannot_relax() -> TestResult {
    for predicate in [
        "fixed_scope",
        "low_risk_reversible",
        "no_unresolved_decision",
    ] {
        let mut unsafe_request = bounded_request();
        unsafe_request["simple_predicates"][predicate] = json!(false);
        check(unsafe_request, fallback())?;
    }
    for (field, value) in [
        ("read_only", json!(false)),
        ("supplied_inputs_only", json!(false)),
        ("input_count", json!(0)),
        ("input_count", json!(4)),
        ("input_char_count", json!(0)),
        ("input_char_count", json!(12_001)),
        ("output_item_limit", json!(0)),
        ("output_item_limit", json!(21)),
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
        unsupported["codex_thread_capabilities"] =
            request("general", pair)["codex_thread_capabilities"].clone();
        check(unsupported, fallback())?;
    }
    let mut unsupported = bounded_request();
    unsupported["codex_thread_capabilities"] = Value::Null;
    check(unsupported, fallback())?;
    // Typed, closed bounds reject malformed or invented authority/capability claims.
    for value in [
        json!({}),
        json!({"read_only":true,"authority":"assumed"}),
        json!(true),
        json!([true, true, 2, 234, 4]),
    ] {
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

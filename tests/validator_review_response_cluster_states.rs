use codexy_runtime::validation::review_response_cluster_diagnostics;
use serde_json::{Value, json};

#[test]
fn receipt_validation_orders_schema_content_and_state_transition_checks() {
    let valid_repair = structural_repair();
    let invalid_repair = json!({
        "kind": "structural",
        "boundary": " ",
        "strategy": "canonical resolver",
        "removed_case_specific_behavior": true
    });
    let case_exception = json!({"kind": "case_exception", "quoted_input": "quoted case"});
    let valid_reopen = json!({"kind": "distinct_invariant", "invariant": "repository ownership"});

    for (state, repair, reopen, expected_valid) in [
        ("planned", None, None, true),
        ("planned", Some(valid_repair.clone()), None, true),
        ("planned", Some(invalid_repair.clone()), None, false),
        ("planned", Some(case_exception.clone()), None, false),
        ("planned", None, Some(valid_reopen.clone()), false),
        ("repaired", None, None, false),
        ("repaired", Some(valid_repair.clone()), None, true),
        ("repaired", Some(invalid_repair.clone()), None, false),
        ("repaired", Some(case_exception.clone()), None, false),
        ("repaired", Some(valid_repair.clone()), Some(valid_reopen.clone()), false),
        ("reopened", None, Some(valid_reopen.clone()), false),
        ("reopened", Some(valid_repair.clone()), None, false),
        ("reopened", Some(valid_repair.clone()), Some(valid_reopen.clone()), true),
        ("reopened", Some(invalid_repair), Some(valid_reopen.clone()), false),
        ("reopened", Some(case_exception), Some(valid_reopen), false),
    ] {
        let receipt = receipt(state, repair, reopen);
        let errors = review_response_cluster_diagnostics(&receipt.to_string());
        assert_eq!(
            errors.is_empty(),
            expected_valid,
            "state={state}, receipt={receipt}, errors={errors:?}"
        );
    }

    for (field, value) in [
        (
            "repair",
            json!({
                "kind": "structural",
                "boundary": "parser",
                "strategy": "resolver",
                "removed_case_specific_behavior": true,
                "unknown": true
            }),
        ),
        (
            "reopen",
            json!({
                "kind": "distinct_invariant",
                "invariant": "repository ownership",
                "unknown": true
            }),
        ),
    ] {
        let mut malformed = receipt("reopened", Some(structural_repair()), Some(json!({
            "kind": "distinct_invariant",
            "invariant": "repository ownership"
        })));
        malformed["clusters"][0][field] = value;
        assert!(
            !review_response_cluster_diagnostics(&malformed.to_string()).is_empty(),
            "unknown {field} subobject unexpectedly passed: {malformed}"
        );
    }
}

#[test]
fn receipt_validation_uses_one_semantic_key_for_classes_and_invariants() {
    let mut duplicate = receipt("repaired", Some(structural_repair()), None);
    duplicate["clusters"].as_array_mut().expect("clusters").push(cluster(
        " classification / boundary ",
        "different invariant",
        Some(structural_repair()),
        None,
    ));
    assert!(
        !review_response_cluster_diagnostics(&duplicate.to_string()).is_empty(),
        "punctuation/case/spacing duplicate unexpectedly passed: {duplicate}"
    );

    let mut distinct = receipt("repaired", Some(structural_repair()), None);
    distinct["clusters"].as_array_mut().expect("clusters").push(cluster(
        "classification-boundary-v2",
        "different invariant",
        Some(structural_repair()),
        None,
    ));
    assert!(
        review_response_cluster_diagnostics(&distinct.to_string()).is_empty(),
        "distinct semantic classes unexpectedly failed: {distinct}"
    );

    let unicode_equivalent = "CAFE\u{301} / OWNER";
    let same_invariant = receipt(
        "reopened",
        Some(structural_repair()),
        Some(json!({"kind": "distinct_invariant", "invariant": unicode_equivalent})),
    );
    assert!(
        !review_response_cluster_diagnostics(&same_invariant.to_string()).is_empty(),
        "unicode/case/punctuation-equivalent invariant unexpectedly passed: {same_invariant}"
    );

    let distinct_invariant = receipt(
        "reopened",
        Some(structural_repair()),
        Some(json!({"kind": "distinct_invariant", "invariant": "repository ownership"})),
    );
    assert!(
        review_response_cluster_diagnostics(&distinct_invariant.to_string()).is_empty(),
        "distinct semantic invariant unexpectedly failed: {distinct_invariant}"
    );

    let mut empty = receipt("repaired", Some(structural_repair()), None);
    empty["clusters"][0]["defect_class"] = json!(" — / \t ");
    assert!(
        !review_response_cluster_diagnostics(&empty.to_string()).is_empty(),
        "empty-after-normalization class unexpectedly passed: {empty}"
    );
}

fn receipt(state: &str, repair: Option<Value>, reopen: Option<Value>) -> Value {
    json!({"state": state, "clusters": [cluster("classification-boundary", "Café owner", repair, reopen)]})
}

fn cluster(
    defect_class: &str,
    violated_invariant: &str,
    repair: Option<Value>,
    reopen: Option<Value>,
) -> Value {
    let mut cluster = json!({
        "defect_class": defect_class,
        "violated_invariant": violated_invariant,
        "structural_boundary": "receipt validator",
        "threads": ["PRRT_current"],
        "matrix": {"positive": ["typed receipt"], "negative": ["quoted exception"]}
    });
    if let Some(repair) = repair {
        cluster["repair"] = repair;
    }
    if let Some(reopen) = reopen {
        cluster["reopen"] = reopen;
    }
    cluster
}

fn structural_repair() -> Value {
    json!({
        "kind": "structural",
        "boundary": "receipt validator",
        "strategy": "phase ordered validator",
        "removed_case_specific_behavior": true
    })
}

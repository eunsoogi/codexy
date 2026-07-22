use codexy_runtime::validation::review_response_cluster_diagnostics;
use serde_json::{Value, json};

#[test]
fn receipt_validation_rejects_canonical_matrix_overlap() {
    for (positive, negative) in [
        ("same case", "same case"),
        ("same case", " SAME-CASE "),
        ("n!\u{0303}", "ñ"),
    ] {
        let receipt = repaired_receipt(vec![positive], vec![negative]);
        assert!(
            !review_response_cluster_diagnostics(&receipt.to_string()).is_empty(),
            "canonical matrix overlap unexpectedly passed: {receipt}"
        );
    }
}

#[test]
fn receipt_validation_rejects_duplicate_or_empty_matrix_cases() {
    for (positive, negative) in [
        (vec![], vec!["visible"]),
        (vec!["visible"], vec![]),
        (vec!["same", "SAME"], vec!["different"]),
        (vec!["different"], vec!["same", " SAME "]),
        (vec!["visible"], vec!["\u{200b}"]),
        (vec!["“?!"], vec!["visible"]),
    ] {
        let receipt = repaired_receipt(positive, negative);
        assert!(
            !review_response_cluster_diagnostics(&receipt.to_string()).is_empty(),
            "invalid matrix unexpectedly passed: {receipt}"
        );
    }
}

#[test]
fn receipt_validation_accepts_disjoint_canonical_matrix_sets() {
    let receipt = repaired_receipt(
        vec!["visible $ symbol", "identity1"],
        vec!["different case", "identity2"],
    );
    assert!(
        review_response_cluster_diagnostics(&receipt.to_string()).is_empty(),
        "disjoint canonical matrix unexpectedly failed: {receipt}"
    );
}

fn repaired_receipt(positive: Vec<&str>, negative: Vec<&str>) -> Value {
    json!({
        "state": "repaired",
        "clusters": [{
            "defect_class": "matrix-polarization",
            "violated_invariant": "positive and negative evidence are disjoint",
            "structural_boundary": "canonical evidence sets",
            "threads": ["PRRT_current"],
            "matrix": {"positive": positive, "negative": negative},
            "repair": {
                "kind": "structural",
                "boundary": "canonical evidence sets",
                "strategy": "polarized semantic evidence",
                "removed_case_specific_behavior": true
            }
        }]
    })
}

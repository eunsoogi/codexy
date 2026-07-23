use codexy_runtime::validation::review_response_cluster_diagnostics;
use serde_json::{Value, json};

#[test]
fn receipt_validation_rejects_ignorable_control_and_combining_only_identity() {
    for value in [
        "\u{200b}",
        "\u{200c}",
        "\u{200d}",
        "\u{202e}",
        "\u{fe0f}",
        "\u{0000}\u{001f}",
        "\u{0301}",
    ] {
        let receipt = repaired_receipt(value);
        assert!(
            !review_response_cluster_diagnostics(&receipt.to_string()).is_empty(),
            "non-material identity unexpectedly passed: {value:?}"
        );
    }
}

#[test]
fn receipt_validation_ignores_embedded_unicode_formatting_without_losing_material_content() {
    for (first, second) in [
        ("receipt", "re\u{200b}ceipt"),
        ("receipt", "re\u{200c}ceipt"),
        ("receipt", "re\u{200d}ceipt"),
        ("receipt", "re\u{202e}ceipt"),
        ("receipt", "re\u{fe0f}ceipt"),
    ] {
        let mut receipt = repaired_receipt(first);
        receipt["clusters"].as_array_mut().expect("clusters").push(cluster(second));
        assert!(
            !review_response_cluster_diagnostics(&receipt.to_string()).is_empty(),
            "embedded ignorable formatting split an identity: {receipt}"
        );
    }
}

#[test]
fn receipt_validation_preserves_visible_and_base_combining_material_identity() {
    for class in ["Cafe\u{301}", "$", "identity1"] {
        let receipt = repaired_receipt(class);
        assert!(
            review_response_cluster_diagnostics(&receipt.to_string()).is_empty(),
            "material identity unexpectedly failed: {receipt}"
        );
    }

    let mut distinct = repaired_receipt("identity1");
    distinct["clusters"].as_array_mut().expect("clusters").push(cluster("identity2"));
    assert!(
        review_response_cluster_diagnostics(&distinct.to_string()).is_empty(),
        "materially distinct identities unexpectedly failed: {distinct}"
    );
}

fn repaired_receipt(defect_class: &str) -> Value {
    json!({"state": "repaired", "clusters": [cluster(defect_class)]})
}

fn cluster(defect_class: &str) -> Value {
    json!({
        "defect_class": defect_class,
        "violated_invariant": "material identity remains visible",
        "structural_boundary": "semantic identity",
        "threads": ["PRRT_current"],
        "matrix": {"positive": ["visible receipt"], "negative": ["format-only receipt"]},
        "repair": {
            "kind": "structural",
            "boundary": "semantic identity",
            "strategy": "material content normalization",
            "removed_case_specific_behavior": true
        }
    })
}

use codexy_runtime::validation::review_response_cluster_diagnostics;

use crate::support::TestResult;
use serde_json::json;
use std::{fs, process::Command};
use tempfile::tempdir;

#[test]
fn receipt_file_content_cases_are_preserved_by_lower_level_diagnostics() -> TestResult {
    let valid = structural_receipt();
    let cases = [
        (
            "typed JSON",
            "not JSON".to_owned(),
            "root-cause review cluster receipt must be typed JSON:",
        ),
        (
            "missing cluster",
            "{}".to_owned(),
            "root-cause review cluster receipt must be typed JSON: missing field `state`",
        ),
        (
            "unknown field",
            valid.replace(
                "\"state\":\"repaired\"",
                "\"state\":\"repaired\",\"extra\":true",
            ),
            "unknown field `extra`",
        ),
        (
            "blank class",
            valid.replace("classification-boundary", "   "),
            "root-cause cluster is missing typed class, invariant, boundary, or thread evidence",
        ),
        (
            "blank invariant",
            valid.replace("owners use authoritative metadata", "   "),
            "root-cause cluster is missing typed class, invariant, boundary, or thread evidence",
        ),
        (
            "blank boundary",
            valid.replace("metadata parser", "   "),
            "root-cause cluster is missing typed class, invariant, boundary, or thread evidence",
        ),
        (
            "blank thread",
            valid.replace("PRRT_classification_one", "   "),
            "root-cause cluster is missing typed class, invariant, boundary, or thread evidence",
        ),
        (
            "blank positive evidence",
            valid.replace("canonical metadata", "   "),
            "root-cause matrix positive cases must contain material evidence",
        ),
        (
            "blank negative evidence",
            valid.replace("GFM owner table", "   "),
            "root-cause matrix negative cases must contain material evidence",
        ),
        (
            "duplicate canonical class",
            with_second_cluster(&valid, " classification-boundary "),
            "root-cause cluster ` classification-boundary ` must consolidate same-class examples",
        ),
        (
            "distinct second class",
            with_second_cluster(&valid, "command-normalization"),
            "",
        ),
    ];

    for (name, receipt, expected_diagnostic) in cases {
        let errors = review_response_cluster_diagnostics(&receipt);
        if expected_diagnostic.is_empty() {
            assert!(errors.is_empty(), "{name} unexpectedly failed: {errors:?}");
        } else {
            assert!(
                errors.iter().any(|error| error.contains(expected_diagnostic)),
                "{name} lost expected diagnostic {expected_diagnostic:?}: {errors:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn shipped_cli_retains_valid_and_missing_receipt_file_boundaries() -> TestResult {
    let valid_output = run_receipt_file(&structural_receipt());
    assert!(
        valid_output.status.success(),
        "valid receipt failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&valid_output.stdout),
        String::from_utf8_lossy(&valid_output.stderr)
    );
    assert!(String::from_utf8_lossy(&valid_output.stdout).contains("plugin config validation ok"));

    let missing = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .arg("--check-review-response-cluster")
        .output()?;
    assert!(!missing.status.success());
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("review-response-cluster-file"),
        "missing-file stderr lost argv contract: {}",
        String::from_utf8_lossy(&missing.stderr)
    );
    Ok(())
}

fn structural_receipt() -> String {
    r#"{"state":"repaired","clusters":[{"defect_class":"classification-boundary","violated_invariant":"owners use authoritative metadata","structural_boundary":"metadata parser","threads":["PRRT_classification_one"],"matrix":{"positive":["canonical metadata"],"negative":["GFM owner table"]},"repair":{"kind":"structural","boundary":"metadata parser","strategy":"authoritative metadata classifier","removed_case_specific_behavior":true}}]}"#.into()
}

fn with_second_cluster(receipt: &str, defect_class: &str) -> String {
    let mut parsed: serde_json::Value = serde_json::from_str(receipt).expect("valid receipt");
    parsed["clusters"]
        .as_array_mut()
        .expect("clusters")
        .push(json!({
            "defect_class": defect_class,
            "violated_invariant": "second invariant",
            "structural_boundary": "command normalizer",
            "threads": ["PRRT_second"],
            "matrix": {"positive": ["canonical command"], "negative": ["foreign repository"]},
            "repair": {
                "kind": "structural",
                "boundary": "command normalizer",
                "strategy": "canonical command resolver",
                "removed_case_specific_behavior": true
            }
        }));
    parsed.to_string()
}

fn run_receipt_file(receipt: &str) -> std::process::Output {
    let directory = tempdir().expect("tempdir");
    let path = directory.path().join("receipt.json");
    fs::write(&path, receipt).expect("receipt");
    Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--check-review-response-cluster",
            "--review-response-cluster-file",
        ])
        .arg(path)
        .output()
        .expect("validator")
}

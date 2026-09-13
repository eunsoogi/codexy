use std::{fs, process::Command};

use crate::support::TestResult;
use serde_json::json;

use super::{BASE_OID, HEAD_OID, direct_state};

#[test]
fn retired_cli_modes_fail_before_reading_or_writing() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let output = temporary.path().join("retired-output.json");
    let sentinel = b"keep this file unchanged";
    fs::write(&output, sentinel)?;
    for flag in [
        "--import-pre-pr-history",
        "--recover-native-review-history",
        "--check-next-review-eligibility",
        "--check-packet",
        "--check-economics",
        "--capture-economics",
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
            .args([flag, "--output"])
            .arg(&output)
            .output()?;
        assert!(!result.status.success(), "retired mode {flag} must fail");
        assert!(
            String::from_utf8_lossy(&result.stderr)
                .contains("legacy review-control processing is no longer supported"),
            "retired mode {flag} must explain the unsupported boundary: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(fs::read(&output)?, sentinel, "retired mode {flag} overwrote output");
    }
    Ok(())
}

#[test]
fn retired_public_review_control_apis_return_explicit_errors() -> TestResult {
    let plugin_root = codexy_runtime::paths::plugin_root();
    let repository_root = codexy_runtime::paths::repository_root();
    let temporary = tempfile::tempdir()?;
    let ledger = temporary.path().join("ledger.json");
    let cases = [
        codexy_runtime::validation::check_review_packet(
            &plugin_root,
            &repository_root,
            &ledger,
            "{}",
        )
        .map(|_| ()),
        codexy_runtime::validation::check_review_economics(
            &plugin_root,
            &repository_root,
            "{}",
        )
        .map(|_| ()),
        codexy_runtime::validation::import_pre_pr_review_history(
            &plugin_root,
            &repository_root,
            "{}",
            "{}",
        )
        .map(|_| ()),
        codexy_runtime::validation::recover_native_review_history(&plugin_root, "{}", "{}").map(
            |_| (),
        ),
        codexy_runtime::validation::check_next_review_eligibility(
            &plugin_root,
            &repository_root,
            "{}",
            "{}",
            "{}",
        )
        .map(|_| ()),
    ];
    for result in cases {
        let error = result.expect_err("retired public operation must fail");
        assert!(
            error
                .to_string()
                .contains("legacy review-control processing is no longer supported")
        );
    }
    Ok(())
}

#[test]
fn compact_producer_preserves_normal_review_snapshot_fields() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("input.json");
    let output = temporary.path().join("control.json");
    let mut current = direct_state::pr_snapshot(725, BASE_OID, HEAD_OID, None);
    current["reviewDecision"] = json!("APPROVED");
    current["latestReviews"] = json!([{"body": "actual review record"}]);
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": direct_state::strict_control(725, HEAD_OID),
            "current_pr_state": current
        }))?,
    )?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--produce-review-control", "--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .output()?;
    assert!(
        result.status.success(),
        "normal GitHub review fields must not be treated as retired control: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(())
}

#[test]
fn producer_rejects_retired_fields_at_all_supported_boundaries() -> TestResult {
    for field in [
        "full_review_count",
        "delta_review_count",
        "terminal_review_count",
        "terminal_review_limit",
        "terminal_review_history",
        "post_cap_re_review",
        "final_disposition",
        "reviewer_migration",
        "pre_pr_import",
        "native_history_recovery",
        "native_history_provenance",
    ] {
        let mut control = direct_state::strict_control(725, HEAD_OID);
        control[field] = json!(null);
        assert_retired_request(json!({"control_state": control}))?;
    }
    assert_retired_request(json!({
        "reviewControl": {"schema": "codexy.review-control-state.v1", "profile": "strict", "full_review_count": null}
    }))?;
    assert_retired_request(json!({
        "control_state": direct_state::strict_control(725, HEAD_OID),
        "current_pr_state": {"reviewControl": {"terminal_review_history": []}}
    }))?;
    assert_retired_request(json!({
        "control_state": direct_state::strict_control(725, HEAD_OID),
        "previous_pr_state": {"reviewControl": {"native_history_recovery": null}}
    }))?;
    for schema in [
        "codexy.review-control-external-finding.v1",
        "codexy.review-control-final-disposition.v1",
        "codexy.review-control-final-authority.v1",
        "codexy.review-control-final-authority-comment.v1",
        "codexy.review-control-native-history-request.v1",
        "codexy.review-control-native-history.v1",
        "codexy.review-control-native-history-capture-provenance.v1",
        "codexy.review-control-finding-disposition.v1",
        "codexy.review-control-migration.v1",
        "codexy.review-control-pre-pr-history.v1",
        "codexy.review-control-next-review-eligibility.v1",
        "codexy.review-economics-capture-request.v1",
    ] {
        assert_retired_request(json!({"schema": schema}))?;
    }
    for field in [
        "previous_control_state",
        "nativeHistoryRecovery",
        "authenticated_external_finding_locator",
        "authenticated_actions_finding_locator",
        "authenticated_finding_disposition_locator",
        "authenticated_final_disposition_locator",
        "authenticated_external_finding",
        "authenticated_external_finding_capture",
        "authenticated_actions_finding",
        "authenticated_actions_finding_capture",
        "authenticated_finding_disposition",
        "authenticated_finding_disposition_capture",
        "finding_disposition",
    ] {
        let mut request = json!({});
        request[field] = serde_json::Value::Null;
        assert_retired_request(request)?;
    }
    Ok(())
}

pub(super) fn assert_retired_request(request: serde_json::Value) -> TestResult {
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("input.json");
    let output = temporary.path().join("output.json");
    let sentinel = b"preserve this output";
    fs::write(&input, serde_json::to_vec(&request)?)?;
    fs::write(&output, sentinel)?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--produce-review-control", "--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .output()?;
    assert!(!result.status.success(), "retired request must fail");
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("legacy review-control processing is no longer supported"),
        "retired request must fail explicitly: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(fs::read(&output)?, sentinel);
    Ok(())
}

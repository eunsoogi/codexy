use std::fs;

use crate::support::TestResult;
use serde_json::json;

use super::{HEAD_OID, direct_state, run_capture, state_files, validate_readiness};

#[test]
fn build_pr_state_rejects_retired_control_aliases_before_output() -> TestResult {
    for alias in ["control_state", "reviewControl"] {
        let temp = tempfile::tempdir()?;
        let mut control = direct_state::strict_control(725, HEAD_OID);
        control[alias] = json!({"full_review_count": null});
        let (base, control_path, previous_path, output) = state_files(temp.path(), &control)?;
        let sentinel = b"preserve this output";
        fs::write(&output, sentinel)?;
        let result = run_capture(&base, &control_path, &previous_path, &output)?;
        assert!(!result.status.success(), "retired {alias} must block build-pr-state");
        assert!(
            String::from_utf8_lossy(&result.stderr)
                .contains("legacy review-control processing is no longer supported")
        );
        assert_eq!(fs::read(&output)?, sentinel);
    }
    Ok(())
}

#[test]
fn direct_review_control_accepts_compact_state_without_legacy_counts() -> TestResult {
    let control = direct_state::strict_control(725, HEAD_OID);
    let output = validate_readiness(control)?;
    assert!(
        output.status.success(),
        "compact current-head state must not require a delta review count: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

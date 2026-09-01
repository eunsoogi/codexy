use super::admission_runtime::TestResult;
use super::merge_admission::{github_plugin_root, state, wrapper_with_payload};

#[test]
fn canonical_wrapper_accepts_equivalent_line_endings() -> TestResult {
    let lf_capture = state().replace(
        r#""body":"Fixes #503\n""#,
        r#""body":"Details\n\nFixes #503\n""#,
    );
    let crlf_capture = state().replace(
        r#""body":"Fixes #503\n""#,
        r#""body":"Details\r\n\r\nFixes #503\n""#,
    );
    let subject = "fix(workflow): require intent (#128)";
    let (output, merged, body) = wrapper_with_payload(
        &github_plugin_root(),
        &crlf_capture,
        false,
        "fix(workflow): require intent (#128)\n\nDetails\n\nFixes #503\n",
        subject,
        "Details\n\nFixes #503\n",
        false,
        false,
    )?;
    assert!(
        output.status.success(),
        "LF body rejected for equivalent CRLF capture: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(merged, "LF body did not reach merge for CRLF capture");
    assert_eq!(body, "Details\n\nFixes #503\n");

    let (output, merged, _) = wrapper_with_payload(
        &github_plugin_root(),
        &lf_capture,
        false,
        "fix(workflow): require intent (#128)\n\nDetails\r\n\r\nFixes #503\n",
        subject,
        "Details\r\n\r\nFixes #503\n",
        false,
        false,
    )?;
    assert!(
        output.status.success(),
        "CRLF body rejected for equivalent LF capture: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(merged, "CRLF body did not reach merge for LF capture");

    let (output, merged, _) = wrapper_with_payload(
        &github_plugin_root(),
        &crlf_capture,
        false,
        "fix(workflow): require intent (#128)\n\nDetailz\n\nFixes #503\n",
        subject,
        "Detailz\n\nFixes #503\n",
        false,
        false,
    )?;
    assert!(!output.status.success(), "changed body content was admitted");
    assert!(!merged, "changed body content reached merge");
    Ok(())
}

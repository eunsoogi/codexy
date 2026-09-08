use super::fixtures;
use super::markdown;
use super::native_history;
use super::{TestResult, rejected};

#[test]
fn actual_host_markdown_without_structured_metadata_is_lossless() -> TestResult {
    let receipt = native_history::normalize_native_history(&markdown::actual_request())?;
    assert_eq!(receipt["events"].as_array().map(Vec::len), Some(2));
    assert_eq!(receipt["events"][0]["message_id"], "actual-message-full");
    assert_eq!(receipt["events"][0]["kind"], "full");
    assert_eq!(receipt["events"][0]["terminal_result"], "BLOCK");
    assert_eq!(receipt["events"][0]["reviewed_head"], fixtures::FULL_HEAD);
    assert_eq!(receipt["events"][1]["message_id"], "actual-message-delta");
    assert_eq!(receipt["events"][1]["kind"], "delta");
    assert_eq!(receipt["events"][1]["terminal_result"], "BLOCK");
    assert_eq!(receipt["events"][1]["reviewed_head"], fixtures::DELTA_HEAD);
    for event in receipt["events"].as_array().ok_or("events")? {
        let text = event["raw_text"].as_str().ok_or("raw text")?;
        for finding in event["findings"].as_array().ok_or("findings")? {
            let span = &finding["source_span"]["raw"];
            let start = span["start"].as_u64().ok_or("start")? as usize;
            let end = span["end"].as_u64().ok_or("end")? as usize;
            assert_eq!(
                &text[start..end],
                finding["text"].as_str().ok_or("finding text")?
            );
        }
    }
    Ok(())
}

#[test]
fn markdown_conflicting_operational_labels_are_rejected() -> TestResult {
    let error = rejected(native_history::normalize_native_history(
        &markdown::conflicting_terminal_request(),
    ));
    assert!(error.contains("markdown terminal result is contradictory"));
    Ok(())
}

#[test]
fn markdown_conflicting_finding_labels_are_rejected() {
    let error = rejected(native_history::normalize_native_history(
        &markdown::conflicting_finding_labels_request(),
    ));
    assert!(error.contains("semantic kind"));
}

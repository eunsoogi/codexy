use super::super::*;
use super::{assert_error_contains, assert_status, call_report, watcher_session};

#[test]
fn parser_enforces_report_text_bounds_and_preserves_flat_fields()
-> Result<(), Box<dyn std::error::Error>> {
    let (_state, mut client, session, watcher_token, target) = watcher_session()?;
    let accepted = call_report(
        &mut client,
        2,
        &session,
        &watcher_token,
        json!({"event":{
            "target":target,
            "eventId":"e".repeat(128),
            "kind":"failure",
            "summary":"s".repeat(128),
            "evidence":["x".repeat(512)]
        }}),
    )?;
    assert_status(&accepted, "accepted")?;

    let byte_boundary = format!("{}ab", "가".repeat(42));
    assert_eq!(byte_boundary.len(), 128);
    let accepted_utf8 = call_report(
        &mut client,
        3,
        &session,
        &watcher_token,
        json!({"event":{
            "target":target,
            "eventId":"utf8-boundary",
            "kind":"drift",
            "summary":byte_boundary
        }}),
    )?;
    assert_status(&accepted_utf8, "accepted")?;

    let over_byte_boundary = call_report(
        &mut client,
        4,
        &session,
        &watcher_token,
        json!({"event":{
            "target":target,
            "eventId":"utf8-over-boundary",
            "kind":"drift",
            "summary":"가".repeat(43)
        }}),
    )?;
    assert_error_contains(&over_byte_boundary, "summary is invalid");

    let control_character = call_report(
        &mut client,
        5,
        &session,
        &watcher_token,
        json!({"event":{
            "target":target,
            "eventId":"control-character",
            "kind":"drift",
            "summary":"line\nbreak"
        }}),
    )?;
    assert_error_contains(&control_character, "summary is invalid");

    let flattened = call_report(
        &mut client,
        6,
        &session,
        &watcher_token,
        json!({
            "target":target,
            "eventId":"legacy-flat-report",
            "kind":"failure",
            "summary":"legacy flat fields remain supported"
        }),
    )?;
    assert_status(&flattened, "accepted")?;

    let over_evidence_items = call_report(
        &mut client,
        7,
        &session,
        &watcher_token,
        json!({"event":{
            "target":target,
            "kind":"failure",
            "summary":"too many locators",
            "evidence":vec!["locator"; 17]
        }}),
    )?;
    assert_error_contains(&over_evidence_items, "at most 16 locators");

    let accepted_evidence_items = call_report(
        &mut client,
        12,
        &session,
        &watcher_token,
        json!({"event":{
            "target":target,
            "kind":"failure",
            "summary":"maximum locator count",
            "evidence":vec!["locator"; 16]
        }}),
    )?;
    assert_status(&accepted_evidence_items, "accepted")?;

    let over_evidence_length = call_report(
        &mut client,
        8,
        &session,
        &watcher_token,
        json!({"event":{
            "target":target,
            "kind":"failure",
            "summary":"oversized locator",
            "evidence":["x".repeat(513)]
        }}),
    )?;
    assert_error_contains(&over_evidence_length, "evidence locator");

    let unsafe_event_id = call_report(
        &mut client,
        9,
        &session,
        &watcher_token,
        json!({"target":target,"eventId":"unsafe/id","kind":"failure","summary":"invalid id"}),
    )?;
    assert_error_contains(&unsafe_event_id, "eventId is invalid");

    let unsupported_field = call_report(
        &mut client,
        10,
        &session,
        &watcher_token,
        json!({"target":target,"unexpected":"value"}),
    )?;
    assert_error_contains(&unsupported_field, "argument is not supported");
    let unsupported_nested_field = call_report(
        &mut client,
        11,
        &session,
        &watcher_token,
        json!({"event":{"target":target,"unexpected":"value"}}),
    )?;
    assert_error_contains(&unsupported_nested_field, "argument is not supported");
    Ok(())
}

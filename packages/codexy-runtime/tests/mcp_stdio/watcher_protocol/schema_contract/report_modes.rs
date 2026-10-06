use super::super::*;
use super::{assert_error_contains, assert_status, call_report, watcher_session};

#[test]
fn parser_matches_report_enums_and_health_field_bounds() -> Result<(), Box<dyn std::error::Error>> {
    let (_state, mut client, session, watcher_token, target) = watcher_session()?;
    for (index, kind) in [
        "terminal",
        "failure",
        "drift",
        "missing_delivery",
        "gate_ready",
        "unavailable",
    ]
    .into_iter()
    .enumerate()
    {
        let response = call_report(
            &mut client,
            10 + index as u64,
            &session,
            &watcher_token,
            json!({"event":{
                "target":target,
                "eventId":format!("kind-{kind}"),
                "kind":kind,
                "summary":"supported kind"
            }}),
        )?;
        assert_status(&response, "accepted")?;
    }

    let unsupported_kind = call_report(
        &mut client,
        20,
        &session,
        &watcher_token,
        json!({"event":{
            "target":target,
            "eventId":"unsupported-kind",
            "kind":"not-supported",
            "summary":"invalid kind"
        }}),
    )?;
    assert_error_contains(&unsupported_kind, "event kind is not supported");

    for (index, state) in [
        "starting",
        "running",
        "idle",
        "error",
        "stopped",
        "unavailable",
    ]
    .into_iter()
    .enumerate()
    {
        let response = call_report(
            &mut client,
            30 + index as u64,
            &session,
            &watcher_token,
            json!({"target":target,"watcherState":state}),
        )?;
        assert_status(&response, "health_updated")?;
    }

    let unsupported_state = call_report(
        &mut client,
        36,
        &session,
        &watcher_token,
        json!({"target":target,"watcherState":"sleeping"}),
    )?;
    assert_error_contains(&unsupported_state, "watcher state is not supported");

    let error_128 = call_report(
        &mut client,
        24,
        &session,
        &watcher_token,
        json!({"target":target,"lastError":"x".repeat(128),"watcherState":"error"}),
    )?;
    assert_status(&error_128, "health_updated")?;
    let error_129 = call_report(
        &mut client,
        25,
        &session,
        &watcher_token,
        json!({"target":target,"lastError":"x".repeat(129),"watcherState":"error"}),
    )?;
    assert_error_contains(&error_129, "lastError is invalid");

    let negative_timestamp = call_report(
        &mut client,
        37,
        &session,
        &watcher_token,
        json!({"target":target,"observedAtMs":-1}),
    )?;
    assert_error_contains(&negative_timestamp, "observedAtMs must be an integer");
    Ok(())
}

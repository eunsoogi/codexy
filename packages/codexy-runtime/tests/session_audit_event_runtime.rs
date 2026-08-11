use std::{fs, process::Command};

use serde_json::Value;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn session_audit_reports_metadata_only_deduplicated_aggregates() -> TestResult {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("events.jsonl");
    fs::write(
        &input,
        concat!(
            "{\"event\":\"turn.completed\",\"session_id\":\"lane-276\",\"turn_id\":\"turn-1\",\"cumulative_tokens\":100,\"prompt\":\"secret prompt\",\"tool_calls\":[{\"tool\":\"functions.exec\",\"output_bytes\":10}]}\n",
            "{\"event\":\"turn.completed\",\"session_id\":\"lane-276\",\"turn_id\":\"turn-2\",\"cumulative_tokens\":160,\"tool_calls\":[{\"tool\":\"functions.exec\",\"output_bytes\":11}]}\n",
            "{\"event\":\"turn.completed\",\"session_id\":\"lane-276\",\"turn_id\":\"turn-2\",\"cumulative_tokens\":160,\"tool_calls\":[{\"tool\":\"functions.exec\",\"output_bytes\":999}]}\n",
            "{\"event\":\"turn.completed\",\"session_id\":\"lane-276\",\"turn_id\":\"turn-3\",\"cumulative_tokens\":220,\"metadata\":{\"tool_calls\":[{\"tool\":\"functions.exec\",\"output_bytes\":999}]},\"ToolCalls\":[{\"tool\":\"wrong.case\",\"output_bytes\":999}],\"tool_calls\":[{\"tool\":\"functions.exec\",\"output_bytes\":12}]}\n"
        ),
    )?;

    let output = audit(&input)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout)?;
    assert!(!stdout.contains("secret prompt"));
    assert!(!stdout.contains("\"prompt\""));
    assert!(!stdout.contains("\"metadata\""));

    let report: Value = serde_json::from_str(&stdout)?;
    assert_eq!(report["session_count"], 1);
    assert_eq!(report["duplicate_events_skipped"], 1);
    assert_eq!(report["sessions"][0]["latest_cumulative_tokens"], 220);
    assert_eq!(report["sessions"][0]["recent_turn_average_tokens"], 73);
    assert_eq!(report["sessions"][0]["tool_calls"]["functions.exec"], 3);
    assert_eq!(report["sessions"][0]["tool_output_bytes"]["functions.exec"], 33);
    assert_eq!(
        report["sessions"][0]["event_ids"],
        serde_json::json!([
            "turn.completed|lane-276|turn-1",
            "turn.completed|lane-276|turn-2",
            "turn.completed|lane-276|turn-3"
        ])
    );
    Ok(())
}

#[test]
fn session_audit_rejects_invalid_tool_names() -> TestResult {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("invalid.jsonl");
    fs::write(
        &input,
        "{\"event\":\"turn.completed\",\"session_id\":\"lane-276\",\"turn_id\":\"turn-1\",\"cumulative_tokens\":100,\"tool_calls\":[{\"tool\":\"invalid tool\",\"output_bytes\":1}]}\n",
    )?;

    let output = audit(&input)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("tool must contain only"));
    Ok(())
}

#[test]
fn session_audit_uses_codex_metadata_without_emitting_content() -> TestResult {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("codex-session.jsonl");
    fs::write(
        &input,
        concat!(
            "{\"type\":\"session_meta\",\"payload\":{\"session_id\":\"lane-276\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"call_id\":\"call-1\",\"name\":\"functions.exec\",\"arguments\":\"private arguments\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call_output\",\"call_id\":\"call-1\",\"output\":\"body\"}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"total_tokens\":100},\"last_token_usage\":{\"total_tokens\":40}}}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"total_tokens\":160},\"last_token_usage\":{\"total_tokens\":60}}}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"total_tokens\":160},\"last_token_usage\":{\"total_tokens\":60}}}}\n"
        ),
    )?;

    let output = audit(&input)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout)?;
    assert!(!stdout.contains("private arguments"));
    assert!(!stdout.contains("\"body\""));
    let report: Value = serde_json::from_str(&stdout)?;
    assert_eq!(report["duplicate_events_skipped"], 1);
    assert_eq!(report["sessions"][0]["latest_cumulative_tokens"], 160);
    assert_eq!(report["sessions"][0]["recent_turn_average_tokens"], 50);
    assert_eq!(report["sessions"][0]["tool_calls"]["functions.exec"], 1);
    assert_eq!(report["sessions"][0]["tool_output_bytes"]["functions.exec"], 4);
    Ok(())
}

#[test]
fn session_audit_keeps_first_duplicate_call_identity_and_output() -> TestResult {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("duplicate-call-id.jsonl");
    fs::write(
        &input,
        concat!(
            "{\"type\":\"session_meta\",\"payload\":{\"session_id\":\"lane-276\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"call_id\":\"call-1\",\"name\":\"functions.exec\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"call_id\":\"call-1\",\"name\":\"functions.exec\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call_output\",\"call_id\":\"call-1\",\"output\":\"body\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call_output\",\"call_id\":\"call-1\",\"output\":\"second body\"}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"total_tokens\":100},\"last_token_usage\":{\"total_tokens\":100}}}}\n"
        ),
    )?;

    let output = audit(&input)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["duplicate_events_skipped"], 2);
    assert_eq!(report["sessions"][0]["tool_calls"]["functions.exec"], 1);
    assert_eq!(report["sessions"][0]["tool_output_bytes"]["functions.exec"], 4);
    Ok(())
}

fn audit(input: &std::path::Path) -> TestResult<std::process::Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-session-audit"))
        .arg("--input")
        .arg(input)
        .output()?)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

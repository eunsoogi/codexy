use std::{fs, process::Command};

use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn high_token_parent_fixture_reports_bounded_families_and_window_metadata() -> TestResult {
    let summary: Value = serde_json::from_str(include_str!(
        "fixtures/session-audit/high-token-parent-summary.json"
    ))?;
    let mut records = vec![json!({
        "type": "session_meta",
        "payload": {
            "session_id": summary["sessionId"],
            "prompt": summary["privateMarker"],
        },
    })];
    append_codex_calls(
        &mut records,
        "functions.exec",
        number(&summary, "execCalls")?,
        number(&summary, "execInputBytes")?,
        number(&summary, "execOutputBytes")?,
        false,
    );
    append_codex_calls(
        &mut records,
        "functions.wait",
        number(&summary, "waitCalls")?,
        8_150,
        470_281,
        true,
    );
    append_codex_calls(
        &mut records,
        "functions.wait_agent",
        number(&summary, "waitAgentCalls")?,
        number(&summary, "waitInputBytes")? - 8_150,
        number(&summary, "waitOutputBytes")? - 470_281,
        true,
    );
    records.push(json!({
        "type": "event_msg",
        "payload": {
            "type": "token_count",
            "info": {
                "total_token_usage": {"total_tokens": summary["latestCumulativeTokens"]},
                "last_token_usage": {"total_tokens": summary["latestCumulativeTokens"]},
            },
        },
    }));
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("high-token-parent.jsonl");
    fs::write(
        &input,
        records
            .into_iter()
            .map(|record| record.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    )?;

    let output = audit(&input)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout)?;
    assert!(!stdout.contains(summary["privateMarker"].as_str().unwrap_or_default()));
    let report: Value = serde_json::from_str(&stdout)?;
    let session = &report["sessions"][0];
    assert_eq!(
        session["latest_cumulative_tokens"],
        summary["latestCumulativeTokens"]
    );
    assert_eq!(session["window"]["policy"], "records");
    assert_eq!(session["window"]["records_observed"], 1_100);
    assert_eq!(session["window"]["turn_events"], 1);
    assert_eq!(session["byte_provenance"], "derived");
    assert_eq!(session["tool_families"]["exec"]["calls"], 404);
    assert_eq!(
        session["tool_families"]["exec"]["input_bytes"],
        summary["execInputBytes"]
    );
    assert_eq!(
        session["tool_families"]["exec"]["output_bytes"],
        summary["execOutputBytes"]
    );
    assert_eq!(session["tool_families"]["wait"]["calls"], 145);
    assert_eq!(session["tool_calls"]["functions.wait"], 95);
    assert_eq!(session["tool_calls"]["functions.wait_agent"], 50);
    assert_eq!(
        session["tool_families"]["wait"]["input_bytes"],
        summary["waitInputBytes"]
    );
    assert_eq!(
        session["tool_families"]["wait"]["output_bytes"],
        summary["waitOutputBytes"]
    );
    Ok(())
}

#[test]
fn codex_tool_input_bytes_preserve_privacy_and_first_binding() -> TestResult {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("codex-input-bytes.jsonl");
    fs::write(
        &input,
        concat!(
            "{\"type\":\"session_meta\",\"payload\":{\"session_id\":\"parent-486\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"call_id\":\"exec-1\",\"name\":\"functions.exec\",\"arguments\":\"é\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"call_id\":\"exec-1\",\"name\":\"functions.exec\",\"arguments\":\"duplicate secret\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call_output\",\"call_id\":\"exec-1\",\"output\":\"body\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"custom_tool_call\",\"call_id\":\"wait-1\",\"name\":\"functions.wait_agent\",\"input\":{\"secret\":\"x\"}}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"custom_tool_call_output\",\"call_id\":\"wait-1\",\"output\":\"ok\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"custom_tool_call_output\",\"call_id\":\"orphan\",\"output\":\"orphan secret\"}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"total_tokens\":10},\"last_token_usage\":{\"total_tokens\":10}}}}\n"
        ),
    )?;

    let output = audit(&input)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout)?;
    for secret in ["é", "duplicate secret", "\"secret\"", "orphan secret"] {
        assert!(!stdout.contains(secret));
    }
    let report: Value = serde_json::from_str(&stdout)?;
    let session = &report["sessions"][0];
    assert_eq!(report["duplicate_events_skipped"], 1);
    assert_eq!(session["tool_input_bytes"]["functions.exec"], 2);
    assert_eq!(
        session["tool_input_bytes"]["functions.wait_agent"],
        serde_json::to_vec(&json!({"secret": "x"}))?.len()
    );
    assert_eq!(session["tool_families"]["exec"]["calls"], 1);
    assert_eq!(session["tool_families"]["wait"]["calls"], 1);
    assert_eq!(session["window"]["records_observed"], 8);
    assert_eq!(session["window"]["turn_events"], 1);
    assert_eq!(session["byte_provenance"], "derived");
    Ok(())
}

#[test]
fn generic_bytes_are_reported_and_suffix_spoofs_are_not_privileged() -> TestResult {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("reported-bytes.jsonl");
    fs::write(
        &input,
        json!({
            "event": "turn.completed",
            "session_id": "parent-486",
            "turn_id": "turn-1",
            "cumulative_tokens": 10,
            "tool_calls": [
                {"tool": "attacker.exec", "input_bytes": 900, "output_bytes": 901},
                {"tool": "functions.exec", "input_bytes": 2, "output_bytes": 3}
            ]
        })
        .to_string(),
    )?;

    let output = audit(&input)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let report: Value = serde_json::from_slice(&output.stdout)?;
    let session = &report["sessions"][0];
    assert_eq!(session["byte_provenance"], "reported");
    assert_eq!(session["tool_families"]["exec"]["calls"], 1);
    assert_eq!(session["tool_families"]["exec"]["input_bytes"], 2);
    assert_eq!(session["tool_families"]["exec"]["output_bytes"], 3);
    Ok(())
}

#[test]
fn mixed_generic_and_codex_formats_are_rejected() -> TestResult {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("mixed.jsonl");
    fs::write(
        &input,
        concat!(
            "{\"event\":\"turn.completed\",\"session_id\":\"parent-486\",\"turn_id\":\"turn-1\",\"cumulative_tokens\":999}\n",
            "{\"type\":\"session_meta\",\"payload\":{\"session_id\":\"parent-486\"}}\n"
        ),
    )?;

    let output = audit(&input)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("mixed generic and Codex session metadata formats"));
    Ok(())
}

fn append_codex_calls(
    records: &mut Vec<Value>,
    tool: &str,
    count: u64,
    input_bytes: u64,
    output_bytes: u64,
    custom: bool,
) {
    for index in 0..count {
        let call_id = format!("{}-{index}", tool.replace('.', "-"));
        let input_len = distributed_bytes(input_bytes, count, index);
        let output_len = distributed_bytes(output_bytes, count, index);
        let call_type = if custom {
            "custom_tool_call"
        } else {
            "function_call"
        };
        let output_type = if custom {
            "custom_tool_call_output"
        } else {
            "function_call_output"
        };
        let mut payload = json!({
            "type": call_type,
            "call_id": call_id,
            "name": tool,
        });
        payload[if custom { "input" } else { "arguments" }] = Value::String("i".repeat(input_len));
        records.push(json!({"type": "response_item", "payload": payload}));
        records.push(json!({
            "type": "response_item",
            "payload": {
                "type": output_type,
                "call_id": call_id,
                "output": "o".repeat(output_len),
            },
        }));
    }
}

fn distributed_bytes(total: u64, count: u64, index: u64) -> usize {
    let bytes = total / count + u64::from(index < total % count);
    usize::try_from(bytes).expect("fixture byte count fits usize")
}

fn number(value: &Value, key: &str) -> TestResult<u64> {
    value[key]
        .as_u64()
        .ok_or_else(|| format!("fixture field {key} must be u64").into())
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

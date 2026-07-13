use std::{fs, process::Command};

use serde_json::Value;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn session_audit_counts_and_deduplicates_custom_tool_metadata() -> TestResult {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("custom-tools.jsonl");
    fs::write(
        &input,
        concat!(
            "{\"type\":\"session_meta\",\"payload\":{\"session_id\":\"lane-276\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"custom_tool_call\",\"call_id\":\"call-1\",\"name\":\"exec\",\"input\":\"secret input\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"custom_tool_call\",\"call_id\":\"call-2\",\"name\":\"exec\",\"input\":\"other secret\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"custom_tool_call\",\"call_id\":\"call-1\",\"name\":\"exec\",\"input\":\"duplicate secret\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"custom_tool_call_output\",\"call_id\":\"call-1\",\"output\":[\"body\"]}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"custom_tool_call_output\",\"call_id\":\"call-1\",\"output\":[\"repeat\"]}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"custom_tool_call_output\",\"call_id\":\"call-2\",\"output\":[{\"type\":\"text\",\"text\":\"body\"}]}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"total_tokens\":100},\"last_token_usage\":{\"total_tokens\":100}}}}\n"
        ),
    )?;

    let output = audit(&input)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout)?;
    for secret in [
        "secret input",
        "other secret",
        "duplicate secret",
        "\"body\"",
    ] {
        assert!(!stdout.contains(secret));
    }
    let report: Value = serde_json::from_str(&stdout)?;
    assert_eq!(report["duplicate_events_skipped"], 2);
    assert_eq!(report["sessions"][0]["tool_calls"]["exec"], 2);
    assert_eq!(report["sessions"][0]["tool_output_bytes"]["exec"], 39);
    Ok(())
}
#[test]
fn session_audit_uses_utf8_bytes_and_rejects_conflicting_or_cross_session_metadata() -> TestResult {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("custom-tool-edges.jsonl");
    fs::write(
        &input,
        concat!(
            "{\"type\":\"session_meta\",\"payload\":{\"session_id\":\"lane-276\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"call_id\":\"call-1\",\"name\":\"function\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"custom_tool_call\",\"call_id\":\"call-1\",\"name\":\"custom\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"custom_tool_call_output\",\"call_id\":\"call-1\",\"output\":[\"é\",\"한\",\"🙂\"]}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"custom_tool_call_output\",\"call_id\":\"orphan\",\"output\":\"private\"}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"total_tokens\":100},\"last_token_usage\":{\"total_tokens\":100}}}}\n"
        ),
    )?;

    let output = audit(&input)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout)?;
    assert!(!stdout.contains("é"));
    assert!(!stdout.contains("한"));
    assert!(!stdout.contains("🙂"));
    assert!(!stdout.contains("private"));
    let report: Value = serde_json::from_str(&stdout)?;
    assert_eq!(report["sessions"][0]["tool_calls"]["function"], 1);
    assert_eq!(report["sessions"][0]["tool_calls"]["custom"], 1);
    assert_eq!(report["sessions"][0]["tool_output_bytes"]["custom"], 19);

    fs::write(
        &input,
        concat!(
            "{\"type\":\"session_meta\",\"payload\":{\"session_id\":\"lane-276\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"custom_tool_call\",\"call_id\":\"call-1\",\"name\":\"exec\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"custom_tool_call\",\"call_id\":\"call-1\",\"name\":\"other\"}}\n"
        ),
    )?;
    let conflict = audit(&input)?;
    assert!(!conflict.status.success());
    assert!(stderr(&conflict).contains("conflicting tool names"));

    fs::write(
        &input,
        concat!(
            "{\"type\":\"session_meta\",\"payload\":{\"session_id\":\"lane-276\"}}\n",
            "{\"type\":\"session_meta\",\"payload\":{\"session_id\":\"lane-277\"}}\n"
        ),
    )?;
    let second_session = audit(&input)?;
    assert!(!second_session.status.success());
    assert!(stderr(&second_session).contains("exactly one session_meta"));
    Ok(())
}
#[test]
fn session_audit_rejects_unsafe_call_ids_and_handles_empty_or_late_metadata() -> TestResult {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("audit-boundaries.jsonl");
    fs::write(
        &input,
        concat!(
            "{\"type\":\"session_meta\",\"payload\":{\"session_id\":\"lane-276\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"custom_tool_call\",\"call_id\":\"unsafe id secret\",\"name\":\"exec\"}}\n"
        ),
    )?;
    let unsafe_id = audit(&input)?;
    assert!(!unsafe_id.status.success());
    assert!(!stderr(&unsafe_id).contains("unsafe id secret"));

    fs::write(
        &input,
        "{\"type\":\"session_meta\",\"payload\":{\"session_id\":\"lane-276\"}}\n",
    )?;
    let no_tokens = audit(&input)?;
    assert!(
        no_tokens.status.success(),
        "stderr:\n{}",
        stderr(&no_tokens)
    );
    let no_tokens_report: Value = serde_json::from_slice(&no_tokens.stdout)?;
    assert_eq!(
        no_tokens_report["sessions"][0]["recent_turn_average_tokens"],
        0
    );

    fs::write(
        &input,
        concat!(
            "{\"event\":\"turn.completed\",\"session_id\":\"lane-276\",\"turn_id\":\"turn-1\",\"cumulative_tokens\":1}\n",
            "{\"event\":\"turn.completed\",\"session_id\":\"lane-277\",\"turn_id\":\"turn-1\",\"cumulative_tokens\":1}\n"
        ),
    )?;
    let multiple_generic_sessions = audit(&input)?;
    assert!(!multiple_generic_sessions.status.success());
    assert!(stderr(&multiple_generic_sessions).contains("exactly one session"));

    let leading = "{}\n".repeat(16);
    fs::write(
        &input,
        format!(
            "{leading}{{\"type\":\"session_meta\",\"payload\":{{\"session_id\":\"lane-276\"}}}}\n"
        ),
    )?;
    let late_meta = audit(&input)?;
    assert!(
        late_meta.status.success(),
        "stderr:\n{}",
        stderr(&late_meta)
    );
    let late_meta_report: Value = serde_json::from_slice(&late_meta.stdout)?;
    assert_eq!(late_meta_report["session_count"], 1);
    Ok(())
}
#[test]
fn session_audit_rejects_aggregate_overflow_without_panicking() -> TestResult {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("audit-overflow.jsonl");
    fs::write(
        &input,
        concat!(
            "{\"event\":\"turn.completed\",\"session_id\":\"lane-276\",\"turn_id\":\"turn-1\",\"cumulative_tokens\":1,\"tool_calls\":[{\"tool\":\"exec\",\"output_bytes\":18446744073709551615}]}\n",
            "{\"event\":\"turn.completed\",\"session_id\":\"lane-276\",\"turn_id\":\"turn-2\",\"cumulative_tokens\":2,\"tool_calls\":[{\"tool\":\"exec\",\"output_bytes\":18446744073709551615}]}\n"
        ),
    )?;
    let generic = audit(&input)?;
    assert!(!generic.status.success());
    assert!(!stderr(&generic).contains("panicked"));
    assert!(stderr(&generic).contains("overflow"));

    fs::write(
        &input,
        concat!(
            "{\"type\":\"session_meta\",\"payload\":{\"session_id\":\"lane-276\"}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"total_tokens\":18446744073709551615},\"last_token_usage\":{\"total_tokens\":18446744073709551615}}}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"total_tokens\":18446744073709551614},\"last_token_usage\":{\"total_tokens\":18446744073709551615}}}}\n"
        ),
    )?;
    let codex = audit(&input)?;
    assert!(!codex.status.success());
    assert!(!stderr(&codex).contains("panicked"));
    assert!(stderr(&codex).contains("overflow"));
    Ok(())
}

#[test]
fn session_audit_rejects_empty_generic_metadata() -> TestResult {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("empty.jsonl");
    fs::write(&input, "")?;

    let output = audit(&input)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("exactly one session"));
    Ok(())
}
#[test]
fn session_audit_accepts_canonical_id_and_null_rate_limit_info() -> TestResult {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("canonical-session.jsonl");
    fs::write(
        &input,
        concat!(
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"lane-276\"}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":null}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"total_tokens\":120},\"last_token_usage\":{\"total_tokens\":40}}}}\n"
        ),
    )?;
    let output = audit(&input)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["sessions"][0]["session_id"], "lane-276");
    assert_eq!(report["sessions"][0]["latest_cumulative_tokens"], 120);
    assert_eq!(report["sessions"][0]["recent_turn_average_tokens"], 40);
    fs::write(
        &input,
        concat!(
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"lane-276\"}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":null}}}\n"
        ),
    )?;
    let malformed = audit(&input)?;
    assert!(!malformed.status.success());
    assert!(stderr(&malformed).contains("total_token_usage.total_tokens"));
    Ok(())
}
#[test]
fn session_audit_rejects_oversized_invalid_utf8_before_decoding() -> TestResult {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("oversized-invalid.jsonl");
    let file = fs::File::create(&input)?;
    file.set_len((8 * 1024 * 1024 + 1) as u64)?;
    drop(file);
    fs::write(&input, [0xff])?;
    let file = fs::OpenOptions::new().write(true).open(&input)?;
    file.set_len((8 * 1024 * 1024 + 1) as u64)?;

    let output = audit(&input)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("session metadata input exceeds 8388608 bytes"));
    assert!(!stderr(&output).contains("not valid UTF-8"));
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

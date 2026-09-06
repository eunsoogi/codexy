use serde_json::{Value, json};

use super::fixtures::{DELTA_HEAD, FULL_HEAD};

pub(crate) fn actual_request() -> Value {
    let mut request = super::fixtures::request();
    request["reviewer"]["pages"][0]["turns"][0]["items"] = json!([
        {
            "type": "userMessage",
            "id": "actual-request-delta",
            "content": [{
                "type": "text",
                "text": format!(
                    "Strict-profile SAME-REVIEWER DELTA only.\nReview current exact PR octo/example head {DELTA_HEAD}."
                )
            }]
        },
        {
            "type": "agentMessage",
            "id": "actual-message-delta",
            "phase": "final_answer",
            "text": actual_markdown(DELTA_HEAD, "src/delta.rs", "delta")
        }
    ]);
    request["reviewer"]["pages"][1]["turns"][0]["items"] = json!([
        {
            "type": "userMessage",
            "id": "actual-request-full",
            "content": [{
                "type": "text",
                "text": format!(
                    "Strict-profile full review.\nReview current exact PR octo/example head {FULL_HEAD}."
                )
            }]
        },
        {
            "type": "agentMessage",
            "id": "actual-message-full",
            "phase": "final_answer",
            "text": actual_markdown(FULL_HEAD, "src/full.rs", "full")
        }
    ]);
    request
}

pub(crate) fn negated_terminal_label_request() -> Value {
    terminal_value_request("No terminal verdict was issued.\n\nThe prior review remained BLOCK.")
}

pub(crate) fn invalid_inline_terminal_value_request() -> Value {
    terminal_value_request("Terminal verdict: not BLOCK")
}

pub(crate) fn invalid_narrative_terminal_value_request() -> Value {
    terminal_value_request("## Terminal verdict\n\nThe prior review remained BLOCK.")
}

fn terminal_value_request(replacement: &str) -> Value {
    let mut request = actual_request();
    let Some(original) = request["reviewer"]["pages"][0]["turns"][0]["items"][1]["text"].as_str()
    else {
        return request;
    };
    let text = original.replace("## Terminal verdict\n\n`BLOCK`", replacement);
    request["reviewer"]["pages"][0]["turns"][0]["items"][1]["text"] = json!(text);
    request
}

#[test]
fn negated_terminal_prose_is_not_a_label() {
    let error = super::rejected(super::native_history::normalize_native_history(
        &negated_terminal_label_request(),
    ));
    assert!(error.contains("terminal result"));
}

#[test]
fn unsupported_terminal_values_are_not_substrings() {
    for request in [
        invalid_inline_terminal_value_request(),
        invalid_narrative_terminal_value_request(),
    ] {
        let error = super::rejected(super::native_history::normalize_native_history(&request));
        assert!(
            error.contains("terminal result"),
            "unexpected error: {error}"
        );
    }
}

#[test]
fn unicode_terminal_values_fail_closed_without_panicking() {
    let request = terminal_value_request("Terminal verdict: 한국어로 결론이 없고 이전 BLOCK을 인용함");
    let result = std::panic::catch_unwind(|| {
        super::native_history::normalize_native_history(&request)
    });
    let error = super::rejected(result.expect("Unicode terminal value must not panic"));
    assert!(error.contains("terminal result"), "unexpected error: {error}");
}

pub(crate) fn request(crlf: bool) -> Value {
    let mut request = super::fixtures::request();
    let delta_prompt = json!({
        "type": "userMessage",
        "id": "request-delta",
        "content": [{
            "type": "text",
            "text": format!(
                "Strict-profile SAME-REVIEWER DELTA only.\nReview current exact PR octo/example head {DELTA_HEAD}."
            )
        }]
    });
    let full_prompt = json!({
        "type": "userMessage",
        "id": "request-full",
        "content": [{"type": "text", "text": "Strict profile, read-only review."}]
    });
    let delta = json!({
        "type": "AgentMessage",
        "id": "markdown-delta",
        "phase": "final_answer",
        "text": markdown_text(DELTA_HEAD, "src/delta.rs", "model.delta", crlf)
    });
    let full = json!({
        "type": "agentMessage",
        "id": "markdown-full",
        "phase": "final_answer",
        "text": markdown_text(FULL_HEAD, "src/full.rs", "model.full", crlf)
    });
    request["reviewer"]["pages"][0]["turns"][0]["items"] = json!([delta_prompt, delta]);
    request["reviewer"]["pages"][1]["turns"][0]["items"] = json!([full_prompt, full]);
    request
}

pub(crate) fn conflicting_terminal_request() -> Value {
    let mut request = request(false);
    let Some(original) = request["reviewer"]["pages"][1]["turns"][0]["items"][1]["text"].as_str()
    else {
        return request;
    };
    let text = original.replace("- Terminal result: BLOCK", "- Terminal result: PASS");
    request["reviewer"]["pages"][1]["turns"][0]["items"][1]["text"] = json!(text);
    request
}

pub(crate) fn conflicting_finding_labels_request() -> Value {
    let mut request = request(false);
    let Some(original) = request["reviewer"]["pages"][1]["turns"][0]["items"][1]["text"].as_str()
    else {
        return request;
    };
    let text = original.replace(
        "- Terminal result: BLOCK",
        "- Semantic kind: code\n- Semantic kind: proof\n- Terminal result: BLOCK",
    );
    request["reviewer"]["pages"][1]["turns"][0]["items"][1]["text"] = json!(text);
    request
}

fn actual_markdown(head: &str, path: &str, label: &str) -> String {
    let terminal_label = if label == "delta" {
        "Terminal verdict"
    } else {
        "Terminal result"
    };
    format!(
        "## Blocking findings\n\n1. **P1 — {label} source issue** — `in_scope_blocker`\n\n[{path}](https://example.test/{path})\n\n## {terminal_label}\n\n`BLOCK`\n\n- Exact head: `{head}`\n"
    )
}

fn markdown_text(head: &str, path: &str, model: &str, crlf: bool) -> String {
    let terminal_label = if path == "src/delta.rs" {
        "Terminal verdict"
    } else {
        "Terminal result"
    };
    let text = format!(
        "## Blocking findings — \x60BLOCK\x60\nAt exact PR HEAD \x60{head}\x60.\n\n1. **High — observed code issue** (\x60in_scope_blocker\x60)\n\n[{path}](https://example.test/{path})\n\n- {terminal_label}: BLOCK\n- Reviewer setting: {model} / medium\n\n2. **Medium — sibling issue (\x60in_scope_blocker\x60).** Same-line prose with (parentheses).\n\n[src/sibling.rs](https://example.test/src/sibling.rs)\n\n> - Terminal verdict: PASS\n> 1. **High — quoted issue** (\x60in_scope_blocker\x60)\n\n\x60\x60\x60text\n- Terminal verdict: PASS\n1. **High — fenced issue** (\x60in_scope_blocker\x60)\n\x60\x60\x60\n\n## Non-blocking follow-up\n- **\x60out_of_scope_followup\x60:** [plugins/followup.toml](https://example.test/plugins/followup.toml:3) remains outside this change.\n"
    );
    if crlf {
        text.replace('\n', "\r\n")
    } else {
        text
    }
}

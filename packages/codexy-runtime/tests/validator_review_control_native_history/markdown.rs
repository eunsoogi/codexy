use serde_json::{Value, json};

use super::fixtures::{DELTA_HEAD, FULL_HEAD};

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

fn markdown_text(head: &str, path: &str, model: &str, crlf: bool) -> String {
    let text = format!(
        "## Blocking findings — \x60BLOCK\x60\nAt exact PR HEAD \x60{head}\x60.\n\n1. **High — observed code issue** (\x60in_scope_blocker\x60)\n\n[{path}](https://example.test/{path})\n\n- Terminal result: BLOCK\n- Reviewer setting: {model} / medium\n\n2. **Medium — sibling issue (\x60in_scope_blocker\x60).** Same-line prose with (parentheses).\n\n[src/sibling.rs](https://example.test/src/sibling.rs)\n\n> - Terminal result: PASS\n> 1. **High — quoted issue** (\x60in_scope_blocker\x60)\n\n\x60\x60\x60text\n- Terminal result: PASS\n1. **High — fenced issue** (\x60in_scope_blocker\x60)\n\x60\x60\x60\n\n## Non-blocking follow-up\n- **\x60out_of_scope_followup\x60:** [plugins/followup.toml](https://example.test/plugins/followup.toml:3) remains outside this change.\n"
    );
    if crlf {
        text.replace('\n', "\r\n")
    } else {
        text
    }
}

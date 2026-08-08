use serde_json::Value;

use crate::lsp::protocol::LspMethod;

pub(super) fn has_publish_diagnostics(notifications: &[Value], uri: &str) -> bool {
    notifications.iter().any(|message| {
        message.get("method").and_then(Value::as_str) == Some("textDocument/publishDiagnostics")
            && message.pointer("/params/uri").and_then(Value::as_str) == Some(uri)
    })
}

pub(super) fn target_diagnostics(
    notifications: &[Value],
    uri: &str,
    method: LspMethod,
) -> Vec<Value> {
    notifications
        .iter()
        .filter(|message| {
            message.get("method").and_then(Value::as_str) == Some("textDocument/publishDiagnostics")
                && (!matches!(method, LspMethod::Diagnostics)
                    || message.pointer("/params/uri").and_then(Value::as_str) == Some(uri))
        })
        .filter_map(|message| message.get("params").cloned())
        .collect()
}

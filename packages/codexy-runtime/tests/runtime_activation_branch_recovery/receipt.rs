use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

pub(super) fn receipt_value() -> Value {
    let candidate = json!({
        "schema": "codexy-runtime-candidate/v1",
        "source": {"repository": "https://github.com/eunsoogi/codexy", "commit": "a".repeat(40)},
        "artifact": {"stagingRunId": 42, "stagingRunAttempt": 1},
        "compatibility": {"bootstrapApi": 1, "pluginRuntimeApi": 1, "transport": "stdio-newline-v1", "mcpProtocol": "2024-11-05"},
        "platforms": {
            "darwin-arm64": {"lsp": {"path": "runtime/codexy-mcp-lsp-darwin-arm64.bin", "sha256": "b".repeat(64)}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-darwin-arm64.bin", "sha256": "c".repeat(64)}},
            "linux-x86_64": {"lsp": {"path": "runtime/codexy-mcp-lsp-linux-x86_64.bin", "sha256": "d".repeat(64)}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-linux-x86_64.bin", "sha256": "e".repeat(64)}},
            "windows-x86_64": {"lsp": {"path": "runtime/codexy-mcp-lsp-windows-x86_64.exe", "sha256": "9".repeat(64)}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-windows-x86_64.exe", "sha256": "a".repeat(64)}}
        }
    });
    let candidate_bytes = serde_json::to_vec(&canonical(candidate.clone())).unwrap();
    json!({
        "schema": "codexy-runtime-candidate-receipt/v1", "candidate": candidate,
        "artifact": {"sha256": "f".repeat(64), "payloadManifestSha256": format!("{:x}", Sha256::digest(candidate_bytes))},
        "provenance": {"repositoryId": 1269350143, "workflowPath": ".github/workflows/runtime-candidate.yml", "runId": 42, "runAttempt": 1, "workflowRunUrl": "https://github.com/eunsoogi/codexy/actions/runs/42"}
    })
}

fn canonical(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries = map.into_iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            Value::Object(entries.into_iter().map(|(key, value)| (key, canonical(value))).collect())
        }
        Value::Array(values) => Value::Array(values.into_iter().map(canonical).collect()),
        other => other,
    }
}

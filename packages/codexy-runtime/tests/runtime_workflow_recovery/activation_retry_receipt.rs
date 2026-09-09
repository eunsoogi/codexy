use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) fn receipt(source: &str, tree: &str) -> Value {
    let mut devtools = json!({});
    let mut handoff = json!({});
    let mut watcher = json!({});
    for (platform, extension, kind) in [
        ("darwin-arm64", "bin", "mach-o"),
        ("linux-x86_64", "bin", "elf"),
        ("windows-x86_64", "exe", "pe"),
    ] {
        for name in ["lsp", "codegraph"] {
            devtools[platform][name] = json!({
                "path": format!("runtime/codexy-mcp-{name}-{platform}.{extension}"),
                "sha256": "b".repeat(64),
            });
        }
        handoff[platform] = json!({
            "path": format!("runtime/codexy-handoff-validate-{platform}.{extension}"),
            "sha256": "c".repeat(64), "kind": kind,
        });
        watcher[platform] = json!({
            "path": format!("runtime/codexy-mcp-watcher-{platform}.{extension}"),
            "sha256": "d".repeat(64), "kind": kind,
        });
    }
    let candidate = json!({
        "schema": "codexy-runtime-candidate/v1",
        "source": {"repository": "https://github.com/eunsoogi/codexy", "commit": source, "tree": tree},
        "artifact": {"stagingRunId": 42, "stagingRunAttempt": 1},
        "compatibility": {"bootstrapApi": 1, "pluginRuntimeApi": 1, "transport": "stdio-newline-v1", "mcpProtocol": "2024-11-05"},
        "platforms": devtools,
        "classes": {
            "devtoolsMcp": {"platforms": devtools},
            "coreHandoff": {"manifest": {"path": "handoff-runtime.json", "sha256": "e".repeat(64)}, "platforms": handoff},
            "coreWatcherMcp": {"platforms": watcher},
        },
    });
    json!({
        "schema": "codexy-runtime-candidate-receipt/v1", "candidate": candidate,
        "artifact": {"sha256": "f".repeat(64), "payloadManifestSha256": format!("{:x}", Sha256::digest(serde_json::to_vec(&canonical(candidate.clone())).unwrap()))},
        "provenance": {"repositoryId": 1269350143, "workflowPath": ".github/workflows/runtime-candidate.yml", "runId": 42, "runAttempt": 1, "workflowRunUrl": "https://github.com/eunsoogi/codexy/actions/runs/42"},
    })
}

fn canonical(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries = map.into_iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, canonical(value)))
                    .collect(),
            )
        }
        Value::Array(values) => Value::Array(values.into_iter().map(canonical).collect()),
        other => other,
    }
}

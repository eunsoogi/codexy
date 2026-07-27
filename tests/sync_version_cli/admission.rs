use std::{fs, path::Path, process::Command};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

#[test]
fn version_admission_matrix_is_ordered_and_fail_closed()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let current = super::archive_repository(&temp, "current")?;
    assert!(admit(&current, "1.2.2")?.status.success());
    assert!(!admit(&current, "1.1.0")?.status.success());

    for case in ["exact", "stale-bootstrap", "stale-runtime", "legacy-runtime", "wrapper-drift"] {
        let root = super::archive_repository(&temp, case)?;
        activate(&root)?;
        match case {
            "exact" => {}
            "stale-bootstrap" => mutate_json(
                &root.join(".agents/plugins/release-publish-contract.json"),
                |value| value["bootstrap"]["selectedVersion"] = json!("1.2.2"),
            )?,
            "stale-runtime" => mutate_json(
                &root.join(".agents/plugins/release-publish-contract.json"),
                |value| value["runtime"]["selectedTag"] = json!("v1.2.2"),
            )?,
            "legacy-runtime" => mutate_json(
                &root.join("plugins/codexy/runtime-release.json"),
                |value| value["state"] = json!("legacy-public"),
            )?,
            "wrapper-drift" => fs::write(
                root.join("plugins/codexy/mcp/codexy-mcp-lsp"),
                "#!/bin/sh\nexec uvx --from getcodexy==1.2.2 codexy-mcp-runtime lsp -- \"$@\"\n",
            )?,
            other => return Err(format!("unknown admission case: {other}").into()),
        }
        assert_eq!(
            admit(&root, "1.3.0")?.status.success(),
            case == "exact",
            "unexpected admission result for {case}"
        );
    }
    Ok(())
}

pub(super) fn activate(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let receipt = root.join("candidate-receipt.json");
    fs::write(&receipt, serde_json::to_vec(&receipt_value())?)?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-activate-runtime"))
        .args(["--repo-root", root.to_str().ok_or("root")?])
        .args(["--bootstrap-version", "1.3.0"])
        .args(["--candidate-receipt", receipt.to_str().ok_or("receipt")?])
        .output()?;
    assert!(
        output.status.success(),
        "activation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn receipt_value() -> Value {
    let digest = "b".repeat(64);
    let candidate = json!({
        "schema": "codexy-runtime-candidate/v1",
        "source": {"repository": "https://github.com/eunsoogi/codexy", "commit": "a".repeat(40)},
        "artifact": {"tag": "runtime-candidate-1.3.0"},
        "compatibility": {"bootstrapApi": 1, "pluginRuntimeApi": 1, "transport": "stdio-newline-v1", "mcpProtocol": "2024-11-05"},
        "platforms": {
            "darwin-arm64": {"lsp": {"path": "runtime/codexy-mcp-lsp-darwin-arm64.bin", "sha256": digest}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-darwin-arm64.bin", "sha256": "c".repeat(64)}},
            "linux-x86_64": {"lsp": {"path": "runtime/codexy-mcp-lsp-linux-x86_64.bin", "sha256": "d".repeat(64)}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-linux-x86_64.bin", "sha256": "e".repeat(64)}}
        }
    });
    let candidate_bytes = canonical(candidate.clone()).to_string();
    let payload_sha = format!("{:x}", Sha256::digest(candidate_bytes.as_bytes()));
    json!({
        "schema": "codexy-runtime-candidate-receipt/v1",
        "candidate": candidate,
        "artifact": {"url": "https://github.com/eunsoogi/codexy/releases/download/runtime-candidate-1.3.0/codexy-marketplace-plugin.tar.gz", "sha256": "f".repeat(64), "payloadManifestSha256": payload_sha},
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

fn admit(root: &Path, version: &str) -> Result<std::process::Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--admit-version", version])
        .env("CODEXY_REPO_ROOT", root)
        .current_dir(root)
        .output()
}

fn mutate_json(
    path: &Path,
    mutation: impl FnOnce(&mut Value),
) -> Result<(), Box<dyn std::error::Error>> {
    let mut value: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    mutation(&mut value);
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(&value)?))?;
    Ok(())
}

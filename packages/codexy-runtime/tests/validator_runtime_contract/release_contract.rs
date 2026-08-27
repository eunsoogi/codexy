use super::*;
use std::io::Write as _;

#[test]
fn validator_accepts_contract_free_public_bootstrap_source() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, plugin_root) = prepared_plugin()?;
    std::fs::remove_file(plugin_root.join("runtime-release.json"))?;
    assert!(validate(&plugin_root)?.status.success());
    Ok(())
}

#[test]
fn validator_accepts_source_selected_pointer_without_tracked_candidate() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, plugin_root) = prepared_plugin()?;
    write_release(&plugin_root, &source_selected_release())?;
    assert!(!plugin_root.join("runtime-candidate.json").exists());
    assert!(validate(&plugin_root)?.status.success());
    Ok(())
}

#[test]
fn validator_accepts_structurally_valid_legacy_public_without_historical_pins() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, plugin_root) = prepared_plugin()?;
    write_release(&plugin_root, &legacy_public_release())?;
    let output = validate(&plugin_root)?;
    assert!(
        output.status.success(),
        "structurally valid legacy-public release rejected: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_legacy_public_malformed_uppercase_and_identity_drift() -> Result<(), Box<dyn std::error::Error>> {
    let cases: [(&str, fn() -> serde_json::Value, fn(&mut serde_json::Value)); 6] = [
        ("malformed digest", legacy_public_release, |release| release["artifact"]["sha256"] = serde_json::json!("not-a-digest")),
        ("uppercase digest", legacy_public_release, |release| release["artifact"]["sha256"] = serde_json::json!("A".repeat(64))),
        ("wrong repository", legacy_public_release, |release| release["source"]["repository"] = serde_json::json!("https://example.com/repo")),
        ("wrong tag", legacy_public_release, |release| release["artifact"]["tag"] = serde_json::json!("v1.2.3")),
        ("wrong URL", legacy_public_release, |release| release["artifact"]["url"] = serde_json::json!("https://github.com/eunsoogi/codexy/releases/download/v1.2.2/wrong.tar.gz")),
        ("wrong provenance", source_selected_release, |release| release["provenance"]["repositoryId"] = serde_json::json!(1)),
    ];
    for (label, make_release, mutate) in cases {
        let (_temp, plugin_root) = prepared_plugin()?;
        let mut release = make_release();
        mutate(&mut release);
        write_release(&plugin_root, &release)?;
        assert!(!validate(&plugin_root)?.status.success(), "{label} accepted");
    }
    Ok(())
}

#[test]
fn validator_rejects_runtime_release_unknown_fields() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, plugin_root) = prepared_plugin()?;
    let path = plugin_root.join("runtime-release.json");
    let mut contract: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    contract["untrusted"] = serde_json::json!(true);
    write_release(&plugin_root, &contract)?;

    let output = validate(&plugin_root)?;
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("untrusted"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_accepts_valid_candidate_proven_and_rejects_unsafe_tags() -> Result<(), Box<dyn std::error::Error>> {
    let valid = tempfile::tempdir()?;
    let valid_root = copy_plugin_to(valid.path())?;
    declare_bundled_platforms(&valid_root)?;
    write_candidate_release(&valid_root, "v1.3.0")?;
    let output = validate(&valid_root)?;
    assert!(
        output.status.success(),
        "valid candidate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for tag in ["runtime-candidate-1.3.0", "v1.3", "v1.3.0-rc1"] {
        let (_temp, plugin_root) = prepared_plugin()?;
        write_candidate_release(&plugin_root, tag)?;
        assert!(!validate(&plugin_root)?.status.success(), "unsafe tag accepted: {tag}");
    }
    Ok(())
}

fn write_candidate_release(plugin_root: &std::path::Path, tag: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = plugin_root.join("runtime-release.json");
    let mut release: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    release["state"] = serde_json::json!("candidate-proven");
    release
        .as_object_mut()
        .ok_or("runtime release object")?
        .remove("provenance");
    release
        .as_object_mut()
        .ok_or("runtime release object")?
        .remove("classes");
    release["source"]
        .as_object_mut()
        .ok_or("runtime release source object")?
        .remove("tree");
    release["artifact"]["tag"] = serde_json::json!(tag);
    release["artifact"]["url"] = serde_json::json!(format!("https://github.com/eunsoogi/codexy/releases/download/{tag}/codexy-runtime-package.tar.gz"));
    for platform in ["darwin-arm64", "linux-x86_64"] {
        for server in ["lsp", "codegraph"] {
            release["platforms"][platform][server]["path"] = serde_json::json!(format!("runtime/codexy-mcp-{server}-{platform}.bin"));
        }
    }
    std::fs::write(&path, serde_json::to_string_pretty(&release)?)?;
    let candidate = serde_json::json!({
        "schema": "codexy-runtime-candidate/v1",
        "source": release["source"].clone(),
        "artifact": {"stagingRunId": 1, "stagingRunAttempt": 1},
        "compatibility": release["compatibility"].clone(),
        "platforms": release["platforms"].clone(),
    });
    std::fs::write(plugin_root.join("runtime-candidate.json"), serde_json::to_string(&candidate)?)?;
    Ok(())
}

fn source_selected_release() -> serde_json::Value {
    let source_platforms = serde_json::json!({
        "darwin-arm64": {
            "lsp": {"path": "runtime/codexy-mcp-lsp-darwin-arm64.bin", "sha256": "b".repeat(64)},
            "codegraph": {"path": "runtime/codexy-mcp-codegraph-darwin-arm64.bin", "sha256": "c".repeat(64)}
        },
        "linux-x86_64": {
            "lsp": {"path": "runtime/codexy-mcp-lsp-linux-x86_64.bin", "sha256": "d".repeat(64)},
            "codegraph": {"path": "runtime/codexy-mcp-codegraph-linux-x86_64.bin", "sha256": "e".repeat(64)}
        }
    });
    serde_json::json!({
        "schema": "codexy-runtime-release/v1",
        "state": "source-selected",
        "source": {
            "repository": "https://github.com/eunsoogi/codexy",
            "commit": "a".repeat(40),
            "tree": "b".repeat(40)
        },
        "artifact": {
            "tag": "v1.5.0",
            "url": "https://github.com/eunsoogi/codexy/releases/download/v1.5.0/codexy-runtime-package.tar.gz",
            "sha256": "c".repeat(64),
            "payloadManifestSha256": "d".repeat(64)
        },
        "provenance": {
            "repositoryId": 1269350143,
            "workflowPath": ".github/workflows/runtime-candidate.yml",
            "runId": 42,
            "runAttempt": 1,
            "workflowRunUrl": "https://github.com/eunsoogi/codexy/actions/runs/42"
        },
        "compatibility": {
            "bootstrapApi": 1,
            "pluginRuntimeApi": 1,
            "transport": "stdio-newline-v1",
            "mcpProtocol": "2024-11-05"
        },
        "platforms": source_platforms,
        "classes": {
            "devtoolsMcp": {"platforms": source_platforms},
            "coreHandoff": {
                "manifest": {"path": "handoff-runtime.json", "sha256": "e".repeat(64)},
                "platforms": {
                    "darwin-arm64": {"path": "runtime/codexy-handoff-validate-darwin-arm64.bin", "sha256": "f".repeat(64), "kind": "mach-o"},
                    "linux-x86_64": {"path": "runtime/codexy-handoff-validate-linux-x86_64.bin", "sha256": "0".repeat(64), "kind": "elf"},
                    "windows-x86_64": {"path": "runtime/codexy-handoff-validate-windows-x86_64.exe", "sha256": "1".repeat(64), "kind": "pe"}
                }
            }
        }
    })
}

fn legacy_public_release() -> serde_json::Value {
    serde_json::json!({
        "schema": "codexy-runtime-release/v1",
        "state": "legacy-public",
        "source": {
            "repository": "https://github.com/eunsoogi/codexy",
            "commit": "f".repeat(40)
        },
        "artifact": {
            "tag": "v1.2.2",
            "url": "https://github.com/eunsoogi/codexy/releases/download/v1.2.2/codexy-marketplace-plugin.tar.gz",
            "sha256": "a".repeat(64),
            "payloadManifestSha256": "b".repeat(64)
        },
        "compatibility": {
            "bootstrapApi": 1,
            "pluginRuntimeApi": 1,
            "transport": "stdio-newline-v1",
            "mcpProtocol": "2024-11-05"
        },
        "platforms": {
            "darwin-arm64": {
                "lsp": {"sha256": "c".repeat(64)},
                "codegraph": {"sha256": "d".repeat(64)}
            },
            "linux-x86_64": {
                "lsp": {"sha256": "e".repeat(64)},
                "codegraph": {"sha256": "f".repeat(64)}
            }
        }
    })
}

fn prepared_plugin() -> Result<(tempfile::TempDir, std::path::PathBuf), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    declare_bundled_platforms(&plugin_root)?;
    Ok((temp, plugin_root))
}

fn write_release(plugin_root: &std::path::Path, release: &serde_json::Value) -> std::io::Result<()> {
    std::fs::write(
        plugin_root.join("runtime-release.json"),
        serde_json::to_string_pretty(release).expect("release serializes"),
    )
}

fn validate(plugin_root: &std::path::Path) -> std::io::Result<std::process::Output> {
    Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .arg("--plugin-root")
        .arg(plugin_root)
        .arg("--check")
        .output()
}

fn declare_bundled_platforms(plugin_root: &std::path::Path) -> std::io::Result<()> {
    for server in ["lsp", "codegraph"] {
        let path = plugin_root.join(format!("mcp/codexy-mcp-{server}"));
        std::fs::OpenOptions::new()
            .append(true)
            .open(path)?
            .write_all(b"\nbundled_platforms=\"darwin-arm64 linux-x86_64\"\n")?;
    }
    Ok(())
}

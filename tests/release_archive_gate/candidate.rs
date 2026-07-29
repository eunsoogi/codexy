use std::path::Path;

use sha2::{Digest as _, Sha256};

use super::*;
use crate::support::FixtureCommand;

#[test]
fn archive_gate_accepts_a_complete_candidate_proven_windows_package() {
    let root = tempdir().expect("candidate package root");
    let plugin_root = complete_plugin_fixture(root.path()).expect("candidate plugin fixture");
    let archive = root.path().join("candidate-proven-windows.tar.gz");
    make_candidate_proven_windows_package(&plugin_root);
    create_archive(root.path(), &archive).expect("candidate archive");

    let output = run_candidate_gate(root.path(), &archive, &plugin_root);
    assert!(
        output.status.success(),
        "candidate-proven archive failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn archive_gate_rejects_a_candidate_windows_entrypoint_with_wrong_identity() {
    let (root, plugin_root, archive) = complete_archive_fixture("candidate-entrypoint-identity");
    make_candidate_proven_windows_package(&plugin_root);
    std::fs::write(
        plugin_root.join("mcp/codexy-mcp-lsp.exe"),
        b"different entrypoint",
    )
    .expect("mutate entrypoint");
    create_archive(root.path(), &archive).expect("candidate archive");

    let output = run_candidate_gate(root.path(), &archive, &plugin_root);
    assert!(!output.status.success());
}

#[test]
fn archive_gate_rejects_a_candidate_runtime_path_outside_its_contract() {
    let (root, plugin_root, archive) = complete_archive_fixture("candidate-runtime-path");
    make_candidate_proven_windows_package(&plugin_root);
    let release_path = plugin_root.join("runtime-release.json");
    let mut release: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&release_path).expect("release contract"))
            .expect("release contract JSON");
    release["platforms"]["windows-x86_64"]["lsp"]["path"] =
        serde_json::json!("runtime/codexy-mcp-lsp-untrusted.exe");
    std::fs::write(
        &release_path,
        serde_json::to_vec_pretty(&release).expect("release JSON"),
    )
    .expect("malformed candidate release");
    create_archive(root.path(), &archive).expect("candidate archive");

    assert!(
        !run_candidate_gate(root.path(), &archive, &plugin_root)
            .status
            .success()
    );
}

fn run_candidate_gate(root: &Path, archive: &Path, plugin_root: &Path) -> std::process::Output {
    let repo_root = root.join("candidate-repository");
    std::fs::create_dir_all(repo_root.join(".agents/plugins")).expect("candidate contract parent");
    std::fs::create_dir_all(repo_root.join(".github/workflows"))
        .expect("candidate workflow parent");
    std::fs::create_dir_all(repo_root.join("scripts")).expect("candidate scripts parent");
    copy_candidate_source(".agents/plugins/release-publish-contract.json", &repo_root);
    copy_candidate_source(".github/workflows/plugin-runtime-binaries.yml", &repo_root);
    copy_candidate_source("scripts/generate-release-changelog", &repo_root);
    let contract = repo_root.join(".agents/plugins/release-publish-contract.json");
    let mut document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&contract).expect("publish contract"))
            .expect("contract JSON");
    document["package"]["platforms"] =
        serde_json::json!(["darwin-arm64", "linux-x86_64", "windows-x86_64"]);
    std::fs::write(
        &contract,
        serde_json::to_vec_pretty(&document).expect("contract JSON"),
    )
    .expect("candidate publish contract");
    let mut command = FixtureCommand::new(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-release-archive"),
    );
    command
        .arg_path(archive)
        .arg_path(plugin_root)
        .env_path("CODEXY_REPO_ROOT", &repo_root)
        .output()
        .expect("archive gate should start")
}

fn copy_candidate_source(relative: &str, repo_root: &Path) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    let target = repo_root.join(relative);
    std::fs::copy(source, target).expect("candidate source contract");
}

fn make_candidate_proven_windows_package(plugin_root: &Path) {
    let mut manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(plugin_root.join(".codex-plugin/plugin.json")).expect("manifest"),
    )
    .expect("manifest JSON");
    manifest["supportedPlatforms"] =
        serde_json::json!(["darwin-arm64", "linux-x86_64", "windows-x86_64"]);
    std::fs::write(
        plugin_root.join(".codex-plugin/plugin.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&manifest).expect("manifest JSON")
        ),
    )
    .expect("candidate manifest");

    for server in ["lsp", "codegraph"] {
        let wrapper = plugin_root.join(format!("mcp/codexy-mcp-{server}"));
        let updated = std::fs::read_to_string(&wrapper).expect("wrapper").replace(
            "bundled_platforms=\"darwin-arm64 linux-x86_64\"",
            "bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"",
        );
        std::fs::write(wrapper, updated).expect("candidate wrapper");
    }

    let mut windows = vec![0; 4096];
    windows[0..2].copy_from_slice(b"MZ");
    windows[0x3c..0x40].copy_from_slice(&0x80_u32.to_le_bytes());
    windows[0x80..0x84].copy_from_slice(b"PE\0\0");
    windows[0x84..0x86].copy_from_slice(&0x8664_u16.to_le_bytes());
    windows[0x86..0x88].copy_from_slice(&1_u16.to_le_bytes());
    windows[0x94..0x96].copy_from_slice(&0xf0_u16.to_le_bytes());
    windows[0x96..0x98].copy_from_slice(&0x0022_u16.to_le_bytes());
    windows[0x98..0x9a].copy_from_slice(&0x20b_u16.to_le_bytes());
    for server in ["lsp", "codegraph"] {
        let runtime = plugin_root.join(format!("runtime/codexy-mcp-{server}-windows-x86_64.exe"));
        std::fs::write(&runtime, &windows).expect("Windows runtime");
        std::fs::write(
            plugin_root.join(format!("mcp/codexy-mcp-{server}.exe")),
            &windows,
        )
        .expect("Windows entrypoint");
    }

    let mut release: serde_json::Value = serde_json::from_slice(
        &std::fs::read(plugin_root.join("runtime-release.json")).expect("release contract"),
    )
    .expect("release contract JSON");
    release["state"] = serde_json::json!("candidate-proven");
    release["artifact"]["tag"] = serde_json::json!("v1.3.0");
    release["artifact"]["url"] = serde_json::json!(
        "https://github.com/eunsoogi/codexy/releases/download/v1.3.0/codexy-runtime-package.tar.gz"
    );
    release["artifact"]["sha256"] = serde_json::json!("f".repeat(64));
    for platform in ["darwin-arm64", "linux-x86_64", "windows-x86_64"] {
        for server in ["lsp", "codegraph"] {
            let extension = if platform == "windows-x86_64" {
                "exe"
            } else {
                "bin"
            };
            let path = format!("runtime/codexy-mcp-{server}-{platform}.{extension}");
            let digest = hex_digest(&std::fs::read(plugin_root.join(&path)).expect("runtime"));
            release["platforms"][platform][server] =
                serde_json::json!({"path": path, "sha256": digest});
        }
    }
    let candidate = serde_json::json!({
        "schema": "codexy-runtime-candidate/v1",
        "source": release["source"].clone(),
        "artifact": {"stagingRunId": 42, "stagingRunAttempt": 1},
        "compatibility": release["compatibility"].clone(),
        "platforms": release["platforms"].clone(),
    });
    let candidate_bytes = serde_json::to_vec(&candidate).expect("candidate JSON");
    release["artifact"]["payloadManifestSha256"] = serde_json::json!(hex_digest(&candidate_bytes));
    std::fs::write(plugin_root.join("runtime-candidate.json"), candidate_bytes)
        .expect("candidate contract");
    std::fs::write(
        plugin_root.join("runtime-release.json"),
        serde_json::to_vec_pretty(&release).expect("release JSON"),
    )
    .expect("candidate release");
}

fn hex_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

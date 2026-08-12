use std::{fs, path::Path};

use super::{Command, candidate::make_candidate_proven_windows_package, complete_plugin_fixture};

#[test]
fn public_release_contract_accepts_exact_windows_delegates() {
    let root = tempfile::tempdir().expect("full plugin fixture");
    let plugin = full_windows_plugin(root.path());
    let output = contract(&plugin);
    assert!(
        output.status.success(),
        "exact Windows delegates must pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_runtime_inventory(
        output.stdout,
        "runtime/codexy-mcp-lsp-darwin-arm64.bin\n\
runtime/codexy-mcp-codegraph-darwin-arm64.bin\n\
runtime/codexy-mcp-lsp-linux-x86_64.bin\n\
runtime/codexy-mcp-codegraph-linux-x86_64.bin\n\
runtime/codexy-mcp-lsp-windows-x86_64.exe\n\
runtime/codexy-mcp-codegraph-windows-x86_64.exe\n",
    );
}

#[test]
fn public_release_contract_rejects_missing_windows_delegate() {
    for server in ["lsp", "codegraph"] {
        let root = tempfile::tempdir().expect("full plugin fixture");
        let plugin = full_windows_plugin(root.path());
        fs::remove_file(plugin.join(format!("mcp/codexy-mcp-{server}.cmd")))
            .expect("remove delegate");
        assert_rejected(
            &plugin,
            &format!("thin {server} delegate"),
            &format!("public Windows package requires the thin {server} delegate"),
        );
    }
}

#[test]
fn public_release_contract_rejects_non_delegate_windows_launchers() {
    for (label, server, body) in [
        ("malformed", "lsp", b"@echo off\nbroken\n".as_slice()),
        (
            "wrong server",
            "lsp",
            b"@echo off\n\"%~dp0codexy-mcp-devtools.exe\" codegraph %*\nexit /b %ERRORLEVEL%\n",
        ),
        ("native bytes", "codegraph", b"MZnot-a-delegate"),
    ] {
        let root = tempfile::tempdir().expect("full plugin fixture");
        let plugin = full_windows_plugin(root.path());
        fs::write(plugin.join(format!("mcp/codexy-mcp-{server}.cmd")), body)
            .expect("replace delegate");
        assert_rejected(
            &plugin,
            label,
            &format!("public Windows package requires the thin {server} delegate"),
        );
    }
}

#[test]
fn public_release_contract_rejects_legacy_native_entrypoints() {
    for server in ["lsp", "codegraph"] {
        let root = tempfile::tempdir().expect("full plugin fixture");
        let plugin = full_windows_plugin(root.path());
        fs::write(
            plugin.join(format!("mcp/codexy-mcp-{server}.exe")),
            b"MZlegacy",
        )
        .expect("add duplicate native entrypoint");
        assert_rejected(
            &plugin,
            &format!("legacy {server} native entrypoint"),
            &format!("public package must not contain the legacy native {server} entrypoint"),
        );
    }
}

#[test]
fn legacy_public_release_contract_accepts_absence_and_rejects_retained_delegate() {
    let root = tempfile::tempdir().expect("legacy plugin fixture");
    let plugin = legacy_plugin(root.path());
    let output = contract(&plugin);
    assert!(
        output.status.success(),
        "legacy archive without Windows delegates must pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_runtime_inventory(
        output.stdout,
        "runtime/codexy-mcp-lsp-darwin-arm64.bin\n\
runtime/codexy-mcp-codegraph-darwin-arm64.bin\n\
runtime/codexy-mcp-lsp-linux-x86_64.bin\n\
runtime/codexy-mcp-codegraph-linux-x86_64.bin\n",
    );
    for server in ["lsp", "codegraph"] {
        fs::write(
            plugin.join(format!("mcp/codexy-mcp-{server}.cmd")),
            format!(
                "@echo off\n\"%~dp0codexy-mcp-devtools.exe\" {server} %*\nexit /b %ERRORLEVEL%\n"
            ),
        )
        .expect("restore delegate");
        assert_rejected(
            &plugin,
            &format!("retained {server} delegate"),
            "dispatcher-free legacy projection must not package Windows runtime files",
        );
        fs::remove_file(plugin.join(format!("mcp/codexy-mcp-{server}.cmd")))
            .expect("remove retained delegate");
    }
}

fn full_windows_plugin(root: &Path) -> std::path::PathBuf {
    let plugin = complete_plugin_fixture(root).expect("plugin fixture");
    make_candidate_proven_windows_package(&plugin);
    remove_runtime_contracts(&plugin);
    plugin
}

fn legacy_plugin(root: &Path) -> std::path::PathBuf {
    let plugin = complete_plugin_fixture(root).expect("plugin fixture");
    remove_runtime_contracts(&plugin);
    for server in ["lsp", "codegraph"] {
        fs::remove_file(plugin.join(format!("mcp/codexy-mcp-{server}.cmd")))
            .expect("remove legacy delegate");
    }
    plugin
}

fn remove_runtime_contracts(plugin: &Path) {
    for name in ["runtime-release.json", "runtime-candidate.json"] {
        let path = plugin.join(name);
        if path.exists() {
            fs::remove_file(path).expect("remove runtime contract");
        }
    }
}

fn contract(plugin: &Path) -> std::process::Output {
    Command::new("python3")
        .arg(
            codexy_runtime::paths::repository_root()
                .join("scripts/inspect-release-archive-contract.py"),
        )
        .args(["public-release"])
        .arg(plugin)
        .output()
        .expect("archive contract should start")
}

#[test]
fn runtime_inventory_normalizes_platform_newlines_before_exact_comparison() {
    let expected = "runtime/lsp\nruntime/codegraph\n";
    assert_eq!(
        normalize_runtime_inventory("runtime/lsp\r\nruntime/codegraph\r\n"),
        expected
    );
    assert_eq!(normalize_runtime_inventory(expected), expected);
}

fn assert_runtime_inventory(stdout: Vec<u8>, expected: &str) {
    assert_eq!(
        normalize_runtime_inventory(&String::from_utf8(stdout).expect("contract stdout")),
        expected
    );
}

fn normalize_runtime_inventory(stdout: &str) -> String {
    stdout.replace("\r\n", "\n")
}

fn assert_rejected(plugin: &Path, label: &str, expected: &str) {
    let output = contract(plugin);
    assert!(
        !output.status.success(),
        "{label} unexpectedly satisfied the archive contract"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "{label} produced unexpected archive-contract failure: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

use std::process::Command;

use serde_json::{Value, json};
use tempfile::tempdir;

#[path = "support/release_archive.rs"]
mod release_archive_support;
use release_archive_support::{complete_plugin_fixture_with_stubbed_runtime, create_archive};

fn run_gate(archive: &std::path::Path, plugin_root: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_MANIFEST_DIR").to_owned() + "/scripts/inspect-release-archive")
        .arg(archive)
        .arg(plugin_root)
        .output()
        .expect("archive gate should start")
}

fn write_mcp_config(plugin_root: &std::path::Path, nested: bool, argv: bool) {
    let lsp_command = if argv {
        json!(["./mcp/codexy-mcp-lsp", "--stdio"])
    } else {
        json!("./mcp/codexy-mcp-lsp")
    };
    let mut servers = json!({
        "lsp": {"command": lsp_command, "cwd": "."},
        "codegraph": {"command": "./mcp/codexy-mcp-codegraph", "args": ["--stdio"], "cwd": "."},
        "grep_app": {"url": "https://mcp.grep.app"}
    });
    let config: Value = if nested {
        json!({"mcp_servers": servers})
    } else {
        servers.take()
    };
    std::fs::write(
        plugin_root.join(".mcp.json"),
        serde_json::to_vec_pretty(&config).expect("MCP config JSON"),
    )
    .expect("write MCP config");
}

fn entrypoints(config: &std::path::Path) -> Vec<String> {
    let output = Command::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-mcp-entrypoints"),
    )
    .arg(config)
    .output()
    .expect("MCP entrypoint inspector should start");
    assert!(
        output.status.success(),
        "MCP entrypoint inspector failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("MCP entrypoint output")
        .lines()
        .map(str::to_owned)
        .collect()
}

#[cfg(unix)]
fn assert_wrapper_mode(label: &str, nested: bool, argv: bool) {
    use std::os::unix::fs::PermissionsExt;

    let root = tempdir().expect("tempdir");
    let plugin_root =
        complete_plugin_fixture_with_stubbed_runtime(root.path()).expect("complete plugin fixture");
    write_mcp_config(&plugin_root, nested, argv);
    assert_eq!(
        entrypoints(&plugin_root.join(".mcp.json")),
        ["mcp/codexy-mcp-lsp", "mcp/codexy-mcp-codegraph"],
        "{label} MCP config must expose both packaged wrappers"
    );

    let wrapper = plugin_root.join("mcp/codexy-mcp-lsp");
    let mut permissions = std::fs::metadata(&wrapper)
        .expect("wrapper metadata")
        .permissions();
    permissions.set_mode(0o644);
    std::fs::set_permissions(&wrapper, permissions).expect("non-executable wrapper fixture");
    let invalid_archive = root.path().join(format!("{label}-invalid.tar.gz"));
    create_archive(root.path(), &invalid_archive).expect("archive fixture");
    let invalid_output = run_gate(&invalid_archive, &plugin_root);
    assert!(!invalid_output.status.success());
    assert!(
        String::from_utf8_lossy(&invalid_output.stderr)
            .contains("packaged MCP wrapper is not executable: mcp/codexy-mcp-lsp")
    );
}

#[cfg(unix)]
#[test]
fn archive_gate_checks_direct_argv_wrapper_mode() {
    assert_wrapper_mode("direct-argv", false, true);
}

#[cfg(unix)]
#[test]
fn archive_gate_checks_nested_server_map_wrapper_mode() {
    assert_wrapper_mode("nested-server-map", true, false);
}

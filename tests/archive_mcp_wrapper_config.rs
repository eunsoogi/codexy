use std::process::Command;

use serde_json::{Value, json};
use tempfile::tempdir;

#[path = "support/release_archive.rs"]
mod release_archive_support;
use release_archive_support::{complete_plugin_fixture, make_executable};

fn run_gate(archive: &std::path::Path, plugin_root: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_MANIFEST_DIR").to_owned() + "/scripts/inspect-release-archive")
        .arg(archive)
        .arg(plugin_root)
        .output()
        .expect("archive gate should start")
}

fn create_archive(root: &std::path::Path, archive: &std::path::Path) {
    let status = Command::new("tar")
        .args(["-C"])
        .arg(root)
        .args(["-czf"])
        .arg(archive)
        .arg("plugins/codexy")
        .status()
        .expect("tar should start");
    assert!(status.success(), "tar failed: {status}");
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

#[cfg(unix)]
#[test]
fn archive_gate_checks_wrapper_modes_for_supported_mcp_config_shapes() {
    use std::os::unix::fs::PermissionsExt;

    for (label, nested, argv) in [
        ("direct-argv", false, true),
        ("nested-server-map", true, false),
    ] {
        let root = tempdir().expect("tempdir");
        let plugin_root = complete_plugin_fixture(root.path()).expect("complete plugin fixture");
        write_mcp_config(&plugin_root, nested, argv);

        let valid_archive = root.path().join(format!("{label}-valid.tar.gz"));
        create_archive(root.path(), &valid_archive);
        let valid_output = run_gate(&valid_archive, &plugin_root);
        assert!(
            valid_output.status.success(),
            "valid {label} fixture failed: {}",
            String::from_utf8_lossy(&valid_output.stderr)
        );

        let wrapper = plugin_root.join("mcp/codexy-mcp-lsp");
        let mut permissions = std::fs::metadata(&wrapper)
            .expect("wrapper metadata")
            .permissions();
        permissions.set_mode(0o644);
        std::fs::set_permissions(&wrapper, permissions).expect("non-executable wrapper fixture");
        let invalid_archive = root.path().join(format!("{label}-invalid.tar.gz"));
        create_archive(root.path(), &invalid_archive);
        let invalid_output = run_gate(&invalid_archive, &plugin_root);
        assert!(!invalid_output.status.success());
        assert!(
            String::from_utf8_lossy(&invalid_output.stderr)
                .contains("packaged MCP wrapper is not executable: mcp/codexy-mcp-lsp")
        );
        make_executable(&wrapper).expect("restore executable wrapper");
    }
}

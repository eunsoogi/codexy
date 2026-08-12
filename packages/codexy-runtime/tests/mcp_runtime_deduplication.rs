use std::path::Path;

#[test]
fn public_mcp_servers_share_one_runtime_delegate_without_windows_server_copies()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let plugin = root.join("plugins/codexy-devtools");
    let mcp: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(plugin.join(".mcp.json"))?)?;

    for server in ["lsp", "codegraph"] {
        assert_eq!(
            mcp[server]["command"].as_str(),
            Some("./mcp/codexy-mcp-devtools"),
            "{server} must use the shared public MCP delegate"
        );
        assert_eq!(
            mcp[server]["args"],
            serde_json::json!([server, "--stdio"]),
            "{server} must retain its public server identity"
        );

        let legacy = std::fs::read_to_string(plugin.join(format!("mcp/codexy-mcp-{server}")))?;
        assert!(
            legacy.contains(&format!("exec \"$self_dir/codexy-mcp-devtools\" {server} \"$@\"")),
            "{server} legacy entrypoint must remain a transparent delegate"
        );
        let windows = std::fs::read(plugin.join(format!("mcp/codexy-mcp-{server}.cmd")))?;
        let expected = format!(
            "@echo off\n\"%~dp0codexy-mcp-devtools.exe\" {server} %*\nexit /b %ERRORLEVEL%\n"
        );
        assert_eq!(windows, expected.as_bytes(), "{server} Windows entrypoint must be a thin delegate");
        assert!(!windows.starts_with(b"MZ"), "{server} Windows entrypoint must not contain native bytes");
    }

    let delegate = std::fs::read_to_string(plugin.join("mcp/codexy-mcp-devtools"))?;
    assert_eq!(delegate.matches("exec uvx --from getcodexy==").count(), 1);
    assert!(delegate.contains("runtime_name=\"codexy-mcp-$server-$platform.$runtime_extension\""));

    let assembly = std::fs::read_to_string(root.join("scripts/assemble-runtime-candidate"))?;
    assert!(assembly.contains("codexy-mcp-devtools-windows-x86_64.exe"));
    for server in ["lsp", "codegraph"] {
        assert!(
            !assembly.contains(&format!("codexy-mcp-{server}.exe")),
            "candidate assembly must not copy the {server} runtime into mcp/"
        );
    }
    assert!(Path::new("plugins/codexy-devtools/mcp/codexy-mcp-devtools").is_relative());
    Ok(())
}

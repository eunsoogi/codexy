use super::super::*;

#[test]
fn codegraph_wrapper_reports_missing_declared_windows_runtime_without_running_macos_binary()
-> Result<(), Box<dyn std::error::Error>> {
    let installed_plugin = installed_plugin_copy()?;
    let output = Command::new(installed_plugin.path.join("mcp/codexy-mcp-codegraph"))
        .current_dir(&installed_plugin.path)
        .env("PATH", "/usr/bin:/bin")
        .env("CODEXY_RUNTIME_PLATFORM", "windows-x86_64")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    assert_eq!(output.status.code(), Some(127));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("windows-x86_64"),
        "unsupported platform failure should name the missing runtime, got {stderr:?}"
    );
    assert!(
        stderr.contains("codexy-mcp-codegraph-windows-x86_64.exe"),
        "missing-runtime failure should name the declared Windows executable, got {stderr:?}"
    );
    assert!(
        !stderr.contains("Exec format"),
        "wrapper must not attempt to execute an incompatible bundled runtime"
    );
    Ok(())
}

#[test]
fn lsp_wrapper_reports_missing_declared_windows_runtime_without_running_macos_binary()
-> Result<(), Box<dyn std::error::Error>> {
    let installed_plugin = installed_plugin_copy()?;
    let output = Command::new(installed_plugin.path.join("mcp/codexy-mcp-lsp"))
        .current_dir(&installed_plugin.path)
        .env("PATH", "/usr/bin:/bin")
        .env("CODEXY_RUNTIME_PLATFORM", "windows-x86_64")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    assert_eq!(output.status.code(), Some(127));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("codexy-mcp-lsp-windows-x86_64.exe"),
        "missing-runtime failure should name the declared Windows executable, got {stderr:?}"
    );
    assert!(
        !stderr.contains("Exec format"),
        "wrapper must not attempt to execute an incompatible bundled runtime"
    );
    Ok(())
}

#[test]
fn codegraph_wrapper_uses_validated_runtime_dir_for_platform_runtime()
-> Result<(), Box<dyn std::error::Error>> {
    let installed_plugin = installed_plugin_copy()?;
    let runtime_dir = temp_runtime_dir(
        "codexy-mcp-codegraph-linux-x86_64.bin",
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
    )?;
    let mut command = Command::new(installed_plugin.path.join("mcp/codexy-mcp-codegraph"));
    command
        .current_dir(&installed_plugin.path)
        .env("PATH", "/usr/bin:/bin")
        .env("CODEXY_RUNTIME_PLATFORM", "linux-x86_64")
        .env("CODEXY_RUNTIME_DIR", &runtime_dir.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut client = McpClient::spawn_command(command)?;
    let init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-codegraph");
    Ok(())
}

#[test]
fn codegraph_wrapper_under_non_codexy_rust_host_uses_runtime_dir_not_cargo()
-> Result<(), Box<dyn std::error::Error>> {
    let installed_plugin = installed_plugin_under_rust_host()?;
    let runtime_dir = temp_runtime_dir(
        "codexy-mcp-codegraph-linux-x86_64.bin",
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
    )?;
    let mut command = Command::new(installed_plugin.path.join("mcp/codexy-mcp-codegraph"));
    command
        .current_dir(&installed_plugin.path)
        .env("PATH", "/usr/bin:/bin")
        .env("CODEXY_RUNTIME_PLATFORM", "linux-x86_64")
        .env("CODEXY_RUNTIME_DIR", &runtime_dir.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut client = McpClient::spawn_command(command)?;
    let init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-codegraph");
    Ok(())
}

#[test]
fn lsp_wrapper_under_non_codexy_rust_host_uses_runtime_dir_not_cargo()
-> Result<(), Box<dyn std::error::Error>> {
    let installed_plugin = installed_plugin_under_rust_host()?;
    let runtime_dir = temp_runtime_dir(
        "codexy-mcp-lsp-linux-x86_64.bin",
        env!("CARGO_BIN_EXE_codexy-mcp-lsp"),
    )?;
    let mut command = Command::new(installed_plugin.path.join("mcp/codexy-mcp-lsp"));
    command
        .current_dir(&installed_plugin.path)
        .env("PATH", "/usr/bin:/bin")
        .env("CODEXY_RUNTIME_PLATFORM", "linux-x86_64")
        .env("CODEXY_RUNTIME_DIR", &runtime_dir.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut client = McpClient::spawn_command(command)?;
    let init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-lsp");
    Ok(())
}

#[test]
fn codegraph_wrapper_in_source_checkout_prefers_runtime_dir_over_cargo()
-> Result<(), Box<dyn std::error::Error>> {
    let runtime_dir = temp_runtime_dir(
        "codexy-mcp-codegraph-linux-x86_64.bin",
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
    )?;
    let mut command = Command::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/mcp/codexy-mcp-codegraph"),
    );
    command
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"))
        .env("PATH", "/usr/bin:/bin")
        .env("CODEXY_RUNTIME_PLATFORM", "linux-x86_64")
        .env("CODEXY_RUNTIME_DIR", &runtime_dir.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut client = McpClient::spawn_command(command)?;
    let init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-codegraph");
    Ok(())
}

#[test]
fn lsp_wrapper_in_source_checkout_prefers_runtime_dir_over_cargo()
-> Result<(), Box<dyn std::error::Error>> {
    let runtime_dir = temp_runtime_dir(
        "codexy-mcp-lsp-linux-x86_64.bin",
        env!("CARGO_BIN_EXE_codexy-mcp-lsp"),
    )?;
    let mut command = Command::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy/mcp/codexy-mcp-lsp"),
    );
    command
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"))
        .env("PATH", "/usr/bin:/bin")
        .env("CODEXY_RUNTIME_PLATFORM", "linux-x86_64")
        .env("CODEXY_RUNTIME_DIR", &runtime_dir.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut client = McpClient::spawn_command(command)?;
    let init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-lsp");
    Ok(())
}

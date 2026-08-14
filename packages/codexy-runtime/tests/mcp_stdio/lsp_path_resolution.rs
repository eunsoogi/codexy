use super::*;
use std::ffi::OsString;

#[test]
fn lsp_path_commands_bind_root_relative_and_direct_paths_before_path_lookup()
-> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    let workspace = repository.path().join("workspace");
    let source = workspace.join("sample.toml");
    let expected = install_fake_lsp(&workspace.join("servers"), "root-lsp")?;
    let competing = install_fake_lsp(&repository.path().join("ambient/servers"), "root-lsp")?;
    std::fs::create_dir_all(&workspace)?;
    std::fs::write(&source, "value = 1\n")?;
    let process_path = prepend_path(
        repository.path().join("ambient").as_path(),
        &std::env::var_os("PATH").ok_or("PATH is missing")?,
    )?;
    let root_relative = expected
        .strip_prefix(&workspace)?
        .to_string_lossy()
        .replace('\\', "/");

    for command in [
        vec![root_relative, "--root-relative".to_owned()],
        vec![expected.display().to_string(), "--direct".to_owned()],
    ] {
        run_path_command(
            repository.path(),
            &workspace,
            command,
            None,
            process_path.clone(),
            &expected,
        )?;
    }
    assert_ne!(expected.canonicalize()?, competing.canonicalize()?);
    Ok(())
}

#[test]
fn lsp_lookup_makes_relative_search_entries_absolute_before_workspace_launch()
-> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    let workspace = repository.path().join("workspace");
    let source = workspace.join("sample.toml");
    let expected = install_fake_lsp(&repository.path().join("lookup"), "relative-lsp")?;
    std::fs::create_dir_all(&workspace)?;
    std::fs::write(&source, "value = 1\n")?;

    run_path_command(
        repository.path(),
        &workspace,
        vec!["relative-lsp".to_owned(), "--relative-lookup".to_owned()],
        Some(OsString::from("lookup")),
        std::env::var_os("PATH").ok_or("PATH is missing")?,
        &expected,
    )
}

fn run_path_command(
    mcp_current_dir: &Path,
    workspace: &Path,
    server_command: Vec<String>,
    lookup_path: Option<OsString>,
    process_path: OsString,
    expected: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let capture = workspace.join(format!("{}-capture.json", server_command[1]));
    let mut command = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-lsp"));
    command
        .current_dir(mcp_current_dir)
        .env("CODEXY_LSP_ALLOW_COMMAND_OVERRIDE", "1")
        .env("CODEXY_FAKE_LSP_CAPTURE", &capture)
        .env("PATH", &process_path);
    if let Some(lookup_path) = lookup_path {
        command.env("CODEXY_LSP_LOOKUP_PATH", lookup_path);
    } else {
        command.env_remove("CODEXY_LSP_LOOKUP_PATH");
    }
    let mut client = McpClient::spawn_command(command)?;
    let _init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    let server = json!({"id":"taplo","command":server_command});
    let status = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_status","arguments":{"root":workspace,"path":"sample.toml","server":server}}
    }))?;
    let status: Value = serde_json::from_str(status["result"]["content"][0]["text"].as_str().ok_or("status text")?)?;
    assert_eq!(status["available"], true);
    assert_path_identity(
        Path::new(status["server"]["resolvedExecutable"].as_str().ok_or("resolved executable")?),
        expected,
    )?;
    let diagnostics = client.send(&json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"lsp_diagnostics","arguments":{"root":workspace,"path":"sample.toml","server":server,"timeoutMs":5000}}
    }))?;
    let diagnostics: Value = serde_json::from_str(diagnostics["result"]["content"][0]["text"].as_str().ok_or("diagnostics text")?)?;
    assert_eq!(diagnostics["status"], "ok");
    let capture: Value = serde_json::from_str(&std::fs::read_to_string(capture)?)?;
    assert_path_identity(
        Path::new(capture["currentExecutable"].as_str().ok_or("captured executable")?),
        expected,
    )
}

fn install_fake_lsp(directory: &Path, name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(directory)?;
    let source = PathBuf::from(env!("CARGO_BIN_EXE_codexy-fake-lsp"));
    let mut target = directory.join(name);
    if let Some(extension) = source.extension() {
        target.set_extension(extension);
    }
    std::fs::copy(&source, &target)?;
    std::fs::set_permissions(&target, std::fs::metadata(source)?.permissions())?;
    Ok(target)
}

fn prepend_path(directory: &Path, inherited: &OsString) -> Result<OsString, Box<dyn std::error::Error>> {
    Ok(std::env::join_paths(
        std::iter::once(directory.to_path_buf()).chain(std::env::split_paths(inherited)),
    )?)
}

fn assert_path_identity(left: &Path, right: &Path) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(left.canonicalize()?, right.canonicalize()?);
    Ok(())
}

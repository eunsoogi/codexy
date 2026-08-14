use super::*;
use std::ffi::OsString;

#[test]
fn lsp_lookup_launches_the_native_command_admitted_by_default_and_scoped_resolution()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let source = root.path().join("sample.toml");
    std::fs::write(&source, "value = 1\n")?;
    let default_dir = root.path().join("default-\u{c720}\u{b2c8}\u{d2f0}\u{c5b4}");
    let scoped_dir = root.path().join("scoped-\u{c720}\u{b2c8}\u{d2f0}\u{c5b4}");
    let competing_dir = root.path().join("competing-\u{c720}\u{b2c8}\u{d2f0}\u{c5b4}");
    std::fs::create_dir_all(&default_dir)?;
    std::fs::create_dir_all(&scoped_dir)?;
    std::fs::create_dir_all(&competing_dir)?;

    let default = install_fake_lsp(&default_dir, "default-lsp")?;
    let scoped = install_fake_lsp(&scoped_dir, "scoped-lsp")?;
    let competing = install_fake_lsp(&competing_dir, "scoped-lsp")?;
    let inherited_path = std::env::var_os("PATH").ok_or("PATH is missing")?;

    assert_lookup_launch_identity(
        root.path(),
        "default-lsp",
        None,
        prepend_path(&default_dir, &inherited_path)?,
        &default,
    )?;
    assert_lookup_launch_identity(
        root.path(),
        "scoped-lsp",
        Some(&scoped_dir),
        prepend_path(&competing_dir, &inherited_path)?,
        &scoped,
    )?;
    assert_ne!(scoped.canonicalize()?, competing.canonicalize()?);
    Ok(())
}

fn assert_lookup_launch_identity(
    root: &Path,
    executable: &str,
    lookup_path: Option<&Path>,
    process_path: OsString,
    expected: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let capture = root.join(format!("{executable}-capture.json"));
    let mut command = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-lsp"));
    command
        .env("CODEXY_LSP_ALLOW_COMMAND_OVERRIDE", "1")
        .env("CODEXY_FAKE_LSP_CAPTURE", &capture)
        .env("PATH", &process_path);
    match lookup_path {
        Some(path) => {
            command.env("CODEXY_LSP_LOOKUP_PATH", path);
        }
        None => {
            command.env_remove("CODEXY_LSP_LOOKUP_PATH");
        }
    }
    let mut client = McpClient::spawn_command(command)?;
    let _init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    let server = json!({"id":"taplo","command":[executable, "--identity", "two words"]});
    let status = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_status","arguments":{"root":root,"path":"sample.toml","server":server}}
    }))?;
    let status: Value = serde_json::from_str(status["result"]["content"][0]["text"].as_str().ok_or("status text")?)?;
    assert_eq!(status["available"], true);
    assert_path_identity(
        Path::new(
            status["server"]["resolvedExecutable"]
                .as_str()
                .ok_or("resolved executable")?,
        ),
        expected,
    )?;
    let diagnostics = client.send(&json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"lsp_diagnostics","arguments":{"root":root,"path":"sample.toml","server":server,"timeoutMs":5000}}
    }))?;
    let diagnostics: Value = serde_json::from_str(diagnostics["result"]["content"][0]["text"].as_str().ok_or("diagnostics text")?)?;
    assert_eq!(diagnostics["status"], "ok");
    let capture: Value = serde_json::from_str(&std::fs::read_to_string(capture)?)?;
    let argv = capture["argv"].as_array().ok_or("captured argv")?;
    assert_path_identity(Path::new(argv[0].as_str().ok_or("captured argv0")?), expected)?;
    assert_path_identity(
        Path::new(capture["currentExecutable"].as_str().ok_or("captured executable")?),
        expected,
    )?;
    assert_eq!(
        capture["path"].as_str().ok_or("captured PATH")?,
        process_path.to_string_lossy()
    );
    assert_eq!(argv[1], "--identity");
    assert_eq!(argv[2], "two words");
    Ok(())
}

fn install_fake_lsp(directory: &Path, name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
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

use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn source_and_version_checks_accept_both_core_mcp_shapes() -> TestResult {
    let temp = tempfile::tempdir()?;
    let (root, _) = super::selected_fixture(&temp, "mcp-shapes")?;

    assert_valid_package(&root, "legacy core")?;

    set_core_declaration(&root, None)?;
    fs::remove_file(root.join("plugins/codexy/.mcp.json"))?;
    assert_valid_package(&root, "intentional no-MCP core")?;
    Ok(())
}

#[test]
fn source_and_version_checks_reject_partial_or_invalid_core_declarations() -> TestResult {
    let temp = tempfile::tempdir()?;
    let (root, _) = super::selected_fixture(&temp, "mcp-invalid-core")?;

    // A config left behind after removing the manifest declaration is not a valid package.
    set_core_declaration(&root, None)?;
    assert_invalid_core(&root, "residual core MCP config")?;

    // Invalid types and declared-but-missing files cannot be interpreted as no MCP.
    let (root, _) = super::selected_fixture(&temp, "mcp-invalid-core-declaration")?;
    set_core_declaration(&root, Some(Value::Null))?;
    assert_invalid_core(&root, "null core MCP declaration")?;

    let (root, _) = super::selected_fixture(&temp, "mcp-missing-core-config")?;
    fs::remove_file(root.join("plugins/codexy/.mcp.json"))?;
    assert_invalid_core(&root, "missing declared core MCP config")?;
    Ok(())
}

#[test]
fn source_and_version_checks_reject_core_path_escape_and_missing_devtools() -> TestResult {
    let temp = tempfile::tempdir()?;
    let (root, _) = super::selected_fixture(&temp, "mcp-path-escape")?;
    fs::write(
        root.join("plugins/outside-mcp.json"),
        fs::read(root.join("plugins/codexy/.mcp.json"))?,
    )?;
    set_core_declaration(&root, Some(json!("../outside-mcp.json")))?;
    assert_invalid_core(&root, "core MCP path escape")?;

    let (root, _) = super::selected_fixture(&temp, "mcp-missing-devtools")?;
    set_core_declaration(&root, None)?;
    fs::remove_file(root.join("plugins/codexy/.mcp.json"))?;
    fs::remove_file(root.join("plugins/codexy-devtools/.mcp.json"))?;
    assert!(
        !run_source_check(&root, "codexy-devtools")?.status.success(),
        "source validator accepted missing devtools MCP registration"
    );
    assert!(
        !super::run_sync(&root, &["--check"])?.status.success(),
        "version check accepted missing devtools MCP registration"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn source_and_version_checks_reject_an_escaping_core_mcp_symlink() -> TestResult {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir()?;
    let (root, _) = super::selected_fixture(&temp, "mcp-symlink")?;
    let outside = root.join("plugins/outside-mcp.json");
    fs::write(
        &outside,
        fs::read(root.join("plugins/codexy/.mcp.json"))?,
    )?;
    fs::remove_file(root.join("plugins/codexy/.mcp.json"))?;
    symlink(&outside, root.join("plugins/codexy/.mcp.json"))?;
    assert_invalid_core(&root, "core MCP symlink escape")?;
    Ok(())
}

fn assert_valid_package(root: &Path, label: &str) -> TestResult {
    let version = super::run_sync(root, &["--check"])?;
    assert!(
        version.status.success(),
        "{label}: version check failed: {}",
        String::from_utf8_lossy(&version.stderr)
    );
    for plugin in ["codexy", "codexy-devtools"] {
        let source = run_source_check(root, plugin)?;
        assert!(
            source.status.success(),
            "{label}: source validation failed for {plugin}: {}",
            String::from_utf8_lossy(&source.stderr)
        );
    }
    Ok(())
}

fn assert_invalid_core(root: &Path, label: &str) -> TestResult {
    let version = super::run_sync(root, &["--check"])?;
    assert!(
        !version.status.success(),
        "{label}: version check unexpectedly passed"
    );
    let core = run_source_check(root, "codexy")?;
    assert!(
        !core.status.success(),
        "{label}: source core validation unexpectedly passed"
    );
    let devtools = run_source_check(root, "codexy-devtools")?;
    assert!(
        devtools.status.success(),
        "{label}: unchanged devtools registration failed: {}",
        String::from_utf8_lossy(&devtools.stderr)
    );
    Ok(())
}

fn run_source_check(root: &Path, plugin: &str) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            root.join("plugins").join(plugin).to_str().ok_or("plugin root")?,
            "--check",
        ])
        .current_dir(root)
        .output()?)
}

fn set_core_declaration(root: &Path, declaration: Option<Value>) -> TestResult {
    let manifest_path = root.join("plugins/codexy/.codex-plugin/plugin.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    let object = manifest.as_object_mut().ok_or("core manifest object")?;
    match declaration {
        Some(value) => {
            object.insert("mcpServers".to_owned(), value);
        }
        None => {
            object.remove("mcpServers");
        }
    }
    fs::write(
        manifest_path,
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    Ok(())
}

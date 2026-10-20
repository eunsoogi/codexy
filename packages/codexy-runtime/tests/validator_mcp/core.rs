use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_accepts_legacy_and_intentional_no_mcp_core_shapes() -> TestResult {
    let (_temp, plugin_root) = core_fixture()?;

    for mode in ["--check-mcp", "--check"] {
        let legacy = super::validator(&plugin_root, mode)?;
        assert!(
            legacy.status.success(),
            "legacy core contract failed in {mode}: {}",
            super::stderr(&legacy)
        );
    }

    // Missing mcpServers is the no-MCP declaration; a leftover config makes it ambiguous.
    let mut manifest = read_manifest(&plugin_root)?;
    manifest
        .as_object_mut()
        .ok_or("core manifest object")?
        .remove("mcpServers");
    write_manifest(&plugin_root, &manifest)?;
    fs::remove_file(plugin_root.join(".mcp.json"))?;

    for mode in ["--check-mcp", "--check"] {
        let no_mcp = super::validator(&plugin_root, mode)?;
        assert!(
            no_mcp.status.success(),
            "intentional no-MCP core contract failed in {mode}: {}",
            super::stderr(&no_mcp)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_partial_and_invalid_core_mcp_declarations() -> TestResult {
    let (_temp, plugin_root) = core_fixture()?;
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let original = fs::read(&manifest_path)?;
    let mut manifest: Value = serde_json::from_slice(&original)?;

    // An omitted declaration is valid only when the package also omits the config.
    manifest
        .as_object_mut()
        .ok_or("core manifest object")?
        .remove("mcpServers");
    write_manifest(&plugin_root, &manifest)?;
    let residual = super::validator(&plugin_root, "--check-mcp")?;
    assert!(!residual.status.success(), "residual core config unexpectedly passed");

    // A declared legacy path cannot be accepted when its file is missing.
    fs::write(&manifest_path, &original)?;
    fs::remove_file(plugin_root.join(".mcp.json"))?;
    let missing = super::validator(&plugin_root, "--check-mcp")?;
    assert!(!missing.status.success(), "missing legacy core config unexpectedly passed");

    // An invalid alias cannot disappear behind a healthy sibling Devtools package.
    manifest = serde_json::from_slice(&original)?;
    let object = manifest.as_object_mut().ok_or("core manifest object")?;
    object.remove("mcpServers");
    object.insert("mcp_servers".to_owned(), json!({"watcher": {}}));
    write_manifest(&plugin_root, &manifest)?;
    let alias = super::validator(&plugin_root, "--check-mcp")?;
    assert!(!alias.status.success(), "invalid mcp_servers alias unexpectedly passed");

    // Null is malformed metadata, not an alternate spelling of no MCP servers.
    manifest = serde_json::from_slice(&original)?;
    manifest["mcpServers"] = Value::Null;
    write_manifest(&plugin_root, &manifest)?;
    let invalid = super::validator(&plugin_root, "--check")?;
    assert!(!invalid.status.success(), "null core MCP declaration unexpectedly passed");
    Ok(())
}

#[test]
fn validator_rejects_core_mcp_paths_that_escape_the_plugin() -> TestResult {
    let (temp, plugin_root) = core_fixture()?;
    let outside = temp.path().join("outside-mcp.json");
    fs::write(
        &outside,
        fs::read(plugin_root.join(".mcp.json"))?,
    )?;
    let mut manifest = read_manifest(&plugin_root)?;
    manifest["mcpServers"] = json!("../outside-mcp.json");
    write_manifest(&plugin_root, &manifest)?;

    let output = super::validator(&plugin_root, "--check-mcp")?;
    assert!(
        !output.status.success(),
        "core MCP config path escaped its plugin root"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn validator_rejects_core_mcp_and_launcher_symlinks() -> TestResult {
    use std::os::unix::fs::symlink;

    let (temp, plugin_root) = core_fixture()?;
    let outside = temp.path().join("outside-mcp.json");
    fs::write(&outside, fs::read(plugin_root.join(".mcp.json"))?)?;
    fs::remove_file(plugin_root.join(".mcp.json"))?;
    symlink(&outside, plugin_root.join(".mcp.json"))?;
    let config_link = super::validator(&plugin_root, "--check-mcp")?;
    assert!(
        !config_link.status.success(),
        "core MCP config symlink escaped its plugin root"
    );

    fs::remove_file(plugin_root.join(".mcp.json"))?;
    fs::write(
        plugin_root.join(".mcp.json"),
        fs::read(temp.path().join("outside-mcp.json"))?,
    )?;
    let launcher = plugin_root.join("mcp/codexy-mcp-watcher.sh");
    let launcher_target = temp.path().join("watcher-launcher.sh");
    fs::copy(&launcher, &launcher_target)?;
    fs::remove_file(&launcher)?;
    symlink(&launcher_target, &launcher)?;
    let launcher_link = super::validator(&plugin_root, "--check-mcp")?;
    assert!(
        !launcher_link.status.success(),
        "core watcher launcher symlink unexpectedly passed"
    );
    Ok(())
}

#[test]
fn validator_keeps_non_core_mcp_declarations_and_devtools_servers_required() -> TestResult {
    let (_temp, core_root) = core_fixture()?;
    let mut core_manifest = read_manifest(&core_root)?;
    core_manifest["name"] = json!("third-party-mcp");
    core_manifest
        .as_object_mut()
        .ok_or("core manifest object")?
        .remove("mcpServers");
    write_manifest(&core_root, &core_manifest)?;
    fs::remove_file(core_root.join(".mcp.json"))?;
    let arbitrary = super::validator(&core_root, "--check")?;
    assert!(
        !arbitrary.status.success(),
        "arbitrary plugin without an MCP declaration was accepted"
    );

    let temp = tempfile::tempdir()?;
    let devtools_root = temp.path().join("codexy-devtools");
    support::copy_devtools_fixture_into_with_mutable_files(
        &devtools_root,
        &[Path::new(".mcp.json")],
    )?;
    let mcp_path = devtools_root.join(".mcp.json");
    let mut devtools_mcp: Value = serde_json::from_slice(&fs::read(&mcp_path)?)?;
    devtools_mcp
        .as_object_mut()
        .ok_or("devtools MCP object")?
        .remove("codegraph");
    fs::write(&mcp_path, serde_json::to_vec_pretty(&devtools_mcp)?)?;
    let missing_server = super::validator(&devtools_root, "--check-mcp")?;
    assert!(
        !missing_server.status.success(),
        "devtools without Codegraph was accepted"
    );
    Ok(())
}

fn read_manifest(plugin_root: &Path) -> TestResult<Value> {
    Ok(serde_json::from_slice(&fs::read(
        plugin_root.join(".codex-plugin/plugin.json"),
    )?)?)
}

fn core_fixture() -> TestResult<(tempfile::TempDir, PathBuf)> {
    let mutable_files = [
        Path::new(".codex-plugin/plugin.json"),
        Path::new(".mcp.json"),
        Path::new("mcp/codexy-mcp-watcher.sh"),
    ];
    // Windows fixture copies are read-only unless every test-mutated file is declared here.
    let (temp, plugin_root) = support::copy_plugin_fixture_with_mutable_files(&mutable_files)?;
    let devtools_root = plugin_root
        .parent()
        .ok_or("core fixture parent")?
        .join("codexy-devtools");
    support::copy_devtools_fixture_into_with_mutable_files(&devtools_root, &[])?;
    // These copies must retain distinct package identities on Windows, where seed trees are reused.
    assert_eq!(read_manifest(&plugin_root)?["name"], "codexy");
    assert_eq!(read_manifest(&devtools_root)?["name"], "codexy-devtools");
    Ok((temp, plugin_root))
}

fn write_manifest(plugin_root: &Path, manifest: &Value) -> TestResult {
    fs::write(
        plugin_root.join(".codex-plugin/plugin.json"),
        serde_json::to_vec_pretty(manifest)?,
    )?;
    Ok(())
}

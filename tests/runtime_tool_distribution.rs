use std::path::Path;

use serde_json::Value;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn plugin_version() -> Result<String, Box<dyn std::error::Error>> {
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(
        repo_root().join("plugins/codexy/.codex-plugin/plugin.json"),
    )?)?;
    Ok(manifest["version"]
        .as_str()
        .ok_or("plugin version must be a string")?
        .to_owned())
}

#[test]
fn runtime_tool_is_a_version_pinned_uvx_distribution() -> Result<(), Box<dyn std::error::Error>> {
    let version = plugin_version()?;
    let pyproject =
        std::fs::read_to_string(repo_root().join("packages/codexy-runtime-tools/pyproject.toml"))?;

    assert!(pyproject.contains("name = \"codexy-runtime-tools\""));
    assert!(pyproject.contains(&format!("version = \"{version}\"")));
    assert!(pyproject.contains("codexy-mcp-runtime = \"codexy_runtime_tools.runtime:main\""));
    assert!(pyproject.contains("codexy-update = \"codexy_runtime_tools.updater:main\""));
    assert!(pyproject.contains("codexy-hook-policy = \"codexy_runtime_tools.hook_policy:main\""));
    Ok(())
}

#[test]
fn runtime_distribution_exports_the_downstream_update_api() -> Result<(), Box<dyn std::error::Error>>
{
    let updater = std::fs::read_to_string(
        repo_root().join("packages/codexy-runtime-tools/src/codexy_runtime_tools/updater.py"),
    )?;
    assert!(updater.contains("def sync_agents("));
    assert!(updater.contains("class SyncResult"));
    assert!(updater.contains("FILE_ATTRIBUTE_REPARSE_POINT"));
    Ok(())
}

#[test]
fn mcp_wrappers_are_thin_pinned_uvx_entrypoints() -> Result<(), Box<dyn std::error::Error>> {
    let version = plugin_version()?;
    for server in ["lsp", "codegraph"] {
        let wrapper = std::fs::read_to_string(
            repo_root().join(format!("plugins/codexy/mcp/codexy-mcp-{server}")),
        )?;
        assert!(wrapper.contains("command -v uvx"));
        assert!(wrapper.contains(&format!("codexy-runtime-tools=={version}")));
        assert!(wrapper.contains(&format!("codexy-mcp-runtime {server}")));
        assert!(wrapper.contains("--no-config"));
        assert!(wrapper.contains("--isolated"));
        assert!(wrapper.contains("CODEXY_UVX_PATH"));
        assert!(wrapper.contains("UV_DEFAULT_INDEX"));
        assert!(!wrapper.contains("dirname"));
        for forbidden in [
            "python3",
            "cargo run",
            "cargo install",
            "curl ",
            "git clone",
        ] {
            assert!(
                !wrapper.contains(forbidden),
                "{server} wrapper must not contain {forbidden:?} fallback"
            );
        }
        assert!(
            wrapper.lines().count() <= 28,
            "{server} wrapper is not thin"
        );
    }
    Ok(())
}

#[test]
fn runtime_tool_has_explicit_offline_and_override_isolation_guards()
-> Result<(), Box<dyn std::error::Error>> {
    let runtime = std::fs::read_to_string(
        repo_root().join("packages/codexy-runtime-tools/src/codexy_runtime_tools/runtime.py"),
    )?;
    let installer = std::fs::read_to_string(
        repo_root().join("packages/codexy-runtime-tools/src/codexy_runtime_tools/installer.py"),
    )?;
    assert!(runtime.contains("UV_OFFLINE"));
    assert!(runtime.contains("CODEXY_RUNTIME_GIT_FALLBACK"));
    assert!(installer.contains("--rev"));
    assert!(!installer.contains("\"--tag\""));
    for source_component in ["package_path", "package_url", "artifacts_api"] {
        assert!(
            runtime.contains(&format!("source_components")) && runtime.contains(source_component),
            "override cache identity must include {source_component}"
        );
    }
    Ok(())
}

#[test]
fn retained_python_runtime_is_only_shipped_in_the_uvx_package()
-> Result<(), Box<dyn std::error::Error>> {
    assert!(
        !repo_root()
            .join("plugins/codexy/mcp/codexy-runtime-cache-key.py")
            .exists()
    );
    assert!(
        repo_root()
            .join("packages/codexy-runtime-tools/src/codexy_runtime_tools/runtime.py")
            .is_file()
    );
    Ok(())
}

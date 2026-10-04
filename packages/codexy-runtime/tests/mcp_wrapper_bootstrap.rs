use crate::support;

use crate::support::FixtureCommand as Command;
use std::path::Path;

use support::{WrapperFixture, make_executable, run_wrapper_command};

fn install_fake_uvx(
    bin: &Path,
    log: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let uvx = bin.join("uvx");
    std::fs::write(
        &uvx,
        format!(
            "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$@\" > '{}'\n",
            log.display()
        ),
    )?;
    make_executable(&uvx)?;
    Ok(())
}

fn selected_runtime_version() -> Result<String, Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    // Read the release contract so wrapper expectations follow the runtime version
    // selected for publication.
    let contract: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        root.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    let tag = contract["runtime"]["selectedTag"]
        .as_str()
        .ok_or("selected runtime tag must be a string")?;
    Ok(tag
        .strip_prefix('v')
        .ok_or("selected runtime tag must start with v")?
        .to_owned())
}

#[test]
fn wrappers_dispatch_only_the_pinned_uvx_contract() -> Result<(), Box<dyn std::error::Error>> {
    let server = "lsp";
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let log = temp.path().join("uvx-args.log");
    install_fake_uvx(&fixture.cargo_bin, &log)?;

    let mut command = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")));
    command
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", fixture.cargo_bin.display()),
        )
        .env("CODEXY_RUNTIME_PLATFORM", "linux-x86_64")
        // Spaced values and a delimiter-like argument exercise exact argv forwarding.
        .args(["--stdio", "value with spaces", "--literal=--"]);
    assert!(run_wrapper_command(&mut command)?.status.success());
    let plugin_root = support::fixture_path_text(&fixture.plugin_root)?;
    let selected_version = selected_runtime_version()?;
    assert_eq!(
        std::fs::read_to_string(log)?
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        vec![
            "--from".to_owned(),
            format!("getcodexy=={selected_version}"),
            "codexy-mcp-runtime".to_owned(),
            server.to_owned(),
            "--plugin-root".to_owned(),
            plugin_root,
            "--".to_owned(),
            "--stdio".to_owned(),
            "value with spaces".to_owned(),
            "--literal=--".to_owned(),
        ]
    );
    Ok(())
}

#[test]
fn wrappers_report_missing_uvx() -> Result<(), Box<dyn std::error::Error>> {
    let server = "lsp";
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
        .env("PATH", "/usr/bin:/bin")
        .env("CODEXY_RUNTIME_PLATFORM", "linux-x86_64")
        .arg("--stdio")
        .output()?;
    assert_eq!(output.status.code(), Some(127));
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires uvx"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn core_source_launcher_bootstraps_watcher_without_bundled_runtime()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("core plugin");
    support::copy_dir(
        codexy_runtime::paths::repository_root().join("plugins/codexy"),
        &plugin_root,
    )?;
    let fake_bin = temp.path().join("fake-bin");
    std::fs::create_dir_all(&fake_bin)?;
    let log = temp.path().join("core-uvx-args.log");
    install_fake_uvx(&fake_bin, &log)?;

    let mut command = Command::new(plugin_root.join("mcp/codexy-mcp-watcher.sh"));
    command
        .env("PATH", format!("{}:/usr/bin:/bin", fake_bin.display()))
        // Removing the bundle override exercises the source checkout's uvx fallback.
        .env_remove("CODEXY_RUNTIME_DIR")
        .args(["--stdio", "value with spaces", "--literal=--"]);
    assert!(run_wrapper_command(&mut command)?.status.success());
    let plugin_root = support::fixture_path_text(&plugin_root)?;
    let selected_version = selected_runtime_version()?;
    assert_eq!(
        std::fs::read_to_string(log)?.lines().map(str::to_owned).collect::<Vec<_>>(),
        vec![
            "--from".to_owned(),
            format!("getcodexy=={selected_version}"),
            "codexy-mcp-runtime".to_owned(),
            "watcher".to_owned(),
            "--plugin-root".to_owned(),
            plugin_root,
            "--".to_owned(),
            "value with spaces".to_owned(),
            "--literal=--".to_owned(),
        ]
    );
    Ok(())
}

#[test]
fn core_windows_launcher_keeps_the_same_bootstrap_contract()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let launcher = std::fs::read_to_string(root.join("plugins/codexy/mcp/codexy-mcp-watcher.cmd"))?;
    let version = selected_runtime_version()?;
    for expected in [
        "codexy-mcp-watcher-windows-x86_64.exe",
        "uvx --from",
        &format!("getcodexy=={version}"),
        "codexy-mcp-runtime watcher",
    ] {
        assert!(launcher.contains(expected), "launcher omitted {expected:?}");
    }
    Ok(())
}

#[test]
fn core_mcp_validator_windows_launcher_accepts_annotations_and_rejects_comment_drift()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("core plugin");
    support::copy_dir(root.join("plugins/codexy"), &plugin_root)?;
    let manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        plugin_root.join(".codex-plugin/plugin.json"),
    )?)?;
    let version = manifest["version"].as_str().ok_or("core version must be a string")?;
    let expected = format!(
        concat!(
            "@echo off\n",
            "@rem Prefer the bundled runtime, then the checkout package, then the version-pinned release.\n",
            "@rem Return the selected launcher's exit status unchanged.\n",
            "set \"plugin_root=%~dp0..\"\n",
            "set \"bundled_runtime=%plugin_root%\\runtime\\codexy-mcp-watcher-windows-x86_64.exe\"\n",
            "if exist \"%bundled_runtime%\" goto bundled_runtime\n",
            "where uvx >nul 2>&1\n",
            "if errorlevel 1 (\n",
            "  echo codexy-mcp-watcher requires uvx on PATH; install uv or provide a bundled runtime 1>&2\n",
            "  exit /b 127\n",
            ")\n",
            "set \"repo_root=%plugin_root%\\..\\..\"\n",
            "set \"runtime_source=%repo_root%\\packages\\getcodexy\"\n",
            "if exist \"%runtime_source%\\pyproject.toml\" goto local_source\n",
            "uvx --from getcodexy=={version} codexy-mcp-runtime watcher --plugin-root \"%plugin_root%\" -- %*\n",
            "exit /b %ERRORLEVEL%\n\n",
            ":local_source\n",
            "uvx --from \"%runtime_source%\" codexy-mcp-runtime watcher --plugin-root \"%plugin_root%\" -- %*\n",
            "exit /b %ERRORLEVEL%\n\n",
            ":bundled_runtime\n",
            "\"%bundled_runtime%\" %*\n",
            "exit /b %ERRORLEVEL%\n"
        ),
        version = version
    );
    let launcher = plugin_root.join("mcp/codexy-mcp-watcher.cmd");
    std::fs::write(&launcher, &expected)?;

    let validate = || {
        std::process::Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
            .arg("--plugin-root")
            .arg(&plugin_root)
            .arg("--check-mcp")
            .output()
    };
    let accepted = validate()?;
    assert!(
        accepted.status.success(),
        "annotated watcher launcher must pass: {}",
        String::from_utf8_lossy(&accepted.stderr)
    );

    let mutated = expected.replace(
        "@rem Prefer the bundled runtime, then the checkout package, then the version-pinned release.",
        "@rem Prefer the bundled runtime.",
    );
    assert_ne!(mutated, expected, "expected template must include its required comment");
    std::fs::write(&launcher, mutated)?;
    let rejected = validate()?;
    assert!(!rejected.status.success(), "comment drift must reject the launcher");
    assert!(
        String::from_utf8_lossy(&rejected.stderr)
            .contains("core watcher Windows launcher must preserve the bundled and source bootstrap contract"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&rejected.stderr)
    );
    Ok(())
}

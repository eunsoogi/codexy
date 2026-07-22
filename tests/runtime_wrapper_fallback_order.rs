use crate::support;

use std::process::Command;
use std::time::Duration;

use support::{WrapperFixture, run_wrapper_command_with_timeout};

#[test]
fn mcp_wrappers_try_bundled_runtime_before_pinned_uvx_bootstrap()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        let wrapper_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/mcp")
            .join(format!("codexy-mcp-{server}"));
        let wrapper = std::fs::read_to_string(&wrapper_path)?;

        assert_bundled_runtime_precedes_uvx(&wrapper, &wrapper_path)?;
    }
    Ok(())
}

#[test]
fn wrapper_subprocess_timeout_is_actionable() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    fixture.replace_wrapper("lsp", "#!/bin/sh\nexec sleep 45\n")?;

    let mut command = Command::new(fixture.plugin_root.join("mcp/codexy-mcp-lsp"));
    let error = run_wrapper_command_with_timeout(&mut command, Duration::from_secs(2))
        .expect_err("wrapper subprocess must time out instead of blocking the test harness");
    let message = error.to_string();
    assert!(
        message.contains("timed out"),
        "timeout should be actionable: {message}"
    );
    assert!(
        message.contains("codexy-mcp-lsp"),
        "timeout should name the wrapper: {message}"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn wrapper_timeout_kills_non_exec_descendant_with_inherited_output()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let marker = format!("codexy-wrapper-descendant-{}", std::process::id());
    fixture.replace_wrapper(
        "lsp",
        &format!("#!/bin/sh\nsh -c 'sleep 4 & wait' {marker} &\nwait\n"),
    )?;

    let mut command = Command::new(fixture.plugin_root.join("mcp/codexy-mcp-lsp"));
    let started = std::time::Instant::now();
    let error = run_wrapper_command_with_timeout(&mut command, Duration::from_secs(1))
        .expect_err("wrapper subprocess must time out instead of blocking the test harness");
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "timeout cleanup waited for a non-exec descendant: {error}"
    );
    assert!(
        error.to_string().contains("timed out"),
        "timeout should be actionable: {error}"
    );
    assert!(
        matching_pids(&marker)?.is_empty(),
        "timeout cleanup left a descendant process behind"
    );
    Ok(())
}

#[cfg(unix)]
fn matching_pids(marker: &str) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    let output = Command::new("pgrep").args(["-f", marker]).output()?;
    if output.status.code() == Some(1) {
        return Ok(Vec::new());
    }
    if !output.status.success() {
        return Err(format!("pgrep failed for {marker:?}").into());
    }
    output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            std::str::from_utf8(line)?
                .parse::<u32>()
                .map_err(Into::into)
        })
        .collect()
}

fn assert_bundled_runtime_precedes_uvx(
    wrapper: &str,
    wrapper_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let bundled_runtime_check = find_required(
        wrapper,
        "if [ -x \"$bundled_runtime\" ]; then",
        wrapper_path,
        "bundled runtime check",
    )?;
    let uvx_check = find_required(
        wrapper,
        "if ! command -v uvx >/dev/null 2>&1; then",
        wrapper_path,
        "uvx availability check",
    )?;
    let manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/.codex-plugin/plugin.json"),
    )?)?;
    let version = manifest["version"].as_str().ok_or("manifest version")?;
    let uvx_bootstrap = find_required(
        wrapper,
        &format!("exec uvx --from getcodexy=={version} codexy-mcp-runtime"),
        wrapper_path,
        "pinned uvx bootstrap",
    )?;

    assert!(
        bundled_runtime_check < uvx_check,
        "{} should check bundled runtime before uvx",
        wrapper_path.display()
    );
    assert!(
        uvx_check < uvx_bootstrap,
        "{} should check uvx before dispatching it",
        wrapper_path.display()
    );
    for forbidden in ["python3", "codexy-runtime-cache-key.py", "cargo ", "curl "] {
        let retains_forbidden_startup = wrapper.contains(forbidden);
        assert!(
            !retains_forbidden_startup,
            "{} must not retain legacy {forbidden:?} startup logic",
            wrapper_path.display(),
        );
    }
    Ok(())
}

fn find_required(
    text: &str,
    needle: &str,
    wrapper_path: &std::path::Path,
    label: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    text.find(needle)
        .ok_or_else(|| format!("{} missing {label}: {needle}", wrapper_path.display()).into())
}

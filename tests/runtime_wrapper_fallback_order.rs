mod support;

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use support::{
    WrapperFixture, assert_wrapper_uses_package_runtime_without_cargo,
    run_wrapper_command_with_timeout,
};

#[test]
fn mcp_wrappers_try_packaged_runtime_before_cargo_bootstrap()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        let wrapper_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/mcp")
            .join(format!("codexy-mcp-{server}"));
        let wrapper = std::fs::read_to_string(&wrapper_path)?;

        assert_package_fallback_precedes_cargo_bootstrap(&wrapper, &wrapper_path)?;
    }
    Ok(())
}

#[test]
fn mcp_wrappers_use_package_runtime_without_invoking_cargo_when_package_exists()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_uses_package_runtime_without_cargo(server)?;
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

fn assert_package_fallback_precedes_cargo_bootstrap(
    wrapper: &str,
    wrapper_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let bundled_runtime_check = find_required(
        wrapper,
        "if [ -x \"$bundled_runtime\" ]; then",
        wrapper_path,
        "bundled runtime check",
    )?;
    let package_fallback = find_required(
        wrapper,
        "if [ \"$runtime_package_requested\" = 1 ]; then",
        wrapper_path,
        "package fallback",
    )?;
    let cargo_bootstrap = find_required(
        wrapper,
        "if [ \"$cargo_available\" = 1 ]; then\n  if [ \"$runtime_ref_is_pinned\" = 1 ]; then",
        wrapper_path,
        "Cargo bootstrap",
    )?;

    assert!(
        bundled_runtime_check < package_fallback,
        "{} should check bundled runtime before package fallback",
        wrapper_path.display()
    );
    assert!(
        package_fallback < cargo_bootstrap,
        "{} should try packaged runtime fallback before Cargo bootstrap",
        wrapper_path.display()
    );
    Ok(())
}

fn find_required(
    text: &str,
    needle: &str,
    wrapper_path: &Path,
    label: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    text.find(needle)
        .ok_or_else(|| format!("{} missing {label}: {needle}", wrapper_path.display()).into())
}

use crate::support::FixtureCommand as Command;
use std::time::{Duration, Instant};

use crate::support::{WrapperFixture, run_wrapper_command_with_timeout};

#[test]
fn mcp_wrappers_order_runtime_dir_then_bundled_then_pinned_uvx()
-> Result<(), Box<dyn std::error::Error>> {
    let path = codexy_runtime::paths::repository_root()
        .join("plugins/codexy-devtools/mcp/codexy-mcp-devtools");
    let wrapper = std::fs::read_to_string(&path)?;
    let override_index = required(&wrapper, "CODEXY_RUNTIME_DIR", &path)?;
    let bundled_index = required(&wrapper, "bundled_runtime=", &path)?;
    let uvx_index = required(&wrapper, "exec uvx --from getcodexy==1.2.2", &path)?;
    assert!(override_index < bundled_index && bundled_index < uvx_index);
    Ok(())
}

#[test]
fn wrapper_subprocess_timeout_is_actionable() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let marker = temp.path().join("wrapper-timeout-descendant-marker");
    fixture.replace_wrapper(
        "lsp",
        "#!/bin/sh\nsleep 45 &\n(sleep 3; printf orphan > \"$CODEXY_WRAPPER_TIMEOUT_MARKER\") &\nwait\n",
    )?;
    let mut command = Command::new(fixture.plugin_root.join("mcp/codexy-mcp-lsp"));
    command.env("CODEXY_WRAPPER_TIMEOUT_MARKER", &marker);
    let started = Instant::now();
    let error = run_wrapper_command_with_timeout(&mut command, Duration::from_secs(2))
        .expect_err("wrapper subprocess must time out");
    let elapsed = started.elapsed();
    assert!(error.to_string().contains("timed out"));
    assert!(
        elapsed < Duration::from_secs(10),
        "wrapper timeout waited for a descendant: {elapsed:?}"
    );
    std::thread::sleep(Duration::from_millis(1_500));
    assert!(
        !marker.exists(),
        "wrapper timeout left a descendant writing after reap: {}",
        marker.display()
    );
    Ok(())
}

fn required(
    text: &str,
    needle: &str,
    path: &std::path::Path,
) -> Result<usize, Box<dyn std::error::Error>> {
    text.find(needle)
        .ok_or_else(|| format!("{} missing {needle}", path.display()).into())
}
